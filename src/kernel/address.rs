#![allow(dead_code)]

use crate::kernel::memory::PAGE_SIZE;

pub const USER_TOP: usize = 0x8000_0000;
pub const KERNEL_HIGH_BASE: usize = 0x8000_0000;
pub const DEVICE_HIGH_BASE: usize = 0xc000_0000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PageFrameNumber(usize);

impl PhysAddr {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub const fn pfn(self) -> PageFrameNumber {
        PageFrameNumber(self.0 / PAGE_SIZE)
    }

    pub const fn is_aligned(self, align: usize) -> bool {
        self.0 & (align - 1) == 0
    }
}

impl VirtAddr {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub const fn is_user(self) -> bool {
        self.0 < USER_TOP
    }
}

impl PageFrameNumber {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn index(self) -> usize {
        self.0
    }

    pub const fn addr(self) -> PhysAddr {
        PhysAddr(self.0 * PAGE_SIZE)
    }
}

pub const fn align_down(value: usize, align: usize) -> usize {
    value & !(align - 1)
}

pub const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}
