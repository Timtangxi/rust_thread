#![allow(dead_code)]

use core::ptr::{copy_nonoverlapping, write_bytes};

use crate::fs::archive;
use crate::fs::vfs::{FileSystemId, FileType, FileView, InodeRef, Metadata};
use crate::kernel::memory::{self, PAGE_SIZE};
use crate::kernel::syscall::{EEXIST, EINVAL, ENOENT, ENOMEM, ENOSPC};

const MAX_INODES: usize = 2048;
const NAME_LEN: usize = 128;
const ROOT_INO: usize = 0;
const FIRST_INO: usize = 1;

#[derive(Clone, Copy)]
struct RamNode {
    used: bool,
    name: [u8; NAME_LEN],
    name_len: usize,
    file_type: FileType,
    mode: u16,
    size: usize,
    capacity: usize,
    data: *mut u8,
}

impl RamNode {
    const fn empty() -> Self {
        Self {
            used: false,
            name: [0; NAME_LEN],
            name_len: 0,
            file_type: FileType::Regular,
            mode: 0,
            size: 0,
            capacity: 0,
            data: core::ptr::null_mut(),
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
            name: [0; NAME_LEN],
            name_len: 0,
            file_type: FileType::Directory,
            mode: 0o755,
            size: 0,
            capacity: 0,
            data: core::ptr::null_mut(),
        };
    }
}

pub fn unpack_archive() -> Result<usize, u32> {
    let mut unpacked = 0usize;
    archive::for_each_entry(|entry| {
        let path = entry.name().as_bytes();
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

pub fn regular_count() -> usize {
    unsafe { REGULAR_COUNT }
}

pub fn lookup(path: &[u8]) -> Option<InodeRef> {
    let path = normalize_path(path);
    if path.is_empty() {
        return Some(inode(ROOT_INO));
    }

    unsafe {
        for idx in FIRST_INO..MAX_INODES {
            if !NODES[idx].used {
                continue;
            }
            if &NODES[idx].name[..NODES[idx].name_len] == path {
                return Some(inode(idx));
            }
        }
    }
    None
}

pub fn metadata(inode_ref: InodeRef) -> Option<Metadata> {
    let node = node(inode_ref)?;
    Some(Metadata {
        inode: inode_ref,
        file_type: node.file_type,
        size: node.size,
        mode: node.mode,
    })
}

pub fn open(inode_ref: InodeRef) -> Option<FileView> {
    let idx = index(inode_ref).ok()?;
    unsafe {
        let node = &raw const NODES[idx];
        let node = &*node;
        if node.file_type != FileType::Regular && node.file_type != FileType::Symlink {
            return None;
        }
        let name = core::str::from_utf8(&node.name[..node.name_len]).ok()?;
        let data = if node.size == 0 {
            &[]
        } else {
            core::slice::from_raw_parts(node.data as *const u8, node.size)
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

pub fn truncate(inode_ref: InodeRef, size: usize) -> Result<(), u32> {
    let idx = index(inode_ref)?;
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

    let slot = unsafe {
        (FIRST_INO..MAX_INODES)
            .find(|idx| !NODES[*idx].used)
            .ok_or(ENOSPC)?
    };
    unsafe {
        NODES[slot] = RamNode::empty();
        NODES[slot].used = true;
        NODES[slot].name[..path.len()].copy_from_slice(path);
        NODES[slot].name_len = path.len();
        NODES[slot].file_type = file_type;
        NODES[slot].mode = mode;
        if file_type == FileType::Regular {
            REGULAR_COUNT += 1;
        }
    }
    Ok(inode(slot))
}

fn ensure_capacity(idx: usize, needed: usize) -> Result<(), u32> {
    unsafe {
        if needed <= NODES[idx].capacity {
            return Ok(());
        }

        let new_capacity = round_capacity(needed);
        let pages = new_capacity / PAGE_SIZE;
        let Some(new_data) = memory::alloc_pages(pages) else {
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
            memory::free_pages(NODES[idx].data, old_pages);
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
