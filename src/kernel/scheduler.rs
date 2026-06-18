#[cfg(feature = "mmu")]
use crate::arch::aarch32::context::USER_INITIAL_CPSR;
use crate::arch::aarch32::context::{TaskContext, TaskEntry, TrapFrame};
use crate::drivers::uart;
use crate::fs::vfs;
use crate::kernel::address::PhysAddr;
#[cfg(feature = "mmu")]
use crate::kernel::address::VirtAddr;
use crate::kernel::console;
use crate::kernel::loader;
#[cfg(feature = "mmu")]
use crate::kernel::loader::UserImage;
use crate::kernel::log::{KernelLog, LogEvent};
#[cfg(feature = "mmu")]
use crate::kernel::memory;
#[cfg(feature = "mmu")]
use crate::kernel::process::USER_STACK_PAGES;
use crate::kernel::process::{AddressSpace, FileObject, ProcessId, ProcessTable, ThreadId};
#[cfg(feature = "mmu")]
use crate::kernel::process::{user_stack_bottom, user_stack_top};
use crate::kernel::queue::ReadyQueue;
use crate::kernel::sleep::SleepQueue;
use crate::kernel::syscall::{
    EAGAIN, EBADF, EINVAL, ENOENT, ENOMEM, SYS_BLOCK, SYS_CLOSE, SYS_EXEC, SYS_EXIT, SYS_OPEN,
    SYS_READ, SYS_SLEEP, SYS_SPAWN, SYS_WAIT, SYS_WAKE, SYS_WRITE, SYS_YIELD,
};
use crate::kernel::syscall::{O_APPEND, O_CREAT, O_TRUNC};
use crate::kernel::task::{MAX_TASKS, TASK_STATES, Task, TaskControlBlock, TaskId, TaskState};
use crate::kernel::user::{UserPtr, copy_from_user, copy_to_user};
use crate::kernel::wait::WaitQueueTable;
use crate::{print, println};

unsafe extern "C" {
    fn context_restore(sp: *mut u32) -> !;
}

pub const DEFAULT_TIME_SLICE_TICKS: u32 = 1;

#[derive(Clone, Copy)]
enum ScheduleReason {
    Tick,
    Yield,
    Sleep,
    Block,
    Wake,
    Exit,
}

enum SyscallAction {
    Return(u32),
    Block { channel: u32 },
}

impl ScheduleReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Tick => "tick",
            Self::Yield => "yield",
            Self::Sleep => "sleep",
            Self::Block => "block",
            Self::Wake => "wake",
            Self::Exit => "exit",
        }
    }
}

const WAIT_CHANNEL_BASE: u32 = 0x8000_0000;

fn child_wait_channel(parent: ProcessId) -> u32 {
    WAIT_CHANNEL_BASE | ((parent.as_usize() as u32) & 0x7fff_ffff)
}

fn wait_status(pid: ProcessId, exit_code: i32) -> u32 {
    ((pid.as_usize() as u32) << 8) | ((exit_code as u32) & 0xff)
}

fn kernel_address_space() -> AddressSpace {
    let root = {
        #[cfg(feature = "mmu")]
        {
            PhysAddr::new(crate::arch::aarch32::mmu::table_base())
        }
        #[cfg(not(feature = "mmu"))]
        {
            PhysAddr::new(0)
        }
    };
    AddressSpace::kernel(root)
}

pub struct Scheduler {
    tasks: [Task; MAX_TASKS],
    ready: ReadyQueue,
    sleep: SleepQueue,
    wait: WaitQueueTable,
    processes: ProcessTable,
    log: KernelLog,
    count: usize,
    current: Option<TaskId>,
    idle: Option<TaskId>,
    ticks: u64,
    switches: u64,
    next_tid: usize,
    #[cfg(feature = "mmu")]
    next_asid: u16,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: [const { TaskControlBlock::empty() }; MAX_TASKS],
            ready: ReadyQueue::new(),
            sleep: SleepQueue::new(),
            wait: WaitQueueTable::new(),
            processes: ProcessTable::new(),
            log: KernelLog::new(),
            count: 0,
            current: None,
            idle: None,
            ticks: 0,
            switches: 0,
            next_tid: 1,
            #[cfg(feature = "mmu")]
            next_asid: 1,
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
        let id = self.alloc_task_slot();
        let root = {
            #[cfg(feature = "mmu")]
            {
                PhysAddr::new(crate::arch::aarch32::mmu::table_base())
            }
            #[cfg(not(feature = "mmu"))]
            {
                PhysAddr::new(0)
            }
        };
        let process_id = self
            .processes
            .create(name, ProcessId::new(0), AddressSpace::kernel(root));
        let thread_id = self.alloc_thread_id();

        self.tasks[id.index()] = TaskControlBlock::new_kernel(
            id,
            process_id,
            thread_id,
            name,
            entry,
            priority,
            time_slice_ticks,
        );
        self.processes.set_main_thread(process_id, thread_id);
        if name == "idle" {
            self.idle = Some(id);
        }
        self.enqueue_ready(id);
        self.count += 1;
        id
    }

    #[cfg(feature = "mmu")]
    pub fn spawn_user_init(
        &mut self,
        image: UserImage,
        priority: u8,
        time_slice_ticks: u32,
    ) -> TaskId {
        let builtin = self
            .spawn_loaded_user(
                ProcessId::new(0),
                "user-shell",
                priority.saturating_add(1),
                time_slice_ticks,
                |address_space| unsafe { loader::load_builtin_into(address_space, image) },
            )
            .unwrap_or_else(|err| panic!("failed to spawn builtin user shell: {}", err as i32));

        if let Ok(inode) = vfs::lookup_builtin(b"/bin/init") {
            if let Some(file) = vfs::open(inode) {
                if let Ok(task) = self.spawn_loaded_user(
                    ProcessId::new(0),
                    "init-elf",
                    priority,
                    time_slice_ticks,
                    |address_space| unsafe {
                        loader::load_elf_into(address_space, "init-elf", file.data)
                    },
                ) {
                    return task;
                }
            }
        }

        builtin
    }

    #[cfg(feature = "mmu")]
    fn spawn_loaded_user<F>(
        &mut self,
        parent: ProcessId,
        name: &'static str,
        priority: u8,
        time_slice_ticks: u32,
        load: F,
    ) -> Result<TaskId, u32>
    where
        F: FnOnce(&mut AddressSpace) -> Result<loader::LoadedImage, u32>,
    {
        let id = self.try_alloc_task_slot()?;
        let stack_slot = id.index() + 1;
        let (address_space, loaded, user_stack) =
            self.build_user_address_space(stack_slot, load)?;

        let process_id = match self.processes.try_create(name, parent, address_space) {
            Ok(pid) => pid,
            Err(err) => {
                loader::release_address_space_regions(address_space);
                return Err(err);
            }
        };
        let thread_id = self.alloc_thread_id();

        self.tasks[id.index()] = TaskControlBlock::new_user(
            id,
            process_id,
            thread_id,
            name,
            loaded.entry.as_usize(),
            user_stack,
            USER_STACK_PAGES,
            address_space.stack_bottom.as_usize(),
            address_space.stack_top.as_usize(),
            priority,
            time_slice_ticks,
        );
        self.processes.set_main_thread(process_id, thread_id);
        self.enqueue_ready(id);
        self.count += 1;
        Ok(id)
    }

    pub unsafe fn start(&mut self) -> ! {
        let next = self
            .pick_next()
            .unwrap_or_else(|| panic!("no runnable tasks"));
        self.current = Some(next);

        let tick = self.ticks;
        {
            let task = self.task_mut(next);
            task.mark_scheduled(tick);
        }
        self.switch_address_space(next);

        if crate::config::CONFIG_BOOT_VERBOSE {
            let task = self.task(next);
            println!(
                "scheduler: start task {} (pid={}, state={}, prio={}, slice={} tick)",
                task.name,
                task.pid,
                task.state.as_str(),
                task.priority,
                task.time_slice_ticks
            );
        }

        unsafe { context_restore(self.task(next).context.sp) }
    }

    pub unsafe fn tick(&mut self, saved_sp: *mut u32) -> *mut u32 {
        self.ticks = self.ticks.wrapping_add(1);
        self.wake_sleepers();
        self.reap_zombies();

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

    pub unsafe fn device_irq(&mut self, irq: u32, saved_sp: *mut u32) -> *mut u32 {
        let Some(current) = self.current else {
            return saved_sp;
        };

        self.task_mut(current).context = TaskContext { sp: saved_sp };

        if irq == uart::irq() {
            let bytes = uart::handle_irq(|byte| {
                console::push_input(byte);
            });
            if bytes != 0 {
                self.wake_console_readers();
                self.log.push(LogEvent::ConsoleInput {
                    tick: self.ticks,
                    bytes,
                    available: console::available(),
                    dropped: console::dropped(),
                });
            }
        }

        if self.task(current).state != TaskState::Running {
            return self.schedule_from_current(current, ScheduleReason::Wake);
        }

        if Some(current) == self.idle && !self.ready.is_empty() {
            return self.schedule_from_current(current, ScheduleReason::Wake);
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

        let frame = unsafe { TrapFrame::from_saved_sp(saved_sp) };
        let syscall = frame.syscall_number();
        let arg0 = frame.syscall_arg0();
        let arg1 = frame.syscall_arg1();
        let arg2 = frame.syscall_arg2();

        {
            let task = self.task_mut(current);
            task.context = TaskContext { sp: saved_sp };
            task.stats.last_syscall = syscall;
        }

        match syscall {
            SYS_YIELD => {
                frame.set_return_value(0);
                self.schedule_from_current(current, ScheduleReason::Yield)
            }
            SYS_SLEEP => {
                let wake_at = self.ticks.wrapping_add(u64::from(arg0.max(1)));
                self.task_mut(current).mark_sleeping(wake_at);
                self.sleep.insert(current, wake_at);
                frame.set_return_value(0);
                self.schedule_from_current(current, ScheduleReason::Sleep)
            }
            SYS_BLOCK => {
                self.task_mut(current).mark_blocked(arg0);
                self.wait.sleep_on(arg0, current);
                frame.set_return_value(0);
                self.schedule_from_current(current, ScheduleReason::Block)
            }
            SYS_WAKE => {
                let count = self.wake_channel(arg0);
                frame.set_return_value(count as u32);
                saved_sp
            }
            SYS_WRITE => {
                let result = self.sys_write(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_READ => match self.sys_read(current, arg0, arg1, arg2) {
                SyscallAction::Return(result) => {
                    frame.set_return_value(result);
                    saved_sp
                }
                SyscallAction::Block { channel } => {
                    frame.set_return_value(EAGAIN);
                    self.wait.sleep_on(channel, current);
                    self.schedule_from_current(current, ScheduleReason::Block)
                }
            },
            SYS_OPEN => {
                let result = self.sys_open(current, arg0, arg1, arg2);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_CLOSE => {
                let result = self.sys_close(current, arg0);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_WAIT => match self.try_wait(current, ProcessId::new(arg0 as usize)) {
                Ok(result) => {
                    frame.set_return_value(result);
                    saved_sp
                }
                Err(EAGAIN) => {
                    frame.set_return_value(EAGAIN);
                    let channel = child_wait_channel(self.task(current).process_id);
                    let task = self.task_mut(current);
                    task.mark_waiting_for_child(channel, arg0 as usize);
                    self.wait.sleep_on(channel, current);
                    self.schedule_from_current(current, ScheduleReason::Block)
                }
                Err(err) => {
                    frame.set_return_value(err);
                    saved_sp
                }
            },
            SYS_SPAWN => {
                let result = self.sys_spawn(current, arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_EXEC => {
                let result = self.sys_exec(current, arg0, arg1);
                frame.set_return_value(result);
                saved_sp
            }
            SYS_EXIT => {
                let exit_code = arg0 as i32;
                let tick = self.ticks;
                let (name, pid, process_id) = {
                    let task = self.task_mut(current);
                    task.mark_zombie(exit_code);
                    (task.name, task.pid, task.process_id)
                };
                self.processes.mark_zombie(process_id, exit_code);
                self.log.push(LogEvent::TaskExit { tick, name, pid });
                frame.set_return_value(0);
                self.schedule_from_current(current, ScheduleReason::Exit)
            }
            _ => {
                frame.set_return_value(EINVAL);
                saved_sp
            }
        }
    }

    pub fn wake_channel(&mut self, channel: u32) -> usize {
        let mut woken_tasks = [None; MAX_TASKS];
        let mut woken_len = 0;
        let count = self.wait.wake_all(channel, |task| {
            woken_tasks[woken_len] = Some(task);
            woken_len += 1;
        });

        for item in woken_tasks.iter().take(woken_len) {
            if let Some(task) = *item {
                if self.task(task).state == TaskState::Blocked {
                    let t = self.task_mut(task);
                    t.state = TaskState::Ready;
                    t.wait_channel = 0;
                    t.wait_pid = 0;
                    t.wait_user_buf = 0;
                    t.wait_len = 0;
                    self.enqueue_ready(task);
                }
            }
        }

        if count != 0 {
            self.log.push(LogEvent::Wake {
                tick: self.ticks,
                channel,
                count,
            });
        }
        count
    }

    pub fn flush_logs(&mut self) {
        self.log.flush();
    }

    pub fn dump_recent_logs(&self) {
        self.log.dump_recent(16);
    }

    pub fn dump_tasks(&self) {
        print!("scheduler: states");
        for state in TASK_STATES {
            print!(" {}", state.as_str());
        }
        println!();
        println!(
            "scheduler: task table (tasks={}, ready={}, sleep={}, ready_empty={})",
            self.count,
            self.ready.len(),
            self.sleep.len(),
            self.ready.is_empty()
        );
        for idx in 0..MAX_TASKS {
            let task = &self.tasks[idx];
            if task.state == TaskState::Empty {
                continue;
            }
            println!(
                "  id={} pid={} tid={} mode={} as={} proc={} name={} state={} prio={} slice={} remain={} wake={} wait={} kstack={:#010x}+{}p ustack={:#010x}..{:#010x}/phys={:#010x}+{}p entry={:#010x} runtime={} scheduled={} sleep={} block={} write={} last_sys={}",
                task.id.index(),
                task.pid,
                task.thread_id.as_usize(),
                task.mode.as_str(),
                self.processes.address_space_kind(task.process_id),
                self.processes.state_str(task.process_id),
                task.name,
                task.state.as_str(),
                task.priority,
                task.time_slice_ticks,
                task.remaining_ticks,
                task.wake_at_tick,
                task.wait_channel,
                task.kernel_stack_start as usize,
                task.kernel_stack_pages,
                task.user_stack_bottom,
                task.user_stack_top,
                task.user_stack_phys as usize,
                task.user_stack_pages,
                task.user_entry,
                task.stats.runtime_ticks,
                task.stats.scheduled_count,
                task.stats.sleep_count,
                task.stats.block_count,
                task.stats.bytes_written,
                task.stats.last_syscall
            );
        }
    }

    pub fn dump_tasks_summary(&self) {
        println!(
            "  ticks={} switches={} tasks={} ready={} sleep={}",
            self.ticks,
            self.switches,
            self.count,
            self.ready.len(),
            self.sleep.len()
        );
        for idx in 0..MAX_TASKS {
            let task = &self.tasks[idx];
            if task.state == TaskState::Empty {
                continue;
            }
            println!(
                "  id={} pid={} tid={} mode={} name={} state={} prio={} runtime={} sched={} last_sys={} kstack={:#010x}+{}p ustack={:#010x}..{:#010x}",
                task.id.index(),
                task.pid,
                task.thread_id.as_usize(),
                task.mode.as_str(),
                task.name,
                task.state.as_str(),
                task.priority,
                task.stats.runtime_ticks,
                task.stats.scheduled_count,
                task.stats.last_syscall,
                task.kernel_stack_start as usize,
                task.kernel_stack_pages,
                task.user_stack_bottom,
                task.user_stack_top
            );
        }
    }

    pub fn current_task_name(&self) -> &'static str {
        self.current
            .map(|id| self.task(id).name)
            .unwrap_or("<none>")
    }

    pub fn current_task_pid(&self) -> usize {
        self.current.map(|id| self.task(id).pid).unwrap_or(0)
    }

    pub fn current_task_last_syscall(&self) -> u32 {
        self.current
            .map(|id| self.task(id).stats.last_syscall)
            .unwrap_or(0)
    }

    fn schedule_from_current(&mut self, current: TaskId, reason: ScheduleReason) -> *mut u32 {
        self.enqueue_if_runnable(current);

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
        self.switch_address_space(next);

        if previous != next {
            self.log.push(LogEvent::Schedule {
                tick: self.ticks,
                from: self.task(previous).name,
                to: self.task(next).name,
                reason: reason.as_str(),
                ready: self.ready.len(),
                switches: self.switches,
            });
        }

        self.task(next).context.sp
    }

    fn enqueue_if_runnable(&mut self, task_id: TaskId) {
        if self.task(task_id).is_runnable() {
            self.task_mut(task_id).state = TaskState::Ready;
            self.enqueue_ready(task_id);
        }
    }

    fn enqueue_ready(&mut self, task_id: TaskId) {
        let priority = self.task(task_id).priority;
        self.ready.push_back(task_id, priority);
    }

    fn wake_sleepers(&mut self) {
        while let Some(task) = self.sleep.pop_expired(self.ticks) {
            if self.task(task).state == TaskState::Sleeping {
                let t = self.task_mut(task);
                t.state = TaskState::Ready;
                t.wake_at_tick = 0;
                self.enqueue_ready(task);
            }
        }
    }

    fn reap_zombies(&mut self) {
        for idx in 0..MAX_TASKS {
            if self.tasks[idx].state != TaskState::Zombie {
                continue;
            }

            if Some(self.tasks[idx].id) == self.current {
                continue;
            }

            let pid = self.tasks[idx].pid;
            let name = self.tasks[idx].name;
            let process_id = self.tasks[idx].process_id;
            let parent = self.processes.parent_of(process_id);
            let old_address_space = self
                .processes
                .mark_resources_reaped(process_id, kernel_address_space());
            #[cfg(feature = "mmu")]
            if let Some(address_space) = old_address_space {
                loader::release_address_space_regions(address_space);
            }
            #[cfg(not(feature = "mmu"))]
            let _ = old_address_space;
            unsafe {
                self.tasks[idx].release_resources();
            }
            self.count = self.count.saturating_sub(1);
            self.log.push(LogEvent::TaskReap {
                tick: self.ticks,
                name,
                pid,
            });

            if self.processes.has_live_parent(process_id) {
                self.wake_waiters(parent);
            } else {
                self.processes.release(process_id);
            }
        }
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        while let Some(candidate) = self.ready.pop_front() {
            if self.task(candidate).state == TaskState::Ready {
                return Some(candidate);
            }
        }

        self.idle
            .filter(|idle| self.task(*idle).state == TaskState::Ready)
    }

    fn alloc_task_slot(&self) -> TaskId {
        self.try_alloc_task_slot()
            .unwrap_or_else(|_| panic!("task table full"))
    }

    fn try_alloc_task_slot(&self) -> Result<TaskId, u32> {
        for idx in 0..MAX_TASKS {
            if self.tasks[idx].state == TaskState::Empty {
                return Ok(TaskId::new(idx));
            }
        }
        Err(ENOMEM)
    }

    fn alloc_thread_id(&mut self) -> ThreadId {
        let id = ThreadId::new(self.next_tid);
        self.next_tid = self.next_tid.wrapping_add(1).max(1);
        id
    }

    #[cfg(feature = "mmu")]
    fn alloc_asid(&mut self) -> u16 {
        let asid = self.next_asid;
        self.next_asid = self.next_asid.wrapping_add(1);
        if self.next_asid == 0 {
            self.next_asid = 1;
        }
        asid
    }

    fn switch_address_space(&self, task_id: TaskId) {
        #[cfg(feature = "mmu")]
        if let Some(address_space) = self.processes.address_space(self.task(task_id).process_id) {
            let root = if address_space.page_table_root.as_usize() == 0 {
                crate::arch::aarch32::mmu::table_base()
            } else {
                address_space.page_table_root.as_usize()
            };
            crate::arch::aarch32::mmu::switch_table(root, address_space.asid);
        }
        #[cfg(not(feature = "mmu"))]
        let _ = task_id;
    }

    #[cfg(feature = "mmu")]
    fn build_user_address_space<F>(
        &mut self,
        stack_slot: usize,
        load: F,
    ) -> Result<(AddressSpace, loader::LoadedImage, *mut u8), u32>
    where
        F: FnOnce(&mut AddressSpace) -> Result<loader::LoadedImage, u32>,
    {
        let stack_bottom = user_stack_bottom(stack_slot);
        let stack_top = user_stack_top(stack_slot);
        let root = unsafe { crate::arch::aarch32::mmu::create_user_table() }.ok_or(ENOMEM)?;
        let asid = self.alloc_asid();

        let mut address_space = AddressSpace::user(
            PhysAddr::new(root),
            asid,
            crate::arch::aarch32::mmu::L1_TABLE_PAGES,
            VirtAddr::new(0),
            VirtAddr::new(0),
            VirtAddr::new(0),
            VirtAddr::new(stack_bottom),
            VirtAddr::new(stack_top),
        );

        let loaded = match load(&mut address_space) {
            Ok(loaded) => loaded,
            Err(err) => {
                loader::release_address_space_regions(address_space);
                return Err(err);
            }
        };
        address_space.entry = loaded.entry;
        address_space.user_start = loaded.user_start;
        address_space.user_end = loaded.user_end;

        let Some(user_stack) = memory::alloc_pages(USER_STACK_PAGES) else {
            loader::release_address_space_regions(address_space);
            return Err(ENOMEM);
        };

        if let Err(err) = self.map_user_stack(&mut address_space, stack_bottom, user_stack as usize)
        {
            unsafe {
                memory::free_pages(user_stack, USER_STACK_PAGES);
            }
            loader::release_address_space_regions(address_space);
            return Err(err);
        }

        Ok((address_space, loaded, user_stack))
    }

    #[cfg(feature = "mmu")]
    fn map_user_stack(
        &mut self,
        address_space: &mut AddressSpace,
        stack_bottom: usize,
        phys: usize,
    ) -> Result<(), u32> {
        let mut addr = stack_bottom;
        let end = stack_bottom + USER_STACK_PAGES * memory::PAGE_SIZE;
        while addr < end {
            let l2 = unsafe {
                crate::arch::aarch32::mmu::ensure_user_l2(
                    address_space.page_table_root.as_usize(),
                    addr,
                )
            }
            .ok_or(ENOMEM)?;
            if let Err(err) = address_space.try_add_owned_l2(PhysAddr::new(l2)) {
                unsafe {
                    crate::arch::aarch32::mmu::free_user_l2(l2);
                }
                return Err(err);
            }
            addr = crate::kernel::address::align_up(addr + 1, 1024 * 1024);
        }
        unsafe {
            crate::arch::aarch32::mmu::map_user_pages_in(
                address_space.page_table_root.as_usize(),
                stack_bottom,
                phys,
                USER_STACK_PAGES,
                crate::arch::aarch32::mmu::UserMapping::Stack,
            );
        }
        address_space.try_add_owned_region(
            VirtAddr::new(stack_bottom),
            PhysAddr::new(phys),
            USER_STACK_PAGES,
        )
    }

    fn sys_write(&mut self, current: TaskId, fd: u32, buf: u32, len: u32) -> u32 {
        let fd = fd as usize;
        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd) else {
            return EBADF;
        };
        if !file.object.can_write() {
            return EBADF;
        }

        let len = len as usize;
        if len == 0 {
            return 0;
        }

        match file.object {
            FileObject::Regular { inode } => {
                let mut done = 0usize;
                let mut chunk = [0u8; 128];
                let mut offset = if file.flags & O_APPEND != 0 {
                    vfs::metadata(inode)
                        .map(|meta| meta.size)
                        .unwrap_or(file.offset)
                } else {
                    file.offset
                };
                while done < len {
                    let count = (len - done).min(chunk.len());
                    let ptr = UserPtr::new((buf as usize).wrapping_add(done));
                    let result = unsafe { copy_from_user(&mut chunk[..count], ptr) };
                    if let Err(err) = result {
                        return if done == 0 { err } else { done as u32 };
                    }
                    match vfs::write(inode, offset, &chunk[..count]) {
                        Ok(written) => {
                            done += written;
                            offset = offset.saturating_add(written);
                            if written < count {
                                break;
                            }
                        }
                        Err(err) => return if done == 0 { err } else { done as u32 },
                    }
                }
                if let Some(open_file) = self.processes.file_mut(process_id, fd) {
                    open_file.offset = offset;
                }
                self.task_mut(current).stats.bytes_written = self
                    .task(current)
                    .stats
                    .bytes_written
                    .wrapping_add(done as u64);
                done as u32
            }
            FileObject::ConsoleOut | FileObject::ConsoleErr => {
                let mut done = 0usize;
                let mut chunk = [0u8; 128];
                while done < len {
                    let count = (len - done).min(chunk.len());
                    let ptr = UserPtr::new((buf as usize).wrapping_add(done));
                    let result = unsafe { copy_from_user(&mut chunk[..count], ptr) };
                    if let Err(err) = result {
                        return if done == 0 { err } else { done as u32 };
                    }

                    for byte in &chunk[..count] {
                        if *byte == b'\n' {
                            uart::put_byte(b'\r');
                        }
                        uart::put_byte(*byte);
                    }
                    done += count;
                }

                self.task_mut(current).stats.bytes_written = self
                    .task(current)
                    .stats
                    .bytes_written
                    .wrapping_add(done as u64);
                done as u32
            }
            _ => EBADF,
        }
    }

    fn sys_read(&mut self, current: TaskId, fd: u32, buf: u32, len: u32) -> SyscallAction {
        let fd = fd as usize;
        if !self
            .processes
            .can_read_fd(self.task(current).process_id, fd)
        {
            return SyscallAction::Return(EBADF);
        }

        let len = len as usize;
        if len == 0 {
            return SyscallAction::Return(0);
        }

        let process_id = self.task(current).process_id;
        let Some(file) = self.processes.file(process_id, fd) else {
            return SyscallAction::Return(EBADF);
        };

        match file.object {
            FileObject::Regular { inode } => {
                let mut chunk = [0u8; 128];
                let mut done = 0usize;
                while done < len {
                    let want = (len - done).min(chunk.len());
                    let count = match vfs::read(inode, file.offset + done, &mut chunk[..want]) {
                        Ok(count) => count,
                        Err(err) => {
                            return SyscallAction::Return(if done == 0 {
                                err
                            } else {
                                done as u32
                            });
                        }
                    };
                    if count == 0 {
                        break;
                    }
                    let dst = UserPtr::new((buf as usize).wrapping_add(done));
                    let result = unsafe { copy_to_user(dst, &chunk[..count]) };
                    if let Err(err) = result {
                        return SyscallAction::Return(if done == 0 { err } else { done as u32 });
                    }
                    done += count;
                }
                if let Some(open_file) = self.processes.file_mut(process_id, fd) {
                    open_file.offset = open_file.offset.saturating_add(done);
                }
                SyscallAction::Return(done as u32)
            }
            FileObject::ConsoleIn => {
                let mut chunk = [0u8; 128];
                let count = console::pop_into(&mut chunk[..len.min(128)]);
                if count != 0 {
                    let result =
                        unsafe { copy_to_user(UserPtr::new(buf as usize), &chunk[..count]) };
                    return match result {
                        Ok(()) => SyscallAction::Return(count as u32),
                        Err(err) => SyscallAction::Return(err),
                    };
                }

                let task = self.task_mut(current);
                task.mark_waiting_for_read(console::INPUT_WAIT_CHANNEL, buf as usize, len);
                SyscallAction::Block {
                    channel: console::INPUT_WAIT_CHANNEL,
                }
            }
            _ => SyscallAction::Return(EBADF),
        }
    }

    fn sys_open(&mut self, current: TaskId, path_ptr: u32, path_len: u32, flags: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        let copied = if len == 0 {
            0
        } else {
            match unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) } {
                Ok(()) => len,
                Err(err) => return err,
            }
        };

        let inode = match vfs::lookup(&path[..copied]) {
            Ok(inode) => inode,
            Err(ENOENT) if flags & O_CREAT != 0 => match vfs::create_file(&path[..copied], 0o644) {
                Ok(inode) => inode,
                Err(err) => return err,
            },
            Err(err) => return err,
        };
        let Some(metadata) = vfs::metadata(inode) else {
            return ENOENT;
        };
        if metadata.file_type != vfs::FileType::Regular
            && metadata.file_type != vfs::FileType::Symlink
        {
            return EINVAL;
        }
        if flags & O_TRUNC != 0 {
            if let Err(err) = vfs::truncate(inode, 0) {
                return err;
            }
        }
        match self.processes.open_file(
            self.task(current).process_id,
            FileObject::Regular { inode },
            flags,
        ) {
            Ok(fd) => fd as u32,
            Err(err) => err,
        }
    }

    fn sys_close(&mut self, current: TaskId, fd: u32) -> u32 {
        match self
            .processes
            .close_file(self.task(current).process_id, fd as usize)
        {
            Ok(()) => 0,
            Err(err) => err,
        }
    }

    fn try_wait(&mut self, current: TaskId, pid: ProcessId) -> Result<u32, u32> {
        let current_pid = self.task(current).process_id;
        self.processes
            .take_zombie_child(current_pid, pid)
            .map(|(child, code)| wait_status(child, code))
    }

    fn sys_spawn(&mut self, current: TaskId, path_ptr: u32, path_len: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        let result = unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) };
        if let Err(err) = result {
            return err;
        }

        #[cfg(feature = "mmu")]
        {
            let (name, data) = match self.lookup_exec_file(&path[..len]) {
                Ok(file) => file,
                Err(err) => return err,
            };
            let parent = self.task(current).process_id;
            let spawned = self.spawn_loaded_user(parent, name, 1, 1, |address_space| unsafe {
                loader::load_elf_into(address_space, name, data)
            });
            return match spawned {
                Ok(task) => self.task(task).pid as u32,
                Err(err) => err,
            };
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = current;
            ENOENT
        }
    }

    fn sys_exec(&mut self, current: TaskId, path_ptr: u32, path_len: u32) -> u32 {
        let mut path = [0u8; 96];
        let len = path_len.min(path.len() as u32) as usize;
        let result = unsafe { copy_from_user(&mut path[..len], UserPtr::new(path_ptr as usize)) };
        if let Err(err) = result {
            return err;
        }

        #[cfg(feature = "mmu")]
        {
            let (name, data) = match self.lookup_exec_file(&path[..len]) {
                Ok(file) => file,
                Err(err) => return err,
            };

            return self.exec_loaded_user(current, name, data);
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = current;
            crate::kernel::syscall::ENOEXEC
        }
    }

    #[cfg(feature = "mmu")]
    fn lookup_exec_file(&self, path: &[u8]) -> Result<(&'static str, &'static [u8]), u32> {
        let inode = vfs::lookup(path)?;
        let Some(metadata) = vfs::metadata(inode) else {
            return Err(ENOENT);
        };
        if metadata.file_type != vfs::FileType::Regular {
            return Err(EINVAL);
        }
        let Some(file) = vfs::open(inode) else {
            return Err(ENOENT);
        };
        if file.data.is_empty() {
            return Err(ENOENT);
        }
        Ok((file.name, file.data))
    }

    #[cfg(feature = "mmu")]
    fn exec_loaded_user(&mut self, current: TaskId, name: &'static str, elf: &[u8]) -> u32 {
        if !self.task(current).mode.is_user() {
            return EINVAL;
        }

        let process_id = self.task(current).process_id;
        let Some(old_space) = self.processes.address_space(process_id) else {
            return EINVAL;
        };
        if !old_space.kind.is_user() {
            return EINVAL;
        }

        let stack_slot = current.index() + 1;
        let (new_space, loaded, user_stack) = match self
            .build_user_address_space(stack_slot, |address_space| unsafe {
                loader::load_elf_into(address_space, name, elf)
            }) {
            Ok(parts) => parts,
            Err(err) => return err,
        };

        let old_space = self
            .processes
            .replace_address_space(process_id, new_space)
            .unwrap_or(old_space);
        self.processes.rename(process_id, loaded.name);
        self.switch_address_space(current);
        loader::release_address_space_regions(old_space);

        let frame = unsafe { TrapFrame::from_saved_sp(self.task(current).context.sp) };
        frame.user_sp = (new_space.stack_top.as_usize() & !0xf) as u32;
        frame.user_lr = 0;
        frame.lr = 0;
        frame.pc = loaded.entry.as_usize() as u32;
        frame.spsr = USER_INITIAL_CPSR;
        frame.set_return_value(0);

        let task = self.task_mut(current);
        task.name = loaded.name;
        task.user_entry = loaded.entry.as_usize();
        task.user_stack_phys = user_stack;
        task.user_stack_pages = USER_STACK_PAGES;
        task.user_stack_bottom = new_space.stack_bottom.as_usize();
        task.user_stack_top = new_space.stack_top.as_usize();
        task.stats.bytes_written = 0;
        0
    }

    fn wake_waiters(&mut self, parent: ProcessId) {
        if !parent.is_valid() {
            return;
        }

        let channel = child_wait_channel(parent);
        let mut woken_tasks = [None; MAX_TASKS];
        let mut woken_len = 0;
        self.wait.wake_all(channel, |task| {
            woken_tasks[woken_len] = Some(task);
            woken_len += 1;
        });

        let mut still_waiting = [None; MAX_TASKS];
        let mut still_waiting_len = 0;
        for item in woken_tasks.iter().take(woken_len) {
            let Some(task_id) = *item else {
                continue;
            };
            if self.task(task_id).state != TaskState::Blocked {
                continue;
            }

            let requested = ProcessId::new(self.task(task_id).wait_pid);
            match self.try_wait(task_id, requested) {
                Ok(result) => {
                    let frame = unsafe { TrapFrame::from_saved_sp(self.task(task_id).context.sp) };
                    frame.set_return_value(result);
                    let task = self.task_mut(task_id);
                    task.state = TaskState::Ready;
                    task.wait_channel = 0;
                    task.wait_pid = 0;
                    self.enqueue_ready(task_id);
                }
                Err(EAGAIN) => {
                    still_waiting[still_waiting_len] = Some(task_id);
                    still_waiting_len += 1;
                }
                Err(err) => {
                    let frame = unsafe { TrapFrame::from_saved_sp(self.task(task_id).context.sp) };
                    frame.set_return_value(err);
                    let task = self.task_mut(task_id);
                    task.state = TaskState::Ready;
                    task.wait_channel = 0;
                    task.wait_pid = 0;
                    self.enqueue_ready(task_id);
                }
            }
        }

        for item in still_waiting.iter().take(still_waiting_len) {
            if let Some(task_id) = *item {
                self.wait.sleep_on(channel, task_id);
            }
        }
    }

    fn wake_console_readers(&mut self) {
        let channel = console::INPUT_WAIT_CHANNEL;
        let mut woken_tasks = [None; MAX_TASKS];
        let mut woken_len = 0;
        self.wait.wake_all(channel, |task| {
            woken_tasks[woken_len] = Some(task);
            woken_len += 1;
        });

        for item in woken_tasks.iter().take(woken_len) {
            let Some(task_id) = *item else {
                continue;
            };
            if self.task(task_id).state != TaskState::Blocked
                || self.task(task_id).wait_channel != channel
            {
                continue;
            }

            if console::available() == 0 {
                self.wait.sleep_on(channel, task_id);
                continue;
            }

            let user_buf = self.task(task_id).wait_user_buf;
            let max_len = self.task(task_id).wait_len.min(128);
            let mut chunk = [0u8; 128];
            let count = console::pop_into(&mut chunk[..max_len]);
            let result = if count == 0 {
                EAGAIN
            } else {
                match self.copy_to_task_user(task_id, user_buf, &chunk[..count]) {
                    Ok(()) => count as u32,
                    Err(err) => err,
                }
            };

            let frame = unsafe { TrapFrame::from_saved_sp(self.task(task_id).context.sp) };
            frame.set_return_value(result);
            let task = self.task_mut(task_id);
            task.state = TaskState::Ready;
            task.wait_channel = 0;
            task.wait_user_buf = 0;
            task.wait_len = 0;
            self.enqueue_ready(task_id);
        }
    }

    fn copy_to_task_user(&self, task_id: TaskId, user_buf: usize, bytes: &[u8]) -> Result<(), u32> {
        #[cfg(feature = "mmu")]
        {
            let restore = self.current;
            self.switch_address_space(task_id);
            let result = unsafe { copy_to_user(UserPtr::new(user_buf), bytes) };
            if let Some(current) = restore {
                self.switch_address_space(current);
            }
            result
        }

        #[cfg(not(feature = "mmu"))]
        {
            let _ = task_id;
            unsafe { copy_to_user(UserPtr::new(user_buf), bytes) }
        }
    }

    fn task(&self, id: TaskId) -> &Task {
        &self.tasks[id.index()]
    }

    fn task_mut(&mut self, id: TaskId) -> &mut Task {
        &mut self.tasks[id.index()]
    }
}
