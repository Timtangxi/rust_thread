use core::fmt::{self, Write};
use core::ptr::{read_volatile, write_volatile};

use aarch32_cpu::asm;

use crate::drivers::device::{self, DeviceClass, DeviceId, DeviceNode, DeviceState};
use crate::platform::{fdt, qemu_virt};

static mut UART_BASE: usize = qemu_virt::UART0_BASE;
static mut UART_IRQ: u32 = qemu_virt::UART0_IRQ;

const UART_DR: usize = 0x000;
const UART_RSR_ECR: usize = 0x004;
const UART_FR: usize = 0x018;
const UART_IBRD: usize = 0x024;
const UART_FBRD: usize = 0x028;
const UART_LCRH: usize = 0x02c;
const UART_CR: usize = 0x030;
const UART_IFLS: usize = 0x034;
const UART_IMSC: usize = 0x038;
const UART_MIS: usize = 0x040;
const UART_ICR: usize = 0x044;

const FR_TXFF: u32 = 1 << 5;
const FR_RXFE: u32 = 1 << 4;

const IMSC_RXIM: u32 = 1 << 4;
const IMSC_RTIM: u32 = 1 << 6;

const MIS_RXMIS: u32 = 1 << 4;
const MIS_RTMIS: u32 = 1 << 6;

const ICR_RXIC: u32 = 1 << 4;
const ICR_RTIC: u32 = 1 << 6;

pub fn init() {
    let device = fdt::uart0();
    unsafe {
        UART_BASE = device.reg.start;
        UART_IRQ = device.irq.irq;
    }

    unsafe {
        write_reg(UART_CR, 0);
        write_reg(UART_ICR, 0x7ff);
        write_reg(UART_IBRD, 1);
        write_reg(UART_FBRD, 40);
        write_reg(UART_LCRH, (3 << 5) | (1 << 4));
        write_reg(UART_IFLS, 0);
        asm::dsb();
        write_reg(UART_IMSC, IMSC_RXIM | IMSC_RTIM);
        write_reg(UART_CR, (1 << 9) | (1 << 8) | 1);
        asm::dsb();
    }
}

pub fn register_device() -> Option<DeviceId> {
    let device_info = fdt::uart0();
    let id = device::register(DeviceNode {
        id: DeviceId::new(usize::MAX),
        name: "pl011",
        driver: "pl011-uart",
        class: DeviceClass::Uart,
        state: DeviceState::Bound,
        mmio_base: device_info.reg.start,
        mmio_size: device_info.reg.size,
        irq: device_info.irq.irq,
        major: 4,
        minor: 64,
        wait_channel: crate::kernel::console::INPUT_WAIT_CHANNEL,
    })
    .ok()?;
    let _ = device::mark_ready(id);
    Some(id)
}

pub fn base() -> usize {
    unsafe { UART_BASE }
}

pub fn irq() -> u32 {
    unsafe { UART_IRQ }
}

pub fn put_byte(byte: u8) {
    unsafe {
        while read_reg(UART_FR) & FR_TXFF != 0 {}
        write_reg(UART_DR, byte as u32);
    }
}

pub fn rx_ready() -> bool {
    unsafe { read_reg(UART_FR) & FR_RXFE == 0 }
}

pub fn handle_irq<F>(mut on_byte: F) -> usize
where
    F: FnMut(u8),
{
    unsafe {
        let status = read_reg(UART_MIS);
        if status & (MIS_RXMIS | MIS_RTMIS) == 0 {
            write_reg(UART_ICR, status);
            asm::dsb();
            return 0;
        }

        let count = drain_rx_fifo(&mut on_byte);

        write_reg(UART_RSR_ECR, 0);
        write_reg(UART_ICR, ICR_RXIC | ICR_RTIC);
        asm::dsb();
        count
    }
}

pub fn poll_rx<F>(mut on_byte: F) -> usize
where
    F: FnMut(u8),
{
    unsafe {
        let count = drain_rx_fifo(&mut on_byte);
        if count != 0 {
            write_reg(UART_RSR_ECR, 0);
            write_reg(UART_ICR, ICR_RXIC | ICR_RTIC);
            asm::dsb();
        }
        count
    }
}

unsafe fn drain_rx_fifo<F>(on_byte: &mut F) -> usize
where
    F: FnMut(u8),
{
    let mut count = 0;
    unsafe {
        while read_reg(UART_FR) & FR_RXFE == 0 {
            let data = read_reg(UART_DR);
            if data & 0x0f00 == 0 {
                on_byte((data & 0xff) as u8);
                count += 1;
            }
        }
    }
    count
}

pub struct UartWriter;

impl Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                put_byte(b'\r');
            }
            put_byte(byte);
        }
        Ok(())
    }
}

pub fn write_fmt(args: fmt::Arguments<'_>) {
    let _ = UartWriter.write_fmt(args);
}

unsafe fn read_reg(offset: usize) -> u32 {
    unsafe { read_volatile((base() + offset) as *const u32) }
}

unsafe fn write_reg(offset: usize, value: u32) {
    unsafe {
        write_volatile((base() + offset) as *mut u32, value);
    }
}
