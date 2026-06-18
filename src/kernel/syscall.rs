pub const SYS_YIELD: u32 = 0;
pub const SYS_SLEEP: u32 = 1;
pub const SYS_EXIT: u32 = 2;
pub const SYS_BLOCK: u32 = 3;
pub const SYS_WAKE: u32 = 4;
pub const SYS_WRITE: u32 = 5;
pub const SYS_READ: u32 = 6;
pub const SYS_OPEN: u32 = 7;
pub const SYS_CLOSE: u32 = 8;
pub const SYS_WAIT: u32 = 9;
pub const SYS_SPAWN: u32 = 10;
pub const SYS_EXEC: u32 = 11;

pub const EINVAL: u32 = (-22i32) as u32;
pub const EFAULT: u32 = (-14i32) as u32;
pub const EBADF: u32 = (-9i32) as u32;
pub const ENOENT: u32 = (-2i32) as u32;
pub const ENOMEM: u32 = (-12i32) as u32;
pub const ECHILD: u32 = (-10i32) as u32;
pub const EAGAIN: u32 = (-11i32) as u32;
pub const ENOEXEC: u32 = (-8i32) as u32;
pub const EMFILE: u32 = (-24i32) as u32;
pub const EEXIST: u32 = (-17i32) as u32;
pub const ENOSPC: u32 = (-28i32) as u32;
pub const EROFS: u32 = (-30i32) as u32;

pub const O_CREAT: u32 = 0o100;
pub const O_TRUNC: u32 = 0o1000;
pub const O_APPEND: u32 = 0o2000;

#[inline]
pub fn yield_now() {
    unsafe {
        syscall1(SYS_YIELD, 0);
    }
}

#[inline]
pub fn sleep(ticks: u32) {
    unsafe {
        syscall1(SYS_SLEEP, ticks);
    }
}

#[inline]
pub fn block(channel: u32) {
    unsafe {
        syscall1(SYS_BLOCK, channel);
    }
}

#[inline]
pub fn wake(channel: u32) -> usize {
    unsafe { syscall1(SYS_WAKE, channel) as usize }
}

#[inline]
#[allow(dead_code)]
pub fn write(fd: u32, bytes: &[u8]) -> isize {
    unsafe { syscall3(SYS_WRITE, fd, bytes.as_ptr() as u32, bytes.len() as u32) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn read(fd: u32, bytes: &mut [u8]) -> isize {
    unsafe { syscall3(SYS_READ, fd, bytes.as_mut_ptr() as u32, bytes.len() as u32) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn open(path: &[u8], flags: u32) -> isize {
    unsafe { syscall3(SYS_OPEN, path.as_ptr() as u32, path.len() as u32, flags) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn close(fd: u32) -> isize {
    unsafe { syscall1(SYS_CLOSE, fd) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn wait(pid: u32) -> isize {
    unsafe { syscall1(SYS_WAIT, pid) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn spawn(path: &[u8]) -> isize {
    unsafe { syscall3(SYS_SPAWN, path.as_ptr() as u32, path.len() as u32, 0) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn exec(path: &[u8]) -> isize {
    unsafe { syscall3(SYS_EXEC, path.as_ptr() as u32, path.len() as u32, 0) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn exit() -> ! {
    exit_with(0)
}

#[inline]
pub fn exit_with(code: i32) -> ! {
    unsafe {
        syscall1(SYS_EXIT, code as u32);
    }

    loop {
        aarch32_cpu::asm::wfi();
    }
}

#[inline]
unsafe fn syscall1(number: u32, arg0: u32) -> u32 {
    let ret: u32;
    unsafe {
        core::arch::asm!(
            "svc #0",
            inlateout("r0") number => ret,
            in("r1") arg0,
            options(nostack)
        );
    }
    ret
}

#[inline]
#[allow(dead_code)]
unsafe fn syscall3(number: u32, arg0: u32, arg1: u32, arg2: u32) -> u32 {
    let ret: u32;
    unsafe {
        core::arch::asm!(
            "svc #0",
            inlateout("r0") number => ret,
            in("r1") arg0,
            in("r2") arg1,
            in("r3") arg2,
            options(nostack)
        );
    }
    ret
}
