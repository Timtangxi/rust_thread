pub const RAM_START: usize = 0x4000_0000;
pub const RAM_SIZE: usize = 128 * 1024 * 1024;
pub const RAM_END: usize = RAM_START + RAM_SIZE;
pub const FDT_SCAN_START: usize = RAM_END - 0x0010_0000;
pub const FDT_SCAN_END: usize = RAM_END;

pub const MMIO_START: usize = 0x0800_0000;
pub const MMIO_END: usize = 0x0a00_0000;

pub const GICD_BASE: usize = 0x0800_0000;
pub const GICC_BASE: usize = 0x0801_0000;
pub const UART0_BASE: usize = 0x0900_0000;

pub const GENERIC_TIMER_PHYS_IRQ: u32 = 30;
pub const UART0_IRQ: u32 = 33;
