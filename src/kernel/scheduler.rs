use crate::arch::aarch32::context::TaskContext;
use crate::arch::aarch32::context::TaskEntry;
use crate::kernel::queue::ReadyQueue;
use crate::kernel::syscall::{SYS_BLOCK, SYS_EXIT, SYS_SLEEP, SYS_YIELD};
use crate::kernel::task::{MAX_TASKS, TASK_STATES, Task, TaskControlBlock, TaskId, TaskState};
use crate::{print, println};

unsafe extern "C" {
    fn context_restore(sp: *mut u32) -> !;
}

pub const DEFAULT_TIME_SLICE_TICKS: u32 = 1;
const LOG_BUFFER_LEN: usize = 16;

#[derive(Clone, Copy)]
enum ScheduleReason {
    Tick,
    Yield,
    Sleep,
    Block,
    Exit,
}

impl ScheduleReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Tick => "tick",
            Self::Yield => "yield",
            Self::Sleep => "sleep",
            Self::Block => "block",
            Self::Exit => "exit",
        }
    }
}

#[derive(Clone, Copy)]
struct SwitchLog {
    tick: u64,
    from: &'static str,
    to: &'static str,
    reason: ScheduleReason,
    ready_len: usize,
    switches: u64,
}

pub struct Scheduler {
    tasks: [Task; MAX_TASKS],
    ready: ReadyQueue,
    count: usize,
    current: Option<TaskId>,
    ticks: u64,
    switches: u64,
    logs: [Option<SwitchLog>; LOG_BUFFER_LEN],
    log_head: usize,
    log_len: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: [const { TaskControlBlock::empty() }; MAX_TASKS],
            ready: ReadyQueue::new(),
            count: 0,
            current: None,
            ticks: 0,
            switches: 0,
            logs: [None; LOG_BUFFER_LEN],
            log_head: 0,
            log_len: 0,
        }
    }

    pub fn spawn(&mut self, name: &'static str, entry: TaskEntry) -> TaskId {
        self.spawn_with_options(name, entry, 0, DEFAULT_TIME_SLICE_TICKS)
    }

    pub fn spawn_with_options(
        &mut self,
        name: &'static str,
        entry: TaskEntry,
        priority: u8,
        time_slice_ticks: u32,
    ) -> TaskId {
        if self.count >= MAX_TASKS {
            panic!("task table full");
        }

        let id = TaskId::new(self.count);
        let pid = self.count + 1;
        self.tasks[id.index()] =
            TaskControlBlock::new(id, pid, name, entry, priority, time_slice_ticks);
        self.ready.push_back(id, priority);
        self.count += 1;
        id
    }

    pub unsafe fn start(&mut self) -> ! {
        let next = self.pick_next().unwrap_or_else(|| panic!("no runnable tasks"));
        self.current = Some(next);

        let tick = self.ticks;
        let task = self.task_mut(next);
        task.mark_scheduled(tick);

        println!(
            "scheduler: start task {} (pid={}, state={}, prio={}, slice={} tick)",
            task.name,
            task.pid,
            task.state.as_str(),
            task.priority,
            task.time_slice_ticks
        );

        unsafe { context_restore(task.context.sp) }
    }

    pub unsafe fn tick(&mut self, saved_sp: *mut u32) -> *mut u32 {
        self.ticks = self.ticks.wrapping_add(1);
        self.wake_sleepers();

        let Some(current) = self.current else {
            return saved_sp;
        };

        {
            let current_task = self.task_mut(current);
            current_task.context = TaskContext { sp: saved_sp };
            current_task.account_tick();
        }

        if self.task(current).remaining_ticks > 0 {
            return saved_sp;
        }

        self.schedule_from_current(current, ScheduleReason::Tick)
    }

    pub unsafe fn syscall(&mut self, saved_sp: *mut u32) -> *mut u32 {
        let Some(current) = self.current else {
            return saved_sp;
        };

        let syscall = unsafe { saved_sp.add(0).read() };
        let arg0 = unsafe { saved_sp.add(1).read() };

        self.task_mut(current).context = TaskContext { sp: saved_sp };

        match syscall {
            SYS_YIELD => self.schedule_from_current(current, ScheduleReason::Yield),
            SYS_SLEEP => {
                let wake_at = self.ticks.wrapping_add(u64::from(arg0.max(1)));
                let task = self.task_mut(current);
                task.state = TaskState::Sleeping;
                task.wake_at_tick = wake_at;
                task.remaining_ticks = 0;
                self.schedule_from_current(current, ScheduleReason::Sleep)
            }
            SYS_BLOCK => {
                let task = self.task_mut(current);
                task.state = TaskState::Blocked;
                task.wait_channel = arg0;
                task.remaining_ticks = 0;
                self.schedule_from_current(current, ScheduleReason::Block)
            }
            SYS_EXIT => {
                let task = self.task_mut(current);
                task.state = TaskState::Zombie;
                task.remaining_ticks = 0;
                self.schedule_from_current(current, ScheduleReason::Exit)
            }
            _ => saved_sp,
        }
    }

    pub fn wake_channel(&mut self, channel: u32) -> usize {
        let mut woken = 0;
        for idx in 0..self.count {
            if self.tasks[idx].state == TaskState::Blocked
                && self.tasks[idx].wait_channel == channel
            {
                self.tasks[idx].state = TaskState::Ready;
                self.tasks[idx].wait_channel = 0;
                let id = self.tasks[idx].id;
                let priority = self.tasks[idx].priority;
                self.ready.push_back(id, priority);
                woken += 1;
            }
        }
        woken
    }

    pub fn flush_logs(&mut self) {
        while self.log_len > 0 {
            if let Some(log) = self.logs[self.log_head].take() {
                println!(
                    "{} {:04}: {} -> {} (ready={}, switches={})",
                    log.reason.as_str(),
                    log.tick,
                    log.from,
                    log.to,
                    log.ready_len,
                    log.switches
                );
            }

            self.log_head = (self.log_head + 1) % LOG_BUFFER_LEN;
            self.log_len -= 1;
        }
    }

    pub fn dump_tasks(&self) {
        print!("scheduler: states");
        for state in TASK_STATES {
            print!(" {}", state.as_str());
        }
        println!();
        println!(
            "scheduler: task table (tasks={}, ready={}, ready_empty={})",
            self.count,
            self.ready.len(),
            self.ready.is_empty()
        );
        for idx in 0..self.count {
            let task = &self.tasks[idx];
            println!(
                "  id={} pid={} name={} state={} prio={} slice={} remain={} wake={} wait={} stack={:#010x}+{}p runtime={} scheduled={}",
                task.id.index(),
                task.pid,
                task.name,
                task.state.as_str(),
                task.priority,
                task.time_slice_ticks,
                task.remaining_ticks,
                task.wake_at_tick,
                task.wait_channel,
                task.stack_start as usize,
                task.stack_pages,
                task.stats.runtime_ticks,
                task.stats.scheduled_count
            );
        }
    }

    fn schedule_from_current(&mut self, current: TaskId, reason: ScheduleReason) -> *mut u32 {
        self.enqueue_if_ready(current);

        let Some(next) = self.pick_next() else {
            if self.task(current).state == TaskState::Running {
                self.current = Some(current);
                let tick = self.ticks;
                let current_task = self.task_mut(current);
                current_task.mark_scheduled(tick);
                return current_task.context.sp;
            }

            panic!("no runnable task after {}", reason.as_str());
        };

        let previous = current;
        self.current = Some(next);
        if previous != next {
            self.switches = self.switches.wrapping_add(1);
        }

        {
            let tick = self.ticks;
            let next_task = self.task_mut(next);
            next_task.mark_scheduled(tick);
        }

        if previous != next {
            self.push_log(SwitchLog {
                tick: self.ticks,
                from: self.task(previous).name,
                to: self.task(next).name,
                reason,
                ready_len: self.ready.len(),
                switches: self.switches,
            });
        }

        self.task(next).context.sp
    }

    fn enqueue_if_ready(&mut self, task_id: TaskId) {
        let task = self.task_mut(task_id);
        if task.is_runnable() {
            task.state = TaskState::Ready;
            let priority = task.priority;
            self.ready.push_back(task_id, priority);
        }
    }

    fn wake_sleepers(&mut self) {
        for idx in 0..self.count {
            if self.tasks[idx].state == TaskState::Sleeping
                && self.tasks[idx].wake_at_tick <= self.ticks
            {
                self.tasks[idx].state = TaskState::Ready;
                self.tasks[idx].wake_at_tick = 0;
                let id = self.tasks[idx].id;
                let priority = self.tasks[idx].priority;
                self.ready.push_back(id, priority);
            }
        }
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        while let Some(candidate) = self.ready.pop_front() {
            if self.task(candidate).state == TaskState::Ready {
                return Some(candidate);
            }
        }

        None
    }

    fn push_log(&mut self, log: SwitchLog) {
        let tail = (self.log_head + self.log_len) % LOG_BUFFER_LEN;
        self.logs[tail] = Some(log);
        if self.log_len == LOG_BUFFER_LEN {
            self.log_head = (self.log_head + 1) % LOG_BUFFER_LEN;
        } else {
            self.log_len += 1;
        }
    }

    fn task(&self, id: TaskId) -> &Task {
        &self.tasks[id.index()]
    }

    fn task_mut(&mut self, id: TaskId) -> &mut Task {
        &mut self.tasks[id.index()]
    }
}
