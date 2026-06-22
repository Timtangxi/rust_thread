#![allow(dead_code)]

use crate::fs::vfs::{DirEntry, FileSystemId, FileType, FileView, InodeRef, Metadata};
use crate::kernel::memory;
use crate::kernel::syscall::ENOENT;

const ROOT_INO: usize = 0;
const MEMINFO_INO: usize = 1;
const MOUNTS_INO: usize = 2;
const SELF_INO: usize = 3;
const SELF_EXE_INO: usize = 4;

#[derive(Clone, Copy)]
struct ProcNode {
    ino: usize,
    name: &'static str,
}

const PROC_NODES: [ProcNode; 3] = [
    ProcNode {
        ino: MEMINFO_INO,
        name: "meminfo",
    },
    ProcNode {
        ino: MOUNTS_INO,
        name: "mounts",
    },
    ProcNode {
        ino: SELF_INO,
        name: "self",
    },
];

const SELF_NODES: [ProcNode; 1] = [ProcNode {
    ino: SELF_EXE_INO,
    name: "exe",
}];

static mut MEMINFO_BUF: [u8; 160] = [0; 160];
static MOUNTS: &[u8] = b"ramfs / ramfs rw 0 0\ninitrd /initrd initrd ro 0 0\ndevfs /dev devfs rw 0 0\nprocfs /proc procfs ro 0 0\n";
static SELF_EXE: &[u8] = b"/bin/init";

pub fn init() {}

pub fn lookup(path: &[u8]) -> Option<InodeRef> {
    let path = normalize(path);
    if path == b"proc" || path == b"proc/" {
        return Some(inode(ROOT_INO));
    }
    let name = path.strip_prefix(b"proc/")?;
    if name == b"self" || name == b"self/" {
        return Some(inode(SELF_INO));
    }
    if let Some(name) = name.strip_prefix(b"self/") {
        return SELF_NODES
            .iter()
            .find(|node| name == node.name.as_bytes())
            .map(|node| inode(node.ino));
    }
    PROC_NODES
        .iter()
        .find(|node| name == node.name.as_bytes())
        .map(|node| inode(node.ino))
}

pub fn metadata(inode_ref: InodeRef) -> Option<Metadata> {
    if inode_ref.fs != FileSystemId::Procfs {
        return None;
    }
    if inode_ref.ino == ROOT_INO {
        return Some(Metadata::new(
            inode_ref,
            FileType::Directory,
            PROC_NODES.len(),
            0o555,
        ));
    }
    if inode_ref.ino == SELF_INO {
        return Some(Metadata::new(
            inode_ref,
            FileType::Directory,
            SELF_NODES.len(),
            0o555,
        ));
    }
    let size = match inode_ref.ino {
        MEMINFO_INO => render_meminfo().len(),
        MOUNTS_INO => MOUNTS.len(),
        SELF_EXE_INO => SELF_EXE.len(),
        _ => return None,
    };
    Some(Metadata::new(
        inode_ref,
        if inode_ref.ino == SELF_EXE_INO {
            FileType::Symlink
        } else {
            FileType::Regular
        },
        size,
        0o444,
    ))
}

pub fn open(inode_ref: InodeRef) -> Option<FileView> {
    if inode_ref.fs != FileSystemId::Procfs {
        return None;
    }

    match inode_ref.ino {
        MEMINFO_INO => Some(FileView {
            inode: inode_ref,
            name: "meminfo",
            data: render_meminfo(),
        }),
        MOUNTS_INO => Some(FileView {
            inode: inode_ref,
            name: "mounts",
            data: MOUNTS,
        }),
        SELF_EXE_INO => Some(FileView {
            inode: inode_ref,
            name: "exe",
            data: SELF_EXE,
        }),
        _ => None,
    }
}

pub fn read_dir(inode_ref: InodeRef, offset: usize, dst: &mut [DirEntry]) -> Result<usize, u32> {
    if inode_ref.fs != FileSystemId::Procfs {
        return Err(ENOENT);
    }

    let nodes = match inode_ref.ino {
        ROOT_INO => PROC_NODES.as_slice(),
        SELF_INO => SELF_NODES.as_slice(),
        _ => return Err(ENOENT),
    };
    let mut written = 0usize;
    for node in nodes.iter().skip(offset) {
        if written >= dst.len() {
            break;
        }
        let file_type = if node.ino == SELF_INO {
            FileType::Directory
        } else if node.ino == SELF_EXE_INO {
            FileType::Symlink
        } else {
            FileType::Regular
        };
        dst[written] = DirEntry::new(inode(node.ino), file_type, node.name.as_bytes());
        written += 1;
    }
    Ok(written)
}

fn render_meminfo() -> &'static [u8] {
    unsafe {
        let buf = &raw mut MEMINFO_BUF;
        let written = write_meminfo(&mut *buf);
        let slice = &*buf;
        &slice[..written]
    }
}

fn write_meminfo(buf: &mut [u8]) -> usize {
    let mut writer = ByteWriter::new(buf);
    writer.write_str("MemTotalPages: ");
    writer.write_usize(memory::total_pages());
    writer.write_str("\nMemFreePages: ");
    writer.write_usize(memory::free_page_count());
    writer.write_str("\nMemReservedPages: ");
    writer.write_usize(memory::reserved_pages());
    writer.write_str("\nMemAllocatedPages: ");
    writer.write_usize(memory::allocated_pages());
    writer.write_str("\n");
    writer.len()
}

struct ByteWriter<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl<'a> ByteWriter<'a> {
    const fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, len: 0 }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn write_str(&mut self, value: &str) {
        self.write_bytes(value.as_bytes());
    }

    fn write_usize(&mut self, mut value: usize) {
        let mut tmp = [0u8; 20];
        let mut len = 0usize;
        if value == 0 {
            self.write_bytes(b"0");
            return;
        }
        while value != 0 {
            tmp[len] = b'0' + (value % 10) as u8;
            value /= 10;
            len += 1;
        }
        while len != 0 {
            len -= 1;
            self.write_bytes(&tmp[len..len + 1]);
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        let count = bytes.len().min(self.buf.len().saturating_sub(self.len));
        self.buf[self.len..self.len + count].copy_from_slice(&bytes[..count]);
        self.len += count;
    }
}

fn inode(ino: usize) -> InodeRef {
    InodeRef {
        fs: FileSystemId::Procfs,
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
