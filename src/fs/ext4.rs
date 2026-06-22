#![allow(dead_code)]

use crate::drivers::virtio;
use crate::fs::vfs::{DirEntry, FileSystemId, FileType, FileView, InodeRef, Metadata};
use crate::kernel::memory::{self, PAGE_SIZE};
use crate::kernel::syscall::{EINVAL, EIO, EISDIR, ENOENT, ENOMEM, ENOSYS};

const EXT4_SUPER_OFFSET: usize = 1024;
const EXT4_MAGIC: u16 = 0xef53;
const ROOT_INO: u32 = 2;
const EXT4_NAME_LEN: usize = 255;
const MAX_CACHE_INODES: usize = 64;
const MAX_EXTENTS: usize = 16;
const MAX_SYMLINK: usize = 96;
const MAX_PATH: usize = 192;
const MAX_READ_CHUNK: usize = 4096;
const EXT4_EXT_MAGIC: u16 = 0xf30a;
const EXT4_EXTENTS_FL: u32 = 0x0008_0000;
const S_IFMT: u16 = 0o170000;
const S_IFREG: u16 = 0o100000;
const S_IFDIR: u16 = 0o040000;
const S_IFLNK: u16 = 0o120000;
const S_IFCHR: u16 = 0o020000;

#[repr(C)]
#[derive(Clone, Copy)]
struct SuperBlock {
    inodes_count: u32,
    blocks_count_lo: u32,
    r_blocks_count_lo: u32,
    free_blocks_count_lo: u32,
    free_inodes_count: u32,
    first_data_block: u32,
    log_block_size: u32,
    log_cluster_size: u32,
    blocks_per_group: u32,
    clusters_per_group: u32,
    inodes_per_group: u32,
    mtime: u32,
    wtime: u32,
    mnt_count: u16,
    max_mnt_count: u16,
    magic: u16,
    state: u16,
    errors: u16,
    minor_rev_level: u16,
    lastcheck: u32,
    checkinterval: u32,
    creator_os: u32,
    rev_level: u32,
    def_resuid: u16,
    def_resgid: u16,
    first_ino: u32,
    inode_size: u16,
    block_group_nr: u16,
    feature_compat: u32,
    feature_incompat: u32,
    feature_ro_compat: u32,
    uuid: [u8; 16],
    volume_name: [u8; 16],
    last_mounted: [u8; 64],
    algorithm_usage_bitmap: u32,
    prealloc_blocks: u8,
    prealloc_dir_blocks: u8,
    reserved_gdt_blocks: u16,
    journal_uuid: [u8; 16],
    journal_inum: u32,
    journal_dev: u32,
    last_orphan: u32,
    hash_seed: [u32; 4],
    def_hash_version: u8,
    jnl_backup_type: u8,
    desc_size: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct GroupDesc64 {
    block_bitmap_lo: u32,
    inode_bitmap_lo: u32,
    inode_table_lo: u32,
    free_blocks_count_lo: u16,
    free_inodes_count_lo: u16,
    used_dirs_count_lo: u16,
    flags: u16,
    exclude_bitmap_lo: u32,
    block_bitmap_csum_lo: u16,
    inode_bitmap_csum_lo: u16,
    itable_unused_lo: u16,
    checksum: u16,
    block_bitmap_hi: u32,
    inode_bitmap_hi: u32,
    inode_table_hi: u32,
    free_blocks_count_hi: u16,
    free_inodes_count_hi: u16,
    used_dirs_count_hi: u16,
    itable_unused_hi: u16,
    exclude_bitmap_hi: u32,
    block_bitmap_csum_hi: u16,
    inode_bitmap_csum_hi: u16,
    reserved: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawInode {
    mode: u16,
    uid: u16,
    size_lo: u32,
    atime: u32,
    ctime: u32,
    mtime: u32,
    dtime: u32,
    gid: u16,
    links_count: u16,
    blocks_lo: u32,
    flags: u32,
    osd1: u32,
    block: [u8; 60],
    generation: u32,
    file_acl_lo: u32,
    size_high: u32,
    obso_faddr: u32,
    extra: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ExtentHeader {
    magic: u16,
    entries: u16,
    max: u16,
    depth: u16,
    generation: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ExtentIdx {
    block: u32,
    leaf_lo: u32,
    leaf_hi: u16,
    unused: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Extent {
    block: u32,
    len: u16,
    start_hi: u16,
    start_lo: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RawDirEntry {
    inode: u32,
    rec_len: u16,
    name_len: u8,
    file_type: u8,
}

#[derive(Clone, Copy)]
struct Mount {
    mounted: bool,
    block_size: usize,
    inode_size: usize,
    blocks_per_group: u32,
    inodes_per_group: u32,
    group_desc_size: usize,
    groups: u32,
    desc_table_block: u64,
}

impl Mount {
    const fn empty() -> Self {
        Self {
            mounted: false,
            block_size: 0,
            inode_size: 0,
            blocks_per_group: 0,
            inodes_per_group: 0,
            group_desc_size: 0,
            groups: 0,
            desc_table_block: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct CachedFile {
    used: bool,
    ino: u32,
    generation: u32,
    mode: u16,
    size: usize,
    data: *mut u8,
    pages: usize,
    name: [u8; EXT4_NAME_LEN],
    name_len: usize,
}

impl CachedFile {
    const fn empty() -> Self {
        Self {
            used: false,
            ino: 0,
            generation: 0,
            mode: 0,
            size: 0,
            data: core::ptr::null_mut(),
            pages: 0,
            name: [0; EXT4_NAME_LEN],
            name_len: 0,
        }
    }
}

static mut MOUNT: Mount = Mount::empty();
static mut CACHE: [CachedFile; MAX_CACHE_INODES] =
    [const { CachedFile::empty() }; MAX_CACHE_INODES];
static mut NEXT_GENERATION: u32 = 1;
static mut LAST_ERROR: u32 = 0;
static mut LAST_LOOKUP_ERROR: u32 = 0;

pub fn init() -> Result<(), u32> {
    unsafe {
        LAST_ERROR = 0;
    }
    match init_inner() {
        Ok(()) => Ok(()),
        Err(err) => {
            unsafe {
                LAST_ERROR = err;
            }
            Err(err)
        }
    }
}

fn init_inner() -> Result<(), u32> {
    unsafe {
        for slot in 0..MAX_CACHE_INODES {
            CACHE[slot] = CachedFile::empty();
        }
        NEXT_GENERATION = 1;
    }

    let mut raw = [0u8; core::mem::size_of::<SuperBlock>()];
    virtio::block_read_at(EXT4_SUPER_OFFSET, &mut raw).map_err(device_err)?;
    let sb = read_struct::<SuperBlock>(&raw);
    if sb.magic != EXT4_MAGIC {
        return Err(EINVAL);
    }

    let block_size = 1024usize.checked_shl(sb.log_block_size).ok_or(EINVAL)?;
    if block_size != 4096 {
        return Err(EINVAL);
    }

    let inode_size = if sb.inode_size == 0 {
        128
    } else {
        sb.inode_size as usize
    };
    if inode_size < core::mem::size_of::<RawInode>() {
        return Err(EINVAL);
    }

    let blocks = u64::from(sb.blocks_count_lo);
    let groups = blocks
        .div_ceil(u64::from(sb.blocks_per_group))
        .try_into()
        .map_err(|_| EINVAL)?;
    let group_desc_size = if sb.desc_size == 0 {
        32
    } else {
        sb.desc_size as usize
    };

    unsafe {
        MOUNT = Mount {
            mounted: true,
            block_size,
            inode_size,
            blocks_per_group: sb.blocks_per_group,
            inodes_per_group: sb.inodes_per_group,
            group_desc_size,
            groups,
            desc_table_block: if block_size == 1024 { 2 } else { 1 },
        };
    }
    Ok(())
}

pub fn is_mounted() -> bool {
    unsafe { MOUNT.mounted }
}

pub fn last_error() -> u32 {
    unsafe { LAST_ERROR }
}

pub fn last_lookup_error() -> u32 {
    unsafe { LAST_LOOKUP_ERROR }
}

pub fn lookup(path: &[u8]) -> Option<InodeRef> {
    if !is_mounted() {
        return None;
    }
    let path = normalize(path);
    if path.is_empty() || path == b"/" {
        return Some(inode(ROOT_INO));
    }

    match lookup_path(path, true, 0) {
        Ok(ino) => Some(inode(ino)),
        Err(err) => {
            unsafe {
                LAST_LOOKUP_ERROR = err;
            }
            None
        }
    }
}

pub fn lookup_nofollow(path: &[u8]) -> Option<InodeRef> {
    if !is_mounted() {
        return None;
    }
    let path = normalize(path);
    if path.is_empty() || path == b"/" {
        return Some(inode(ROOT_INO));
    }

    match lookup_path(path, false, 0) {
        Ok(ino) => Some(inode(ino)),
        Err(_) => None,
    }
}

pub fn metadata(inode_ref: InodeRef) -> Option<Metadata> {
    let raw = read_inode(inode_ref.ino as u32).ok()?;
    let file_type = raw_file_type(raw.mode)?;
    let mut meta = Metadata::new(inode_ref, file_type, inode_size(raw), raw.mode & 0o7777);
    meta.nlink = u32::from(raw.links_count);
    meta.uid = u32::from(raw.uid);
    meta.gid = u32::from(raw.gid);
    meta.atime = raw.atime;
    meta.mtime = raw.mtime;
    meta.ctime = raw.ctime;
    Some(meta)
}

pub fn open(inode_ref: InodeRef) -> Option<FileView> {
    let raw = read_inode(inode_ref.ino as u32).ok()?;
    let file_type = raw_file_type(raw.mode)?;
    if file_type != FileType::Regular {
        return None;
    }
    let cache = cache_file(inode_ref.ino as u32, raw).ok()?;
    Some(FileView {
        inode: inode_ref,
        name: cache_name(cache),
        data: cache_data(cache),
    })
}

pub fn read(inode_ref: InodeRef, offset: usize, dst: &mut [u8]) -> Result<usize, u32> {
    let raw = read_inode(inode_ref.ino as u32)?;
    if raw_file_type(raw.mode) == Some(FileType::Directory) {
        return Err(EISDIR);
    }
    let size = inode_size(raw);
    let start = offset.min(size);
    let count = (size - start).min(dst.len());
    if count == 0 {
        return Ok(0);
    }
    read_inode_data(&raw, start, &mut dst[..count])?;
    Ok(count)
}

pub fn read_dir(inode_ref: InodeRef, offset: usize, dst: &mut [DirEntry]) -> Result<usize, u32> {
    let raw = read_inode(inode_ref.ino as u32)?;
    if raw_file_type(raw.mode) != Some(FileType::Directory) {
        return Err(EINVAL);
    }

    let size = inode_size(raw);
    let mut file_offset = 0usize;
    let mut logical = 0usize;
    let mut written = 0usize;
    let mut block = [0u8; MAX_READ_CHUNK];
    while file_offset < size {
        let count = (size - file_offset).min(block.len());
        read_inode_data(&raw, file_offset, &mut block[..count])?;
        let mut cursor = 0usize;
        while cursor + core::mem::size_of::<RawDirEntry>() <= count {
            let entry = read_struct::<RawDirEntry>(&block[cursor..]);
            if entry.rec_len == 0 {
                break;
            }
            let rec_len = usize::from(entry.rec_len);
            if cursor + rec_len > count {
                break;
            }
            if entry.inode != 0 && entry.name_len != 0 {
                if logical >= offset {
                    if written >= dst.len() {
                        return Ok(written);
                    }
                    let name_len = usize::from(entry.name_len).min(EXT4_NAME_LEN);
                    let name_start = cursor + core::mem::size_of::<RawDirEntry>();
                    let name_end = name_start + name_len;
                    if name_end <= cursor + rec_len {
                        dst[written] = DirEntry::new(
                            inode(entry.inode),
                            dir_file_type(entry.file_type),
                            &block[name_start..name_end],
                        );
                        written += 1;
                    }
                }
                logical += 1;
            }
            cursor += rec_len;
        }
        file_offset += count;
    }
    Ok(written)
}

pub fn readlink(path: &[u8], dst: &mut [u8]) -> Result<usize, u32> {
    let inode_ref = lookup_nofollow(path).ok_or(ENOENT)?;
    let raw = read_inode(inode_ref.ino as u32)?;
    if raw_file_type(raw.mode) != Some(FileType::Symlink) {
        return Err(EINVAL);
    }
    let size = inode_size(raw).min(dst.len());
    read_inode_data(&raw, 0, &mut dst[..size])?;
    Ok(size)
}

pub fn readlink_inode(inode_ref: InodeRef, dst: &mut [u8]) -> Result<usize, u32> {
    let raw = read_inode(inode_ref.ino as u32)?;
    if raw_file_type(raw.mode) != Some(FileType::Symlink) {
        return Err(EINVAL);
    }
    let size = inode_size(raw).min(dst.len());
    read_inode_data(&raw, 0, &mut dst[..size])?;
    Ok(size)
}

fn lookup_path(path: &[u8], follow_last: bool, depth: usize) -> Result<u32, u32> {
    if depth > 8 {
        return Err(EINVAL);
    }

    let mut normalized = [0u8; MAX_PATH];
    let normalized_len = normalize_components(path, &mut normalized);
    let path = &normalized[..normalized_len];
    let mut current = ROOT_INO;
    let mut cursor = 0usize;
    while cursor < path.len() {
        while cursor < path.len() && path[cursor] == b'/' {
            cursor += 1;
        }
        if cursor >= path.len() {
            break;
        }
        let start = cursor;
        while cursor < path.len() && path[cursor] != b'/' {
            cursor += 1;
        }
        let component = &path[start..cursor];
        if component == b"." {
            continue;
        }
        if component == b".." {
            return lookup_path(parent_path(path, start), follow_last, depth + 1);
        }
        let child = lookup_child(current, component)?;
        let raw = read_inode(child)?;
        let is_last = {
            let mut probe = cursor;
            while probe < path.len() && path[probe] == b'/' {
                probe += 1;
            }
            probe >= path.len()
        };
        if raw_file_type(raw.mode) == Some(FileType::Symlink) && (follow_last || !is_last) {
            let mut link = [0u8; MAX_SYMLINK];
            let link_len = inode_size(raw).min(link.len());
            read_inode_data(&raw, 0, &mut link[..link_len])?;
            let mut combined = [0u8; MAX_PATH];
            let combined_len = if link[..link_len].starts_with(b"/") {
                copy_path(&mut combined, &link[..link_len])
            } else {
                let parent_len = parent_prefix(path, start, &mut combined);
                let link_written = append_component(&mut combined, parent_len, &link[..link_len]);
                link_written
            };
            let mut final_len = combined_len;
            if cursor < path.len() {
                final_len = append_component(&mut combined, final_len, &path[cursor..]);
            }
            let mut normalized = [0u8; MAX_PATH];
            let normalized_len = normalize_components(&combined[..final_len], &mut normalized);
            return lookup_path(&normalized[..normalized_len], follow_last, depth + 1);
        }
        current = child;
    }
    Ok(current)
}

fn lookup_child(parent: u32, name: &[u8]) -> Result<u32, u32> {
    let raw = read_inode(parent)?;
    if raw_file_type(raw.mode) != Some(FileType::Directory) {
        return Err(EINVAL);
    }
    let size = inode_size(raw);
    let mut file_offset = 0usize;
    let mut block = [0u8; MAX_READ_CHUNK];
    while file_offset < size {
        let count = (size - file_offset).min(block.len());
        read_inode_data(&raw, file_offset, &mut block[..count])?;
        let mut cursor = 0usize;
        while cursor + core::mem::size_of::<RawDirEntry>() <= count {
            let entry = read_struct::<RawDirEntry>(&block[cursor..]);
            if entry.rec_len == 0 {
                break;
            }
            let rec_len = usize::from(entry.rec_len);
            if cursor + rec_len > count {
                break;
            }
            let name_len = usize::from(entry.name_len).min(EXT4_NAME_LEN);
            let name_start = cursor + core::mem::size_of::<RawDirEntry>();
            let name_end = name_start + name_len;
            if entry.inode != 0
                && name_end <= cursor + rec_len
                && &block[name_start..name_end] == name
            {
                return Ok(entry.inode);
            }
            cursor += rec_len;
        }
        file_offset += count;
    }
    Err(ENOENT)
}

fn read_inode(ino: u32) -> Result<RawInode, u32> {
    if ino == 0 {
        return Err(ENOENT);
    }
    let mount = unsafe { MOUNT };
    if !mount.mounted {
        return Err(ENOENT);
    }
    let group = (ino - 1) / mount.inodes_per_group;
    if group >= mount.groups {
        return Err(ENOENT);
    }
    let index = (ino - 1) % mount.inodes_per_group;
    let group_desc = read_group_desc(group)?;
    let inode_table =
        u64::from(group_desc.inode_table_lo) | (u64::from(group_desc.inode_table_hi) << 32);
    let offset = inode_table as usize * mount.block_size + index as usize * mount.inode_size;
    let mut raw = [0u8; core::mem::size_of::<RawInode>()];
    read_disk(offset, &mut raw)?;
    Ok(read_struct::<RawInode>(&raw))
}

fn read_group_desc(group: u32) -> Result<GroupDesc64, u32> {
    let mount = unsafe { MOUNT };
    let offset =
        mount.desc_table_block as usize * mount.block_size + group as usize * mount.group_desc_size;
    let mut raw = [0u8; core::mem::size_of::<GroupDesc64>()];
    read_disk(offset, &mut raw)?;
    Ok(read_struct::<GroupDesc64>(&raw))
}

fn read_inode_data(raw: &RawInode, offset: usize, dst: &mut [u8]) -> Result<(), u32> {
    if dst.is_empty() {
        return Ok(());
    }
    if raw_file_type(raw.mode) == Some(FileType::Symlink)
        && raw.flags & EXT4_EXTENTS_FL == 0
        && inode_size(*raw) <= raw.block.len()
    {
        dst.copy_from_slice(&raw.block[offset..offset + dst.len()]);
        return Ok(());
    }

    if raw.flags & EXT4_EXTENTS_FL == 0 {
        return Err(ENOSYS);
    }

    let mut extents = [Extent::empty(); MAX_EXTENTS];
    let extent_count = collect_extents(&raw.block, &mut extents)?;
    let mount = unsafe { MOUNT };
    let mut done = 0usize;
    while done < dst.len() {
        let logical = (offset + done) / mount.block_size;
        let in_block = (offset + done) % mount.block_size;
        let count = (mount.block_size - in_block).min(dst.len() - done);
        let Some(physical) = find_extent_block(&extents[..extent_count], logical as u32) else {
            for byte in &mut dst[done..done + count] {
                *byte = 0;
            }
            done += count;
            continue;
        };
        let disk_offset = physical as usize * mount.block_size + in_block;
        read_disk(disk_offset, &mut dst[done..done + count])?;
        done += count;
    }
    Ok(())
}

fn collect_extents(root: &[u8; 60], out: &mut [Extent; MAX_EXTENTS]) -> Result<usize, u32> {
    let header = read_struct::<ExtentHeader>(root);
    if header.magic != EXT4_EXT_MAGIC {
        return Err(EINVAL);
    }
    collect_extent_node(root, header, out)
}

fn collect_extent_node(
    node: &[u8],
    header: ExtentHeader,
    out: &mut [Extent],
) -> Result<usize, u32> {
    let mut written = 0usize;
    let entries = usize::from(header.entries);
    let base = core::mem::size_of::<ExtentHeader>();
    if header.depth == 0 {
        for idx in 0..entries {
            if written >= out.len() {
                return Err(ENOMEM);
            }
            let start = base + idx * core::mem::size_of::<Extent>();
            let end = start + core::mem::size_of::<Extent>();
            if end > node.len() {
                return Err(EINVAL);
            }
            out[written] = read_struct::<Extent>(&node[start..end]);
            written += 1;
        }
        return Ok(written);
    }

    for idx in 0..entries {
        let start = base + idx * core::mem::size_of::<ExtentIdx>();
        let end = start + core::mem::size_of::<ExtentIdx>();
        if end > node.len() {
            return Err(EINVAL);
        }
        let index = read_struct::<ExtentIdx>(&node[start..end]);
        let physical = u64::from(index.leaf_lo) | (u64::from(index.leaf_hi) << 32);
        let mut block = [0u8; MAX_READ_CHUNK];
        let mount = unsafe { MOUNT };
        read_disk(physical as usize * mount.block_size, &mut block)?;
        let child_header = read_struct::<ExtentHeader>(&block);
        if child_header.magic != EXT4_EXT_MAGIC {
            return Err(EINVAL);
        }
        let count = collect_extent_node(&block, child_header, &mut out[written..])?;
        written += count;
    }
    Ok(written)
}

fn find_extent_block(extents: &[Extent], logical: u32) -> Option<u64> {
    for extent in extents {
        let len = u32::from(extent.len & 0x7fff);
        if logical >= extent.block && logical < extent.block + len {
            let start = u64::from(extent.start_lo) | (u64::from(extent.start_hi) << 32);
            return Some(start + u64::from(logical - extent.block));
        }
    }
    None
}

impl Extent {
    const fn empty() -> Self {
        Self {
            block: 0,
            len: 0,
            start_hi: 0,
            start_lo: 0,
        }
    }
}

fn cache_file(ino: u32, raw: RawInode) -> Result<&'static CachedFile, u32> {
    unsafe {
        for slot in 0..MAX_CACHE_INODES {
            if CACHE[slot].used && CACHE[slot].ino == ino {
                return Ok(&CACHE[slot]);
            }
        }

        let Some(slot) = (0..MAX_CACHE_INODES).find(|slot| !CACHE[*slot].used) else {
            return Err(ENOMEM);
        };
        let size = inode_size(raw);
        let pages = size.div_ceil(PAGE_SIZE).max(1);
        let data = memory::alloc_exact_pages(pages).ok_or(ENOMEM)?;
        let buf = core::slice::from_raw_parts_mut(data, pages * PAGE_SIZE);
        read_inode_data(&raw, 0, &mut buf[..size])?;
        CACHE[slot] = CachedFile {
            used: true,
            ino,
            generation: NEXT_GENERATION,
            mode: raw.mode,
            size,
            data,
            pages,
            name: [0; EXT4_NAME_LEN],
            name_len: 0,
        };
        NEXT_GENERATION = NEXT_GENERATION.wrapping_add(1).max(1);
        Ok(&CACHE[slot])
    }
}

fn cache_data(cache: &CachedFile) -> &'static [u8] {
    if cache.size == 0 {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(cache.data as *const u8, cache.size) }
    }
}

fn cache_name(_cache: &CachedFile) -> &'static str {
    "ext4-file"
}

fn read_disk(offset: usize, dst: &mut [u8]) -> Result<(), u32> {
    virtio::block_read_at(offset, dst)
        .map(|_| ())
        .map_err(device_err)
}

fn read_struct<T: Copy>(bytes: &[u8]) -> T {
    unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const T) }
}

fn inode_size(raw: RawInode) -> usize {
    (u64::from(raw.size_lo) | (u64::from(raw.size_high) << 32)) as usize
}

fn raw_file_type(mode: u16) -> Option<FileType> {
    match mode & S_IFMT {
        S_IFREG => Some(FileType::Regular),
        S_IFDIR => Some(FileType::Directory),
        S_IFLNK => Some(FileType::Symlink),
        S_IFCHR => Some(FileType::Device),
        _ => None,
    }
}

fn dir_file_type(raw: u8) -> FileType {
    match raw {
        2 => FileType::Directory,
        3 => FileType::Device,
        7 => FileType::Symlink,
        _ => FileType::Regular,
    }
}

fn inode(ino: u32) -> InodeRef {
    InodeRef {
        fs: FileSystemId::Ext4,
        ino: ino as usize,
    }
}

fn normalize(path: &[u8]) -> &[u8] {
    let path = match path.iter().position(|byte| *byte == 0) {
        Some(end) => &path[..end],
        None => path,
    };
    path.strip_prefix(b"/").unwrap_or(path)
}

fn copy_path(dst: &mut [u8], path: &[u8]) -> usize {
    let path = normalize(path);
    let len = path.len().min(dst.len());
    dst[..len].copy_from_slice(&path[..len]);
    len
}

fn parent_prefix(path: &[u8], component_start: usize, dst: &mut [u8]) -> usize {
    let Some(slash) = path[..component_start]
        .iter()
        .rposition(|byte| *byte == b'/')
    else {
        return 0;
    };
    let len = slash.min(dst.len());
    dst[..len].copy_from_slice(&path[..len]);
    len
}

fn append_component(dst: &mut [u8], mut len: usize, component: &[u8]) -> usize {
    let component = component.strip_prefix(b"/").unwrap_or(component);
    if len != 0 && len < dst.len() && dst[len - 1] != b'/' {
        dst[len] = b'/';
        len += 1;
    }
    let count = component.len().min(dst.len().saturating_sub(len));
    dst[len..len + count].copy_from_slice(&component[..count]);
    len + count
}

fn normalize_components(path: &[u8], out: &mut [u8]) -> usize {
    let path = normalize(path);
    let mut components: [&[u8]; 32] = [&[]; 32];
    let mut component_count = 0usize;
    let mut cursor = 0usize;
    while cursor < path.len() {
        while cursor < path.len() && path[cursor] == b'/' {
            cursor += 1;
        }
        let start = cursor;
        while cursor < path.len() && path[cursor] != b'/' {
            cursor += 1;
        }
        if start == cursor {
            continue;
        }
        let component = &path[start..cursor];
        if component == b"." {
            continue;
        }
        if component == b".." {
            component_count = component_count.saturating_sub(1);
            continue;
        }
        if component_count < components.len() {
            components[component_count] = component;
            component_count += 1;
        }
    }

    let mut len = 0usize;
    for component in components.iter().take(component_count) {
        if len != 0 && len < out.len() {
            out[len] = b'/';
            len += 1;
        }
        let count = component.len().min(out.len().saturating_sub(len));
        out[len..len + count].copy_from_slice(&component[..count]);
        len += count;
    }
    len
}

fn parent_path(path: &[u8], component_start: usize) -> &[u8] {
    path[..component_start]
        .iter()
        .rposition(|byte| *byte == b'/')
        .map(|slash| &path[..slash])
        .unwrap_or(b"")
}

fn device_err(err: crate::drivers::device::DeviceError) -> u32 {
    match err {
        crate::drivers::device::DeviceError::NotPresent => ENOENT,
        crate::drivers::device::DeviceError::Unsupported => ENOSYS,
        crate::drivers::device::DeviceError::Busy => ENOMEM,
        crate::drivers::device::DeviceError::Invalid => EINVAL,
        crate::drivers::device::DeviceError::Io => EIO,
    }
}
