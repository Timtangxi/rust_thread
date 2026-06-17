pub const SYS_YIELD: u32 = 0;
pub const SYS_SLEEP: u32 = 1;
pub const SYS_EXIT: u32 = 2;
pub const SYS_BLOCK: u32 = 3;

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
pub fn exit() -> ! {
    unsafe {
        syscall1(SYS_EXIT, 0);
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
