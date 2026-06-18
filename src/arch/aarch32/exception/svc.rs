use crate::arch::aarch32::exception::KernelTrapHandler;

pub fn handle<H: KernelTrapHandler>(handler: &mut H, saved_sp: *mut u32) -> *mut u32 {
    unsafe { handler.on_svc(saved_sp) }
}
