#![allow(dead_code)]

use crate::config;
use crate::fs::devfs;
use crate::fs::ext4;
use crate::fs::initrd;
use crate::fs::procfs;
use crate::fs::ramfs;
use crate::kernel::syscall::{EINVAL, EISDIR, ENOENT, EROFS};

pub const S_IFMT: u32 = 0o170000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFLNK: u32 = 0o120000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    Device,
    Symlink,
}

impl FileType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Directory => "directory",
            Self::Device => "device",
            Self::Symlink => "symlink",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileSystemId {
    Initrd,
    Ramfs,
    Devfs,
    Procfs,
    Ext4,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InodeRef {
    pub fs: FileSystemId,
    pub ino: usize,
}

#[derive(Clone, Copy)]
pub struct Metadata {
    pub inode: InodeRef,
    pub file_type: FileType,
    pub size: usize,
    pub mode: u16,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub atime: u32,
    pub mtime: u32,
    pub ctime: u32,
}

impl Metadata {
    pub const fn new(inode: InodeRef, file_type: FileType, size: usize, mode: u16) -> Self {
        Self {
            inode,
            file_type,
            size,
            mode,
            nlink: if matches!(file_type, FileType::Directory) {
                2
            } else {
                1
            },
            uid: 0,
            gid: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
        }
    }

    pub const fn linux_mode(self) -> u32 {
        file_type_mode(self.file_type) | (self.mode as u32 & 0o7777)
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,
    pub __pad1: u32,
    pub st_ino: u32,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub __pad2: u32,
    pub st_size: i64,
    pub st_blksize: u32,
    pub st_blocks: u64,
    pub st_atime: u32,
    pub st_atime_nsec: u32,
    pub st_mtime: u32,
    pub st_mtime_nsec: u32,
    pub st_ctime: u32,
    pub st_ctime_nsec: u32,
    pub st_ino64: u64,
}

impl Stat {
    pub fn from_metadata(metadata: Metadata) -> Self {
        Self {
            st_dev: fs_dev(metadata.inode.fs),
            __pad1: 0,
            st_ino: metadata.inode.ino as u32,
            st_mode: metadata.linux_mode(),
            st_nlink: metadata.nlink,
            st_uid: metadata.uid,
            st_gid: metadata.gid,
            st_rdev: if metadata.file_type == FileType::Device {
                metadata.inode.ino as u64
            } else {
                0
            },
            __pad2: 0,
            st_size: metadata.size as i64,
            st_blksize: 4096,
            st_blocks: metadata.size.div_ceil(512) as u64,
            st_atime: metadata.atime,
            st_atime_nsec: 0,
            st_mtime: metadata.mtime,
            st_mtime_nsec: 0,
            st_ctime: metadata.ctime,
            st_ctime_nsec: 0,
            st_ino64: metadata.inode.ino as u64,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StatxTimestamp {
    pub tv_sec: i64,
    pub tv_nsec: u32,
    pub __reserved: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Statx {
    pub stx_mask: u32,
    pub stx_blksize: u32,
    pub stx_attributes: u64,
    pub stx_nlink: u32,
    pub stx_uid: u32,
    pub stx_gid: u32,
    pub stx_mode: u16,
    pub __spare0: [u16; 1],
    pub stx_ino: u64,
    pub stx_size: u64,
    pub stx_blocks: u64,
    pub stx_attributes_mask: u64,
    pub stx_atime: StatxTimestamp,
    pub stx_btime: StatxTimestamp,
    pub stx_ctime: StatxTimestamp,
    pub stx_mtime: StatxTimestamp,
    pub stx_rdev_major: u32,
    pub stx_rdev_minor: u32,
    pub stx_dev_major: u32,
    pub stx_dev_minor: u32,
    pub __spare2: [u64; 14],
}

impl Statx {
    pub fn from_metadata(metadata: Metadata) -> Self {
        Self {
            stx_mask: 0x0000_1fff,
            stx_blksize: 4096,
            stx_attributes: 0,
            stx_nlink: metadata.nlink,
            stx_uid: metadata.uid,
            stx_gid: metadata.gid,
            stx_mode: metadata.linux_mode() as u16,
            __spare0: [0; 1],
            stx_ino: metadata.inode.ino as u64,
            stx_size: metadata.size as u64,
            stx_blocks: metadata.size.div_ceil(512) as u64,
            stx_attributes_mask: 0,
            stx_atime: statx_time(metadata.atime),
            stx_btime: statx_time(0),
            stx_ctime: statx_time(metadata.ctime),
            stx_mtime: statx_time(metadata.mtime),
            stx_rdev_major: 0,
            stx_rdev_minor: if metadata.file_type == FileType::Device {
                metadata.inode.ino as u32
            } else {
                0
            },
            stx_dev_major: 0,
            stx_dev_minor: fs_dev(metadata.inode.fs) as u32,
            __spare2: [0; 14],
        }
    }
}

#[derive(Clone, Copy)]
pub struct DirEntry {
    pub inode: InodeRef,
    pub file_type: FileType,
    pub name: [u8; DIR_NAME_LEN],
    pub name_len: usize,
}

pub const DIR_NAME_LEN: usize = 96;

impl DirEntry {
    pub const fn empty() -> Self {
        Self {
            inode: InodeRef {
                fs: FileSystemId::Ramfs,
                ino: 0,
            },
            file_type: FileType::Regular,
            name: [0; DIR_NAME_LEN],
            name_len: 0,
        }
    }

    pub fn new(inode: InodeRef, file_type: FileType, name: &[u8]) -> Self {
        let mut entry = Self {
            inode,
            file_type,
            name: [0; DIR_NAME_LEN],
            name_len: name.len().min(DIR_NAME_LEN),
        };
        entry.name[..entry.name_len].copy_from_slice(&name[..entry.name_len]);
        entry
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserDirEntry {
    pub ino: u32,
    pub file_type: u32,
    pub name_len: u32,
    pub name: [u8; DIR_NAME_LEN],
}

impl UserDirEntry {
    pub const fn empty() -> Self {
        Self {
            ino: 0,
            file_type: 0,
            name_len: 0,
            name: [0; DIR_NAME_LEN],
        }
    }

    pub fn from_dir_entry(entry: DirEntry) -> Self {
        Self {
            ino: entry.inode.ino as u32,
            file_type: file_type_code(entry.file_type),
            name_len: entry.name_len as u32,
            name: entry.name,
        }
    }
}

#[derive(Clone, Copy)]
pub struct FileView {
    pub inode: InodeRef,
    pub name: &'static str,
    pub data: &'static [u8],
}

pub fn init() {
    ramfs::init();
    devfs::init();
    procfs::init();
    if config::CONFIG_EXT4_ROOTFS {
        let _ = ext4::init();
    }
    let _ = ramfs::mkdir(b"tmp", 0o777);
    if config::CONFIG_INITRD_EXTERNAL {
        let _ = ramfs::unpack_archive();
    }
    if config::CONFIG_INITRD {
        initrd::init();
    }
}

pub fn mounted_files() -> usize {
    let mut files = 0;
    if config::CONFIG_INITRD_EXTERNAL {
        files += ramfs::regular_count();
    }
    if config::CONFIG_INITRD {
        files += initrd::file_count();
    }
    files
}

pub fn external_files() -> usize {
    if config::CONFIG_INITRD_EXTERNAL {
        ramfs::regular_count()
    } else {
        0
    }
}

pub fn builtin_files() -> usize {
    if config::CONFIG_INITRD {
        initrd::file_count()
    } else {
        0
    }
}

pub fn lookup(path: &[u8]) -> Result<InodeRef, u32> {
    lookup_follow(path, 0).or_else(|err| {
        let stripped = strip_absolute(path);
        if stripped.len() != path.len() {
            lookup_follow(stripped, 0)
        } else {
            Err(err)
        }
    })
}

pub fn lookup_nofollow(path: &[u8]) -> Result<InodeRef, u32> {
    if let Some(stripped) = stripped_absolute_retry(path) {
        if let Ok(inode) = lookup_nofollow(stripped) {
            return Ok(inode);
        }
    }
    if let Some(inode) = devfs::lookup(path) {
        return Ok(inode);
    }
    if let Some(inode) = procfs::lookup(path) {
        return Ok(inode);
    }
    if is_ramfs_overlay_path(path) {
        if let Some(inode) = ramfs::lookup(path) {
            return Ok(inode);
        }
    }
    if config::CONFIG_EXT4_ROOTFS {
        if let Some(inode) = ext4::lookup_nofollow(path) {
            return Ok(inode);
        }
    }
    if config::CONFIG_INITRD_EXTERNAL && !is_ramfs_overlay_path(path) {
        if let Some(inode) = ramfs::lookup(path) {
            return Ok(inode);
        }
    }
    if config::CONFIG_INITRD {
        if let Some(inode) = initrd::lookup(path) {
            return Ok(inode);
        }
    }
    Err(ENOENT)
}

pub fn lookup_child(parent: InodeRef, name: &[u8], follow: bool) -> Result<InodeRef, u32> {
    let name = strip_absolute(name);
    if name.is_empty() || name == b"." {
        return Ok(parent);
    }
    if name == b".." {
        return Err(ENOENT);
    }

    let mut current = parent;
    let mut cursor = 0usize;
    while cursor < name.len() {
        while cursor < name.len() && name[cursor] == b'/' {
            cursor += 1;
        }
        if cursor >= name.len() {
            break;
        }
        let start = cursor;
        while cursor < name.len() && name[cursor] != b'/' {
            cursor += 1;
        }
        let component = &name[start..cursor];
        if component == b"." {
            continue;
        }
        if component == b".." {
            return Err(ENOENT);
        }
        let is_last = {
            let mut probe = cursor;
            while probe < name.len() && name[probe] == b'/' {
                probe += 1;
            }
            probe >= name.len()
        };
        current = lookup_direct_child(current, component, follow || !is_last)?;
    }
    Ok(current)
}

fn lookup_direct_child(parent: InodeRef, name: &[u8], follow: bool) -> Result<InodeRef, u32> {
    let mut entries = [DirEntry::empty(); 8];
    let mut offset = 0usize;
    loop {
        let count = read_dir(parent, offset, &mut entries)?;
        if count == 0 {
            break;
        }
        for entry in entries.iter().take(count) {
            if entry.name_len == name.len() && &entry.name[..entry.name_len] == name {
                return if follow {
                    follow_symlink(entry.inode, 0)
                } else {
                    Ok(entry.inode)
                };
            }
        }
        offset += count;
    }
    Err(ENOENT)
}

fn lookup_follow(path: &[u8], depth: usize) -> Result<InodeRef, u32> {
    if depth > 8 {
        return Err(EINVAL);
    }
    if depth == 0 {
        if let Some(stripped) = stripped_absolute_retry(path) {
            if let Ok(inode) = lookup_follow(stripped, depth + 1) {
                return Ok(inode);
            }
        }
    }
    if let Some(inode) = devfs::lookup(path) {
        return Ok(inode);
    }
    if let Some(inode) = procfs::lookup(path) {
        return Ok(inode);
    }
    if is_ramfs_overlay_path(path) {
        if let Some(inode) = ramfs::lookup(path) {
            return follow_symlink(inode, depth);
        }
    }
    if config::CONFIG_EXT4_ROOTFS {
        if let Some(inode) = ext4::lookup(path) {
            return Ok(inode);
        }
    }
    if config::CONFIG_INITRD_EXTERNAL && !is_ramfs_overlay_path(path) {
        if let Some(inode) = ramfs::lookup(path) {
            return follow_symlink(inode, depth);
        }
    }
    if config::CONFIG_INITRD {
        if let Some(inode) = initrd::lookup(path) {
            return follow_symlink(inode, depth);
        }
    }
    Err(ENOENT)
}

fn follow_symlink(inode: InodeRef, depth: usize) -> Result<InodeRef, u32> {
    let Some(meta) = metadata(inode) else {
        return Err(ENOENT);
    };
    if meta.file_type != FileType::Symlink {
        return Ok(inode);
    }
    let Some(file) = open(inode) else {
        return Err(ENOENT);
    };
    lookup_follow(file.data, depth + 1)
}

fn is_ramfs_overlay_path(path: &[u8]) -> bool {
    let path = strip_absolute(path);
    path == b"tmp" || path.starts_with(b"tmp/")
}

pub fn lookup_builtin(path: &[u8]) -> Result<InodeRef, u32> {
    if config::CONFIG_INITRD {
        if let Some(inode) = initrd::lookup(path) {
            return Ok(inode);
        }
    }
    Err(ENOENT)
}

pub fn metadata(inode: InodeRef) -> Option<Metadata> {
    match inode.fs {
        FileSystemId::Initrd => initrd::metadata(inode),
        FileSystemId::Ramfs => ramfs::metadata(inode),
        FileSystemId::Devfs => devfs::metadata(inode),
        FileSystemId::Procfs => procfs::metadata(inode),
        FileSystemId::Ext4 => ext4::metadata(inode),
    }
}

pub fn open(inode: InodeRef) -> Option<FileView> {
    match inode.fs {
        FileSystemId::Initrd => initrd::open(inode),
        FileSystemId::Ramfs => ramfs::open(inode),
        FileSystemId::Devfs => devfs::open(inode),
        FileSystemId::Procfs => procfs::open(inode),
        FileSystemId::Ext4 => ext4::open(inode),
    }
}

pub fn read(inode: InodeRef, offset: usize, dst: &mut [u8]) -> Result<usize, u32> {
    if metadata(inode)
        .map(|meta| meta.file_type == FileType::Directory)
        .unwrap_or(false)
    {
        return Err(EISDIR);
    }
    if inode.fs == FileSystemId::Devfs {
        return devfs::read(inode, offset, dst);
    }
    if inode.fs == FileSystemId::Ext4 {
        return ext4::read(inode, offset, dst);
    }
    let Some(file) = open(inode) else {
        return Err(ENOENT);
    };
    let start = offset.min(file.data.len());
    let count = (file.data.len() - start).min(dst.len());
    dst[..count].copy_from_slice(&file.data[start..start + count]);
    Ok(count)
}

pub fn write(inode: InodeRef, offset: usize, src: &[u8]) -> Result<usize, u32> {
    match inode.fs {
        FileSystemId::Ramfs => ramfs::write(inode, offset, src),
        FileSystemId::Devfs => devfs::write(inode, offset, src),
        FileSystemId::Initrd => Err(EROFS),
        FileSystemId::Procfs => Err(EROFS),
        FileSystemId::Ext4 => Err(EROFS),
    }
}

pub fn truncate(inode: InodeRef, size: usize) -> Result<(), u32> {
    match inode.fs {
        FileSystemId::Ramfs => ramfs::truncate(inode, size),
        FileSystemId::Initrd => Err(EROFS),
        FileSystemId::Devfs => Err(EINVAL),
        FileSystemId::Procfs => Err(EROFS),
        FileSystemId::Ext4 => Err(EROFS),
    }
}

pub fn create_file(path: &[u8], mode: u16) -> Result<InodeRef, u32> {
    ramfs::create_file(path, mode)
}

pub fn mkdir(path: &[u8], mode: u16) -> Result<InodeRef, u32> {
    ramfs::mkdir(path, mode)
}

pub fn unlink(path: &[u8]) -> Result<(), u32> {
    ramfs::unlink(path)
}

pub fn link(old_path: &[u8], new_path: &[u8]) -> Result<(), u32> {
    ramfs::link(old_path, new_path)
}

pub fn rename(old_path: &[u8], new_path: &[u8]) -> Result<(), u32> {
    ramfs::rename(old_path, new_path)
}

pub fn read_dir(inode: InodeRef, offset: usize, dst: &mut [DirEntry]) -> Result<usize, u32> {
    match inode.fs {
        FileSystemId::Ramfs => ramfs::read_dir(inode, offset, dst),
        FileSystemId::Initrd => initrd::read_dir(inode, offset, dst),
        FileSystemId::Devfs => devfs::read_dir(inode, offset, dst),
        FileSystemId::Procfs => procfs::read_dir(inode, offset, dst),
        FileSystemId::Ext4 => ext4::read_dir(inode, offset, dst),
    }
}

pub fn file_data(inode: InodeRef) -> Option<&'static [u8]> {
    open(inode).map(|file| file.data)
}

pub fn file_name(inode: InodeRef) -> Option<&'static str> {
    open(inode).map(|file| file.name)
}

pub fn readlink(path: &[u8], dst: &mut [u8]) -> Result<usize, u32> {
    let inode = lookup_nofollow(path)?;
    readlink_inode(inode, dst)
}

pub fn readlink_inode(inode: InodeRef, dst: &mut [u8]) -> Result<usize, u32> {
    let Some(meta) = metadata(inode) else {
        return Err(ENOENT);
    };
    if meta.file_type != FileType::Symlink {
        return Err(EINVAL);
    }
    match inode.fs {
        FileSystemId::Ext4 => ext4::readlink_inode(inode, dst),
        _ => {
            let Some(file) = open(inode) else {
                return Err(ENOENT);
            };
            let count = file.data.len().min(dst.len());
            dst[..count].copy_from_slice(&file.data[..count]);
            Ok(count)
        }
    }
}

pub const fn file_type_code(file_type: FileType) -> u32 {
    match file_type {
        FileType::Regular => 1,
        FileType::Directory => 2,
        FileType::Device => 3,
        FileType::Symlink => 4,
    }
}

pub const fn file_type_mode(file_type: FileType) -> u32 {
    match file_type {
        FileType::Regular => S_IFREG,
        FileType::Directory => S_IFDIR,
        FileType::Device => S_IFCHR,
        FileType::Symlink => S_IFLNK,
    }
}

pub const fn fs_dev(fs: FileSystemId) -> u64 {
    match fs {
        FileSystemId::Initrd => 1,
        FileSystemId::Ramfs => 2,
        FileSystemId::Devfs => 3,
        FileSystemId::Procfs => 4,
        FileSystemId::Ext4 => 5,
    }
}

pub fn ext4_mounted() -> bool {
    config::CONFIG_EXT4_ROOTFS && ext4::is_mounted()
}

pub fn ext4_error() -> u32 {
    if config::CONFIG_EXT4_ROOTFS {
        ext4::last_error()
    } else {
        0
    }
}

pub fn ext4_lookup_error() -> u32 {
    if config::CONFIG_EXT4_ROOTFS {
        ext4::last_lookup_error()
    } else {
        0
    }
}

const fn statx_time(seconds: u32) -> StatxTimestamp {
    StatxTimestamp {
        tv_sec: seconds as i64,
        tv_nsec: 0,
        __reserved: 0,
    }
}

fn strip_absolute(path: &[u8]) -> &[u8] {
    let path = match path.iter().position(|byte| *byte == 0) {
        Some(end) => &path[..end],
        None => path,
    };
    path.strip_prefix(b"/").unwrap_or(path)
}

fn stripped_absolute_retry(path: &[u8]) -> Option<&[u8]> {
    let stripped = strip_absolute(path);
    (stripped.len() != path.len()).then_some(stripped)
}
