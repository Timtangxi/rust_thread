use crate::arch::aarch32::exception::KernelTrapHandler;
use crate::drivers::{gic, timer, uart, virtio};

pub fn handle<H: KernelTrapHandler>(handler: &mut H, saved_sp: *mut u32) -> *mut u32 {
    let irq = gic::acknowledge();
    if irq == timer::physical_timer_irq() {
        timer::reload();
        gic::end_of_interrupt(irq);
        unsafe { handler.on_irq(saved_sp) }
    } else if irq == uart::irq() || virtio::owns_irq(irq) {
        let next_sp = unsafe { handler.on_device_irq(irq, saved_sp) };
        gic::end_of_interrupt(irq);
        next_sp
    } else if irq == gic::SPURIOUS_IRQ {
        saved_sp
    } else {
        gic::end_of_interrupt(irq);
        saved_sp
    }
}
