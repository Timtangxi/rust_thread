#![no_std]

pub const SYS_YIELD: u32 = 0;
pub const SYS_SLEEP: u32 = 1;
pub const SYS_EXIT: u32 = 2;
pub const SYS_WRITE: u32 = 5;
pub const SYS_READ: u32 = 6;
pub const SYS_OPEN: u32 = 7;
pub const SYS_CLOSE: u32 = 8;
pub const SYS_SPAWN: u32 = 10;
pub const SYS_EXEC: u32 = 11;
pub const SYS_GETDENTS: u32 = 14;
pub const SYS_EXECVE: u32 = 35;
pub const SYS_GETPID: u32 = 30;
pub const SYS_UNAME: u32 = 32;
pub const KERNEL_ABI_MAGIC: u32 = 0x8000_0000;

pub const O_CREAT: u32 = 0o100;
pub const DIR_NAME_LEN: usize = 96;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DirEntry {
    pub ino: u32,
    pub file_type: u32,
    pub name_len: u32,
    pub name: [u8; DIR_NAME_LEN],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UtsName {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

impl UtsName {
    pub const fn empty() -> Self {
        Self {
            sysname: [0; 65],
            nodename: [0; 65],
            release: [0; 65],
            version: [0; 65],
            machine: [0; 65],
            domainname: [0; 65],
        }
    }
}

pub fn write(fd: u32, bytes: &[u8]) -> isize {
    unsafe { syscall3(SYS_WRITE, fd, bytes.as_ptr() as u32, bytes.len() as u32) as i32 as isize }
}

pub fn read(fd: u32, bytes: &mut [u8]) -> isize {
    unsafe { syscall3(SYS_READ, fd, bytes.as_mut_ptr() as u32, bytes.len() as u32) as i32 as isize }
}

pub fn open(path: &[u8], flags: u32) -> isize {
    unsafe { syscall3(SYS_OPEN, path.as_ptr() as u32, path.len() as u32, flags) as i32 as isize }
}

pub fn close(fd: u32) -> isize {
    unsafe { syscall1(SYS_CLOSE, fd) as i32 as isize }
}

pub fn spawn(path: &[u8]) -> isize {
    unsafe { syscall3(SYS_SPAWN, path.as_ptr() as u32, path.len() as u32, 0) as i32 as isize }
}

pub fn spawnve(path: &[u8], argv: &[*const u8]) -> isize {
    unsafe {
        syscall3(
            SYS_SPAWN,
            path.as_ptr() as u32,
            path.len() as u32,
            argv.as_ptr() as u32,
        ) as i32 as isize
    }
}

pub fn exec(path: &[u8]) -> isize {
    unsafe { syscall3(SYS_EXEC, path.as_ptr() as u32, path.len() as u32, 0) as i32 as isize }
}

pub fn execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> isize {
    unsafe { syscall3(SYS_EXECVE, path as u32, argv as u32, envp as u32) as i32 as isize }
}

pub fn getdents(fd: u32, entries: &mut [DirEntry]) -> isize {
    unsafe {
        syscall3(
            SYS_GETDENTS,
            fd,
            entries.as_mut_ptr() as u32,
            core::mem::size_of_val(entries) as u32,
        ) as i32 as isize
    }
}

pub fn getpid() -> isize {
    unsafe { syscall1(SYS_GETPID, 0) as i32 as isize }
}

pub fn uname(uts: &mut UtsName) -> isize {
    unsafe { syscall1(SYS_UNAME, uts as *mut UtsName as u32) as i32 as isize }
}

pub fn sleep(ticks: u32) {
    unsafe {
        syscall1(SYS_SLEEP, ticks);
    }
}

pub fn yield_now() {
    unsafe {
        syscall1(SYS_YIELD, 0);
    }
}

pub fn exit(code: i32) -> ! {
    unsafe {
        syscall1(SYS_EXIT, code as u32);
    }
    loop {
        core::hint::spin_loop();
    }
}

pub fn print(bytes: &[u8]) {
    let _ = write(1, bytes);
}

pub fn println(bytes: &[u8]) {
    let _ = write(1, bytes);
    let _ = write(1, b"\n");
}

#[inline]
unsafe fn syscall1(number: u32, arg0: u32) -> u32 {
    let ret: u32;
    unsafe {
        core::arch::asm!(
            "svc #0",
            inlateout("r0") number => ret,
            in("r1") arg0,
            in("r7") KERNEL_ABI_MAGIC,
            options(nostack)
        );
    }
    ret
}

#[inline]
unsafe fn syscall3(number: u32, arg0: u32, arg1: u32, arg2: u32) -> u32 {
    let ret: u32;
    unsafe {
        core::arch::asm!(
            "svc #0",
            inlateout("r0") number => ret,
            in("r1") arg0,
            in("r2") arg1,
            in("r3") arg2,
            in("r7") KERNEL_ABI_MAGIC,
            options(nostack)
        );
    }
    ret
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    exit(101)
}
