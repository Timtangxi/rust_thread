#![allow(dead_code)]

pub const INPUT_WAIT_CHANNEL: u32 = 0x434f_4e53;

pub const INPUT_CAPACITY: usize = 256;
pub const LINE_CAPACITY: usize = 128;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConsoleMode {
    Canonical,
    Raw,
}

impl ConsoleMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::Raw => "raw",
        }
    }
}

struct RingBuffer<const N: usize> {
    buf: [u8; N],
    head: usize,
    len: usize,
}

impl<const N: usize> RingBuffer<N> {
    const fn new() -> Self {
        Self {
            buf: [0; N],
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, byte: u8) -> bool {
        if self.len == N {
            return false;
        }

        let tail = (self.head + self.len) % N;
        self.buf[tail] = byte;
        self.len += 1;
        true
    }

    fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }

        let byte = self.buf[self.head];
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(byte)
    }

    fn pop_into(&mut self, dst: &mut [u8]) -> usize {
        let mut copied = 0;
        while copied < dst.len() {
            let Some(byte) = self.pop() else {
                break;
            };
            dst[copied] = byte;
            copied += 1;
        }
        copied
    }

    const fn len(&self) -> usize {
        self.len
    }
}

struct LineBuffer {
    buf: [u8; LINE_CAPACITY],
    len: usize,
}

impl LineBuffer {
    const fn new() -> Self {
        Self {
            buf: [0; LINE_CAPACITY],
            len: 0,
        }
    }

    fn push(&mut self, byte: u8) -> bool {
        if self.len == LINE_CAPACITY {
            return false;
        }

        self.buf[self.len] = byte;
        self.len += 1;
        true
    }

    fn backspace(&mut self) -> bool {
        if self.len == 0 {
            return false;
        }

        self.len -= 1;
        true
    }

    fn flush_into(&mut self, dst: &mut RingBuffer<INPUT_CAPACITY>) -> bool {
        let mut ok = true;
        for index in 0..self.len {
            ok &= dst.push(self.buf[index]);
        }
        self.len = 0;
        ok
    }
}

struct ConsoleInput {
    mode: ConsoleMode,
    ready: RingBuffer<INPUT_CAPACITY>,
    line: LineBuffer,
    dropped: u64,
    received: u64,
    lines: u64,
}

impl ConsoleInput {
    const fn new() -> Self {
        Self {
            mode: ConsoleMode::Canonical,
            ready: RingBuffer::new(),
            line: LineBuffer::new(),
            dropped: 0,
            received: 0,
            lines: 0,
        }
    }

    fn push(&mut self, byte: u8) -> bool {
        self.received = self.received.wrapping_add(1);
        match self.mode {
            ConsoleMode::Raw => self.push_ready(byte),
            ConsoleMode::Canonical => self.push_canonical(byte),
        }
    }

    fn push_canonical(&mut self, byte: u8) -> bool {
        match byte {
            b'\r' | b'\n' => {
                if !self.line.push(b'\n') {
                    self.dropped = self.dropped.wrapping_add(1);
                    self.line.len = 0;
                    return false;
                }
                let ok = self.line.flush_into(&mut self.ready);
                if ok {
                    self.lines = self.lines.wrapping_add(1);
                } else {
                    self.dropped = self.dropped.wrapping_add(1);
                }
                ok
            }
            0x08 | 0x7f => self.line.backspace(),
            byte if byte >= 0x20 || byte == b'\t' => {
                if self.line.push(byte) {
                    true
                } else {
                    self.dropped = self.dropped.wrapping_add(1);
                    false
                }
            }
            _ => true,
        }
    }

    fn push_ready(&mut self, byte: u8) -> bool {
        if self.ready.push(byte) {
            true
        } else {
            self.dropped = self.dropped.wrapping_add(1);
            false
        }
    }

    fn pop_into(&mut self, dst: &mut [u8]) -> usize {
        self.ready.pop_into(dst)
    }
}

static mut INPUT: ConsoleInput = ConsoleInput::new();

pub fn set_mode(mode: ConsoleMode) {
    unsafe {
        let input = core::ptr::addr_of_mut!(INPUT);
        (*input).mode = mode;
    }
}

pub fn mode() -> ConsoleMode {
    unsafe {
        let input = core::ptr::addr_of!(INPUT);
        (*input).mode
    }
}

pub fn push_input(byte: u8) -> bool {
    unsafe {
        let input = core::ptr::addr_of_mut!(INPUT);
        (*input).push(byte)
    }
}

pub fn pop_input() -> Option<u8> {
    let mut byte = [0u8; 1];
    if pop_into(&mut byte) == 1 {
        Some(byte[0])
    } else {
        None
    }
}

pub fn pop_into(dst: &mut [u8]) -> usize {
    unsafe {
        let input = core::ptr::addr_of_mut!(INPUT);
        (*input).pop_into(dst)
    }
}

pub fn available() -> usize {
    unsafe {
        let input = core::ptr::addr_of!(INPUT);
        (*input).ready.len()
    }
}

pub fn pending_line_len() -> usize {
    unsafe {
        let input = core::ptr::addr_of!(INPUT);
        (*input).line.len
    }
}

pub fn dropped() -> u64 {
    unsafe {
        let input = core::ptr::addr_of!(INPUT);
        (*input).dropped
    }
}

pub fn received() -> u64 {
    unsafe {
        let input = core::ptr::addr_of!(INPUT);
        (*input).received
    }
}

pub fn lines() -> u64 {
    unsafe {
        let input = core::ptr::addr_of!(INPUT);
        (*input).lines
    }
}
