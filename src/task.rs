use crate::context::{TaskContext, TaskEntry};

pub const MAX_TASKS: usize = 4;
const TASK_STACK_SIZE: usize = 16 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Empty,
    Ready,
    Running,
}

pub struct Task {
    pub id: usize,
    pub name: &'static str,
    pub context: TaskContext,
    pub state: TaskState,
}

impl Task {
    pub const fn empty() -> Self {
        Self {
            id: 0,
            name: "",
            context: TaskContext::empty(),
            state: TaskState::Empty,
        }
    }

    pub fn new(id: usize, name: &'static str, entry: TaskEntry) -> Self {
        unsafe {
            let idx = alloc_stack_index();
            let stack = core::ptr::addr_of_mut!(TASK_STACKS[idx].0) as *mut u8;
            let context = TaskContext::new(stack, TASK_STACK_SIZE, entry);

            Self {
                id,
                name,
                context,
                state: TaskState::Ready,
            }
        }
    }
}

#[repr(C, align(16))]
struct TaskStack([u8; TASK_STACK_SIZE]);

static mut TASK_STACKS: [TaskStack; MAX_TASKS] = [
    TaskStack([0; TASK_STACK_SIZE]),
    TaskStack([0; TASK_STACK_SIZE]),
    TaskStack([0; TASK_STACK_SIZE]),
    TaskStack([0; TASK_STACK_SIZE]),
];

static mut NEXT_STACK: usize = 0;

unsafe fn alloc_stack_index() -> usize {
    let idx = unsafe { NEXT_STACK };
    if idx >= MAX_TASKS {
        panic!("too many tasks");
    }
    unsafe {
        NEXT_STACK = idx + 1;
    }
    idx
}
