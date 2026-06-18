use aarch32_cpu::asm;

unsafe extern "C" {
    static vectors: u32;
}

#[derive(Clone, Copy)]
pub enum ExceptionKind {
    Undefined,
    PrefetchAbort,
    DataAbort,
    Fiq,
    Reserved,
    Unknown(u32),
}

impl ExceptionKind {
    pub const fn from_raw(raw: u32) -> Self {
        match raw {
            1 => Self::Undefined,
            3 => Self::PrefetchAbort,
            4 => Self::DataAbort,
            5 => Self::Reserved,
            7 => Self::Fiq,
            other => Self::Unknown(other),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Undefined => "undefined",
            Self::PrefetchAbort => "prefetch-abort",
            Self::DataAbort => "data-abort",
            Self::Reserved => "reserved",
            Self::Fiq => "fiq",
            Self::Unknown(_) => "unknown",
        }
    }
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

pub fn cpsr() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!("mrs {0}, cpsr", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

pub fn dfsr() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!("mrc p15, 0, {0}, c5, c0, 0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

pub fn ifsr() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!("mrc p15, 0, {0}, c5, c0, 1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

pub fn dfar() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!("mrc p15, 0, {0}, c6, c0, 0", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

pub fn ifar() -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!("mrc p15, 0, {0}, c6, c0, 2", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

#[derive(Clone, Copy)]
pub struct BankedRegisters {
    pub svc_sp: u32,
    pub svc_lr: u32,
    pub irq_sp: u32,
    pub irq_lr: u32,
    pub usr_sp: u32,
    pub usr_lr: u32,
}

impl BankedRegisters {
    pub fn capture() -> Self {
        let svc_sp: u32;
        let svc_lr: u32;
        let irq_sp: u32;
        let irq_lr: u32;
        let usr_sp: u32;
        let usr_lr: u32;

        unsafe {
            core::arch::asm!(
                "mrs r12, cpsr",
                "cpsid i",
                "cps #0x13",
                "mov {svc_sp}, sp",
                "mov {svc_lr}, lr",
                "cps #0x12",
                "mov {irq_sp}, sp",
                "mov {irq_lr}, lr",
                "cps #0x1f",
                "mov {usr_sp}, sp",
                "mov {usr_lr}, lr",
                "msr cpsr_c, r12",
                svc_sp = out(reg) svc_sp,
                svc_lr = out(reg) svc_lr,
                irq_sp = out(reg) irq_sp,
                irq_lr = out(reg) irq_lr,
                usr_sp = out(reg) usr_sp,
                usr_lr = out(reg) usr_lr,
                out("r12") _,
                options(nostack)
            );
        }

        Self {
            svc_sp,
            svc_lr,
            irq_sp,
            irq_lr,
            usr_sp,
            usr_lr,
        }
    }
}
