#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::panic::PanicInfo;

use aarch32_cpu::asm;

mod arch;
mod drivers;
mod fs;
mod kernel;
mod platform;
mod print;

use drivers::{gic, timer, uart, virtio};
use kernel::console;
#[cfg(feature = "mmu")]
use kernel::loader;
use kernel::memory;
use kernel::scheduler::Scheduler;
use kernel::syscall;

use crate::arch::aarch32::cpu;
use crate::arch::aarch32::exception::{self, KernelTrapHandler};
#[cfg(feature = "mmu")]
use crate::arch::aarch32::mmu;

mod config {
    #![allow(dead_code)]
    include!("../include/generated/autoconf.rs");
}

static mut SCHEDULER: Scheduler = Scheduler::new();

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    let boot_info = platform::fdt::init();
    let initrd_image = platform::initrd::init();
    uart::init();

    println!();
    println!("rust aarch32 round-robin kernel");
    if config::CONFIG_BOOT_VERBOSE {
        println!("machine: qemu virt / cortex-a15 / armv7-a");
        println!(
            "boot: source={} fdt={:#010x} ram={:#010x}..{:#010x} mmio={:#010x}..{:#010x}",
            boot_info.source_str(),
            boot_info.fdt_ptr,
            boot_info.ram_start,
            boot_info.ram_end,
            boot_info.mmio_start,
            boot_info.mmio_end
        );
        if boot_info.root_compatible_len != 0 {
            println!("fdt: compatible={}", boot_info.root_compatible_str());
        }
        println!(
            "devices: uart=({:#010x}, irq={}) gicd={:#010x} gicc={:#010x} timer_irq={} virtio_mmio={}",
            uart::base(),
            uart::irq(),
            boot_info.devices.gic.distributor.start,
            boot_info.devices.gic.cpu_interface.start,
            timer::physical_timer_irq(),
            boot_info.devices.virtio_mmio_count
        );
    }

    unsafe {
        memory::init();
    }
    platform::initrd::reserve_loaded_pages(initrd_image);
    let heap_frame = memory::page_frame(memory::heap_start()).unwrap_or_else(|| {
        panic!("heap start is outside physical page metadata");
    });
    let heap_state = memory::page_state(memory::heap_start())
        .map(|state| state.as_str())
        .unwrap_or("unknown");
    let heap_pfn = memory::phys_to_pfn(crate::kernel::address::PhysAddr::new(memory::heap_start()))
        .map(|pfn| pfn.index())
        .unwrap_or(usize::MAX);
    if config::CONFIG_BOOT_VERBOSE {
        println!(
            "memory: buddy allocator heap={:#010x} heap_pfn={} high={:#010x} total={} free={} reserved={} allocated={} free_blocks={} largest_order={} heap_state={} heap_order={} heap_ref={} heap_flags={:#x}",
            memory::heap_start(),
            heap_pfn,
            memory::next_free(),
            memory::total_pages(),
            memory::free_page_count(),
            memory::reserved_pages(),
            memory::allocated_pages(),
            memory::free_ranges(),
            memory::largest_free_order().unwrap_or(0),
            heap_state,
            heap_frame.order,
            heap_frame.ref_count,
            heap_frame.flags
        );
        println!(
            "memory: buddy free orders o0={} o1={} o2={} o3={} o4={} o5={} o6={} o7={} o8={} o9={} o10={} o11={} o12={} o13={} o14={} o15={}",
            memory::buddy_order_count(0),
            memory::buddy_order_count(1),
            memory::buddy_order_count(2),
            memory::buddy_order_count(3),
            memory::buddy_order_count(4),
            memory::buddy_order_count(5),
            memory::buddy_order_count(6),
            memory::buddy_order_count(7),
            memory::buddy_order_count(8),
            memory::buddy_order_count(9),
            memory::buddy_order_count(10),
            memory::buddy_order_count(11),
            memory::buddy_order_count(12),
            memory::buddy_order_count(13),
            memory::buddy_order_count(14),
            memory::buddy_order_count(15)
        );
    }

    cpu::install_exception_vectors();

    #[cfg(feature = "mmu")]
    {
        let info = unsafe { mmu::init() };
        if config::CONFIG_BOOT_VERBOSE {
            println!(
                "mmu: enabled={} l1={:#010x} kernel={:#010x}..{:#010x} ram_pages={} text_pages={} rodata_pages={} user_text_pages={} user_rodata_pages={} data_pages={} device_pages={} high_linear_pages={} high_device_pages={} l2_tables={} icache={} dcache={} sctlr={:#010x}",
                info.enabled,
                info.table_base,
                info.kernel_start,
                info.kernel_end,
                info.ram_pages,
                info.kernel_text_pages,
                info.kernel_rodata_pages,
                info.user_text_pages,
                info.user_rodata_pages,
                info.kernel_data_pages,
                info.device_pages,
                info.high_linear_pages,
                info.high_device_pages,
                info.l2_tables,
                info.icache_enabled,
                info.dcache_enabled,
                info.sctlr
            );
        }
    }

    #[cfg(not(feature = "mmu"))]
    println!("mmu: disabled (built without default feature `mmu`)");

    fs::vfs::init();
    println!(
        "fs: initrd={} external={} format={} files={} builtin={}",
        config::CONFIG_INITRD,
        initrd_image.is_present(),
        initrd_image.format.as_str(),
        fs::vfs::external_files(),
        fs::vfs::builtin_files()
    );
    if initrd_image.is_present() {
        if let Some((path, size)) = rootfs_probe() {
            println!("fs: rootfs probe {} size={}", path, size);
        }
    }

    gic::init();

    if config::CONFIG_VIRTIO_MMIO {
        let mut virtio_probes =
            [const { virtio::VirtioProbe::empty() }; platform::fdt::MAX_VIRTIO_MMIO];
        let virtio_count = virtio::probe_all(&mut virtio_probes);
        if config::CONFIG_BOOT_VERBOSE {
            for probe in virtio_probes.iter().take(virtio_count) {
                println!(
                    "virtio-mmio: base={:#010x} size={:#x} irq={} version={} device={} vendor={:#010x} queue0={} kind={} supported={}",
                    probe.device.reg.start,
                    probe.device.reg.size,
                    probe.device.irq.irq,
                    probe.version,
                    probe.device_id,
                    probe.vendor_id,
                    probe.queue0_max,
                    probe.kind().as_str(),
                    probe.supported
                );
            }
        }
    }

    let tick_hz = timer::init(100);
    if config::CONFIG_BOOT_VERBOSE {
        println!(
            "timer: generic physical timer at {} Hz irq={}",
            tick_hz,
            timer::physical_timer_irq()
        );
        println!(
            "console: pl011 irq={} mode={} input_buffer={} line_buffer={} bytes",
            uart::irq(),
            console::mode().as_str(),
            console::INPUT_CAPACITY,
            console::LINE_CAPACITY
        );
    }

    unsafe {
        let scheduler = &raw mut SCHEDULER;
        (*scheduler).spawn_with_options("idle", task_idle, 0, 1);
        if config::CONFIG_DEMO_KERNEL_TASKS {
            (*scheduler).spawn_with_options("init", task_init, 1, 2);
            (*scheduler).spawn("shell", task_shell);
            (*scheduler).spawn_with_options("worker", task_worker, 1, 1);
            (*scheduler).spawn_with_options("waker", task_waker, 2, 1);
        }
        #[cfg(feature = "mmu")]
        (*scheduler).spawn_user_init(loader::builtin_init_image(), 1, 1);
        if config::CONFIG_BOOT_VERBOSE {
            (*scheduler).dump_tasks();
        }
        (*scheduler).start();
    }
}

fn rootfs_probe() -> Option<(&'static str, usize)> {
    for path in [
        b"bin/busybox".as_slice(),
        b"sbin/init".as_slice(),
        b"init".as_slice(),
    ] {
        let Ok(inode) = fs::vfs::lookup(path) else {
            continue;
        };
        let Some(metadata) = fs::vfs::metadata(inode) else {
            continue;
        };
        if metadata.file_type == fs::vfs::FileType::Regular
            || metadata.file_type == fs::vfs::FileType::Symlink
        {
            return Some((core::str::from_utf8(path).unwrap_or("<?>"), metadata.size));
        }
    }
    None
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_rust_entry(saved_sp: *mut u32) -> *mut u32 {
    unsafe {
        let scheduler = &raw mut SCHEDULER;
        exception::handle_irq(&mut *scheduler, saved_sp)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn svc_rust_entry(saved_sp: *mut u32) -> *mut u32 {
    unsafe {
        let scheduler = &raw mut SCHEDULER;
        exception::handle_svc(&mut *scheduler, saved_sp)
    }
}

fn flush_scheduler_logs() {
    if !config::CONFIG_BOOT_VERBOSE {
        return;
    }
    cpu::with_irq_disabled(|| unsafe {
        let scheduler = &raw mut SCHEDULER;
        (*scheduler).flush_logs();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn exception_rust_entry(kind: u32, lr: u32, spsr: u32) -> ! {
    let kind = cpu::ExceptionKind::from_raw(kind);
    unsafe {
        let scheduler = &raw mut SCHEDULER;
        exception::handle_fatal(&*scheduler, kind, lr, spsr);
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
            syscall::exit_with(0);
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
            let woken = syscall::wake(7);
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
    unsafe {
        let scheduler = &raw mut SCHEDULER;
        exception::print_panic_diagnostics(&*scheduler);
    }
    loop {
        asm::wfi();
    }
}

impl KernelTrapHandler for Scheduler {
    unsafe fn on_irq(&mut self, saved_sp: *mut u32) -> *mut u32 {
        unsafe { self.tick(saved_sp) }
    }

    unsafe fn on_device_irq(&mut self, irq: u32, saved_sp: *mut u32) -> *mut u32 {
        unsafe { self.device_irq(irq, saved_sp) }
    }

    unsafe fn on_svc(&mut self, saved_sp: *mut u32) -> *mut u32 {
        unsafe { self.syscall(saved_sp) }
    }

    fn current_task_name(&self) -> &'static str {
        self.current_task_name()
    }

    fn current_task_pid(&self) -> usize {
        self.current_task_pid()
    }

    fn current_task_last_syscall(&self) -> u32 {
        self.current_task_last_syscall()
    }

    fn dump_tasks_summary(&self) {
        self.dump_tasks_summary();
    }

    fn dump_recent_logs(&self) {
        self.dump_recent_logs();
    }
}
