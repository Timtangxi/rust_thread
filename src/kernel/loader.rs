#![allow(dead_code)]

#[cfg(feature = "mmu")]
use core::{mem::size_of, ptr::copy_nonoverlapping};

use crate::kernel::address::VirtAddr;
#[cfg(feature = "mmu")]
use crate::kernel::address::{PhysAddr, align_down, align_up};
#[cfg(feature = "mmu")]
use crate::kernel::memory::{self, PAGE_SIZE};
#[cfg(feature = "mmu")]
use crate::kernel::process::{
    AddressSpace, MAX_ADDRESS_SPACE_L2, VM_EXEC, VM_FILE, VM_READ, VM_USER, VM_WRITE,
};

pub const USER_LOAD_BASE: usize = 0x0010_0000;
#[cfg(feature = "mmu")]
pub const INTERP_LOAD_BASE: usize = 0x1000_0000;
#[cfg(feature = "mmu")]
pub const AT_NULL: u32 = 0;
#[cfg(feature = "mmu")]
pub const AT_PHDR: u32 = 3;
#[cfg(feature = "mmu")]
pub const AT_PHENT: u32 = 4;
#[cfg(feature = "mmu")]
pub const AT_PHNUM: u32 = 5;
#[cfg(feature = "mmu")]
pub const AT_PAGESZ: u32 = 6;
#[cfg(feature = "mmu")]
pub const AT_BASE: u32 = 7;
#[cfg(feature = "mmu")]
pub const AT_ENTRY: u32 = 9;
#[cfg(feature = "mmu")]
pub const AT_FLAGS: u32 = 8;
#[cfg(feature = "mmu")]
pub const AT_UID: u32 = 11;
#[cfg(feature = "mmu")]
pub const AT_EUID: u32 = 12;
#[cfg(feature = "mmu")]
pub const AT_GID: u32 = 13;
#[cfg(feature = "mmu")]
pub const AT_EGID: u32 = 14;
#[cfg(feature = "mmu")]
pub const AT_HWCAP: u32 = 16;
#[cfg(feature = "mmu")]
pub const AT_CLKTCK: u32 = 17;
#[cfg(feature = "mmu")]
pub const AT_SECURE: u32 = 23;
#[cfg(feature = "mmu")]
pub const AT_RANDOM: u32 = 25;
#[cfg(feature = "mmu")]
pub const AT_HWCAP2: u32 = 26;
#[cfg(feature = "mmu")]
pub const AT_EXECFN: u32 = 31;
#[cfg(feature = "mmu")]
pub const AT_PLATFORM: u32 = 15;
#[cfg(feature = "mmu")]
pub const AT_BASE_PLATFORM: u32 = 24;
#[cfg(feature = "mmu")]
pub const AT_SYSINFO_EHDR: u32 = 33;

#[cfg(feature = "mmu")]
const ARM_HWCAP_SWP: u32 = 1 << 0;
#[cfg(feature = "mmu")]
const ARM_HWCAP_HALF: u32 = 1 << 1;
#[cfg(feature = "mmu")]
const ARM_HWCAP_THUMB: u32 = 1 << 2;
#[cfg(feature = "mmu")]
const ARM_HWCAP_FAST_MULT: u32 = 1 << 4;
#[cfg(feature = "mmu")]
const ARM_HWCAP_VFP: u32 = 1 << 6;
#[cfg(feature = "mmu")]
const ARM_HWCAP_EDSP: u32 = 1 << 7;
#[cfg(feature = "mmu")]
const ARM_HWCAP_NEON: u32 = 1 << 12;
#[cfg(feature = "mmu")]
const ARM_HWCAP_VFPV3: u32 = 1 << 13;
#[cfg(feature = "mmu")]
const ARM_HWCAP_VFPV3D16: u32 = 1 << 14;
#[cfg(feature = "mmu")]
const ARM_HWCAP_TLS: u32 = 1 << 15;
#[cfg(feature = "mmu")]
const ARM_HWCAP_VFPV4: u32 = 1 << 16;
#[cfg(feature = "mmu")]
const ARM_HWCAP_IDIVA: u32 = 1 << 17;
#[cfg(feature = "mmu")]
const ARM_HWCAP_IDIVT: u32 = 1 << 18;
#[cfg(feature = "mmu")]
const ARM_HWCAP_ARMV7: u32 = ARM_HWCAP_SWP
    | ARM_HWCAP_HALF
    | ARM_HWCAP_THUMB
    | ARM_HWCAP_FAST_MULT
    | ARM_HWCAP_VFP
    | ARM_HWCAP_EDSP
    | ARM_HWCAP_NEON
    | ARM_HWCAP_VFPV3
    | ARM_HWCAP_VFPV3D16
    | ARM_HWCAP_TLS
    | ARM_HWCAP_VFPV4
    | ARM_HWCAP_IDIVA
    | ARM_HWCAP_IDIVT;

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
    pub brk_start: VirtAddr,
    pub phdr: VirtAddr,
    pub phnum: usize,
    pub phent: usize,
    pub interpreter_base: VirtAddr,
    pub interpreter_entry: VirtAddr,
    pub executable_entry: VirtAddr,
    pub executable_base: VirtAddr,
}

#[cfg(feature = "mmu")]
pub struct InitialStack {
    pub sp: usize,
}

#[cfg(feature = "mmu")]
pub const MAX_INIT_ARGS: usize = 8;
#[cfg(feature = "mmu")]
pub const MAX_INIT_ENVS: usize = 8;
#[cfg(feature = "mmu")]
pub const INIT_ARG_LEN: usize = 96;

#[cfg(feature = "mmu")]
#[derive(Clone, Copy)]
pub struct UserArg {
    pub bytes: [u8; INIT_ARG_LEN],
    pub len: usize,
}

#[cfg(feature = "mmu")]
impl UserArg {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; INIT_ARG_LEN],
            len: 0,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut arg = Self::empty();
        arg.len = bytes.len().min(INIT_ARG_LEN - 1);
        arg.bytes[..arg.len].copy_from_slice(&bytes[..arg.len]);
        arg
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
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
    address_space.try_add_vma(
        image.text_start,
        image.text_end,
        VM_READ | VM_EXEC | VM_USER,
        None,
    )?;
    map_existing_user_range(
        address_space,
        image.rodata_start.as_usize(),
        image.rodata_start.as_usize(),
        image.rodata_end.as_usize() - image.rodata_start.as_usize(),
        crate::arch::aarch32::mmu::UserMapping::RoData,
    )?;
    address_space.try_add_vma(
        image.rodata_start,
        image.rodata_end,
        VM_READ | VM_USER,
        None,
    )?;

    Ok(LoadedImage {
        name: image.name,
        entry: image.entry,
        user_start: image.user_start(),
        user_end: image.user_end(),
        brk_start: image.user_end(),
        phdr: VirtAddr::new(0),
        phnum: 0,
        phent: 0,
        interpreter_base: VirtAddr::new(0),
        interpreter_entry: VirtAddr::new(0),
        executable_entry: image.entry,
        executable_base: image.user_start(),
    })
}

#[cfg(feature = "mmu")]
pub unsafe fn load_elf_into(
    address_space: &mut AddressSpace,
    name: &'static str,
    elf: &[u8],
) -> Result<LoadedImage, u32> {
    let executable = unsafe { load_elf_object(address_space, name, elf, USER_LOAD_BASE) }?;
    let mut loaded = LoadedImage {
        name,
        entry: executable.entry,
        user_start: executable.user_start,
        user_end: executable.user_end,
        brk_start: VirtAddr::new(align_up(executable.user_end.as_usize(), PAGE_SIZE)),
        phdr: executable.phdr,
        phnum: executable.phnum,
        phent: executable.phent,
        interpreter_base: VirtAddr::new(0),
        interpreter_entry: VirtAddr::new(0),
        executable_entry: executable.entry,
        executable_base: executable.load_base,
    };

    if executable.interpreter_len != 0 {
        let interpreter_path =
            core::str::from_utf8(&executable.interpreter[..executable.interpreter_len])
                .map_err(|_| crate::kernel::syscall::ENOEXEC)?;
        let inode = lookup_loader_path(interpreter_path.as_bytes())?;
        let Some(file) = crate::fs::vfs::open(inode) else {
            return Err(crate::kernel::syscall::ENOENT);
        };
        let interpreter =
            unsafe { load_elf_object(address_space, "ld.so", file.data, INTERP_LOAD_BASE) }?;
        loaded.entry = interpreter.entry;
        loaded.user_start = VirtAddr::new(
            loaded
                .user_start
                .as_usize()
                .min(interpreter.user_start.as_usize()),
        );
        loaded.user_end = VirtAddr::new(
            loaded
                .user_end
                .as_usize()
                .max(interpreter.user_end.as_usize()),
        );
        loaded.interpreter_base = interpreter.load_base;
        loaded.interpreter_entry = interpreter.entry;
    }

    Ok(loaded)
}

#[cfg(feature = "mmu")]
pub unsafe fn load_elf_from_inode_into(
    address_space: &mut AddressSpace,
    name: &'static str,
    inode: crate::fs::vfs::InodeRef,
) -> Result<LoadedImage, u32> {
    let metadata = crate::fs::vfs::metadata(inode).ok_or(crate::kernel::syscall::ENOENT)?;
    let executable_data = alloc_read_file(inode, metadata.size)?;
    let executable = unsafe {
        load_elf_object(
            address_space,
            name,
            core::slice::from_raw_parts(executable_data, metadata.size),
            USER_LOAD_BASE,
        )
    };
    unsafe {
        memory::free_exact_pages(executable_data, metadata.size.div_ceil(PAGE_SIZE).max(1));
    }
    let executable = executable?;

    let mut loaded = LoadedImage {
        name,
        entry: executable.entry,
        user_start: executable.user_start,
        user_end: executable.user_end,
        brk_start: VirtAddr::new(align_up(executable.user_end.as_usize(), PAGE_SIZE)),
        phdr: executable.phdr,
        phnum: executable.phnum,
        phent: executable.phent,
        interpreter_base: VirtAddr::new(0),
        interpreter_entry: VirtAddr::new(0),
        executable_entry: executable.entry,
        executable_base: executable.load_base,
    };

    if executable.interpreter_len != 0 {
        let interpreter_path =
            core::str::from_utf8(&executable.interpreter[..executable.interpreter_len])
                .map_err(|_| crate::kernel::syscall::ENOEXEC)?;
        let interpreter_inode = lookup_loader_path(interpreter_path.as_bytes())?;
        let interpreter_meta =
            crate::fs::vfs::metadata(interpreter_inode).ok_or(crate::kernel::syscall::ENOENT)?;
        let interpreter_data = alloc_read_file(interpreter_inode, interpreter_meta.size)?;
        let interpreter = unsafe {
            load_elf_object(
                address_space,
                "ld.so",
                core::slice::from_raw_parts(interpreter_data, interpreter_meta.size),
                INTERP_LOAD_BASE,
            )
        };
        unsafe {
            memory::free_exact_pages(
                interpreter_data,
                interpreter_meta.size.div_ceil(PAGE_SIZE).max(1),
            );
        }
        let interpreter = interpreter?;
        loaded.entry = interpreter.entry;
        loaded.user_start = VirtAddr::new(
            loaded
                .user_start
                .as_usize()
                .min(interpreter.user_start.as_usize()),
        );
        loaded.user_end = VirtAddr::new(
            loaded
                .user_end
                .as_usize()
                .max(interpreter.user_end.as_usize()),
        );
        loaded.interpreter_base = interpreter.load_base;
        loaded.interpreter_entry = interpreter.entry;
    }

    Ok(loaded)
}

#[cfg(feature = "mmu")]
pub fn build_initial_stack(
    stack_phys: *mut u8,
    stack_bottom: usize,
    stack_top: usize,
    loaded: &LoadedImage,
    argv0: &str,
) -> Result<InitialStack, u32> {
    let argv0 = UserArg::from_bytes(argv0.as_bytes());
    build_initial_stack_with_args(stack_phys, stack_bottom, stack_top, loaded, &[argv0], &[])
}

#[cfg(feature = "mmu")]
pub fn build_initial_stack_with_args(
    stack_phys: *mut u8,
    stack_bottom: usize,
    stack_top: usize,
    loaded: &LoadedImage,
    argv: &[UserArg],
    envp: &[UserArg],
) -> Result<InitialStack, u32> {
    let mut sp = stack_top;
    let argc = argv.len().min(MAX_INIT_ARGS);
    let envc = envp.len().min(MAX_INIT_ENVS);
    let mut argv_user = [0usize; MAX_INIT_ARGS];
    let mut envp_user = [0usize; MAX_INIT_ENVS];
    let execfn = if argc != 0 {
        argv[0].as_bytes()
    } else {
        loaded.name.as_bytes()
    };
    sp = push_bytes(stack_phys, stack_bottom, sp, &[0])?;
    sp = push_bytes(stack_phys, stack_bottom, sp, execfn)?;
    let execfn_user = sp;
    sp = push_bytes(stack_phys, stack_bottom, sp, b"v7l\0")?;
    let platform_user = sp;
    sp = push_bytes(stack_phys, stack_bottom, sp, b"v7l\0")?;
    let base_platform_user = sp;
    for index in (0..envc).rev() {
        sp = push_bytes(stack_phys, stack_bottom, sp, &[0])?;
        sp = push_bytes(stack_phys, stack_bottom, sp, envp[index].as_bytes())?;
        envp_user[index] = sp;
    }
    for index in (0..argc).rev() {
        sp = push_bytes(stack_phys, stack_bottom, sp, &[0])?;
        sp = push_bytes(stack_phys, stack_bottom, sp, argv[index].as_bytes())?;
        argv_user[index] = sp;
    }
    let random = [
        0x52, 0x53, 0x54, 0x4b, 0x41, 0x52, 0x4d, 0x37, 0x4c, 0x38, 0x44, 0x59, 0x4e, 0x00, 0x5a,
        0x11,
    ];
    sp = push_bytes(stack_phys, stack_bottom, sp, &random)?;
    let random_user = sp;

    sp &= !0xf;
    sp = push_auxv(sp, AT_NULL, 0, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_SYSINFO_EHDR, 0, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_EXECFN, execfn_user as u32, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_HWCAP2, 0, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_SECURE, 0, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_RANDOM, random_user as u32, stack_phys, stack_bottom)?;
    sp = push_auxv(
        sp,
        AT_BASE_PLATFORM,
        base_platform_user as u32,
        stack_phys,
        stack_bottom,
    )?;
    sp = push_auxv(sp, AT_CLKTCK, 100, stack_phys, stack_bottom)?;
    sp = push_auxv(
        sp,
        AT_PLATFORM,
        platform_user as u32,
        stack_phys,
        stack_bottom,
    )?;
    sp = push_auxv(sp, AT_HWCAP, ARM_HWCAP_ARMV7, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_EGID, 0, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_GID, 0, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_EUID, 0, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_UID, 0, stack_phys, stack_bottom)?;
    sp = push_auxv(
        sp,
        AT_ENTRY,
        loaded.executable_entry.as_usize() as u32,
        stack_phys,
        stack_bottom,
    )?;
    sp = push_auxv(sp, AT_FLAGS, 0, stack_phys, stack_bottom)?;
    sp = push_auxv(
        sp,
        AT_BASE,
        loaded.interpreter_base.as_usize() as u32,
        stack_phys,
        stack_bottom,
    )?;
    sp = push_auxv(sp, AT_PAGESZ, PAGE_SIZE as u32, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_PHNUM, loaded.phnum as u32, stack_phys, stack_bottom)?;
    sp = push_auxv(sp, AT_PHENT, loaded.phent as u32, stack_phys, stack_bottom)?;
    sp = push_auxv(
        sp,
        AT_PHDR,
        loaded.phdr.as_usize() as u32,
        stack_phys,
        stack_bottom,
    )?;
    sp = push_word(sp, 0, stack_phys, stack_bottom)?;
    for index in (0..envc).rev() {
        sp = push_word(sp, envp_user[index] as u32, stack_phys, stack_bottom)?;
    }
    sp = push_word(sp, 0, stack_phys, stack_bottom)?;
    for index in (0..argc).rev() {
        sp = push_word(sp, argv_user[index] as u32, stack_phys, stack_bottom)?;
    }
    sp = push_word(sp, argc as u32, stack_phys, stack_bottom)?;
    Ok(InitialStack { sp })
}

#[cfg(feature = "mmu")]
fn push_auxv(
    sp: usize,
    key: u32,
    value: u32,
    stack_phys: *mut u8,
    stack_bottom: usize,
) -> Result<usize, u32> {
    let sp = push_word(sp, value, stack_phys, stack_bottom)?;
    push_word(sp, key, stack_phys, stack_bottom)
}

#[cfg(feature = "mmu")]
fn push_word(
    sp: usize,
    value: u32,
    stack_phys: *mut u8,
    stack_bottom: usize,
) -> Result<usize, u32> {
    let sp = sp
        .checked_sub(size_of::<u32>())
        .ok_or(crate::kernel::syscall::EFAULT)?;
    if sp < stack_bottom {
        return Err(crate::kernel::syscall::EFAULT);
    }
    let offset = sp - stack_bottom;
    unsafe {
        stack_phys.add(offset).cast::<u32>().write_unaligned(value);
    }
    Ok(sp)
}

#[cfg(feature = "mmu")]
fn push_bytes(
    stack_phys: *mut u8,
    stack_bottom: usize,
    sp: usize,
    bytes: &[u8],
) -> Result<usize, u32> {
    let sp = sp
        .checked_sub(bytes.len())
        .ok_or(crate::kernel::syscall::EFAULT)?;
    if sp < stack_bottom {
        return Err(crate::kernel::syscall::EFAULT);
    }
    let offset = sp - stack_bottom;
    unsafe {
        copy_nonoverlapping(bytes.as_ptr(), stack_phys.add(offset), bytes.len());
    }
    Ok(sp)
}

#[cfg(feature = "mmu")]
unsafe fn load_elf_object(
    address_space: &mut AddressSpace,
    name: &'static str,
    elf: &[u8],
    dyn_base: usize,
) -> Result<LoadedObject, u32> {
    let header = ElfHeader::parse(elf).ok_or(crate::kernel::syscall::ENOEXEC)?;
    if header.ty != ET_EXEC && header.ty != ET_DYN {
        return Err(crate::kernel::syscall::ENOEXEC);
    }

    let load_bias = if header.ty == ET_DYN {
        let low = lowest_load_vaddr(elf, header)?;
        align_up(dyn_base, PAGE_SIZE).saturating_sub(align_down(low, PAGE_SIZE))
    } else {
        0
    };

    let mut user_start = usize::MAX;
    let mut user_end = 0usize;
    let mut phdr = 0usize;
    let mut interpreter = [0u8; INTERP_PATH_MAX];
    let mut interpreter_len = 0usize;

    for index in 0..header.phnum {
        let ph = ProgramHeader::parse(elf, header.phoff as usize, header.phentsize, index)
            .ok_or(crate::kernel::syscall::ENOEXEC)?;
        match ph.p_type {
            PT_LOAD => {
                if ph.memsz == 0 {
                    continue;
                }
                let seg_start = (ph.vaddr as usize).wrapping_add(load_bias);
                let virt_start = align_down(seg_start, PAGE_SIZE);
                let virt_end = align_up(seg_start + ph.memsz as usize, PAGE_SIZE);
                let pages = (virt_end - virt_start) / PAGE_SIZE;
                let phys =
                    memory::alloc_exact_pages(pages).ok_or(crate::kernel::syscall::ENOMEM)?;
                let file_start = ph.offset as usize;
                let file_end = file_start
                    .checked_add(ph.filesz as usize)
                    .ok_or(crate::kernel::syscall::ENOEXEC)?;
                if file_end > elf.len() || ph.filesz > ph.memsz {
                    unsafe {
                        memory::free_exact_pages(phys, pages);
                    }
                    return Err(crate::kernel::syscall::ENOEXEC);
                }

                unsafe {
                    copy_nonoverlapping(
                        elf[file_start..file_end].as_ptr(),
                        phys.add(seg_start - virt_start),
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
                        memory::free_exact_pages(phys, pages);
                    }
                    return Err(err);
                }

                let mut vma_flags = VM_READ | VM_USER | VM_FILE;
                if ph.flags & PF_X != 0 {
                    vma_flags |= VM_EXEC;
                }
                if ph.flags & PF_W != 0 {
                    vma_flags |= VM_WRITE;
                }
                address_space.try_add_vma(
                    VirtAddr::new(virt_start),
                    VirtAddr::new(virt_end),
                    vma_flags,
                    None,
                )?;
                user_start = user_start.min(virt_start);
                user_end = user_end.max(virt_end);
            }
            PT_PHDR => {
                phdr = (ph.vaddr as usize).wrapping_add(load_bias);
            }
            PT_INTERP => {
                interpreter_len = read_interp(elf, ph, &mut interpreter)?;
            }
            _ => {}
        }
    }

    if user_start == usize::MAX {
        return Err(crate::kernel::syscall::ENOEXEC);
    }
    if phdr == 0 && (header.phoff as usize) < elf.len() {
        phdr = (header.phoff as usize).wrapping_add(load_bias);
    }

    Ok(LoadedObject {
        name,
        entry: VirtAddr::new((header.entry as usize).wrapping_add(load_bias)),
        load_base: VirtAddr::new(load_bias),
        user_start: VirtAddr::new(user_start),
        user_end: VirtAddr::new(user_end),
        phdr: VirtAddr::new(phdr),
        phnum: header.phnum as usize,
        phent: header.phentsize as usize,
        interpreter,
        interpreter_len,
    })
}

#[cfg(feature = "mmu")]
fn alloc_read_file(inode: crate::fs::vfs::InodeRef, size: usize) -> Result<*mut u8, u32> {
    if size == 0 {
        return Err(crate::kernel::syscall::ENOEXEC);
    }
    let pages = size.div_ceil(PAGE_SIZE).max(1);
    let ptr = memory::alloc_exact_pages(pages).ok_or(crate::kernel::syscall::ENOMEM)?;
    let dst = unsafe { core::slice::from_raw_parts_mut(ptr, pages * PAGE_SIZE) };
    let mut done = 0usize;
    while done < size {
        let count = crate::fs::vfs::read(inode, done, &mut dst[done..size])?;
        if count == 0 {
            unsafe {
                memory::free_exact_pages(ptr, pages);
            }
            return Err(crate::kernel::syscall::ENOEXEC);
        }
        done += count;
    }
    Ok(ptr)
}

#[cfg(feature = "mmu")]
fn lookup_loader_path(path: &[u8]) -> Result<crate::fs::vfs::InodeRef, u32> {
    match crate::fs::vfs::lookup(path) {
        Ok(inode) => Ok(inode),
        Err(err) => {
            let stripped = path.strip_prefix(b"/").unwrap_or(path);
            if stripped.len() != path.len() {
                crate::fs::vfs::lookup(stripped)
            } else {
                Err(err)
            }
        }
    }
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
            memory::free_exact_pages(region.phys.as_usize() as *mut u8, region.pages);
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

    if address_space.l1_pages != 0 {
        unsafe {
            crate::arch::aarch32::mmu::free_user_table(address_space.page_table_root.as_usize());
        }
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
const PT_DYNAMIC: u32 = 2;
#[cfg(feature = "mmu")]
const PT_INTERP: u32 = 3;
#[cfg(feature = "mmu")]
const PT_PHDR: u32 = 6;
#[cfg(feature = "mmu")]
const ET_EXEC: u16 = 2;
#[cfg(feature = "mmu")]
const ET_DYN: u16 = 3;
#[cfg(feature = "mmu")]
const PF_X: u32 = 1;
#[cfg(feature = "mmu")]
const PF_W: u32 = 2;
#[cfg(feature = "mmu")]
const INTERP_PATH_MAX: usize = 96;

#[cfg(feature = "mmu")]
#[derive(Clone, Copy)]
struct LoadedObject {
    name: &'static str,
    entry: VirtAddr,
    load_base: VirtAddr,
    user_start: VirtAddr,
    user_end: VirtAddr,
    phdr: VirtAddr,
    phnum: usize,
    phent: usize,
    interpreter: [u8; INTERP_PATH_MAX],
    interpreter_len: usize,
}

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

#[cfg(feature = "mmu")]
fn lowest_load_vaddr(elf: &[u8], header: ElfHeader) -> Result<usize, u32> {
    let mut lowest = usize::MAX;
    for index in 0..header.phnum {
        let ph = ProgramHeader::parse(elf, header.phoff as usize, header.phentsize, index)
            .ok_or(crate::kernel::syscall::ENOEXEC)?;
        if ph.p_type == PT_LOAD && ph.memsz != 0 {
            lowest = lowest.min(ph.vaddr as usize);
        }
    }
    if lowest == usize::MAX {
        Err(crate::kernel::syscall::ENOEXEC)
    } else {
        Ok(lowest)
    }
}

#[cfg(feature = "mmu")]
fn read_interp(
    elf: &[u8],
    ph: ProgramHeader,
    out: &mut [u8; INTERP_PATH_MAX],
) -> Result<usize, u32> {
    let start = ph.offset as usize;
    let end = start
        .checked_add(ph.filesz as usize)
        .ok_or(crate::kernel::syscall::ENOEXEC)?;
    if end > elf.len() || ph.filesz == 0 {
        return Err(crate::kernel::syscall::ENOEXEC);
    }
    let raw = &elf[start..end];
    let len = raw
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(raw.len())
        .min(INTERP_PATH_MAX);
    if len == 0 || len >= INTERP_PATH_MAX {
        return Err(crate::kernel::syscall::ENOEXEC);
    }
    out[..len].copy_from_slice(&raw[..len]);
    Ok(len)
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
