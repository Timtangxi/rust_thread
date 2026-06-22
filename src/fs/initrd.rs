#![allow(dead_code)]

use crate::fs::vfs::{DirEntry, FileSystemId, FileType, FileView, InodeRef, Metadata};
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
        return Some(Metadata::new(
            inode_ref,
            FileType::Directory,
            INITRD_FILES.len(),
            0o555,
        ));
    }

    let file = file_by_ino(inode_ref.ino)?;
    Some(Metadata::new(
        inode_ref,
        FileType::Regular,
        file.data.len(),
        file.mode,
    ))
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

pub fn read_dir(inode_ref: InodeRef, offset: usize, dst: &mut [DirEntry]) -> Result<usize, u32> {
    if inode_ref.fs != FileSystemId::Initrd || inode_ref.ino != ROOT_INO {
        return Err(crate::kernel::syscall::EINVAL);
    }

    let mut written = 0usize;
    for (index, file) in INITRD_FILES.iter().enumerate().skip(offset) {
        if written >= dst.len() {
            break;
        }
        dst[written] = DirEntry::new(
            inode(FIRST_FILE_INO + index),
            FileType::Regular,
            trim_name(file.name.as_bytes()),
        );
        written += 1;
    }
    Ok(written)
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

fn trim_name(bytes: &[u8]) -> &[u8] {
    match bytes.iter().rposition(|byte| *byte == b'/') {
        Some(pos) => &bytes[pos + 1..],
        None => bytes,
    }
}
