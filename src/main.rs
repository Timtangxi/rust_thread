#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::panic::PanicInfo;

use aarch32_cpu::asm;

mod context;
mod cpu;
mod gic;
mod print;
mod scheduler;
mod task;
mod timer;
mod uart;

use scheduler::Scheduler;
use task::Task;

core::arch::global_asm!(include_str!("boot.S"));
core::arch::global_asm!(include_str!("switch.S"));

static mut SCHEDULER: Scheduler = Scheduler::new();

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    uart::init();

    println!();
    println!("rust aarch32 round-robin kernel");
    println!("machine: qemu virt / cortex-a15 / armv7-a");

    cpu::install_exception_vectors();
    gic::init();

    let tick_hz = timer::init(100);
    println!("timer: generic physical timer at {} Hz", tick_hz);

    unsafe {
        let scheduler = &raw mut SCHEDULER;
        (*scheduler).add(Task::new(1, "init", task_init));
        (*scheduler).add(Task::new(2, "shell", task_shell));
        (*scheduler).add(Task::new(3, "worker", task_worker));
        (*scheduler).start();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_rust_entry(saved_sp: *mut u32) -> *mut u32 {
    let irq = gic::acknowledge();
    match irq {
        timer::PHYSICAL_TIMER_IRQ => {
            timer::reload();
            gic::end_of_interrupt(irq);
            unsafe {
                let scheduler = &raw mut SCHEDULER;
                (*scheduler).tick(saved_sp)
            }
        }
        gic::SPURIOUS_IRQ => saved_sp,
        other => {
            println!("irq: unexpected id {}", other);
            gic::end_of_interrupt(other);
            saved_sp
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn exception_rust_entry(kind: u32, lr: u32, spsr: u32) -> ! {
    println!();
    println!("fatal exception kind={} lr={:#010x} spsr={:#010x}", kind, lr, spsr);
    loop {
        asm::wfi();
    }
}

extern "C" fn task_init() -> ! {
    let mut n = 0u32;
    loop {
        println!("[init] tick work item {}", n);
        n = n.wrapping_add(1);
        busy_delay(60_000);
    }
}

extern "C" fn task_shell() -> ! {
    let mut n = 0u32;
    loop {
        println!("[shell] prompt refresh {}", n);
        n = n.wrapping_add(1);
        busy_delay(90_000);
    }
}

extern "C" fn task_worker() -> ! {
    let mut n = 0u32;
    loop {
        println!("[worker] background pass {}", n);
        n = n.wrapping_add(1);
        busy_delay(120_000);
    }
}

#[inline(never)]
fn busy_delay(iterations: u32) {
    for _ in 0..iterations {
        asm::nop();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    println!("panic: {}", info);
    loop {
        asm::wfi();
    }
}
