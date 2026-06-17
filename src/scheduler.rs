use crate::context::TaskContext;
use crate::println;
use crate::task::{MAX_TASKS, Task, TaskState};

unsafe extern "C" {
    fn context_restore(sp: *mut u32) -> !;
}

pub struct Scheduler {
    tasks: [Task; MAX_TASKS],
    count: usize,
    current: usize,
    ticks: u64,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: [const { Task::empty() }; MAX_TASKS],
            count: 0,
            current: 0,
            ticks: 0,
        }
    }

    pub fn add(&mut self, task: Task) {
        if self.count >= MAX_TASKS {
            panic!("task table full");
        }

        self.tasks[self.count] = task;
        self.count += 1;
    }

    pub unsafe fn start(&mut self) -> ! {
        if self.count == 0 {
            panic!("no runnable tasks");
        }

        self.current = 0;
        self.tasks[0].state = TaskState::Running;
        println!("scheduler: start task {} ({})", self.tasks[0].name, self.tasks[0].id);
        unsafe { context_restore(self.tasks[0].context.sp) }
    }

    pub unsafe fn tick(&mut self, saved_sp: *mut u32) -> *mut u32 {
        if self.count == 0 {
            return saved_sp;
        }

        self.ticks = self.ticks.wrapping_add(1);
        self.tasks[self.current].context = TaskContext { sp: saved_sp };
        self.tasks[self.current].state = TaskState::Ready;

        let previous = self.current;
        self.current = (self.current + 1) % self.count;
        self.tasks[self.current].state = TaskState::Running;

        if previous != self.current {
            println!(
                "tick {:04}: {} -> {}",
                self.ticks, self.tasks[previous].name, self.tasks[self.current].name
            );
        }

        self.tasks[self.current].context.sp
    }
}
