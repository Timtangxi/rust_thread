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

pub fn with_irq_disabled<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let was_unmasked = irq_unmasked();
    asm::irq_disable();
    let result = f();
    if was_unmasked {
        unsafe {
            asm::irq_enable();
        }
    }
    result
}

fn irq_unmasked() -> bool {
    let cpsr: u32;
    unsafe {
        core::arch::asm!("mrs {0}, cpsr", out(reg) cpsr, options(nomem, nostack, preserves_flags));
    }
    cpsr & (1 << 7) == 0
}
