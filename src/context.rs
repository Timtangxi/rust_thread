use core::mem::size_of;

pub type TaskEntry = extern "C" fn() -> !;

pub const SVC_MODE: u32 = 0x13;
pub const FIQ_MASK: u32 = 1 << 6;
pub const INITIAL_CPSR: u32 = SVC_MODE | FIQ_MASK;

#[repr(C)]
pub struct TaskContext {
    pub sp: *mut u32,
}

impl TaskContext {
    pub const fn empty() -> Self {
        Self {
            sp: core::ptr::null_mut(),
        }
    }

    pub unsafe fn new(stack: *mut u8, stack_len: usize, entry: TaskEntry) -> Self {
        let stack_top = unsafe { stack.add(stack_len) } as usize;
        let aligned_top = stack_top & !0xf;
        let frame_words = 16usize;
        let frame = (aligned_top - frame_words * size_of::<u32>()) as *mut u32;

        unsafe {
            for idx in 0..frame_words {
                frame.add(idx).write(0);
            }

            frame.add(13).write(task_exit as usize as u32);
            frame.add(14).write(entry as usize as u32);
            frame.add(15).write(INITIAL_CPSR);
        }

        Self { sp: frame }
    }
}

extern "C" fn task_exit() -> ! {
    loop {
        aarch32_cpu::asm::wfi();
    }
}
