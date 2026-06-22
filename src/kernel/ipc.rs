#![allow(dead_code)]

use crate::kernel::syscall::{ENOENT, ENOSPC};

pub const MAX_PIPES: usize = 8;
pub const PIPE_CAPACITY: usize = 256;
const PIPE_WAIT_BASE: u32 = 0x5049_0000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PipeId(usize);

impl PipeId {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PipeEnd {
    Read,
    Write,
}

#[derive(Clone, Copy)]
pub enum PipeIo {
    Read { bytes: usize, wake_writer: bool },
    Write { bytes: usize, wake_reader: bool },
    WouldBlock { channel: u32 },
    Closed,
    Error(u32),
}

#[derive(Clone, Copy)]
struct Pipe {
    used: bool,
    buffer: [u8; PIPE_CAPACITY],
    read_pos: usize,
    len: usize,
    readers: u16,
    writers: u16,
}

impl Pipe {
    const fn empty() -> Self {
        Self {
            used: false,
            buffer: [0; PIPE_CAPACITY],
            read_pos: 0,
            len: 0,
            readers: 0,
            writers: 0,
        }
    }

    fn write_pos(self) -> usize {
        (self.read_pos + self.len) % PIPE_CAPACITY
    }
}

static mut PIPES: [Pipe; MAX_PIPES] = [const { Pipe::empty() }; MAX_PIPES];

pub fn create() -> Result<PipeId, u32> {
    unsafe {
        for index in 0..MAX_PIPES {
            if PIPES[index].used {
                continue;
            }
            PIPES[index] = Pipe::empty();
            PIPES[index].used = true;
            PIPES[index].readers = 1;
            PIPES[index].writers = 1;
            return Ok(PipeId::new(index));
        }
    }
    Err(ENOSPC)
}

pub fn read(id: PipeId, dst: &mut [u8]) -> PipeIo {
    let Some(pipe) = pipe_mut(id) else {
        return PipeIo::Error(ENOENT);
    };
    if dst.is_empty() {
        return PipeIo::Read {
            bytes: 0,
            wake_writer: false,
        };
    }
    if pipe.len == 0 {
        if pipe.writers == 0 {
            return PipeIo::Closed;
        }
        return PipeIo::WouldBlock {
            channel: read_wait_channel(id),
        };
    }

    let count = dst.len().min(pipe.len);
    for slot in dst.iter_mut().take(count) {
        *slot = pipe.buffer[pipe.read_pos];
        pipe.read_pos = (pipe.read_pos + 1) % PIPE_CAPACITY;
    }
    pipe.len -= count;
    PipeIo::Read {
        bytes: count,
        wake_writer: pipe.len < PIPE_CAPACITY,
    }
}

pub fn write(id: PipeId, src: &[u8]) -> PipeIo {
    let Some(pipe) = pipe_mut(id) else {
        return PipeIo::Error(ENOENT);
    };
    if src.is_empty() {
        return PipeIo::Write {
            bytes: 0,
            wake_reader: false,
        };
    }
    if pipe.readers == 0 {
        return PipeIo::Closed;
    }
    if pipe.len == PIPE_CAPACITY {
        return PipeIo::WouldBlock {
            channel: write_wait_channel(id),
        };
    }

    let count = src.len().min(PIPE_CAPACITY - pipe.len);
    let mut write_pos = pipe.write_pos();
    for byte in src.iter().take(count) {
        pipe.buffer[write_pos] = *byte;
        write_pos = (write_pos + 1) % PIPE_CAPACITY;
    }
    pipe.len += count;
    PipeIo::Write {
        bytes: count,
        wake_reader: count != 0,
    }
}

pub fn close(id: PipeId, end: PipeEnd) {
    let Some(pipe) = pipe_mut(id) else {
        return;
    };
    match end {
        PipeEnd::Read => pipe.readers = pipe.readers.saturating_sub(1),
        PipeEnd::Write => pipe.writers = pipe.writers.saturating_sub(1),
    }
    if pipe.readers == 0 && pipe.writers == 0 {
        *pipe = Pipe::empty();
    }
}

pub fn add_ref(id: PipeId, end: PipeEnd) {
    let Some(pipe) = pipe_mut(id) else {
        return;
    };
    match end {
        PipeEnd::Read => pipe.readers = pipe.readers.saturating_add(1),
        PipeEnd::Write => pipe.writers = pipe.writers.saturating_add(1),
    }
}

pub fn read_wait_channel(id: PipeId) -> u32 {
    PIPE_WAIT_BASE | ((id.as_usize() as u32) << 1)
}

pub fn write_wait_channel(id: PipeId) -> u32 {
    PIPE_WAIT_BASE | ((id.as_usize() as u32) << 1) | 1
}

pub fn active_count() -> usize {
    let mut count = 0usize;
    for index in 0..MAX_PIPES {
        unsafe {
            let pipe = &raw const PIPES[index];
            if (*pipe).used {
                count += 1;
            }
        }
    }
    count
}

pub fn available(id: PipeId) -> usize {
    if id.as_usize() >= MAX_PIPES {
        return 0;
    }
    unsafe {
        let pipe = &raw const PIPES[id.as_usize()];
        if (*pipe).used { (*pipe).len } else { 0 }
    }
}

pub fn has_writers(id: PipeId) -> bool {
    if id.as_usize() >= MAX_PIPES {
        return false;
    }
    unsafe {
        let pipe = &raw const PIPES[id.as_usize()];
        (*pipe).used && (*pipe).writers != 0
    }
}

fn pipe_mut(id: PipeId) -> Option<&'static mut Pipe> {
    if id.as_usize() >= MAX_PIPES {
        return None;
    }
    unsafe {
        let pipe = &raw mut PIPES[id.as_usize()];
        if (*pipe).used { Some(&mut *pipe) } else { None }
    }
}
