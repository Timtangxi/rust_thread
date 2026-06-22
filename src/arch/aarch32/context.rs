use core::mem::size_of;

pub type TaskEntry = extern "C" fn() -> !;

pub const SVC_MODE: u32 = 0x13;
pub const FIQ_MASK: u32 = 1 << 6;
pub const INITIAL_CPSR: u32 = SVC_MODE | FIQ_MASK;
#[cfg(feature = "mmu")]
pub const USER_MODE: u32 = 0x10;
#[cfg(feature = "mmu")]
pub const USER_INITIAL_CPSR: u32 = USER_MODE | FIQ_MASK;

#[repr(C)]
pub struct TaskContext {
    pub sp: *mut u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrapFrame {
    pub user_sp: u32,
    pub user_lr: u32,
    pub r: [u32; 13],
    pub lr: u32,
    pub pc: u32,
    pub spsr: u32,
}

impl TrapFrame {
    pub unsafe fn from_saved_sp(saved_sp: *mut u32) -> &'static mut Self {
        unsafe { &mut *(saved_sp as *mut Self) }
    }

    pub fn syscall_number(&self) -> u32 {
        self.r[0]
    }

    pub fn syscall_arg0(&self) -> u32 {
        self.r[1]
    }

    pub fn syscall_arg1(&self) -> u32 {
        self.r[2]
    }

    pub fn syscall_arg2(&self) -> u32 {
        self.r[3]
    }

    pub fn syscall_arg3(&self) -> u32 {
        self.r[4]
    }

    pub fn syscall_arg4(&self) -> u32 {
        self.r[5]
    }

    pub fn syscall_arg5(&self) -> u32 {
        self.r[12]
    }

    pub fn kernel_abi_magic(&self) -> u32 {
        self.r[7]
    }

    pub fn linux_syscall_number(&self) -> u32 {
        self.r[7]
    }

    pub fn linux_arg0(&self) -> u32 {
        self.r[0]
    }

    pub fn linux_arg1(&self) -> u32 {
        self.r[1]
    }

    pub fn linux_arg2(&self) -> u32 {
        self.r[2]
    }

    pub fn linux_arg3(&self) -> u32 {
        self.r[3]
    }

    pub fn linux_arg4(&self) -> u32 {
        self.r[4]
    }

    pub fn linux_arg5(&self) -> u32 {
        self.r[5]
    }

    pub fn linux_arg6(&self) -> u32 {
        self.r[6]
    }

    pub fn set_return_value(&mut self, value: u32) {
        self.r[0] = value;
    }
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
        let frame_words = size_of::<TrapFrame>() / size_of::<u32>();
        let frame = (aligned_top - frame_words * size_of::<u32>()) as *mut u32;

        unsafe {
            for idx in 0..frame_words {
                frame.add(idx).write(0);
            }

            let trap = &mut *(frame as *mut TrapFrame);
            trap.lr = task_exit as usize as u32;
            trap.pc = entry as usize as u32;
            trap.spsr = INITIAL_CPSR;
        }

        Self { sp: frame }
    }

    #[cfg(feature = "mmu")]
    pub unsafe fn new_user(
        kernel_stack: *mut u8,
        kernel_stack_len: usize,
        user_entry: usize,
        initial_sp: usize,
    ) -> Self {
        let stack_top = unsafe { kernel_stack.add(kernel_stack_len) } as usize;
        let aligned_top = stack_top & !0xf;
        let frame_words = size_of::<TrapFrame>() / size_of::<u32>();
        let frame = (aligned_top - frame_words * size_of::<u32>()) as *mut u32;

        unsafe {
            for idx in 0..frame_words {
                frame.add(idx).write(0);
            }

            let trap = &mut *(frame as *mut TrapFrame);
            trap.user_sp = initial_sp as u32;
            trap.user_lr = 0;
            trap.lr = 0;
            trap.pc = user_entry as u32;
            trap.spsr = USER_INITIAL_CPSR;
        }

        Self { sp: frame }
    }
}

extern "C" fn task_exit() -> ! {
    loop {
        aarch32_cpu::asm::wfi();
    }
}
