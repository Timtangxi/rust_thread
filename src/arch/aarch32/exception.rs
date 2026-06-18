mod fault;
mod irq;
mod svc;

pub trait KernelTrapHandler {
    unsafe fn on_irq(&mut self, saved_sp: *mut u32) -> *mut u32;
    unsafe fn on_device_irq(&mut self, irq: u32, saved_sp: *mut u32) -> *mut u32;
    unsafe fn on_svc(&mut self, saved_sp: *mut u32) -> *mut u32;
    fn current_task_name(&self) -> &'static str;
    fn current_task_pid(&self) -> usize;
    fn current_task_last_syscall(&self) -> u32;
    fn dump_tasks_summary(&self);
    fn dump_recent_logs(&self);
}

#[derive(Clone, Copy)]
pub struct FaultInfo {
    pub status: u32,
    pub address: u32,
    pub class: &'static str,
}

pub fn handle_irq<H: KernelTrapHandler>(handler: &mut H, saved_sp: *mut u32) -> *mut u32 {
    irq::handle(handler, saved_sp)
}

pub fn handle_svc<H: KernelTrapHandler>(handler: &mut H, saved_sp: *mut u32) -> *mut u32 {
    svc::handle(handler, saved_sp)
}

pub use fault::{handle_fatal, print_panic_diagnostics};
