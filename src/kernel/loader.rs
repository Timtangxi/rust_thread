#![allow(dead_code)]

#[cfg(feature = "mmu")]
use core::{mem::size_of, ptr::copy_nonoverlapping};

use crate::kernel::address::VirtAddr;
#[cfg(feature = "mmu")]
use crate::kernel::address::{PhysAddr, align_down, align_up};
#[cfg(feature = "mmu")]
use crate::kernel::memory::{self, PAGE_SIZE};
#[cfg(feature = "mmu")]
use crate::kernel::process::{AddressSpace, MAX_ADDRESS_SPACE_L2};

pub const USER_LOAD_BASE: usize = 0x0010_0000;

#[derive(Clone, Copy)]
pub struct UserImage {
    pub name: &'static str,
    pub entry: VirtAddr,
    pub text_start: VirtAddr,
    pub text_end: VirtAddr,
    pub rodata_start: VirtAddr,
    pub rodata_end: VirtAddr,
}

pub struct LoadedImage {
    pub name: &'static str,
    pub entry: VirtAddr,
    pub user_start: VirtAddr,
    pub user_end: VirtAddr,
}

impl UserImage {
    pub const fn user_start(self) -> VirtAddr {
        self.text_start
    }

    pub const fn user_end(self) -> VirtAddr {
        self.rodata_end
    }
}

pub(crate) const INIT_ELF: &[u8] = &[
    0x7f, 0x45, 0x4c, 0x46, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x02, 0x00, 0x28, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x34, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x05, 0x34, 0x00, 0x20, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00, 0x48, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
    0x00, 0x10, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0xcc, 0x01, 0x00, 0x00, 0x00, 0x10, 0x10, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x40, 0xa0, 0xe3, 0x05, 0x00, 0xa0, 0xe3, 0x01, 0x10, 0xa0, 0xe3, 0x2c, 0x20, 0x9f, 0xe5,
    0x2c, 0x30, 0x9f, 0xe5, 0x00, 0x00, 0x00, 0xef, 0x01, 0x00, 0xa0, 0xe3, 0x03, 0x10, 0xa0, 0xe3,
    0x00, 0x00, 0x00, 0xef, 0x01, 0x40, 0x84, 0xe2, 0x01, 0x00, 0x54, 0xe3, 0xf4, 0xff, 0xff, 0xba,
    0x02, 0x00, 0xa0, 0xe3, 0x00, 0x10, 0xa0, 0xe3, 0x00, 0x00, 0x00, 0xef, 0xfe, 0xff, 0xff, 0xea,
    0x00, 0x10, 0x10, 0x00, 0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x5b, 0x65, 0x6c, 0x66,
    0x2d, 0x69, 0x6e, 0x69, 0x74, 0x5d, 0x20, 0x77, 0x72, 0x69, 0x74, 0x65, 0x28, 0x66, 0x64, 0x3d,
    0x31, 0x29, 0x20, 0x66, 0x72, 0x6f, 0x6d, 0x20, 0x6c, 0x6f, 0x61, 0x64, 0x65, 0x64, 0x20, 0x45,
    0x4c, 0x46, 0x20, 0x76, 0x69, 0x61, 0x20, 0x53, 0x56, 0x43, 0x0a,
];

#[cfg(feature = "mmu")]
pub unsafe fn load_builtin_into(
    address_space: &mut AddressSpace,
    image: UserImage,
) -> Result<LoadedImage, u32> {
    map_existing_user_range(
        address_space,
        image.text_start.as_usize(),
        image.text_start.as_usize(),
        image.text_end.as_usize() - image.text_start.as_usize(),
        crate::arch::aarch32::mmu::UserMapping::Rx,
    )?;
    map_existing_user_range(
        address_space,
        image.rodata_start.as_usize(),
        image.rodata_start.as_usize(),
        image.rodata_end.as_usize() - image.rodata_start.as_usize(),
        crate::arch::aarch32::mmu::UserMapping::RoData,
    )?;

    Ok(LoadedImage {
        name: image.name,
        entry: image.entry,
        user_start: image.user_start(),
        user_end: image.user_end(),
    })
}

#[cfg(feature = "mmu")]
pub unsafe fn load_elf_into(
    address_space: &mut AddressSpace,
    name: &'static str,
    elf: &[u8],
) -> Result<LoadedImage, u32> {
    let header = ElfHeader::parse(elf).ok_or(crate::kernel::syscall::ENOEXEC)?;
    let mut user_start = usize::MAX;
    let mut user_end = 0usize;

    for index in 0..header.phnum {
        let ph = ProgramHeader::parse(elf, header.phoff as usize, header.phentsize, index)
            .ok_or(crate::kernel::syscall::ENOEXEC)?;
        if ph.p_type != PT_LOAD {
            continue;
        }

        if ph.memsz == 0 {
            continue;
        }

        let virt_start = align_down(ph.vaddr as usize, PAGE_SIZE);
        let virt_end = align_up(ph.vaddr as usize + ph.memsz as usize, PAGE_SIZE);
        let pages = (virt_end - virt_start) / PAGE_SIZE;
        let phys = memory::alloc_pages(pages).ok_or(crate::kernel::syscall::ENOMEM)?;
        let file_start = ph.offset as usize;
        let file_end = file_start
            .checked_add(ph.filesz as usize)
            .ok_or(crate::kernel::syscall::ENOEXEC)?;
        if file_end > elf.len() {
            unsafe {
                memory::free_pages(phys, pages);
            }
            return Err(crate::kernel::syscall::ENOEXEC);
        }

        unsafe {
            copy_nonoverlapping(
                elf[file_start..file_end].as_ptr(),
                phys.add((ph.vaddr as usize) - virt_start),
                ph.filesz as usize,
            );
        }

        let mapping = if ph.flags & PF_X != 0 {
            crate::arch::aarch32::mmu::UserMapping::Rx
        } else if ph.flags & PF_W != 0 {
            crate::arch::aarch32::mmu::UserMapping::RwData
        } else {
            crate::arch::aarch32::mmu::UserMapping::RoData
        };
        if let Err(err) =
            map_owned_user_range(address_space, virt_start, phys as usize, pages, mapping)
        {
            unsafe {
                memory::free_pages(phys, pages);
            }
            return Err(err);
        }
        user_start = user_start.min(virt_start);
        user_end = user_end.max(virt_end);
    }

    if user_start == usize::MAX {
        return Err(crate::kernel::syscall::ENOEXEC);
    }

    Ok(LoadedImage {
        name,
        entry: VirtAddr::new(header.entry as usize),
        user_start: VirtAddr::new(user_start),
        user_end: VirtAddr::new(user_end),
    })
}

#[cfg(feature = "mmu")]
pub fn release_address_space_regions(address_space: AddressSpace) {
    for region in address_space
        .owned_regions
        .iter()
        .take(address_space.owned_region_count)
    {
        unsafe {
            crate::arch::aarch32::mmu::unmap_pages_in(
                address_space.page_table_root.as_usize(),
                region.virt.as_usize(),
                region.pages,
            );
            memory::free_pages(region.phys.as_usize() as *mut u8, region.pages);
        }
    }

    for table in address_space
        .owned_l2
        .iter()
        .take(address_space.owned_l2_count.min(MAX_ADDRESS_SPACE_L2))
    {
        unsafe {
            crate::arch::aarch32::mmu::free_user_l2(table.as_usize());
        }
    }

    unsafe {
        crate::arch::aarch32::mmu::free_user_table(address_space.page_table_root.as_usize());
    }
}

#[cfg(feature = "mmu")]
fn map_existing_user_range(
    address_space: &mut AddressSpace,
    virt_start: usize,
    phys_start: usize,
    size: usize,
    mapping: crate::arch::aarch32::mmu::UserMapping,
) -> Result<(), u32> {
    if size == 0 {
        return Ok(());
    }

    let virt = align_down(virt_start, PAGE_SIZE);
    let phys = align_down(phys_start, PAGE_SIZE);
    let pages = (align_up(virt_start + size, PAGE_SIZE) - virt) / PAGE_SIZE;
    ensure_private_l2(address_space, virt, pages)?;
    unsafe {
        crate::arch::aarch32::mmu::map_user_pages_in(
            address_space.page_table_root.as_usize(),
            virt,
            phys,
            pages,
            mapping,
        );
    }
    Ok(())
}

#[cfg(feature = "mmu")]
fn map_owned_user_range(
    address_space: &mut AddressSpace,
    virt: usize,
    phys: usize,
    pages: usize,
    mapping: crate::arch::aarch32::mmu::UserMapping,
) -> Result<(), u32> {
    ensure_private_l2(address_space, virt, pages)?;
    unsafe {
        crate::arch::aarch32::mmu::map_user_pages_in(
            address_space.page_table_root.as_usize(),
            virt,
            phys,
            pages,
            mapping,
        );
    }
    address_space.try_add_owned_region(VirtAddr::new(virt), PhysAddr::new(phys), pages)
}

#[cfg(feature = "mmu")]
fn ensure_private_l2(
    address_space: &mut AddressSpace,
    virt: usize,
    pages: usize,
) -> Result<(), u32> {
    let mut addr = virt;
    let end = virt + pages * PAGE_SIZE;
    while addr < end {
        let l2 = unsafe {
            crate::arch::aarch32::mmu::ensure_user_l2(
                address_space.page_table_root.as_usize(),
                addr,
            )
        }
        .ok_or(crate::kernel::syscall::ENOMEM)?;
        if let Err(err) = address_space.try_add_owned_l2(PhysAddr::new(l2)) {
            unsafe {
                crate::arch::aarch32::mmu::free_user_l2(l2);
            }
            return Err(err);
        }
        addr = align_up(addr + 1, 1024 * 1024);
    }
    Ok(())
}

#[cfg(feature = "mmu")]
const PT_LOAD: u32 = 1;
#[cfg(feature = "mmu")]
const PF_X: u32 = 1;
#[cfg(feature = "mmu")]
const PF_W: u32 = 2;

#[cfg(feature = "mmu")]
#[derive(Clone, Copy)]
#[repr(C)]
struct ElfHeader {
    ident: [u8; 16],
    ty: u16,
    machine: u16,
    version: u32,
    entry: u32,
    phoff: u32,
    shoff: u32,
    flags: u32,
    ehsize: u16,
    phentsize: u16,
    phnum: u16,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16,
}

#[cfg(feature = "mmu")]
impl ElfHeader {
    fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < size_of::<Self>() {
            return None;
        }

        let header = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const Self) };
        if &header.ident[..4] != b"\x7fELF" {
            return None;
        }
        if header.ident[4] != 1 || header.ident[5] != 1 {
            return None;
        }
        if header.machine != 40 || header.phentsize as usize != size_of::<ProgramHeader>() {
            return None;
        }
        Some(header)
    }
}

#[cfg(feature = "mmu")]
#[derive(Clone, Copy)]
#[repr(C)]
struct ProgramHeader {
    p_type: u32,
    offset: u32,
    vaddr: u32,
    paddr: u32,
    filesz: u32,
    memsz: u32,
    flags: u32,
    align: u32,
}

#[cfg(feature = "mmu")]
impl ProgramHeader {
    fn parse(bytes: &[u8], phoff: usize, phentsize: u16, index: u16) -> Option<Self> {
        let start = phoff.checked_add(index as usize * phentsize as usize)?;
        let end = start.checked_add(size_of::<Self>())?;
        if end > bytes.len() {
            return None;
        }
        Some(unsafe { core::ptr::read_unaligned(bytes[start..].as_ptr() as *const Self) })
    }
}

unsafe extern "C" {
    static __user_init_entry: u8;
    static __user_text_start: u8;
    static __user_text_end: u8;
    static __user_rodata_start: u8;
    static __user_rodata_end: u8;
}

pub fn builtin_init_image() -> UserImage {
    UserImage {
        name: "builtin-init",
        entry: VirtAddr::new(sym_addr(&raw const __user_init_entry)),
        text_start: VirtAddr::new(sym_addr(&raw const __user_text_start)),
        text_end: VirtAddr::new(sym_addr(&raw const __user_text_end)),
        rodata_start: VirtAddr::new(sym_addr(&raw const __user_rodata_start)),
        rodata_end: VirtAddr::new(sym_addr(&raw const __user_rodata_end)),
    }
}

fn sym_addr(sym: *const u8) -> usize {
    sym as usize
}
