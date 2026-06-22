use core::sync::atomic::{AtomicU32, Ordering};

use aarch32_cpu::generic_timer::{El1PhysicalTimer, GenericTimer};

use crate::drivers::device::{self, DeviceClass, DeviceId, DeviceNode, DeviceState};

static TIMER_TICKS: AtomicU32 = AtomicU32::new(0);

pub fn physical_timer_irq() -> u32 {
    crate::platform::fdt::timer().physical_irq.irq
}

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

pub fn register_device() -> Option<DeviceId> {
    let id = device::register(DeviceNode {
        id: DeviceId::new(usize::MAX),
        name: "arm-physical-timer",
        driver: "arm-generic-timer",
        class: DeviceClass::Timer,
        state: DeviceState::Bound,
        mmio_base: 0,
        mmio_size: 0,
        irq: physical_timer_irq(),
        major: 0,
        minor: 0,
        wait_channel: 0,
    })
    .ok()?;
    let _ = device::mark_ready(id);
    Some(id)
}

pub fn reload() {
    let ticks = TIMER_TICKS.load(Ordering::Relaxed);
    let mut timer = unsafe { El1PhysicalTimer::new() };
    timer.countdown_set(ticks);
    timer.interrupt_mask(false);
    timer.enable(true);
}

pub fn counter() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "mrrc p15, 0, {low}, {high}, c14",
            low = out(reg) low,
            high = out(reg) high,
            options(nomem, nostack, preserves_flags)
        );
    }
    ((high as u64) << 32) | low as u64
}
