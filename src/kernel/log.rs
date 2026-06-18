use crate::println;

const LOG_CAPACITY: usize = 64;

#[derive(Clone, Copy)]
pub enum LogEvent {
    Schedule {
        tick: u64,
        reason: &'static str,
        from: &'static str,
        to: &'static str,
        ready: usize,
        switches: u64,
    },
    TaskExit {
        tick: u64,
        name: &'static str,
        pid: usize,
    },
    TaskReap {
        tick: u64,
        name: &'static str,
        pid: usize,
    },
    Wake {
        tick: u64,
        channel: u32,
        count: usize,
    },
    ConsoleInput {
        tick: u64,
        bytes: usize,
        available: usize,
        dropped: u64,
    },
}

pub struct KernelLog {
    entries: [Option<LogEvent>; LOG_CAPACITY],
    head: usize,
    len: usize,
    dropped: u64,
}

impl KernelLog {
    pub const fn new() -> Self {
        Self {
            entries: [None; LOG_CAPACITY],
            head: 0,
            len: 0,
            dropped: 0,
        }
    }

    pub fn push(&mut self, event: LogEvent) {
        let tail = (self.head + self.len) % LOG_CAPACITY;
        self.entries[tail] = Some(event);
        if self.len == LOG_CAPACITY {
            self.head = (self.head + 1) % LOG_CAPACITY;
            self.dropped = self.dropped.wrapping_add(1);
        } else {
            self.len += 1;
        }
    }

    pub fn flush(&mut self) {
        while self.len > 0 {
            if let Some(event) = self.entries[self.head].take() {
                print_event(event);
            }
            self.head = (self.head + 1) % LOG_CAPACITY;
            self.len -= 1;
        }

        if self.dropped != 0 {
            println!("log: dropped {} event(s)", self.dropped);
            self.dropped = 0;
        }
    }

    pub fn dump_recent(&self, max: usize) {
        let count = self.len.min(max);
        let start = (self.head + self.len - count) % LOG_CAPACITY;
        for offset in 0..count {
            let idx = (start + offset) % LOG_CAPACITY;
            if let Some(event) = self.entries[idx] {
                print_event(event);
            }
        }

        if self.dropped != 0 {
            println!("log: dropped {} event(s)", self.dropped);
        }
    }
}

fn print_event(event: LogEvent) {
    match event {
        LogEvent::Schedule {
            tick,
            reason,
            from,
            to,
            ready,
            switches,
        } => println!(
            "{} {:04}: {} -> {} (ready={}, switches={})",
            reason, tick, from, to, ready, switches
        ),
        LogEvent::TaskExit { tick, name, pid } => {
            println!("exit {:04}: {} pid={}", tick, name, pid)
        }
        LogEvent::TaskReap { tick, name, pid } => {
            println!("reap {:04}: {} pid={}", tick, name, pid)
        }
        LogEvent::Wake {
            tick,
            channel,
            count,
        } => println!("wake {:04}: channel={} tasks={}", tick, channel, count),
        LogEvent::ConsoleInput {
            tick,
            bytes,
            available,
            dropped,
        } => println!(
            "console {:04}: input bytes={} available={} dropped={}",
            tick, bytes, available, dropped
        ),
    }
}
