use core::fmt::{self, Write};
use core::ptr::{read_volatile, write_volatile};

const UART0_BASE: usize = 0x0900_0000;
const UART_DR: usize = UART0_BASE + 0x000;
const UART_FR: usize = UART0_BASE + 0x018;
const UART_IBRD: usize = UART0_BASE + 0x024;
const UART_FBRD: usize = UART0_BASE + 0x028;
const UART_LCRH: usize = UART0_BASE + 0x02c;
const UART_CR: usize = UART0_BASE + 0x030;
const UART_IMSC: usize = UART0_BASE + 0x038;
const UART_ICR: usize = UART0_BASE + 0x044;

const FR_TXFF: u32 = 1 << 5;

pub fn init() {
    unsafe {
        write_volatile(UART_CR as *mut u32, 0);
        write_volatile(UART_ICR as *mut u32, 0x7ff);
        write_volatile(UART_IBRD as *mut u32, 1);
        write_volatile(UART_FBRD as *mut u32, 40);
        write_volatile(UART_LCRH as *mut u32, (3 << 5) | (1 << 4));
        write_volatile(UART_IMSC as *mut u32, 0);
        write_volatile(UART_CR as *mut u32, (1 << 9) | (1 << 8) | 1);
    }
}

pub fn put_byte(byte: u8) {
    unsafe {
        while read_volatile(UART_FR as *const u32) & FR_TXFF != 0 {}
        write_volatile(UART_DR as *mut u32, byte as u32);
    }
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
