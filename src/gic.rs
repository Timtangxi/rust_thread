use core::ptr::{read_volatile, write_volatile};

const GICD_BASE: usize = 0x0800_0000;
const GICC_BASE: usize = 0x0801_0000;

const GICD_CTLR: usize = GICD_BASE;
const GICD_ISENABLER: usize = GICD_BASE + 0x100;
const GICD_IPRIORITYR: usize = GICD_BASE + 0x400;
const GICD_ITARGETSR: usize = GICD_BASE + 0x800;

const GICC_CTLR: usize = GICC_BASE;
const GICC_PMR: usize = GICC_BASE + 0x004;
const GICC_IAR: usize = GICC_BASE + 0x00c;
const GICC_EOIR: usize = GICC_BASE + 0x010;

pub const SPURIOUS_IRQ: u32 = 1023;

pub fn init() {
    unsafe {
        write_volatile(GICD_CTLR as *mut u32, 0);
        enable_irq(crate::timer::PHYSICAL_TIMER_IRQ);
        write_volatile(GICC_PMR as *mut u32, 0xff);
        write_volatile(GICC_CTLR as *mut u32, 1);
        write_volatile(GICD_CTLR as *mut u32, 1);
    }
}

pub fn acknowledge() -> u32 {
    unsafe { read_volatile(GICC_IAR as *const u32) & 0x3ff }
}

pub fn end_of_interrupt(irq: u32) {
    unsafe {
        write_volatile(GICC_EOIR as *mut u32, irq);
    }
}

unsafe fn enable_irq(irq: u32) {
    let irq = irq as usize;
    let enable = (GICD_ISENABLER + (irq / 32) * 4) as *mut u32;
    let priority = (GICD_IPRIORITYR + irq) as *mut u8;
    let targets = (GICD_ITARGETSR + irq) as *mut u8;

    unsafe {
        write_volatile(priority, 0x80);
        write_volatile(targets, 0x01);
        write_volatile(enable, 1u32 << (irq % 32));
    }
}
