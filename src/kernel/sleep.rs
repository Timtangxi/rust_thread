use crate::kernel::task::{MAX_TASKS, TaskId};

#[derive(Clone, Copy)]
pub struct SleepEntry {
    pub task: TaskId,
    pub wake_at: u64,
}

pub struct SleepQueue {
    entries: [Option<SleepEntry>; MAX_TASKS],
    len: usize,
}

impl SleepQueue {
    pub const fn new() -> Self {
        Self {
            entries: [None; MAX_TASKS],
            len: 0,
        }
    }

    pub fn insert(&mut self, task: TaskId, wake_at: u64) {
        if self.len >= MAX_TASKS {
            panic!("sleep queue full");
        }

        let mut pos = self.len;
        while pos > 0 {
            let prev = self.entries[pos - 1].unwrap();
            if prev.wake_at <= wake_at {
                break;
            }
            self.entries[pos] = self.entries[pos - 1];
            pos -= 1;
        }

        self.entries[pos] = Some(SleepEntry { task, wake_at });
        self.len += 1;
    }

    pub fn pop_expired(&mut self, now: u64) -> Option<TaskId> {
        let first = self.entries[0]?;
        if first.wake_at > now {
            return None;
        }

        let task = first.task;
        for idx in 1..self.len {
            self.entries[idx - 1] = self.entries[idx];
        }
        self.len -= 1;
        self.entries[self.len] = None;
        Some(task)
    }

    pub fn remove_task(&mut self, task: TaskId) {
        let mut idx = 0;
        while idx < self.len {
            if self.entries[idx]
                .map(|entry| entry.task == task)
                .unwrap_or(false)
            {
                for next in idx + 1..self.len {
                    self.entries[next - 1] = self.entries[next];
                }
                self.len -= 1;
                self.entries[self.len] = None;
                continue;
            }
            idx += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}
