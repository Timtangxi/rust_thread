#![allow(dead_code)]

use crate::fs::vfs::{DirEntry, FileSystemId, FileType, FileView, InodeRef, Metadata};
use crate::kernel::syscall::{EINVAL, ENOENT};

const ROOT_INO: usize = 0;
const DEV_CONSOLE_INO: usize = 1;
const DEV_NULL_INO: usize = 2;
const DEV_ZERO_INO: usize = 3;
const DEV_URANDOM_INO: usize = 4;
const DEV_TTY_INO: usize = 5;

#[derive(Clone, Copy)]
struct DevNode {
    ino: usize,
    name: &'static str,
    file_type: FileType,
    mode: u16,
}

const DEV_NODES: [DevNode; 5] = [
    DevNode {
        ino: DEV_CONSOLE_INO,
        name: "console",
        file_type: FileType::Device,
        mode: 0o666,
    },
    DevNode {
        ino: DEV_NULL_INO,
        name: "null",
        file_type: FileType::Device,
        mode: 0o666,
    },
    DevNode {
        ino: DEV_ZERO_INO,
        name: "zero",
        file_type: FileType::Device,
        mode: 0o666,
    },
    DevNode {
        ino: DEV_URANDOM_INO,
        name: "urandom",
        file_type: FileType::Device,
        mode: 0o444,
    },
    DevNode {
        ino: DEV_TTY_INO,
        name: "tty",
        file_type: FileType::Device,
        mode: 0o666,
    },
];

static mut URANDOM_STATE: u32 = 0x6d2b_79f5;

pub fn init() {}

pub fn lookup(path: &[u8]) -> Option<InodeRef> {
    let path = normalize(path);
    if path == b"dev" || path == b"dev/" {
        return Some(inode(ROOT_INO));
    }
    let name = path.strip_prefix(b"dev/")?;
    DEV_NODES
        .iter()
        .find(|node| name == node.name.as_bytes())
        .map(|node| inode(node.ino))
}

pub fn metadata(inode_ref: InodeRef) -> Option<Metadata> {
    if inode_ref.fs != FileSystemId::Devfs {
        return None;
    }
    if inode_ref.ino == ROOT_INO {
        return Some(Metadata::new(
            inode_ref,
            FileType::Directory,
            DEV_NODES.len(),
            0o755,
        ));
    }
    let node = DEV_NODES.iter().find(|node| node.ino == inode_ref.ino)?;
    Some(Metadata::new(inode_ref, node.file_type, 0, node.mode))
}

pub fn open(inode_ref: InodeRef) -> Option<FileView> {
    if inode_ref.fs != FileSystemId::Devfs {
        return None;
    }
    let node = DEV_NODES.iter().find(|node| node.ino == inode_ref.ino)?;
    Some(FileView {
        inode: inode_ref,
        name: node.name,
        data: &[],
    })
}

pub fn write(inode_ref: InodeRef, _offset: usize, src: &[u8]) -> Result<usize, u32> {
    match inode_ref.ino {
        DEV_NULL_INO => Ok(src.len()),
        DEV_CONSOLE_INO | DEV_TTY_INO => {
            for byte in src {
                if *byte == b'\n' {
                    crate::drivers::uart::put_byte(b'\r');
                }
                crate::drivers::uart::put_byte(*byte);
            }
            Ok(src.len())
        }
        _ => Err(EINVAL),
    }
}

pub fn read(inode_ref: InodeRef, _offset: usize, dst: &mut [u8]) -> Result<usize, u32> {
    match inode_ref.ino {
        DEV_NULL_INO | DEV_CONSOLE_INO | DEV_TTY_INO => Ok(0),
        DEV_ZERO_INO => {
            dst.fill(0);
            Ok(dst.len())
        }
        DEV_URANDOM_INO => {
            fill_urandom(dst);
            Ok(dst.len())
        }
        _ => Err(EINVAL),
    }
}

pub fn read_dir(inode_ref: InodeRef, offset: usize, dst: &mut [DirEntry]) -> Result<usize, u32> {
    if inode_ref.fs != FileSystemId::Devfs || inode_ref.ino != ROOT_INO {
        return Err(ENOENT);
    }

    let mut written = 0usize;
    for node in DEV_NODES.iter().skip(offset) {
        if written >= dst.len() {
            break;
        }
        dst[written] = DirEntry::new(inode(node.ino), node.file_type, node.name.as_bytes());
        written += 1;
    }
    Ok(written)
}

fn inode(ino: usize) -> InodeRef {
    InodeRef {
        fs: FileSystemId::Devfs,
        ino,
    }
}

fn normalize(path: &[u8]) -> &[u8] {
    let path = trim_nul(path);
    let path = path.strip_prefix(b"/").unwrap_or(path);
    path.strip_prefix(b"./").unwrap_or(path)
}

fn trim_nul(bytes: &[u8]) -> &[u8] {
    match bytes.iter().position(|byte| *byte == 0) {
        Some(end) => &bytes[..end],
        None => bytes,
    }
}

fn fill_urandom(dst: &mut [u8]) {
    unsafe {
        let mut state = URANDOM_STATE
            ^ crate::kernel::memory::free_page_count() as u32
            ^ crate::drivers::timer::counter() as u32;
        for byte in dst {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *byte = (state >> 24) as u8;
        }
        URANDOM_STATE = state.wrapping_add(0x9e37_79b9);
    }
}
