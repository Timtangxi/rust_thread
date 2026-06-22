use crate::arch::aarch32::context::{TaskContext, TaskEntry};
use crate::kernel::memory::{self, PAGE_SIZE};
use crate::kernel::process::{ProcessId, ThreadId, ThreadMode};

pub const MAX_TASKS: usize = 8;
const TASK_STACK_SIZE: usize = 64 * 1024;
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
    pub sleep_count: u64,
    pub block_count: u64,
    pub exit_code: i32,
    pub last_syscall: u32,
    pub bytes_written: u64,
}

impl TaskStats {
    pub const fn new() -> Self {
        Self {
            scheduled_count: 0,
            runtime_ticks: 0,
            last_scheduled_at: 0,
            sleep_count: 0,
            block_count: 0,
            exit_code: 0,
            last_syscall: 0,
            bytes_written: 0,
        }
    }
}

pub struct TaskControlBlock {
    pub id: TaskId,
    pub process_id: ProcessId,
    pub thread_id: ThreadId,
    pub pid: usize,
    pub name: &'static str,
    pub mode: ThreadMode,
    pub context: TaskContext,
    pub state: TaskState,
    pub priority: u8,
    pub time_slice_ticks: u32,
    pub remaining_ticks: u32,
    pub wake_at_tick: u64,
    pub wait_channel: u32,
    pub wait_pid: usize,
    pub wait_user_buf: usize,
    pub wait_len: usize,
    pub kernel_stack_start: *mut u8,
    pub kernel_stack_pages: usize,
    pub user_stack_phys: *mut u8,
    pub user_stack_pages: usize,
    pub user_stack_bottom: usize,
    pub user_stack_top: usize,
    pub user_entry: usize,
    pub user_tls: u32,
    pub stats: TaskStats,
}

impl TaskControlBlock {
    pub const fn empty() -> Self {
        Self {
            id: TaskId::new(usize::MAX),
            process_id: ProcessId::new(0),
            thread_id: ThreadId::new(0),
            pid: 0,
            name: "",
            mode: ThreadMode::Kernel,
            context: TaskContext::empty(),
            state: TaskState::Empty,
            priority: 0,
            time_slice_ticks: 1,
            remaining_ticks: 0,
            wake_at_tick: 0,
            wait_channel: 0,
            wait_pid: 0,
            wait_user_buf: 0,
            wait_len: 0,
            kernel_stack_start: core::ptr::null_mut(),
            kernel_stack_pages: 0,
            user_stack_phys: core::ptr::null_mut(),
            user_stack_pages: 0,
            user_stack_bottom: 0,
            user_stack_top: 0,
            user_entry: 0,
            user_tls: 0,
            stats: TaskStats::new(),
        }
    }

    pub fn new_kernel(
        id: TaskId,
        process_id: ProcessId,
        thread_id: ThreadId,
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
                process_id,
                thread_id,
                pid: process_id.as_usize(),
                name,
                mode: ThreadMode::Kernel,
                context,
                state: TaskState::Ready,
                priority,
                time_slice_ticks,
                remaining_ticks: time_slice_ticks,
                wake_at_tick: 0,
                wait_channel: 0,
                wait_pid: 0,
                wait_user_buf: 0,
                wait_len: 0,
                kernel_stack_start: stack,
                kernel_stack_pages: TASK_STACK_PAGES,
                user_stack_phys: core::ptr::null_mut(),
                user_stack_pages: 0,
                user_stack_bottom: 0,
                user_stack_top: 0,
                user_entry: 0,
                user_tls: 0,
                stats: TaskStats::new(),
            }
        }
    }

    #[cfg(feature = "mmu")]
    pub fn new_user(
        id: TaskId,
        process_id: ProcessId,
        thread_id: ThreadId,
        name: &'static str,
        user_entry: usize,
        user_stack_phys: *mut u8,
        user_stack_pages: usize,
        user_stack_bottom: usize,
        user_stack_top: usize,
        initial_sp: usize,
        priority: u8,
        time_slice_ticks: u32,
    ) -> Self {
        unsafe {
            let kernel_stack = alloc_stack();

            let context =
                TaskContext::new_user(kernel_stack, TASK_STACK_SIZE, user_entry, initial_sp);
            let time_slice_ticks = time_slice_ticks.max(1);

            Self {
                id,
                process_id,
                thread_id,
                pid: process_id.as_usize(),
                name,
                mode: ThreadMode::User,
                context,
                state: TaskState::Ready,
                priority,
                time_slice_ticks,
                remaining_ticks: time_slice_ticks,
                wake_at_tick: 0,
                wait_channel: 0,
                wait_pid: 0,
                wait_user_buf: 0,
                wait_len: 0,
                kernel_stack_start: kernel_stack,
                kernel_stack_pages: TASK_STACK_PAGES,
                user_stack_phys,
                user_stack_pages,
                user_stack_bottom,
                user_stack_top,
                user_entry,
                user_tls: 0,
                stats: TaskStats::new(),
            }
        }
    }

    pub fn mark_scheduled(&mut self, tick: u64) {
        self.state = TaskState::Running;
        self.remaining_ticks = self.time_slice_ticks;
        self.wait_channel = 0;
        self.wait_pid = 0;
        self.wait_user_buf = 0;
        self.wait_len = 0;
        self.stats.scheduled_count = self.stats.scheduled_count.wrapping_add(1);
        self.stats.last_scheduled_at = tick;
    }

    pub fn account_tick(&mut self) {
        self.stats.runtime_ticks = self.stats.runtime_ticks.wrapping_add(1);
        self.remaining_ticks = self.remaining_ticks.saturating_sub(1);
    }

    pub fn mark_sleeping(&mut self, wake_at_tick: u64) {
        self.state = TaskState::Sleeping;
        self.wake_at_tick = wake_at_tick;
        self.remaining_ticks = 0;
        self.stats.sleep_count = self.stats.sleep_count.wrapping_add(1);
    }

    pub fn mark_blocked(&mut self, channel: u32) {
        self.state = TaskState::Blocked;
        self.wait_channel = channel;
        self.wait_pid = 0;
        self.wait_user_buf = 0;
        self.wait_len = 0;
        self.remaining_ticks = 0;
        self.stats.block_count = self.stats.block_count.wrapping_add(1);
    }

    pub fn mark_waiting_for_read(&mut self, channel: u32, user_buf: usize, len: usize) {
        self.state = TaskState::Blocked;
        self.wait_channel = channel;
        self.wait_pid = 0;
        self.wait_user_buf = user_buf;
        self.wait_len = len;
        self.remaining_ticks = 0;
        self.stats.block_count = self.stats.block_count.wrapping_add(1);
    }

    pub fn mark_waiting_for_poll(&mut self, channel: u32, pollfds: usize, nfds: usize) {
        self.state = TaskState::Blocked;
        self.wait_channel = channel;
        self.wait_pid = usize::MAX;
        self.wait_user_buf = pollfds;
        self.wait_len = nfds;
        self.remaining_ticks = 0;
        self.stats.block_count = self.stats.block_count.wrapping_add(1);
    }

    pub fn mark_waiting_for_child(
        &mut self,
        channel: u32,
        pid: usize,
        status_ptr: usize,
        rusage_ptr: usize,
    ) {
        self.state = TaskState::Blocked;
        self.wait_channel = channel;
        self.wait_pid = pid;
        self.wait_user_buf = status_ptr;
        self.wait_len = rusage_ptr;
        self.remaining_ticks = 0;
        self.stats.block_count = self.stats.block_count.wrapping_add(1);
    }

    pub fn mark_zombie(&mut self, exit_code: i32) {
        self.state = TaskState::Zombie;
        self.remaining_ticks = 0;
        self.stats.exit_code = exit_code;
    }

    pub unsafe fn release_resources(&mut self) {
        if !self.kernel_stack_start.is_null() && self.kernel_stack_pages != 0 {
            unsafe {
                memory::free_pages(self.kernel_stack_start, self.kernel_stack_pages);
            }
        }

        *self = Self::empty();
    }

    pub fn is_runnable(&self) -> bool {
        matches!(self.state, TaskState::Ready | TaskState::Running)
    }
}

pub type Task = TaskControlBlock;

unsafe fn alloc_stack() -> *mut u8 {
    memory::alloc_pages(TASK_STACK_PAGES).unwrap_or_else(|| panic!("out of pages for task stack"))
}
