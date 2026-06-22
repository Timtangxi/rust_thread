use crate::kernel::task::{MAX_TASKS, TaskId};

pub const WAIT_QUEUE_COUNT: usize = 16;

pub struct WaitQueueTable {
    queues: [WaitQueue; WAIT_QUEUE_COUNT],
}

impl WaitQueueTable {
    pub const fn new() -> Self {
        Self {
            queues: [const { WaitQueue::new() }; WAIT_QUEUE_COUNT],
        }
    }

    pub fn sleep_on(&mut self, channel: u32, task: TaskId) {
        self.queues[index(channel)].push(channel, task);
    }

    pub fn wake_all<F>(&mut self, channel: u32, mut wake: F) -> usize
    where
        F: FnMut(TaskId),
    {
        let mut count = 0;
        while let Some(task) = self.queues[index(channel)].pop_channel(channel) {
            wake(task);
            count += 1;
        }
        count
    }

    pub fn remove_task(&mut self, task: TaskId) {
        for queue in &mut self.queues {
            queue.remove_task(task);
        }
    }
}

#[derive(Clone, Copy)]
struct WaitQueue {
    items: [Option<WaitEntry>; MAX_TASKS],
    len: usize,
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self {
            items: [None; MAX_TASKS],
            len: 0,
        }
    }

    pub fn push(&mut self, channel: u32, task: TaskId) {
        if self.len >= MAX_TASKS {
            panic!("wait queue full");
        }
        self.items[self.len] = Some(WaitEntry { channel, task });
        self.len += 1;
    }

    pub fn pop_channel(&mut self, channel: u32) -> Option<TaskId> {
        let mut idx = 0;
        while idx < self.len {
            if self.items[idx]
                .map(|entry| entry.channel == channel)
                .unwrap_or(false)
            {
                let task = self.items[idx].take().map(|entry| entry.task);
                self.len -= 1;
                self.items[idx] = self.items[self.len].take();
                return task;
            }
            idx += 1;
        }
        None
    }

    pub fn remove_task(&mut self, task: TaskId) {
        let mut idx = 0;
        while idx < self.len {
            if self.items[idx]
                .map(|entry| entry.task == task)
                .unwrap_or(false)
            {
                self.len -= 1;
                self.items[idx] = self.items[self.len].take();
                continue;
            }
            idx += 1;
        }
    }
}

#[derive(Clone, Copy)]
struct WaitEntry {
    channel: u32,
    task: TaskId,
}

fn index(channel: u32) -> usize {
    channel as usize % WAIT_QUEUE_COUNT
}
