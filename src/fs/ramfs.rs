#![allow(dead_code)]

use core::ptr::{copy_nonoverlapping, write_bytes};

use crate::fs::archive;
use crate::fs::vfs::{DirEntry, FileSystemId, FileType, FileView, InodeRef, Metadata};
use crate::kernel::memory::{self, PAGE_SIZE};
use crate::kernel::syscall::{EEXIST, EINVAL, EISDIR, ENOENT, ENOMEM, ENOSPC, ENOTEMPTY, EPERM};

const MAX_INODES: usize = 2048;
const NAME_LEN: usize = 128;
const ROOT_INO: usize = 0;
const FIRST_INO: usize = 1;

#[derive(Clone, Copy)]
struct RamNode {
    used: bool,
    parent: usize,
    name: [u8; NAME_LEN],
    name_len: usize,
    file_type: FileType,
    mode: u16,
    size: usize,
    capacity: usize,
    data: *mut u8,
    link_target: usize,
    nlink: u32,
}

impl RamNode {
    const fn empty() -> Self {
        Self {
            used: false,
            parent: ROOT_INO,
            name: [0; NAME_LEN],
            name_len: 0,
            file_type: FileType::Regular,
            mode: 0,
            size: 0,
            capacity: 0,
            data: core::ptr::null_mut(),
            link_target: 0,
            nlink: 0,
        }
    }
}

static mut NODES: [RamNode; MAX_INODES] = [const { RamNode::empty() }; MAX_INODES];
static mut REGULAR_COUNT: usize = 0;

pub fn init() {
    unsafe {
        for idx in 0..MAX_INODES {
            NODES[idx] = RamNode::empty();
        }
        REGULAR_COUNT = 0;
        NODES[ROOT_INO] = RamNode {
            used: true,
            parent: ROOT_INO,
            name: [0; NAME_LEN],
            name_len: 0,
            file_type: FileType::Directory,
            mode: 0o755,
            size: 0,
            capacity: 0,
            data: core::ptr::null_mut(),
            link_target: ROOT_INO,
            nlink: 2,
        };
    }
}

pub fn unpack_archive() -> Result<usize, u32> {
    let mut unpacked = 0usize;
    archive::for_each_entry(|entry| {
        let path = entry.name().as_bytes();
        ensure_parent_dirs(path)?;
        match entry.file_type {
            FileType::Directory => {
                let _ = mkdir(path, entry.mode);
            }
            FileType::Regular | FileType::Symlink | FileType::Device => {
                let inode = match create_node(path, entry.file_type, entry.mode, false) {
                    Ok(inode) => inode,
                    Err(EEXIST) => lookup(path).ok_or(EEXIST)?,
                    Err(err) => return Err(err),
                };
                if entry.file_type == FileType::Regular || entry.file_type == FileType::Symlink {
                    write(inode, 0, entry.data)?;
                }
            }
        }
        unpacked += 1;
        Ok(())
    })?;
    Ok(unpacked)
}

fn ensure_parent_dirs(path: &[u8]) -> Result<(), u32> {
    let path = normalize_path(path);
    let Some(last_slash) = path.iter().rposition(|byte| *byte == b'/') else {
        return Ok(());
    };

    let mut offset = 0usize;
    while offset <= last_slash {
        let Some(relative) = path[offset..=last_slash]
            .iter()
            .position(|byte| *byte == b'/')
        else {
            break;
        };
        let end = offset + relative;
        if end != 0 {
            let component_path = &path[..end];
            if lookup_path(component_path).is_none() {
                create_node(component_path, FileType::Directory, 0o755, false)?;
            }
        }
        offset = end + 1;
    }
    Ok(())
}

pub fn regular_count() -> usize {
    unsafe { REGULAR_COUNT }
}

pub fn lookup(path: &[u8]) -> Option<InodeRef> {
    let path = normalize_path(path);
    lookup_path(path).map(inode)
}

pub fn metadata(inode_ref: InodeRef) -> Option<Metadata> {
    let node = node(inode_ref)?;
    let data_node = data_node_ref(node);
    let mut meta = Metadata::new(inode_ref, node.file_type, data_node.size, node.mode);
    meta.nlink = data_node.nlink.max(1);
    Some(meta)
}

pub fn open(inode_ref: InodeRef) -> Option<FileView> {
    let idx = index(inode_ref).ok()?;
    unsafe {
        let node = &raw const NODES[idx];
        let node = &*node;
        if node.file_type != FileType::Regular && node.file_type != FileType::Symlink {
            return None;
        }
        let data_node = data_node_ref(*node);
        let name = core::str::from_utf8(&node.name[..node.name_len]).ok()?;
        let data = if data_node.size == 0 {
            &[]
        } else {
            core::slice::from_raw_parts(data_node.data as *const u8, data_node.size)
        };
        Some(FileView {
            inode: inode_ref,
            name,
            data,
        })
    }
}

pub fn create_file(path: &[u8], mode: u16) -> Result<InodeRef, u32> {
    create_node(path, FileType::Regular, mode, true)
}

pub fn mkdir(path: &[u8], mode: u16) -> Result<InodeRef, u32> {
    create_node(path, FileType::Directory, mode, false)
}

pub fn unlink(path: &[u8]) -> Result<(), u32> {
    let path = normalize_path(path);
    let idx = lookup_path(path).ok_or(ENOENT)?;
    if idx == ROOT_INO {
        return Err(EINVAL);
    }

    unsafe {
        if NODES[idx].file_type == FileType::Directory && has_children(idx) {
            return Err(ENOTEMPTY);
        }
        if NODES[idx].file_type == FileType::Directory {
            NODES[idx] = RamNode::empty();
            return Ok(());
        }

        let owner = data_index(idx);
        if owner != idx {
            NODES[owner].nlink = NODES[owner].nlink.saturating_sub(1);
            NODES[idx] = RamNode::empty();
            return Ok(());
        }

        if NODES[idx].nlink > 1 {
            if let Some(new_owner) = find_alias(idx) {
                NODES[new_owner].data = NODES[idx].data;
                NODES[new_owner].capacity = NODES[idx].capacity;
                NODES[new_owner].size = NODES[idx].size;
                NODES[new_owner].nlink = NODES[idx].nlink - 1;
                NODES[new_owner].link_target = new_owner;
                for alias in FIRST_INO..MAX_INODES {
                    if alias != idx && alias != new_owner && NODES[alias].used {
                        if NODES[alias].link_target == idx {
                            NODES[alias].link_target = new_owner;
                        }
                    }
                }
                NODES[idx].data = core::ptr::null_mut();
                NODES[idx].capacity = 0;
                NODES[idx].size = 0;
                NODES[idx] = RamNode::empty();
                return Ok(());
            }
        }

        if NODES[idx].file_type == FileType::Regular {
            REGULAR_COUNT = REGULAR_COUNT.saturating_sub(1);
        }
        if !NODES[idx].data.is_null() {
            memory::free_exact_pages(NODES[idx].data, NODES[idx].capacity / PAGE_SIZE);
        }
        NODES[idx] = RamNode::empty();
    }
    Ok(())
}

pub fn link(old_path: &[u8], new_path: &[u8]) -> Result<(), u32> {
    let old_path = normalize_path(old_path);
    let new_path = normalize_path(new_path);
    if old_path.is_empty() || new_path.is_empty() || new_path.len() > NAME_LEN {
        return Err(EINVAL);
    }
    if lookup_path(new_path).is_some() {
        return Err(EEXIST);
    }

    let old_idx = lookup_path(old_path).ok_or(ENOENT)?;
    unsafe {
        if NODES[old_idx].file_type == FileType::Directory {
            return Err(EPERM);
        }
    }
    let (parent, basename) = parent_and_basename(new_path)?;
    let slot = unsafe {
        (FIRST_INO..MAX_INODES)
            .find(|idx| !NODES[*idx].used)
            .ok_or(ENOSPC)?
    };

    unsafe {
        let owner = data_index(old_idx);
        NODES[owner].nlink = NODES[owner].nlink.saturating_add(1);
        NODES[slot] = RamNode::empty();
        NODES[slot].used = true;
        NODES[slot].parent = parent;
        NODES[slot].name[..basename.len()].copy_from_slice(basename);
        NODES[slot].name_len = basename.len();
        NODES[slot].file_type = NODES[old_idx].file_type;
        NODES[slot].mode = NODES[old_idx].mode;
        NODES[slot].link_target = owner;
    }
    Ok(())
}

pub fn rename(old_path: &[u8], new_path: &[u8]) -> Result<(), u32> {
    let old_path = normalize_path(old_path);
    let new_path = normalize_path(new_path);
    if old_path.is_empty() || new_path.is_empty() || new_path.len() > NAME_LEN {
        return Err(EINVAL);
    }

    let idx = lookup_path(old_path).ok_or(ENOENT)?;
    if idx == ROOT_INO {
        return Err(EINVAL);
    }
    if let Some(existing) = lookup_path(new_path) {
        if existing != idx {
            unlink(new_path)?;
        }
    }

    let (parent, basename) = parent_and_basename(new_path)?;
    unsafe {
        NODES[idx].parent = parent;
        NODES[idx].name = [0; NAME_LEN];
        NODES[idx].name[..basename.len()].copy_from_slice(basename);
        NODES[idx].name_len = basename.len();
    }
    Ok(())
}

pub fn read_dir(inode_ref: InodeRef, offset: usize, dst: &mut [DirEntry]) -> Result<usize, u32> {
    let idx = index(inode_ref)?;
    unsafe {
        if NODES[idx].file_type != FileType::Directory {
            return Err(EINVAL);
        }
    }

    let mut seen = 0usize;
    let mut written = 0usize;
    unsafe {
        for child in 0..MAX_INODES {
            if !NODES[child].used || (child != ROOT_INO && NODES[child].parent != idx) {
                continue;
            }
            if idx != ROOT_INO && child == idx {
                continue;
            }
            if seen < offset {
                seen += 1;
                continue;
            }
            if written >= dst.len() {
                break;
            }
            let name = if child == ROOT_INO {
                b".".as_slice()
            } else {
                &NODES[child].name[..NODES[child].name_len]
            };
            dst[written] = DirEntry::new(inode(child), NODES[child].file_type, name);
            written += 1;
            seen += 1;
        }
    }
    Ok(written)
}

pub fn truncate(inode_ref: InodeRef, size: usize) -> Result<(), u32> {
    let idx = index(inode_ref)?;
    unsafe {
        if NODES[idx].file_type == FileType::Directory {
            return Err(EISDIR);
        }
    }
    let idx = data_index(idx);
    ensure_capacity(idx, size)?;
    unsafe {
        if size > NODES[idx].size {
            write_bytes(
                NODES[idx].data.add(NODES[idx].size),
                0,
                size - NODES[idx].size,
            );
        }
        NODES[idx].size = size;
    }
    Ok(())
}

pub fn write(inode_ref: InodeRef, offset: usize, src: &[u8]) -> Result<usize, u32> {
    let idx = index(inode_ref)?;
    unsafe {
        if NODES[idx].file_type != FileType::Regular && NODES[idx].file_type != FileType::Symlink {
            return Err(EINVAL);
        }
    }
    let idx = data_index(idx);
    let end = offset.checked_add(src.len()).ok_or(ENOMEM)?;
    ensure_capacity(idx, end)?;
    unsafe {
        if offset > NODES[idx].size {
            write_bytes(
                NODES[idx].data.add(NODES[idx].size),
                0,
                offset - NODES[idx].size,
            );
        }
        copy_nonoverlapping(src.as_ptr(), NODES[idx].data.add(offset), src.len());
        NODES[idx].size = NODES[idx].size.max(end);
    }
    Ok(src.len())
}

fn create_node(
    path: &[u8],
    file_type: FileType,
    mode: u16,
    fail_if_exists: bool,
) -> Result<InodeRef, u32> {
    let path = normalize_path(path);
    if path.is_empty() || path.len() > NAME_LEN {
        return Err(EINVAL);
    }
    if let Some(existing) = lookup(path) {
        return if fail_if_exists {
            Err(EEXIST)
        } else {
            Ok(existing)
        };
    }
    let (parent, basename) = parent_and_basename(path)?;

    let slot = unsafe {
        (FIRST_INO..MAX_INODES)
            .find(|idx| !NODES[*idx].used)
            .ok_or(ENOSPC)?
    };
    unsafe {
        NODES[slot] = RamNode::empty();
        NODES[slot].used = true;
        NODES[slot].parent = parent;
        NODES[slot].name[..basename.len()].copy_from_slice(basename);
        NODES[slot].name_len = basename.len();
        NODES[slot].file_type = file_type;
        NODES[slot].mode = mode;
        NODES[slot].link_target = slot;
        NODES[slot].nlink = if file_type == FileType::Directory {
            2
        } else {
            1
        };
        if file_type == FileType::Regular {
            REGULAR_COUNT += 1;
        }
    }
    Ok(inode(slot))
}

fn lookup_path(path: &[u8]) -> Option<usize> {
    if path.is_empty() {
        return Some(ROOT_INO);
    }

    let mut current = ROOT_INO;
    let mut rest = path;
    while !rest.is_empty() {
        let (component, tail) = split_component(rest);
        if component.is_empty() || component == b"." {
            rest = tail;
            continue;
        }
        if component == b".." {
            current = unsafe { NODES[current].parent };
            rest = tail;
            continue;
        }
        let next = lookup_child(current, component)?;
        current = next;
        rest = tail;
    }
    Some(current)
}

fn parent_and_basename(path: &[u8]) -> Result<(usize, &[u8]), u32> {
    let Some(split) = path.iter().rposition(|byte| *byte == b'/') else {
        return Ok((ROOT_INO, path));
    };
    let parent_path = &path[..split];
    let basename = &path[split + 1..];
    if basename.is_empty() || basename.len() > NAME_LEN {
        return Err(EINVAL);
    }
    let parent = lookup_path(parent_path).ok_or(ENOENT)?;
    unsafe {
        if NODES[parent].file_type != FileType::Directory {
            return Err(EINVAL);
        }
    }
    Ok((parent, basename))
}

fn lookup_child(parent: usize, name: &[u8]) -> Option<usize> {
    unsafe {
        for idx in FIRST_INO..MAX_INODES {
            if !NODES[idx].used || NODES[idx].parent != parent {
                continue;
            }
            if &NODES[idx].name[..NODES[idx].name_len] == name {
                return Some(idx);
            }
        }
    }
    None
}

fn has_children(parent: usize) -> bool {
    unsafe { (FIRST_INO..MAX_INODES).any(|idx| NODES[idx].used && NODES[idx].parent == parent) }
}

fn split_component(path: &[u8]) -> (&[u8], &[u8]) {
    match path.iter().position(|byte| *byte == b'/') {
        Some(pos) => (&path[..pos], &path[pos + 1..]),
        None => (path, &[]),
    }
}

fn ensure_capacity(idx: usize, needed: usize) -> Result<(), u32> {
    unsafe {
        if needed <= NODES[idx].capacity {
            return Ok(());
        }

        let new_capacity = round_capacity(needed);
        let pages = new_capacity / PAGE_SIZE;
        let Some(new_data) = memory::alloc_exact_pages(pages) else {
            return Err(ENOMEM);
        };

        if !NODES[idx].data.is_null() && NODES[idx].size != 0 {
            copy_nonoverlapping(NODES[idx].data as *const u8, new_data, NODES[idx].size);
        }
        if new_capacity > NODES[idx].size {
            write_bytes(
                new_data.add(NODES[idx].size),
                0,
                new_capacity - NODES[idx].size,
            );
        }
        if !NODES[idx].data.is_null() {
            let old_pages = NODES[idx].capacity / PAGE_SIZE;
            memory::free_exact_pages(NODES[idx].data, old_pages);
        }
        NODES[idx].data = new_data;
        NODES[idx].capacity = new_capacity;
    }
    Ok(())
}

fn node(inode_ref: InodeRef) -> Option<RamNode> {
    let idx = index(inode_ref).ok()?;
    unsafe { Some(NODES[idx]) }
}

fn data_node_ref(node: RamNode) -> RamNode {
    unsafe {
        if node.link_target < MAX_INODES && NODES[node.link_target].used {
            NODES[node.link_target]
        } else {
            node
        }
    }
}

fn data_index(idx: usize) -> usize {
    unsafe {
        let target = NODES[idx].link_target;
        if target < MAX_INODES && NODES[target].used {
            target
        } else {
            idx
        }
    }
}

fn find_alias(target: usize) -> Option<usize> {
    unsafe {
        (FIRST_INO..MAX_INODES)
            .find(|idx| *idx != target && NODES[*idx].used && NODES[*idx].link_target == target)
    }
}

fn index(inode_ref: InodeRef) -> Result<usize, u32> {
    if inode_ref.fs != FileSystemId::Ramfs || inode_ref.ino >= MAX_INODES {
        return Err(ENOENT);
    }
    unsafe {
        if !NODES[inode_ref.ino].used {
            return Err(ENOENT);
        }
    }
    Ok(inode_ref.ino)
}

fn inode(ino: usize) -> InodeRef {
    InodeRef {
        fs: FileSystemId::Ramfs,
        ino,
    }
}

fn normalize_path(path: &[u8]) -> &[u8] {
    let path = trim_nul(path);
    let path = strip_prefix(path, b"/");
    strip_prefix(path, b"./")
}

fn strip_prefix<'a>(bytes: &'a [u8], prefix: &[u8]) -> &'a [u8] {
    if bytes.starts_with(prefix) {
        &bytes[prefix.len()..]
    } else {
        bytes
    }
}

fn trim_nul(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|byte| *byte == 0) {
        Some(end) => &bytes[..end],
        None => bytes,
    }
}

fn round_capacity(size: usize) -> usize {
    if size == 0 {
        PAGE_SIZE
    } else {
        crate::kernel::address::align_up(size, PAGE_SIZE)
    }
}
