#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::panic::PanicInfo;

use aarch32_cpu::asm;

mod arch;
mod drivers;
mod kernel;
mod print;

use drivers::{gic, timer, uart};
use kernel::memory;
use kernel::scheduler::Scheduler;
use kernel::syscall;

use crate::arch::aarch32::cpu;

static mut SCHEDULER: Scheduler = Scheduler::new();

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    uart::init();

    println!();
    println!("rust aarch32 round-robin kernel");
    println!("machine: qemu virt / cortex-a15 / armv7-a");

    unsafe {
        memory::init();
    }
    println!(
        "memory: page allocator next={:#010x} allocated_pages={}",
        memory::next_free(),
        memory::allocated_pages()
    );

    cpu::install_exception_vectors();
    gic::init();

    let tick_hz = timer::init(100);
    println!("timer: generic physical timer at {} Hz", tick_hz);

    unsafe {
        let scheduler = &raw mut SCHEDULER;
        (*scheduler).spawn_with_options("idle", task_idle, 0, 1);
        (*scheduler).spawn_with_options("init", task_init, 1, 2);
        (*scheduler).spawn("shell", task_shell);
        (*scheduler).spawn_with_options("worker", task_worker, 1, 1);
        (*scheduler).spawn_with_options("waker", task_waker, 2, 1);
        (*scheduler).dump_tasks();
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
pub extern "C" fn svc_rust_entry(saved_sp: *mut u32) -> *mut u32 {
    unsafe {
        let scheduler = &raw mut SCHEDULER;
        (*scheduler).syscall(saved_sp)
    }
}

pub fn wake_channel(channel: u32) -> usize {
    cpu::with_irq_disabled(|| unsafe {
        let scheduler = &raw mut SCHEDULER;
        (*scheduler).wake_channel(channel)
    })
}

fn flush_scheduler_logs() {
    cpu::with_irq_disabled(|| unsafe {
        let scheduler = &raw mut SCHEDULER;
        (*scheduler).flush_logs();
    });
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
        flush_scheduler_logs();
        println!("[init] tick work item {}", n);
        n = n.wrapping_add(1);
        if n % 3 == 0 {
            syscall::sleep(5);
        } else {
            syscall::yield_now();
        }
    }
}

extern "C" fn task_idle() -> ! {
    loop {
        flush_scheduler_logs();
        asm::wfi();
    }
}

extern "C" fn task_shell() -> ! {
    let mut n = 0u32;
    loop {
        flush_scheduler_logs();
        println!("[shell] prompt refresh {}", n);
        n = n.wrapping_add(1);
        if n == 4 {
            println!("[shell] waiting on channel 7");
            syscall::block(7);
            println!("[shell] woke from channel 7");
        }
        syscall::sleep(2);
    }
}

extern "C" fn task_worker() -> ! {
    let mut n = 0u32;
    loop {
        flush_scheduler_logs();
        println!("[worker] background pass {}", n);
        n = n.wrapping_add(1);
        if n == 6 {
            println!("[worker] finished");
            syscall::exit();
        }
        busy_delay(30_000);
    }
}

extern "C" fn task_waker() -> ! {
    let mut n = 0u32;
    loop {
        flush_scheduler_logs();
        println!("[waker] heartbeat {}", n);
        n = n.wrapping_add(1);
        if n == 8 {
            let woken = wake_channel(7);
            println!("[waker] wake channel 7 -> {} task(s)", woken);
        }
        syscall::sleep(1);
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
