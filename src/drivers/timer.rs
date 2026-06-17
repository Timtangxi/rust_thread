use core::sync::atomic::{AtomicU32, Ordering};

use aarch32_cpu::generic_timer::{El1PhysicalTimer, GenericTimer};

pub const PHYSICAL_TIMER_IRQ: u32 = 30;

static TIMER_TICKS: AtomicU32 = AtomicU32::new(0);

pub fn init(hz: u32) -> u32 {
    let mut timer = unsafe { El1PhysicalTimer::new() };
    let frequency = timer.frequency_hz();
    let ticks = frequency / hz.max(1);

    TIMER_TICKS.store(ticks, Ordering::Relaxed);
    timer.interrupt_mask(false);
    timer.countdown_set(ticks);
    timer.enable(true);

    frequency
}

pub fn reload() {
    let ticks = TIMER_TICKS.load(Ordering::Relaxed);
    let mut timer = unsafe { El1PhysicalTimer::new() };
    timer.countdown_set(ticks);
    timer.interrupt_mask(false);
    timer.enable(true);
}
