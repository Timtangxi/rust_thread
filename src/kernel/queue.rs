use crate::kernel::task::{MAX_TASKS, TaskId};

pub const PRIORITY_LEVELS: usize = 4;

pub struct ReadyQueue {
    queues: [[Option<TaskId>; MAX_TASKS]; PRIORITY_LEVELS],
    heads: [usize; PRIORITY_LEVELS],
    lens: [usize; PRIORITY_LEVELS],
    total_len: usize,
}

impl ReadyQueue {
    pub const fn new() -> Self {
        Self {
            queues: [[None; MAX_TASKS]; PRIORITY_LEVELS],
            heads: [0; PRIORITY_LEVELS],
            lens: [0; PRIORITY_LEVELS],
            total_len: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_len == 0
    }

    pub fn len(&self) -> usize {
        self.total_len
    }

    pub fn push_back(&mut self, task_id: TaskId, priority: u8) {
        if self.total_len >= MAX_TASKS {
            panic!("ready queue full");
        }

        let priority = normalize_priority(priority);
        let tail = (self.heads[priority] + self.lens[priority]) % MAX_TASKS;
        self.queues[priority][tail] = Some(task_id);
        self.lens[priority] += 1;
        self.total_len += 1;
    }

    pub fn pop_front(&mut self) -> Option<TaskId> {
        for priority in (0..PRIORITY_LEVELS).rev() {
            if self.lens[priority] == 0 {
                continue;
            }

            let head = self.heads[priority];
            let item = self.queues[priority][head].take();
            self.heads[priority] = (head + 1) % MAX_TASKS;
            self.lens[priority] -= 1;
            self.total_len -= 1;
            return item;
        }

        None
    }
}

fn normalize_priority(priority: u8) -> usize {
    let priority = priority as usize;
    if priority >= PRIORITY_LEVELS {
        PRIORITY_LEVELS - 1
    } else {
        priority
    }
}
