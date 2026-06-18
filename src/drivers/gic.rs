use core::ptr::{read_volatile, write_volatile};

use aarch32_cpu::asm;

use crate::platform::{fdt, qemu_virt};

static mut GICD_BASE: usize = qemu_virt::GICD_BASE;
static mut GICC_BASE: usize = qemu_virt::GICC_BASE;

const GICD_CTLR: usize = 0x000;
const GICD_ISENABLER: usize = 0x100;
const GICD_IPRIORITYR: usize = 0x400;
const GICD_ITARGETSR: usize = 0x800;

const GICC_CTLR: usize = 0x000;
const GICC_PMR: usize = 0x004;
const GICC_IAR: usize = 0x00c;
const GICC_EOIR: usize = 0x010;

pub const SPURIOUS_IRQ: u32 = 1023;

pub fn init() {
    let gic = fdt::gic();
    unsafe {
        GICD_BASE = gic.distributor.start;
        GICC_BASE = gic.cpu_interface.start;
    }

    unsafe {
        write_distributor(GICD_CTLR, 0);
        asm::dsb();
        enable_irq(crate::drivers::timer::physical_timer_irq());
        enable_irq(crate::drivers::uart::irq());
        write_cpu(GICC_PMR, 0xff);
        write_cpu(GICC_CTLR, 1);
        asm::dsb();
        write_distributor(GICD_CTLR, 1);
        asm::dsb();
        asm::isb();
    }
}

pub fn distributor_base() -> usize {
    unsafe { GICD_BASE }
}

pub fn cpu_interface_base() -> usize {
    unsafe { GICC_BASE }
}

pub fn acknowledge() -> u32 {
    unsafe {
        let irq = read_cpu(GICC_IAR) & 0x3ff;
        asm::dsb();
        irq
    }
}

pub fn end_of_interrupt(irq: u32) {
    unsafe {
        asm::dsb();
        write_cpu(GICC_EOIR, irq);
        asm::dsb();
    }
}

unsafe fn enable_irq(irq: u32) {
    let irq = irq as usize;
    let enable = GICD_ISENABLER + (irq / 32) * 4;
    let priority = GICD_IPRIORITYR + irq;
    let targets = GICD_ITARGETSR + irq;

    unsafe {
        write_distributor_u8(priority, 0x80);
        write_distributor_u8(targets, 0x01);
        asm::dsb();
        write_distributor(enable, 1u32 << (irq % 32));
        asm::dsb();
    }
}

unsafe fn read_cpu(offset: usize) -> u32 {
    unsafe { read_volatile((cpu_interface_base() + offset) as *const u32) }
}

unsafe fn write_cpu(offset: usize, value: u32) {
    unsafe {
        write_volatile((cpu_interface_base() + offset) as *mut u32, value);
    }
}

unsafe fn write_distributor(offset: usize, value: u32) {
    unsafe {
        write_volatile((distributor_base() + offset) as *mut u32, value);
    }
}

unsafe fn write_distributor_u8(offset: usize, value: u8) {
    unsafe {
        write_volatile((distributor_base() + offset) as *mut u8, value);
    }
}
