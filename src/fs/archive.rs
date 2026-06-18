#![allow(dead_code)]

use crate::fs::vfs::FileType;
use crate::platform::initrd::{self, InitrdFormat};

const CPIO_HEADER: usize = 110;
const TAR_BLOCK: usize = 512;

#[derive(Clone, Copy)]
pub struct ArchiveEntry {
    pub index: usize,
    name: &'static str,
    pub file_type: FileType,
    pub mode: u16,
    pub size: usize,
    pub data: &'static [u8],
}

impl ArchiveEntry {
    pub const fn name(self) -> &'static str {
        self.name
    }
}

pub fn is_mounted() -> bool {
    initrd::image().is_present()
}

pub fn format() -> InitrdFormat {
    initrd::image().format
}

pub fn file_count() -> usize {
    let image = initrd::image();
    if !image.is_present() {
        return 0;
    }

    let bytes = unsafe { image.bytes() };
    let mut cursor = Cursor::new(bytes, image.format);
    let mut count = 0;
    while let Some(entry) = cursor.next() {
        if entry.file_type == FileType::Regular {
            count += 1;
        }
    }
    count
}

pub fn for_each_entry(mut f: impl FnMut(ArchiveEntry) -> Result<(), u32>) -> Result<usize, u32> {
    let image = initrd::image();
    if !image.is_present() {
        return Ok(0);
    }

    let bytes = unsafe { image.bytes() };
    let mut cursor = Cursor::new(bytes, image.format);
    let mut count = 0;
    while let Some(entry) = cursor.next() {
        f(entry)?;
        count += 1;
    }
    Ok(count)
}

struct Cursor {
    bytes: &'static [u8],
    format: InitrdFormat,
    offset: usize,
    index: usize,
}

impl Cursor {
    const fn new(bytes: &'static [u8], format: InitrdFormat) -> Self {
        Self {
            bytes,
            format,
            offset: 0,
            index: 0,
        }
    }

    fn next(&mut self) -> Option<ArchiveEntry> {
        loop {
            let entry = match self.format {
                InitrdFormat::CpioNewc => self.next_cpio()?,
                InitrdFormat::TarUstar => self.next_tar()?,
                InitrdFormat::None => return None,
            };

            if entry.name.is_empty() || entry.name == "." || entry.name.ends_with('/') {
                continue;
            }
            return Some(entry);
        }
    }

    fn next_cpio(&mut self) -> Option<ArchiveEntry> {
        if self.offset + CPIO_HEADER > self.bytes.len() {
            return None;
        }
        let header = &self.bytes[self.offset..self.offset + CPIO_HEADER];
        if &header[..6] != b"070701" && &header[..6] != b"070702" {
            return None;
        }

        let mode = parse_hex(header, 14, 8)? as u32;
        let file_size = parse_hex(header, 54, 8)?;
        let name_size = parse_hex(header, 94, 8)?;
        let name_start = self.offset + CPIO_HEADER;
        let name_end = name_start.checked_add(name_size)?;
        if name_size == 0 || name_end > self.bytes.len() {
            return None;
        }

        let name_bytes = trim_nul(&self.bytes[name_start..name_end]);
        if name_bytes == b"TRAILER!!!" {
            return None;
        }
        let data_start = align4(name_end);
        let data_end = data_start.checked_add(file_size)?;
        if data_end > self.bytes.len() {
            return None;
        }

        self.offset = align4(data_end);
        let index = self.index;
        self.index += 1;
        Some(ArchiveEntry {
            index,
            name: bytes_to_static_str(normalize_entry_name(name_bytes))?,
            file_type: cpio_file_type(mode),
            mode: (mode & 0o7777) as u16,
            size: file_size,
            data: &self.bytes[data_start..data_end],
        })
    }

    fn next_tar(&mut self) -> Option<ArchiveEntry> {
        while self.offset + TAR_BLOCK <= self.bytes.len() {
            let header = &self.bytes[self.offset..self.offset + TAR_BLOCK];
            self.offset += TAR_BLOCK;
            if header.iter().all(|byte| *byte == 0) {
                return None;
            }

            let size = parse_tar_octal(&header[124..136])?;
            let typeflag = header[156];
            let data_start = self.offset;
            let data_end = data_start.checked_add(size)?;
            if data_end > self.bytes.len() {
                return None;
            }
            self.offset = align512(data_end);

            let name = tar_name(header)?;
            let index = self.index;
            self.index += 1;
            return Some(ArchiveEntry {
                index,
                name,
                file_type: tar_file_type(typeflag),
                mode: parse_tar_octal(&header[100..108]).unwrap_or(0) as u16,
                size,
                data: &self.bytes[data_start..data_end],
            });
        }
        None
    }
}

fn cpio_file_type(mode: u32) -> FileType {
    match mode & 0o170000 {
        0o040000 => FileType::Directory,
        0o120000 => FileType::Symlink,
        0o020000 | 0o060000 => FileType::Device,
        _ => FileType::Regular,
    }
}

fn tar_file_type(typeflag: u8) -> FileType {
    match typeflag {
        b'5' => FileType::Directory,
        b'2' => FileType::Symlink,
        b'3' | b'4' | b'6' => FileType::Device,
        _ => FileType::Regular,
    }
}

fn tar_name(header: &'static [u8]) -> Option<&'static str> {
    let name = trim_nul(&header[..100]);
    let prefix = trim_nul(&header[345..500]);
    if prefix.is_empty() {
        return bytes_to_static_str(normalize_entry_name(name));
    }
    bytes_to_static_str(normalize_entry_name(name))
}

fn normalize_entry_name(path: &[u8]) -> &[u8] {
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

fn parse_hex(bytes: &[u8], offset: usize, len: usize) -> Option<usize> {
    let mut value = 0usize;
    for byte in &bytes[offset..offset + len] {
        value <<= 4;
        value |= match *byte {
            b'0'..=b'9' => (*byte - b'0') as usize,
            b'a'..=b'f' => (*byte - b'a' + 10) as usize,
            b'A'..=b'F' => (*byte - b'A' + 10) as usize,
            _ => return None,
        };
    }
    Some(value)
}

fn parse_tar_octal(bytes: &[u8]) -> Option<usize> {
    let mut value = 0usize;
    for byte in bytes {
        match *byte {
            0 | b' ' => {}
            b'0'..=b'7' => value = (value << 3) + (*byte - b'0') as usize,
            _ => return None,
        }
    }
    Some(value)
}

fn bytes_to_static_str(bytes: &'static [u8]) -> Option<&'static str> {
    core::str::from_utf8(bytes).ok()
}

const fn align4(value: usize) -> usize {
    (value + 3) & !3
}

const fn align512(value: usize) -> usize {
    (value + TAR_BLOCK - 1) & !(TAR_BLOCK - 1)
}
