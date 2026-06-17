use aarch32_cpu::asm;

unsafe extern "C" {
    static vectors: u32;
}

pub fn install_exception_vectors() {
    unsafe {
        let addr = core::ptr::addr_of!(vectors) as u32;
        core::arch::asm!("mcr p15, 0, {0}, c12, c0, 0", in(reg) addr, options(nostack, preserves_flags));
        asm::dsb();
        asm::isb();
    }
}
