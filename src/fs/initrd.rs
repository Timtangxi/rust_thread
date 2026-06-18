#![allow(dead_code)]

use crate::fs::vfs::{FileSystemId, FileType, FileView, InodeRef, Metadata};
use crate::kernel::loader;

const ROOT_INO: usize = 0;
const FIRST_FILE_INO: usize = 1;

#[derive(Clone, Copy)]
pub struct InitrdEntry {
    pub name: &'static str,
    pub data: &'static [u8],
    pub mode: u16,
}

const INITRD_FILES: [InitrdEntry; 1] = [InitrdEntry {
    name: "/bin/init",
    data: loader::INIT_ELF,
    mode: 0o555,
}];

pub fn init() {}

pub fn file_count() -> usize {
    INITRD_FILES.len()
}

pub fn lookup(path: &[u8]) -> Option<InodeRef> {
    let path = trim_nul(path);
    if path == b"/" {
        return Some(inode(ROOT_INO));
    }

    INITRD_FILES
        .iter()
        .position(|file| path == file.name.as_bytes())
        .map(|index| inode(FIRST_FILE_INO + index))
}

pub fn metadata(inode_ref: InodeRef) -> Option<Metadata> {
    if inode_ref.fs != FileSystemId::Initrd {
        return None;
    }

    if inode_ref.ino == ROOT_INO {
        return Some(Metadata {
            inode: inode_ref,
            file_type: FileType::Directory,
            size: INITRD_FILES.len(),
            mode: 0o555,
        });
    }

    let file = file_by_ino(inode_ref.ino)?;
    Some(Metadata {
        inode: inode_ref,
        file_type: FileType::Regular,
        size: file.data.len(),
        mode: file.mode,
    })
}

pub fn open(inode_ref: InodeRef) -> Option<FileView> {
    if inode_ref.fs != FileSystemId::Initrd {
        return None;
    }

    let file = file_by_ino(inode_ref.ino)?;
    Some(FileView {
        inode: inode_ref,
        name: file.name,
        data: file.data,
    })
}

fn inode(ino: usize) -> InodeRef {
    InodeRef {
        fs: FileSystemId::Initrd,
        ino,
    }
}

fn file_by_ino(ino: usize) -> Option<InitrdEntry> {
    let index = ino.checked_sub(FIRST_FILE_INO)?;
    INITRD_FILES.get(index).copied()
}

fn trim_nul(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|byte| *byte == 0) {
        Some(end) => &bytes[..end],
        None => bytes,
    }
}
