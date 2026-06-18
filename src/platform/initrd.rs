#![allow(dead_code)]

use crate::config;
use crate::kernel::address::{align_down, align_up};
use crate::kernel::memory::{self, PAGE_SIZE};

pub const MAGIC: u32 = 0x4452_5352;
const HEADER_SIZE: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InitrdFormat {
    None,
    CpioNewc,
    TarUstar,
}

impl InitrdFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::CpioNewc => "cpio-newc",
            Self::TarUstar => "tar-ustar",
        }
    }
}

#[derive(Clone, Copy)]
pub struct InitrdImage {
    pub base: usize,
    pub data: usize,
    pub size: usize,
    pub format: InitrdFormat,
}

impl InitrdImage {
    pub const fn none() -> Self {
        Self {
            base: 0,
            data: 0,
            size: 0,
            format: InitrdFormat::None,
        }
    }

    pub const fn is_present(self) -> bool {
        self.size != 0 && !matches!(self.format, InitrdFormat::None)
    }

    pub unsafe fn bytes(self) -> &'static [u8] {
        if !self.is_present() {
            return &[];
        }
        unsafe { core::slice::from_raw_parts(self.data as *const u8, self.size) }
    }
}

static mut IMAGE: InitrdImage = InitrdImage::none();

pub fn init() -> InitrdImage {
    let image = detect();
    unsafe {
        IMAGE = image;
    }
    image
}

pub fn image() -> InitrdImage {
    unsafe { IMAGE }
}

fn detect() -> InitrdImage {
    if !config::CONFIG_INITRD_EXTERNAL {
        return InitrdImage::none();
    }

    let base = config::CONFIG_INITRD_LOAD_ADDR;
    let header = unsafe { core::slice::from_raw_parts(base as *const u8, HEADER_SIZE) };
    if read_le32(header, 0) != MAGIC {
        return InitrdImage::none();
    }

    let format = match read_le32(header, 8) {
        1 => InitrdFormat::CpioNewc,
        2 => InitrdFormat::TarUstar,
        _ => InitrdFormat::None,
    };
    let offset = read_le32(header, 12) as usize;
    let size = read_le32(header, 16) as usize;
    if offset < HEADER_SIZE || size == 0 {
        return InitrdImage::none();
    }

    InitrdImage {
        base,
        data: base + offset,
        size,
        format,
    }
}

pub fn reserve_loaded_pages(image: InitrdImage) {
    if !image.is_present() {
        return;
    }

    let start = align_down(image.base, PAGE_SIZE);
    let end = align_up(image.data + image.size, PAGE_SIZE);
    unsafe {
        memory::reserve_range(start, end - start);
    }
}

fn read_le32(bytes: &[u8], offset: usize) -> u32 {
    let mut value = [0u8; 4];
    value.copy_from_slice(&bytes[offset..offset + 4]);
    u32::from_le_bytes(value)
}
