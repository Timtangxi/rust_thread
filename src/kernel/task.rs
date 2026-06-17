use crate::arch::aarch32::context::{TaskContext, TaskEntry};
use crate::kernel::memory::{self, PAGE_SIZE};

pub const MAX_TASKS: usize = 8;
const TASK_STACK_SIZE: usize = 16 * 1024;
const TASK_STACK_PAGES: usize = TASK_STACK_SIZE / PAGE_SIZE;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TaskId(usize);

impl TaskId {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Empty,
    Ready,
    Running,
    Sleeping,
    Blocked,
    Zombie,
}

pub const TASK_STATES: [TaskState; 6] = [
    TaskState::Empty,
    TaskState::Ready,
    TaskState::Running,
    TaskState::Sleeping,
    TaskState::Blocked,
    TaskState::Zombie,
];

impl TaskState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Sleeping => "sleeping",
            Self::Blocked => "blocked",
            Self::Zombie => "zombie",
        }
    }
}

#[derive(Clone, Copy)]
pub struct TaskStats {
    pub scheduled_count: u64,
    pub runtime_ticks: u64,
    pub last_scheduled_at: u64,
}

impl TaskStats {
    pub const fn new() -> Self {
        Self {
            scheduled_count: 0,
            runtime_ticks: 0,
            last_scheduled_at: 0,
        }
    }
}

pub struct TaskControlBlock {
    pub id: TaskId,
    pub pid: usize,
    pub name: &'static str,
    pub context: TaskContext,
    pub state: TaskState,
    pub priority: u8,
    pub time_slice_ticks: u32,
    pub remaining_ticks: u32,
    pub wake_at_tick: u64,
    pub wait_channel: u32,
    pub stack_start: *mut u8,
    pub stack_pages: usize,
    pub stats: TaskStats,
}

impl TaskControlBlock {
    pub const fn empty() -> Self {
        Self {
            id: TaskId::new(usize::MAX),
            pid: 0,
            name: "",
            context: TaskContext::empty(),
            state: TaskState::Empty,
            priority: 0,
            time_slice_ticks: 1,
            remaining_ticks: 0,
            wake_at_tick: 0,
            wait_channel: 0,
            stack_start: core::ptr::null_mut(),
            stack_pages: 0,
            stats: TaskStats::new(),
        }
    }

    pub fn new(
        id: TaskId,
        pid: usize,
        name: &'static str,
        entry: TaskEntry,
        priority: u8,
        time_slice_ticks: u32,
    ) -> Self {
        unsafe {
            let stack = alloc_stack();
            let context = TaskContext::new(stack, TASK_STACK_SIZE, entry);
            let time_slice_ticks = time_slice_ticks.max(1);

            Self {
                id,
                pid,
                name,
                context,
                state: TaskState::Ready,
                priority,
                time_slice_ticks,
                remaining_ticks: time_slice_ticks,
                wake_at_tick: 0,
                wait_channel: 0,
                stack_start: stack,
                stack_pages: TASK_STACK_PAGES,
                stats: TaskStats::new(),
            }
        }
    }

    pub fn mark_scheduled(&mut self, tick: u64) {
        self.state = TaskState::Running;
        self.remaining_ticks = self.time_slice_ticks;
        self.wait_channel = 0;
        self.stats.scheduled_count = self.stats.scheduled_count.wrapping_add(1);
        self.stats.last_scheduled_at = tick;
    }

    pub fn account_tick(&mut self) {
        self.stats.runtime_ticks = self.stats.runtime_ticks.wrapping_add(1);
        self.remaining_ticks = self.remaining_ticks.saturating_sub(1);
    }

    pub fn is_runnable(&self) -> bool {
        matches!(self.state, TaskState::Ready | TaskState::Running)
    }
}

pub type Task = TaskControlBlock;

unsafe fn alloc_stack() -> *mut u8 {
    memory::alloc_pages(TASK_STACK_PAGES).unwrap_or_else(|| panic!("out of pages for task stack"))
}
