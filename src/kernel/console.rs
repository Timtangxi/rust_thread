#![allow(dead_code)]

pub const INPUT_WAIT_CHANNEL: u32 = 0x434f_4e53;

pub const INPUT_CAPACITY: usize = 256;
pub const LINE_CAPACITY: usize = 128;

const LFLAG_ISIG: u32 = 0x0000_0001;
const LFLAG_ICANON: u32 = 0x0000_0002;
const LFLAG_ECHO: u32 = 0x0000_0008;
const LFLAG_ECHOE: u32 = 0x0000_0010;
const LFLAG_ECHOK: u32 = 0x0000_0020;
const LFLAG_ECHOCTL: u32 = 0x0000_0200;
const LFLAG_IEXTEN: u32 = 0x0000_8000;

pub const DEFAULT_IFLAG: u32 = 0x0000_0100 | 0x0000_0400;
pub const DEFAULT_OFLAG: u32 = 0x0000_0001 | 0x0000_0004;
pub const DEFAULT_CFLAG: u32 = 0x0000_00bf;
pub const DEFAULT_LFLAG: u32 = LFLAG_ISIG
    | LFLAG_ICANON
    | LFLAG_ECHO
    | LFLAG_ECHOE
    | LFLAG_ECHOK
    | LFLAG_ECHOCTL
    | LFLAG_IEXTEN;

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

    fn clear(&mut self) {
        self.len = 0;
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
    iflag: u32,
    oflag: u32,
    cflag: u32,
    lflag: u32,
    cc: [u8; 19],
    ready: RingBuffer<INPUT_CAPACITY>,
    line: LineBuffer,
    dropped: u64,
    received: u64,
    lines: u64,
    interrupts: u64,
}

impl ConsoleInput {
    const fn new() -> Self {
        Self {
            mode: ConsoleMode::Canonical,
            iflag: DEFAULT_IFLAG,
            oflag: DEFAULT_OFLAG,
            cflag: DEFAULT_CFLAG,
            lflag: DEFAULT_LFLAG,
            cc: default_cc(),
            ready: RingBuffer::new(),
            line: LineBuffer::new(),
            dropped: 0,
            received: 0,
            lines: 0,
            interrupts: 0,
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
                if self.echo_enabled() {
                    echo_bytes(b"\r\n");
                }
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
            0x03 => {
                if self.echo_enabled() && self.echoctl_enabled() {
                    echo_bytes(b"^C\r\n");
                }
                self.line.clear();
                if self.isig_enabled() {
                    self.interrupts = self.interrupts.wrapping_add(1);
                }
                if self.ready.push(b'\n') {
                    self.lines = self.lines.wrapping_add(1);
                    true
                } else {
                    self.dropped = self.dropped.wrapping_add(1);
                    false
                }
            }
            0x08 | 0x7f => {
                let erased = self.line.backspace();
                if erased && self.echo_enabled() {
                    echo_bytes(b"\x08 \x08");
                }
                erased
            }
            byte if byte >= 0x20 || byte == b'\t' => {
                if self.line.push(byte) {
                    if self.echo_enabled() {
                        echo_byte(byte);
                    }
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
        if byte == 0x03 && self.isig_enabled() {
            if self.echo_enabled() && self.echoctl_enabled() {
                echo_bytes(b"^C\r\n");
            }
            self.interrupts = self.interrupts.wrapping_add(1);
            return true;
        }
        if self.echo_enabled() {
            match byte {
                b'\r' | b'\n' => echo_bytes(b"\r\n"),
                0x08 | 0x7f => echo_bytes(b"\x08 \x08"),
                byte if byte >= 0x20 || byte == b'\t' => echo_byte(byte),
                _ => {}
            }
        }
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

    fn set_termios(&mut self, iflag: u32, oflag: u32, cflag: u32, lflag: u32, cc: [u8; 19]) {
        self.iflag = iflag;
        self.oflag = oflag;
        self.cflag = cflag;
        self.lflag = lflag;
        self.cc = cc;
        self.mode = if lflag & LFLAG_ICANON != 0 {
            ConsoleMode::Canonical
        } else {
            ConsoleMode::Raw
        };
    }

    const fn echo_enabled(&self) -> bool {
        self.lflag & LFLAG_ECHO != 0
    }

    const fn echoctl_enabled(&self) -> bool {
        self.lflag & LFLAG_ECHOCTL != 0
    }

    const fn isig_enabled(&self) -> bool {
        self.lflag & LFLAG_ISIG != 0
    }
}

static mut INPUT: ConsoleInput = ConsoleInput::new();

const fn default_cc() -> [u8; 19] {
    [
        0x03, 0x1c, 0x7f, 0x15, 0x04, 0, 1, 0, 0x11, 0x13, 0x1a, 0, 0x12, 0x0f, 0x17, 0x16, 0, 0, 0,
    ]
}

pub fn set_mode(mode: ConsoleMode) {
    unsafe {
        let input = core::ptr::addr_of_mut!(INPUT);
        (*input).mode = mode;
        match mode {
            ConsoleMode::Canonical => (*input).lflag |= LFLAG_ICANON | LFLAG_ECHO | LFLAG_ISIG,
            ConsoleMode::Raw => (*input).lflag &= !LFLAG_ICANON,
        }
    }
}

pub fn set_termios(iflag: u32, oflag: u32, cflag: u32, lflag: u32, cc: [u8; 19]) {
    unsafe {
        let input = core::ptr::addr_of_mut!(INPUT);
        (*input).set_termios(iflag, oflag, cflag, lflag, cc);
    }
}

pub fn termios() -> (u32, u32, u32, u32, [u8; 19]) {
    unsafe {
        let input = core::ptr::addr_of!(INPUT);
        (
            (*input).iflag,
            (*input).oflag,
            (*input).cflag,
            (*input).lflag,
            (*input).cc,
        )
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

pub fn take_interrupts() -> u64 {
    unsafe {
        let input = core::ptr::addr_of_mut!(INPUT);
        let interrupts = (*input).interrupts;
        (*input).interrupts = 0;
        interrupts
    }
}

fn echo_byte(byte: u8) {
    crate::drivers::uart::put_byte(byte);
}

fn echo_bytes(bytes: &[u8]) {
    for byte in bytes {
        echo_byte(*byte);
    }
}
