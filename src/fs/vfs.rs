#![allow(dead_code)]

use crate::config;
use crate::fs::initrd;
use crate::fs::ramfs;
use crate::kernel::syscall::{ENOENT, EROFS};

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
}

#[derive(Clone, Copy)]
pub struct FileView {
    pub inode: InodeRef,
    pub name: &'static str,
    pub data: &'static [u8],
}

pub fn init() {
    ramfs::init();
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
    if config::CONFIG_INITRD_EXTERNAL {
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
    }
}

pub fn open(inode: InodeRef) -> Option<FileView> {
    match inode.fs {
        FileSystemId::Initrd => initrd::open(inode),
        FileSystemId::Ramfs => ramfs::open(inode),
    }
}

pub fn read(inode: InodeRef, offset: usize, dst: &mut [u8]) -> Result<usize, u32> {
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
        FileSystemId::Initrd => Err(EROFS),
    }
}

pub fn truncate(inode: InodeRef, size: usize) -> Result<(), u32> {
    match inode.fs {
        FileSystemId::Ramfs => ramfs::truncate(inode, size),
        FileSystemId::Initrd => Err(EROFS),
    }
}

pub fn create_file(path: &[u8], mode: u16) -> Result<InodeRef, u32> {
    ramfs::create_file(path, mode)
}

pub fn file_data(inode: InodeRef) -> Option<&'static [u8]> {
    open(inode).map(|file| file.data)
}

pub fn file_name(inode: InodeRef) -> Option<&'static str> {
    open(inode).map(|file| file.name)
}
