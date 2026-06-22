#![allow(dead_code)]

use core::mem;

use crate::fs::vfs::InodeRef;
use crate::kernel::address::{PhysAddr, USER_TOP, VirtAddr};
use crate::kernel::ipc::{PipeEnd, PipeId};
use crate::kernel::memory::PAGE_SIZE;

pub const MAX_PROCESSES: usize = 16;
pub const MAX_FILES: usize = 32;
pub const MAX_FILE_HANDLES: usize = 64;
pub const MAX_ADDRESS_SPACE_L2: usize = 128;
pub const MAX_ADDRESS_SPACE_REGIONS: usize = 128;
pub const MAX_VMAS: usize = 128;
pub const USER_STACK_PAGES: usize = 32;
pub const USER_STACK_SIZE: usize = USER_STACK_PAGES * PAGE_SIZE;
pub const USER_STACK_SLOT_SIZE: usize = 1024 * 1024;
pub const USER_MMAP_BASE: usize = 0x5000_0000;
pub const USER_MMAP_TOP: usize = 0x7000_0000;

pub const VM_READ: u32 = 1 << 0;
pub const VM_WRITE: u32 = 1 << 1;
pub const VM_EXEC: u32 = 1 << 2;
pub const VM_USER: u32 = 1 << 3;
pub const VM_STACK: u32 = 1 << 4;
pub const VM_ANON: u32 = 1 << 5;
pub const VM_FILE: u32 = 1 << 6;
pub const VM_HEAP: u32 = 1 << 7;
pub const VM_MMAP: u32 = 1 << 8;
pub const VM_FIXED: u32 = 1 << 9;

pub const FD_CLOEXEC: u32 = 1 << 0;
pub const CWD_MAX: usize = 96;

#[derive(Clone, Copy)]
pub struct Credentials {
    pub uid: u32,
    pub euid: u32,
    pub gid: u32,
    pub egid: u32,
    pub umask: u16,
}

impl Credentials {
    pub const fn root() -> Self {
        Self {
            uid: 0,
            euid: 0,
            gid: 0,
            egid: 0,
            umask: 0o022,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ProcessId(usize);

impl ProcessId {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ThreadId(usize);

impl ThreadId {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThreadMode {
    Kernel,
    User,
}

impl ThreadMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::User => "user",
        }
    }

    pub const fn is_user(self) -> bool {
        matches!(self, Self::User)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Empty,
    Alive,
    Zombie,
}

impl ProcessState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Alive => "alive",
            Self::Zombie => "zombie",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceKind {
    Kernel,
    User,
}

impl AddressSpaceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::User => "user",
        }
    }

    pub const fn is_user(self) -> bool {
        matches!(self, Self::User)
    }
}

#[derive(Clone, Copy)]
pub struct AddressSpace {
    pub kind: AddressSpaceKind,
    pub page_table_root: PhysAddr,
    pub asid: u16,
    pub l1_pages: usize,
    pub owned_l2: [PhysAddr; MAX_ADDRESS_SPACE_L2],
    pub owned_l2_count: usize,
    pub owned_regions: [UserRegion; MAX_ADDRESS_SPACE_REGIONS],
    pub owned_region_count: usize,
    pub vmas: [VmArea; MAX_VMAS],
    pub vma_count: usize,
    pub entry: VirtAddr,
    pub user_start: VirtAddr,
    pub user_end: VirtAddr,
    pub brk_start: VirtAddr,
    pub brk: VirtAddr,
    pub mmap_base: VirtAddr,
    pub mmap_next: VirtAddr,
    pub phdr: VirtAddr,
    pub phnum: usize,
    pub phent: usize,
    pub interp_base: VirtAddr,
    pub interp_entry: VirtAddr,
    pub stack_bottom: VirtAddr,
    pub stack_top: VirtAddr,
}

impl AddressSpace {
    pub const fn kernel(page_table_root: PhysAddr) -> Self {
        Self {
            kind: AddressSpaceKind::Kernel,
            page_table_root,
            asid: 0,
            l1_pages: 0,
            owned_l2: [PhysAddr::new(0); MAX_ADDRESS_SPACE_L2],
            owned_l2_count: 0,
            owned_regions: [const { UserRegion::empty() }; MAX_ADDRESS_SPACE_REGIONS],
            owned_region_count: 0,
            vmas: [const { VmArea::empty() }; MAX_VMAS],
            vma_count: 0,
            entry: VirtAddr::new(0),
            user_start: VirtAddr::new(0),
            user_end: VirtAddr::new(0),
            brk_start: VirtAddr::new(0),
            brk: VirtAddr::new(0),
            mmap_base: VirtAddr::new(0),
            mmap_next: VirtAddr::new(0),
            phdr: VirtAddr::new(0),
            phnum: 0,
            phent: 0,
            interp_base: VirtAddr::new(0),
            interp_entry: VirtAddr::new(0),
            stack_bottom: VirtAddr::new(0),
            stack_top: VirtAddr::new(0),
        }
    }

    pub const fn user(
        page_table_root: PhysAddr,
        asid: u16,
        l1_pages: usize,
        entry: VirtAddr,
        user_start: VirtAddr,
        user_end: VirtAddr,
        stack_bottom: VirtAddr,
        stack_top: VirtAddr,
    ) -> Self {
        Self {
            kind: AddressSpaceKind::User,
            page_table_root,
            asid,
            l1_pages,
            owned_l2: [PhysAddr::new(0); MAX_ADDRESS_SPACE_L2],
            owned_l2_count: 0,
            owned_regions: [const { UserRegion::empty() }; MAX_ADDRESS_SPACE_REGIONS],
            owned_region_count: 0,
            vmas: [const { VmArea::empty() }; MAX_VMAS],
            vma_count: 0,
            entry,
            user_start,
            user_end,
            brk_start: VirtAddr::new(0),
            brk: VirtAddr::new(0),
            mmap_base: VirtAddr::new(USER_MMAP_BASE),
            mmap_next: VirtAddr::new(USER_MMAP_BASE),
            phdr: VirtAddr::new(0),
            phnum: 0,
            phent: 0,
            interp_base: VirtAddr::new(0),
            interp_entry: VirtAddr::new(0),
            stack_bottom,
            stack_top,
        }
    }

    pub fn add_owned_l2(&mut self, table: PhysAddr) {
        self.try_add_owned_l2(table)
            .unwrap_or_else(|_| panic!("address space l2 ownership table full"));
    }

    pub fn try_add_owned_l2(&mut self, table: PhysAddr) -> Result<(), u32> {
        if table.as_usize() == 0 {
            return Ok(());
        }

        if self.owned_l2_count >= MAX_ADDRESS_SPACE_L2 {
            return Err(crate::kernel::syscall::ENOMEM);
        }

        self.owned_l2[self.owned_l2_count] = table;
        self.owned_l2_count += 1;
        Ok(())
    }

    pub fn add_owned_region(&mut self, virt: VirtAddr, phys: PhysAddr, pages: usize) {
        self.try_add_owned_region(virt, phys, pages)
            .unwrap_or_else(|_| panic!("address space region ownership table full"));
    }

    pub fn try_add_owned_region(
        &mut self,
        virt: VirtAddr,
        phys: PhysAddr,
        pages: usize,
    ) -> Result<(), u32> {
        if pages == 0 {
            return Ok(());
        }

        if self.owned_region_count >= MAX_ADDRESS_SPACE_REGIONS {
            return Err(crate::kernel::syscall::ENOMEM);
        }

        self.owned_regions[self.owned_region_count] = UserRegion { virt, phys, pages };
        self.owned_region_count += 1;
        Ok(())
    }

    pub fn remove_owned_region(&mut self, virt: VirtAddr, pages: usize) -> Option<UserRegion> {
        let end = virt.as_usize().checked_add(pages * PAGE_SIZE)?;
        for index in 0..self.owned_region_count {
            let region = self.owned_regions[index];
            if region.virt == virt && region.pages == pages {
                let removed = region;
                let mut cursor = index;
                while cursor + 1 < self.owned_region_count {
                    self.owned_regions[cursor] = self.owned_regions[cursor + 1];
                    cursor += 1;
                }
                self.owned_region_count -= 1;
                self.owned_regions[self.owned_region_count] = UserRegion::empty();
                return Some(removed);
            }

            if virt.as_usize() >= region.virt.as_usize()
                && end <= region.virt.as_usize() + region.pages * PAGE_SIZE
            {
                return None;
            }
        }
        None
    }

    pub fn remove_owned_range(&mut self, start: VirtAddr, pages: usize) -> Result<UserRegion, u32> {
        let len = pages
            .checked_mul(PAGE_SIZE)
            .ok_or(crate::kernel::syscall::EINVAL)?;
        let end = start
            .as_usize()
            .checked_add(len)
            .ok_or(crate::kernel::syscall::EINVAL)?;

        for index in 0..self.owned_region_count {
            let region = self.owned_regions[index];
            let region_start = region.virt.as_usize();
            let region_end = region_start + region.pages * PAGE_SIZE;
            if start.as_usize() < region_start || end > region_end {
                continue;
            }

            let phys_offset = start.as_usize() - region_start;
            let removed = UserRegion {
                virt: start,
                phys: PhysAddr::new(region.phys.as_usize() + phys_offset),
                pages,
            };
            let prefix_pages = phys_offset / PAGE_SIZE;
            let suffix_pages = (region_end - end) / PAGE_SIZE;

            if prefix_pages == 0 && suffix_pages == 0 {
                self.shift_owned_regions_left(index);
            } else if prefix_pages != 0 && suffix_pages == 0 {
                self.owned_regions[index].pages = prefix_pages;
            } else if prefix_pages == 0 {
                self.owned_regions[index] = UserRegion {
                    virt: VirtAddr::new(end),
                    phys: PhysAddr::new(region.phys.as_usize() + phys_offset + len),
                    pages: suffix_pages,
                };
            } else {
                if self.owned_region_count >= MAX_ADDRESS_SPACE_REGIONS {
                    return Err(crate::kernel::syscall::ENOMEM);
                }
                self.owned_regions[index].pages = prefix_pages;
                self.owned_regions[self.owned_region_count] = UserRegion {
                    virt: VirtAddr::new(end),
                    phys: PhysAddr::new(region.phys.as_usize() + phys_offset + len),
                    pages: suffix_pages,
                };
                self.owned_region_count += 1;
                self.sort_owned_regions();
            }
            return Ok(removed);
        }

        Err(crate::kernel::syscall::EINVAL)
    }

    pub fn add_vma(&mut self, start: VirtAddr, end: VirtAddr, flags: u32, file: Option<InodeRef>) {
        self.try_add_vma(start, end, flags, file)
            .unwrap_or_else(|_| panic!("address space vma table full"));
    }

    pub fn try_add_vma(
        &mut self,
        start: VirtAddr,
        end: VirtAddr,
        flags: u32,
        file: Option<InodeRef>,
    ) -> Result<(), u32> {
        if start.as_usize() >= end.as_usize() {
            return Err(crate::kernel::syscall::EINVAL);
        }

        if end.as_usize() > USER_TOP {
            return Err(crate::kernel::syscall::EFAULT);
        }

        if self.vma_count >= MAX_VMAS {
            return Err(crate::kernel::syscall::ENOMEM);
        }

        for vma in self.vmas.iter().take(self.vma_count) {
            if start.as_usize() < vma.end.as_usize() && end.as_usize() > vma.start.as_usize() {
                return Err(crate::kernel::syscall::EEXIST);
            }
        }

        self.vmas[self.vma_count] = VmArea {
            start,
            end,
            flags,
            file,
        };
        self.vma_count += 1;
        self.sort_vmas();
        Ok(())
    }

    pub fn find_vma(&self, addr: VirtAddr) -> Option<VmArea> {
        self.vmas
            .iter()
            .take(self.vma_count)
            .copied()
            .find(|vma| vma.contains(addr))
    }

    pub fn remove_vma_exact(&mut self, start: VirtAddr, end: VirtAddr) -> Option<VmArea> {
        for index in 0..self.vma_count {
            let vma = self.vmas[index];
            if vma.start == start && vma.end == end {
                let removed = vma;
                self.shift_vmas_left(index);
                return Some(removed);
            }
        }
        None
    }

    pub fn unmap_vma_range(&mut self, start: VirtAddr, end: VirtAddr) -> Result<(), u32> {
        if start.as_usize() >= end.as_usize() {
            return Err(crate::kernel::syscall::EINVAL);
        }

        let mut cursor = start.as_usize();
        while cursor < end.as_usize() {
            let Some(vma) = self.find_vma(VirtAddr::new(cursor)) else {
                return Err(crate::kernel::syscall::EINVAL);
            };
            if vma.start.as_usize() > cursor {
                return Err(crate::kernel::syscall::EINVAL);
            }
            cursor = vma.end.as_usize().min(end.as_usize());
        }

        let mut index = 0usize;
        while index < self.vma_count {
            let vma = self.vmas[index];
            if end.as_usize() <= vma.start.as_usize() || start.as_usize() >= vma.end.as_usize() {
                index += 1;
                continue;
            }

            let left = start.as_usize() > vma.start.as_usize();
            let right = end.as_usize() < vma.end.as_usize();
            match (left, right) {
                (false, false) => {
                    self.shift_vmas_left(index);
                }
                (true, false) => {
                    self.vmas[index].end = start;
                    index += 1;
                }
                (false, true) => {
                    self.vmas[index].start = end;
                    index += 1;
                }
                (true, true) => {
                    if self.vma_count >= MAX_VMAS {
                        return Err(crate::kernel::syscall::ENOMEM);
                    }
                    let right_vma = VmArea {
                        start: end,
                        end: vma.end,
                        flags: vma.flags,
                        file: vma.file,
                    };
                    self.vmas[index].end = start;
                    self.vmas[self.vma_count] = right_vma;
                    self.vma_count += 1;
                    self.sort_vmas();
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn unmap_vma_range_lenient(&mut self, start: VirtAddr, end: VirtAddr) -> Result<(), u32> {
        if start.as_usize() >= end.as_usize() {
            return Err(crate::kernel::syscall::EINVAL);
        }

        let mut index = 0usize;
        while index < self.vma_count {
            let vma = self.vmas[index];
            if end.as_usize() <= vma.start.as_usize() || start.as_usize() >= vma.end.as_usize() {
                index += 1;
                continue;
            }

            let left = start.as_usize() > vma.start.as_usize();
            let right = end.as_usize() < vma.end.as_usize();
            match (left, right) {
                (false, false) => {
                    self.shift_vmas_left(index);
                }
                (true, false) => {
                    self.vmas[index].end = start;
                    index += 1;
                }
                (false, true) => {
                    self.vmas[index].start = end;
                    index += 1;
                }
                (true, true) => {
                    if self.vma_count >= MAX_VMAS {
                        return Err(crate::kernel::syscall::ENOMEM);
                    }
                    let right_vma = VmArea {
                        start: end,
                        end: vma.end,
                        flags: vma.flags,
                        file: vma.file,
                    };
                    self.vmas[index].end = start;
                    self.vmas[self.vma_count] = right_vma;
                    self.vma_count += 1;
                    self.sort_vmas();
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn protect_vma_range(
        &mut self,
        start: VirtAddr,
        end: VirtAddr,
        prot_flags: u32,
    ) -> Result<(), u32> {
        if start.as_usize() >= end.as_usize() {
            return Err(crate::kernel::syscall::EINVAL);
        }

        let mut cursor = start.as_usize();
        while cursor < end.as_usize() {
            let Some(vma) = self.find_vma(VirtAddr::new(cursor)) else {
                return Err(crate::kernel::syscall::EINVAL);
            };
            if vma.start.as_usize() > cursor {
                return Err(crate::kernel::syscall::EINVAL);
            }
            cursor = vma.end.as_usize().min(end.as_usize());
        }

        let mut index = 0usize;
        while index < self.vma_count {
            let vma = self.vmas[index];
            if end.as_usize() <= vma.start.as_usize() || start.as_usize() >= vma.end.as_usize() {
                index += 1;
                continue;
            }

            if start.as_usize() > vma.start.as_usize() {
                if self.vma_count >= MAX_VMAS {
                    return Err(crate::kernel::syscall::ENOMEM);
                }
                let right = VmArea {
                    start,
                    end: vma.end,
                    flags: vma.flags,
                    file: vma.file,
                };
                self.vmas[index].end = start;
                self.vmas[self.vma_count] = right;
                self.vma_count += 1;
                self.sort_vmas();
                continue;
            }

            if end.as_usize() < vma.end.as_usize() {
                if self.vma_count >= MAX_VMAS {
                    return Err(crate::kernel::syscall::ENOMEM);
                }
                let right = VmArea {
                    start: end,
                    end: vma.end,
                    flags: vma.flags,
                    file: vma.file,
                };
                self.vmas[index].end = end;
                self.vmas[self.vma_count] = right;
                self.vma_count += 1;
                self.sort_vmas();
                continue;
            }

            self.vmas[index].flags =
                (self.vmas[index].flags & !(VM_READ | VM_WRITE | VM_EXEC)) | prot_flags | VM_USER;
            index += 1;
        }
        self.merge_adjacent_vmas();
        Ok(())
    }

    pub fn contains_user_range(&self, start: VirtAddr, len: usize, write: bool) -> bool {
        if len == 0 {
            return true;
        }

        let Some(end) = start.as_usize().checked_add(len) else {
            return false;
        };
        let mut cursor = start.as_usize();
        while cursor < end {
            let Some(vma) = self.find_vma(VirtAddr::new(cursor)) else {
                return false;
            };
            if write && !vma.can_write() {
                return false;
            }
            if !write && !vma.can_read() {
                return false;
            }
            cursor = vma.end.as_usize().min(end);
        }
        true
    }

    fn sort_vmas(&mut self) {
        let mut i = 1;
        while i < self.vma_count {
            let current = self.vmas[i];
            let mut j = i;
            while j > 0 && self.vmas[j - 1].start.as_usize() > current.start.as_usize() {
                self.vmas[j] = self.vmas[j - 1];
                j -= 1;
            }
            self.vmas[j] = current;
            i += 1;
        }
    }

    fn shift_vmas_left(&mut self, index: usize) {
        let mut cursor = index;
        while cursor + 1 < self.vma_count {
            self.vmas[cursor] = self.vmas[cursor + 1];
            cursor += 1;
        }
        self.vma_count -= 1;
        self.vmas[self.vma_count] = VmArea::empty();
    }

    fn merge_adjacent_vmas(&mut self) {
        self.sort_vmas();
        let mut index = 0usize;
        while index + 1 < self.vma_count {
            let current = self.vmas[index];
            let next = self.vmas[index + 1];
            if current.end == next.start && current.flags == next.flags && current.file == next.file
            {
                self.vmas[index].end = next.end;
                self.shift_vmas_left(index + 1);
            } else {
                index += 1;
            }
        }
    }

    fn sort_owned_regions(&mut self) {
        let mut i = 1;
        while i < self.owned_region_count {
            let current = self.owned_regions[i];
            let mut j = i;
            while j > 0 && self.owned_regions[j - 1].virt.as_usize() > current.virt.as_usize() {
                self.owned_regions[j] = self.owned_regions[j - 1];
                j -= 1;
            }
            self.owned_regions[j] = current;
            i += 1;
        }
    }

    fn shift_owned_regions_left(&mut self, index: usize) {
        let mut cursor = index;
        while cursor + 1 < self.owned_region_count {
            self.owned_regions[cursor] = self.owned_regions[cursor + 1];
            cursor += 1;
        }
        self.owned_region_count -= 1;
        self.owned_regions[self.owned_region_count] = UserRegion::empty();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileObject {
    Closed,
    Console,
    ConsoleIn,
    ConsoleOut,
    ConsoleErr,
    Regular { inode: InodeRef },
    Directory { inode: InodeRef },
    Device { inode: InodeRef },
    Pipe { id: PipeId, end: PipeEnd },
}

impl FileObject {
    pub const fn can_read(self) -> bool {
        matches!(
            self,
            Self::Console
                | Self::ConsoleIn
                | Self::Regular { .. }
                | Self::Directory { .. }
                | Self::Device { .. }
                | Self::Pipe {
                    end: PipeEnd::Read,
                    ..
                }
        )
    }

    pub const fn can_write(self) -> bool {
        matches!(
            self,
            Self::Console
                | Self::ConsoleOut
                | Self::ConsoleErr
                | Self::Regular { .. }
                | Self::Device { .. }
                | Self::Pipe {
                    end: PipeEnd::Write,
                    ..
                }
        )
    }

    pub const fn inode(self) -> Option<InodeRef> {
        match self {
            Self::Regular { inode } | Self::Directory { inode } | Self::Device { inode } => {
                Some(inode)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct UserRegion {
    pub virt: VirtAddr,
    pub phys: PhysAddr,
    pub pages: usize,
}

#[derive(Clone, Copy)]
pub struct VmArea {
    pub start: VirtAddr,
    pub end: VirtAddr,
    pub flags: u32,
    pub file: Option<InodeRef>,
}

impl VmArea {
    pub const fn empty() -> Self {
        Self {
            start: VirtAddr::new(0),
            end: VirtAddr::new(0),
            flags: 0,
            file: None,
        }
    }

    pub const fn contains(self, addr: VirtAddr) -> bool {
        addr.as_usize() >= self.start.as_usize() && addr.as_usize() < self.end.as_usize()
    }

    pub const fn can_read(self) -> bool {
        self.flags & VM_READ != 0
    }

    pub const fn can_write(self) -> bool {
        self.flags & VM_WRITE != 0
    }

    pub const fn can_exec(self) -> bool {
        self.flags & VM_EXEC != 0
    }
}

impl UserRegion {
    pub const fn empty() -> Self {
        Self {
            virt: VirtAddr::new(0),
            phys: PhysAddr::new(0),
            pages: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FileHandleId(usize);

impl FileHandleId {
    pub const fn invalid() -> Self {
        Self(usize::MAX)
    }

    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }

    pub const fn is_valid(self) -> bool {
        self.0 != usize::MAX
    }
}

#[derive(Clone, Copy)]
pub struct FileHandle {
    pub object: FileObject,
    pub offset: usize,
    pub ref_count: u16,
    pub flags: u32,
}

impl FileHandle {
    pub const fn closed() -> Self {
        Self {
            object: FileObject::Closed,
            offset: 0,
            ref_count: 0,
            flags: 0,
        }
    }

    pub const fn new(object: FileObject, flags: u32) -> Self {
        Self {
            object,
            offset: 0,
            ref_count: 1,
            flags,
        }
    }

    pub const fn is_open(self) -> bool {
        !matches!(self.object, FileObject::Closed)
    }
}

#[derive(Clone, Copy)]
pub struct FileDescriptor {
    pub handle: FileHandleId,
    pub fd_flags: u32,
}

impl FileDescriptor {
    pub const fn closed() -> Self {
        Self {
            handle: FileHandleId::invalid(),
            fd_flags: 0,
        }
    }

    pub const fn new(handle: FileHandleId, fd_flags: u32) -> Self {
        Self { handle, fd_flags }
    }

    pub const fn is_open(self) -> bool {
        self.handle.is_valid()
    }

    pub const fn close_on_exec(self) -> bool {
        self.fd_flags & FD_CLOEXEC != 0
    }
}

#[derive(Clone, Copy)]
pub struct FileTable {
    fds: [FileDescriptor; MAX_FILES],
    handles: [FileHandle; MAX_FILE_HANDLES],
}

impl FileTable {
    pub const fn empty() -> Self {
        Self {
            fds: [FileDescriptor::closed(); MAX_FILES],
            handles: [FileHandle::closed(); MAX_FILE_HANDLES],
        }
    }

    pub const fn with_stdio() -> Self {
        let mut fds = [FileDescriptor::closed(); MAX_FILES];
        let mut handles = [FileHandle::closed(); MAX_FILE_HANDLES];
        handles[0] = FileHandle::new(FileObject::ConsoleIn, 0);
        handles[1] = FileHandle::new(FileObject::ConsoleOut, 0);
        handles[2] = FileHandle::new(FileObject::ConsoleErr, 0);
        fds[0] = FileDescriptor::new(FileHandleId::new(0), 0);
        fds[1] = FileDescriptor::new(FileHandleId::new(1), 0);
        fds[2] = FileDescriptor::new(FileHandleId::new(2), 0);
        Self { fds, handles }
    }

    pub fn get(&self, fd: usize) -> Option<FileHandle> {
        let desc = self.fds.get(fd).copied()?;
        if !desc.is_open() {
            return None;
        }
        self.handle(desc.handle)
    }

    pub fn get_mut(&mut self, fd: usize) -> Option<&mut FileHandle> {
        let handle = self.fds.get(fd).filter(|desc| desc.is_open())?.handle;
        self.handle_mut(handle)
    }

    pub fn fd_flags(&self, fd: usize) -> Option<u32> {
        self.fds
            .get(fd)
            .filter(|desc| desc.is_open())
            .map(|desc| desc.fd_flags)
    }

    pub fn set_fd_flags(&mut self, fd: usize, flags: u32) -> Result<(), u32> {
        let Some(desc) = self.fds.get_mut(fd).filter(|desc| desc.is_open()) else {
            return Err(crate::kernel::syscall::EBADF);
        };
        desc.fd_flags = flags;
        Ok(())
    }

    pub fn open(&mut self, object: FileObject, flags: u32, fd_flags: u32) -> Result<usize, u32> {
        let Some(fd) = self.fds.iter().position(|file| !file.is_open()) else {
            return Err(crate::kernel::syscall::EMFILE);
        };
        let handle = self.alloc_handle(object, flags)?;

        self.fds[fd] = FileDescriptor::new(handle, fd_flags);
        Ok(fd)
    }

    pub fn duplicate_from(
        &mut self,
        old_fd: usize,
        min_fd: usize,
        fd_flags: u32,
    ) -> Result<usize, u32> {
        let old = self
            .fds
            .get(old_fd)
            .copied()
            .ok_or(crate::kernel::syscall::EBADF)?;
        if !old.is_open() {
            return Err(crate::kernel::syscall::EBADF);
        }
        let Some(fd) = self
            .fds
            .iter()
            .enumerate()
            .skip(min_fd)
            .find_map(|(fd, entry)| (!entry.is_open()).then_some(fd))
        else {
            return Err(crate::kernel::syscall::EMFILE);
        };

        self.inc_ref(old.handle)?;
        self.fds[fd] = FileDescriptor::new(old.handle, fd_flags);
        Ok(fd)
    }

    pub fn duplicate_to(
        &mut self,
        old_fd: usize,
        new_fd: usize,
        fd_flags: u32,
    ) -> Result<Option<FileObject>, u32> {
        if old_fd >= MAX_FILES || new_fd >= MAX_FILES {
            return Err(crate::kernel::syscall::EBADF);
        }
        let old = self.fds[old_fd];
        if !old.is_open() {
            return Err(crate::kernel::syscall::EBADF);
        }
        if old_fd == new_fd {
            self.fds[new_fd].fd_flags = fd_flags;
            return Ok(None);
        }

        self.inc_ref(old.handle)?;
        let closed = self.detach_fd(new_fd)?;
        self.fds[new_fd] = FileDescriptor::new(old.handle, fd_flags);
        Ok(closed)
    }

    pub fn close(&mut self, fd: usize) -> Result<Option<FileObject>, u32> {
        if fd >= MAX_FILES || !self.fds[fd].is_open() {
            return Err(crate::kernel::syscall::EBADF);
        }
        self.detach_fd(fd)
    }

    pub fn close_on_exec(&mut self) -> ClosedFiles {
        let mut closed = ClosedFiles::new();
        for fd in 0..MAX_FILES {
            if self.fds[fd].is_open() && self.fds[fd].close_on_exec() {
                if let Ok(Some(object)) = self.detach_fd(fd) {
                    closed.push(object);
                }
            }
        }
        closed
    }

    pub fn close_all(&mut self) -> ClosedFiles {
        let mut closed = ClosedFiles::new();
        for fd in 0..MAX_FILES {
            if self.fds[fd].is_open() {
                if let Ok(Some(object)) = self.detach_fd(fd) {
                    closed.push(object);
                }
            }
        }
        closed
    }

    pub fn open_count(&self) -> usize {
        self.fds.iter().filter(|file| file.is_open()).count()
    }

    pub fn for_each_open_object<F>(&self, mut f: F)
    where
        F: FnMut(FileObject),
    {
        for handle in self.handles.iter().filter(|handle| handle.is_open()) {
            f(handle.object);
        }
    }

    fn alloc_handle(&mut self, object: FileObject, flags: u32) -> Result<FileHandleId, u32> {
        let Some(index) = self.handles.iter().position(|handle| !handle.is_open()) else {
            return Err(crate::kernel::syscall::EMFILE);
        };
        self.handles[index] = FileHandle::new(object, flags);
        Ok(FileHandleId::new(index))
    }

    fn handle(&self, handle: FileHandleId) -> Option<FileHandle> {
        self.handles
            .get(handle.as_usize())
            .copied()
            .filter(|file| file.is_open())
    }

    fn handle_mut(&mut self, handle: FileHandleId) -> Option<&mut FileHandle> {
        self.handles
            .get_mut(handle.as_usize())
            .filter(|file| file.is_open())
    }

    fn inc_ref(&mut self, handle: FileHandleId) -> Result<(), u32> {
        let Some(file) = self.handle_mut(handle) else {
            return Err(crate::kernel::syscall::EBADF);
        };
        file.ref_count = file
            .ref_count
            .checked_add(1)
            .ok_or(crate::kernel::syscall::EMFILE)?;
        Ok(())
    }

    fn detach_fd(&mut self, fd: usize) -> Result<Option<FileObject>, u32> {
        let desc = self.fds.get_mut(fd).ok_or(crate::kernel::syscall::EBADF)?;
        if !desc.is_open() {
            return Err(crate::kernel::syscall::EBADF);
        }
        let handle = desc.handle;
        *desc = FileDescriptor::closed();

        let Some(file) = self.handle_mut(handle) else {
            return Ok(None);
        };
        if file.ref_count > 1 {
            file.ref_count -= 1;
            return Ok(None);
        }
        let object = file.object;
        *file = FileHandle::closed();
        Ok(Some(object))
    }
}

pub struct ClosedFiles {
    objects: [FileObject; MAX_FILES],
    len: usize,
}

impl ClosedFiles {
    pub const fn new() -> Self {
        Self {
            objects: [FileObject::Closed; MAX_FILES],
            len: 0,
        }
    }

    pub fn push(&mut self, object: FileObject) {
        if self.len < MAX_FILES {
            self.objects[self.len] = object;
            self.len += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = FileObject> + '_ {
        self.objects[..self.len].iter().copied()
    }
}

#[derive(Clone, Copy)]
pub struct ProcessControlBlock {
    pub pid: ProcessId,
    pub parent: ProcessId,
    pub name: &'static str,
    pub state: ProcessState,
    pub address_space: AddressSpace,
    pub files: FileTable,
    pub creds: Credentials,
    pub cwd: [u8; CWD_MAX],
    pub cwd_len: usize,
    pub main_thread: ThreadId,
    pub thread_count: usize,
    pub exit_code: i32,
    pub resources_reaped: bool,
}

impl ProcessControlBlock {
    pub const fn empty() -> Self {
        Self {
            pid: ProcessId::new(0),
            parent: ProcessId::new(0),
            name: "",
            state: ProcessState::Empty,
            address_space: AddressSpace::kernel(PhysAddr::new(0)),
            files: FileTable::empty(),
            creds: Credentials::root(),
            cwd: [0; CWD_MAX],
            cwd_len: 0,
            main_thread: ThreadId::new(0),
            thread_count: 0,
            exit_code: 0,
            resources_reaped: true,
        }
    }

    pub fn new(
        pid: ProcessId,
        parent: ProcessId,
        name: &'static str,
        address_space: AddressSpace,
    ) -> Self {
        let mut cwd = [0; CWD_MAX];
        cwd[0] = b'/';
        Self {
            pid,
            parent,
            name,
            state: ProcessState::Alive,
            address_space,
            files: FileTable::with_stdio(),
            creds: Credentials::root(),
            cwd,
            cwd_len: 1,
            main_thread: ThreadId::new(0),
            thread_count: 1,
            exit_code: 0,
            resources_reaped: false,
        }
    }
}

pub struct ProcessTable {
    entries: [ProcessControlBlock; MAX_PROCESSES],
    next_pid: usize,
    init_pid: ProcessId,
}

impl ProcessTable {
    pub const fn new() -> Self {
        Self {
            entries: [const { ProcessControlBlock::empty() }; MAX_PROCESSES],
            next_pid: 1,
            init_pid: ProcessId::new(0),
        }
    }

    pub fn create(
        &mut self,
        name: &'static str,
        parent: ProcessId,
        address_space: AddressSpace,
    ) -> ProcessId {
        self.try_create(name, parent, address_space)
            .unwrap_or_else(|_| panic!("process table full"))
    }

    pub fn try_create(
        &mut self,
        name: &'static str,
        parent: ProcessId,
        address_space: AddressSpace,
    ) -> Result<ProcessId, u32> {
        let Some(slot) = self
            .entries
            .iter()
            .position(|process| process.state == ProcessState::Empty)
        else {
            return Err(crate::kernel::syscall::ENOMEM);
        };

        let pid = ProcessId::new(self.next_pid);
        self.next_pid = self.next_pid.wrapping_add(1).max(1);
        self.entries[slot] = ProcessControlBlock::new(pid, parent, name, address_space);
        Ok(pid)
    }

    pub fn try_fork_from(
        &mut self,
        parent_pid: ProcessId,
        address_space: AddressSpace,
    ) -> Result<ProcessId, u32> {
        let parent = *self.get(parent_pid).ok_or(crate::kernel::syscall::EINVAL)?;
        let Some(slot) = self
            .entries
            .iter()
            .position(|process| process.state == ProcessState::Empty)
        else {
            return Err(crate::kernel::syscall::ENOMEM);
        };

        let pid = ProcessId::new(self.next_pid);
        self.next_pid = self.next_pid.wrapping_add(1).max(1);
        let mut child = parent;
        child.pid = pid;
        child.parent = parent_pid;
        child.state = ProcessState::Alive;
        child.address_space = address_space;
        child.thread_count = 1;
        child.exit_code = 0;
        child.resources_reaped = false;
        self.entries[slot] = child;
        Ok(pid)
    }

    pub fn has_free_slot(&self) -> bool {
        self.entries
            .iter()
            .any(|process| process.state == ProcessState::Empty)
    }

    pub fn rename(&mut self, pid: ProcessId, name: &'static str) {
        if let Some(process) = self.get_mut(pid) {
            process.name = name;
        }
    }

    pub fn set_main_thread(&mut self, pid: ProcessId, tid: ThreadId) {
        if let Some(process) = self.get_mut(pid) {
            process.main_thread = tid;
        }
    }

    pub fn set_init_process(&mut self, pid: ProcessId) {
        if self
            .get(pid)
            .map(|process| process.state == ProcessState::Alive)
            .unwrap_or(false)
        {
            self.init_pid = pid;
        }
    }

    pub fn mark_zombie(&mut self, pid: ProcessId, exit_code: i32) {
        if let Some(process) = self.get_mut(pid) {
            process.state = ProcessState::Zombie;
            process.exit_code = exit_code;
        }
    }

    pub fn mark_resources_reaped(
        &mut self,
        pid: ProcessId,
        replacement_address_space: AddressSpace,
    ) -> Option<AddressSpace> {
        let process = self.get_mut(pid)?;
        if process.resources_reaped {
            return None;
        }

        process.resources_reaped = true;
        process.thread_count = 0;
        process.files = FileTable::empty();
        Some(mem::replace(
            &mut process.address_space,
            replacement_address_space,
        ))
    }

    pub fn release(&mut self, pid: ProcessId) {
        if let Some(slot) = self.index_of(pid) {
            self.entries[slot] = ProcessControlBlock::empty();
            self.orphan_children(pid);
        }
    }

    pub fn address_space(&self, pid: ProcessId) -> Option<AddressSpace> {
        self.get(pid).map(|process| process.address_space)
    }

    pub fn address_space_mut(&mut self, pid: ProcessId) -> Option<&mut AddressSpace> {
        self.get_mut(pid).map(|process| &mut process.address_space)
    }

    pub fn replace_address_space(
        &mut self,
        pid: ProcessId,
        address_space: AddressSpace,
    ) -> Option<AddressSpace> {
        let process = self.get_mut(pid)?;
        let old = process.address_space;
        process.address_space = address_space;
        Some(old)
    }

    pub fn close_on_exec(&mut self, pid: ProcessId) -> Result<ClosedFiles, u32> {
        Ok(self
            .get_mut(pid)
            .ok_or(crate::kernel::syscall::EINVAL)?
            .files
            .close_on_exec())
    }

    pub fn close_all_files(&mut self, pid: ProcessId) -> Result<ClosedFiles, u32> {
        Ok(self
            .get_mut(pid)
            .ok_or(crate::kernel::syscall::EINVAL)?
            .files
            .close_all())
    }

    pub fn cwd(&self, pid: ProcessId) -> Option<&[u8]> {
        self.get(pid).map(|process| &process.cwd[..process.cwd_len])
    }

    pub fn set_cwd(&mut self, pid: ProcessId, path: &[u8]) -> Result<(), u32> {
        if path.is_empty() || path.len() >= CWD_MAX {
            return Err(crate::kernel::syscall::EINVAL);
        }
        let process = self.get_mut(pid).ok_or(crate::kernel::syscall::EINVAL)?;
        process.cwd = [0; CWD_MAX];
        process.cwd[..path.len()].copy_from_slice(path);
        process.cwd_len = path.len();
        Ok(())
    }

    pub fn take_zombie_child(
        &mut self,
        parent: ProcessId,
        requested: ProcessId,
    ) -> Result<(ProcessId, i32), u32> {
        let mut has_child = false;

        for slot in 0..self.entries.len() {
            let process = self.entries[slot];
            if process.state == ProcessState::Empty || process.parent != parent {
                continue;
            }

            if requested.is_valid() && process.pid != requested {
                continue;
            }

            has_child = true;
            if process.state == ProcessState::Zombie && process.resources_reaped {
                let pid = process.pid;
                let exit_code = process.exit_code;
                self.release(pid);
                return Ok((pid, exit_code));
            }
        }

        if has_child {
            Err(crate::kernel::syscall::EAGAIN)
        } else {
            Err(crate::kernel::syscall::ECHILD)
        }
    }

    pub fn parent_of(&self, pid: ProcessId) -> ProcessId {
        self.get(pid)
            .map(|process| process.parent)
            .unwrap_or(ProcessId::new(0))
    }

    pub fn has_live_parent(&self, pid: ProcessId) -> bool {
        let parent = self.parent_of(pid);
        parent.is_valid()
            && self
                .get(parent)
                .map(|process| process.state == ProcessState::Alive)
                .unwrap_or(false)
    }

    pub fn can_write_fd(&self, pid: ProcessId, fd: usize) -> bool {
        self.get(pid)
            .and_then(|process| process.files.get(fd))
            .map(|file| file.object.can_write())
            .unwrap_or(false)
    }

    pub fn can_read_fd(&self, pid: ProcessId, fd: usize) -> bool {
        self.get(pid)
            .and_then(|process| process.files.get(fd))
            .map(|file| file.object.can_read())
            .unwrap_or(false)
    }

    pub fn file(&self, pid: ProcessId, fd: usize) -> Option<FileHandle> {
        self.get(pid).and_then(|process| process.files.get(fd))
    }

    pub fn file_mut(&mut self, pid: ProcessId, fd: usize) -> Option<&mut FileHandle> {
        self.get_mut(pid)
            .and_then(|process| process.files.get_mut(fd))
    }

    pub fn fd_flags(&self, pid: ProcessId, fd: usize) -> Option<u32> {
        self.get(pid).and_then(|process| process.files.fd_flags(fd))
    }

    pub fn set_fd_flags(&mut self, pid: ProcessId, fd: usize, flags: u32) -> Result<(), u32> {
        self.get_mut(pid)
            .ok_or(crate::kernel::syscall::EINVAL)?
            .files
            .set_fd_flags(fd, flags)
    }

    pub fn open_file(
        &mut self,
        pid: ProcessId,
        object: FileObject,
        flags: u32,
    ) -> Result<usize, u32> {
        self.get_mut(pid)
            .ok_or(crate::kernel::syscall::EINVAL)?
            .files
            .open(
                object,
                flags,
                if flags & crate::kernel::syscall::O_CLOEXEC != 0 {
                    FD_CLOEXEC
                } else {
                    0
                },
            )
    }

    pub fn close_file(&mut self, pid: ProcessId, fd: usize) -> Result<Option<FileObject>, u32> {
        self.get_mut(pid)
            .ok_or(crate::kernel::syscall::EINVAL)?
            .files
            .close(fd)
    }

    pub fn duplicate_file_from(
        &mut self,
        pid: ProcessId,
        fd: usize,
        min_fd: usize,
        cloexec: bool,
    ) -> Result<usize, u32> {
        let process = self.get_mut(pid).ok_or(crate::kernel::syscall::EINVAL)?;
        process
            .files
            .duplicate_from(fd, min_fd, if cloexec { FD_CLOEXEC } else { 0 })
    }

    pub fn duplicate_file_to(
        &mut self,
        pid: ProcessId,
        old_fd: usize,
        new_fd: usize,
        cloexec: bool,
    ) -> Result<Option<FileObject>, u32> {
        self.get_mut(pid)
            .ok_or(crate::kernel::syscall::EINVAL)?
            .files
            .duplicate_to(old_fd, new_fd, if cloexec { FD_CLOEXEC } else { 0 })
    }

    pub fn open_file_count(&self, pid: ProcessId) -> usize {
        self.get(pid)
            .map(|process| process.files.open_count())
            .unwrap_or(0)
    }

    pub fn for_each_file_object<F>(&self, pid: ProcessId, f: F) -> Result<(), u32>
    where
        F: FnMut(FileObject),
    {
        self.get(pid)
            .ok_or(crate::kernel::syscall::EINVAL)?
            .files
            .for_each_open_object(f);
        Ok(())
    }

    pub fn credentials(&self, pid: ProcessId) -> Option<Credentials> {
        self.get(pid).map(|process| process.creds)
    }

    pub fn umask(&self, pid: ProcessId) -> u16 {
        self.credentials(pid)
            .map(|creds| creds.umask)
            .unwrap_or(0o022)
    }

    pub fn vma_count(&self, pid: ProcessId) -> usize {
        self.get(pid)
            .map(|process| process.address_space.vma_count)
            .unwrap_or(0)
    }

    pub fn owned_region_count(&self, pid: ProcessId) -> usize {
        self.get(pid)
            .map(|process| process.address_space.owned_region_count)
            .unwrap_or(0)
    }

    pub fn state_str(&self, pid: ProcessId) -> &'static str {
        self.get(pid)
            .map(|process| process.state.as_str())
            .unwrap_or("missing")
    }

    pub fn address_space_kind(&self, pid: ProcessId) -> &'static str {
        self.get(pid)
            .map(|process| process.address_space.kind.as_str())
            .unwrap_or("missing")
    }

    pub fn reparent_children(&mut self, old_parent: ProcessId, new_parent: ProcessId) -> usize {
        let mut count = 0;
        for process in &mut self.entries {
            if process.state == ProcessState::Empty || process.parent != old_parent {
                continue;
            }
            process.parent = new_parent;
            count += 1;
        }
        count
    }

    pub fn orphan_children_of(&mut self, parent: ProcessId) {
        self.orphan_children(parent);
    }

    fn get(&self, pid: ProcessId) -> Option<&ProcessControlBlock> {
        self.index_of(pid).map(|slot| &self.entries[slot])
    }

    fn get_mut(&mut self, pid: ProcessId) -> Option<&mut ProcessControlBlock> {
        self.index_of(pid).map(|slot| &mut self.entries[slot])
    }

    fn index_of(&self, pid: ProcessId) -> Option<usize> {
        if !pid.is_valid() {
            return None;
        }

        self.entries
            .iter()
            .position(|process| process.pid == pid && process.state != ProcessState::Empty)
    }

    fn orphan_children(&mut self, parent: ProcessId) {
        let adoptive_parent = self.adoptive_parent(parent);
        for slot in 0..self.entries.len() {
            if self.entries[slot].state == ProcessState::Empty
                || self.entries[slot].parent != parent
            {
                continue;
            }

            if self.entries[slot].state == ProcessState::Zombie
                && self.entries[slot].resources_reaped
            {
                let child = self.entries[slot].pid;
                self.entries[slot] = ProcessControlBlock::empty();
                self.orphan_children(child);
            } else {
                self.entries[slot].parent = adoptive_parent;
            }
        }
    }

    fn adoptive_parent(&self, old_parent: ProcessId) -> ProcessId {
        if self.init_pid.is_valid() && self.init_pid != old_parent {
            if self
                .get(self.init_pid)
                .map(|process| process.state == ProcessState::Alive)
                .unwrap_or(false)
            {
                return self.init_pid;
            }
        }
        ProcessId::new(0)
    }
}

pub const fn user_stack_top(slot: usize) -> usize {
    USER_TOP - slot * USER_STACK_SLOT_SIZE
}

pub const fn user_stack_bottom(slot: usize) -> usize {
    user_stack_top(slot) - USER_STACK_SIZE
}
