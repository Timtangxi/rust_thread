use crate::arch::aarch32::cpu;
use crate::arch::aarch32::exception::{FaultInfo, KernelTrapHandler};
use crate::println;

pub fn handle_fatal<H: KernelTrapHandler>(
    handler: &H,
    kind: cpu::ExceptionKind,
    lr: u32,
    spsr: u32,
) -> ! {
    let banked = cpu::BankedRegisters::capture();

    println!();
    println!(
        "fatal exception kind={} task={} pid={} lr={:#010x} spsr={:#010x} cpsr={:#010x} last_syscall={}",
        kind.as_str(),
        handler.current_task_name(),
        handler.current_task_pid(),
        lr,
        spsr,
        cpu::cpsr(),
        handler.current_task_last_syscall()
    );
    println!(
        "banked: svc_sp={:#010x} svc_lr={:#010x} irq_sp={:#010x} irq_lr={:#010x} usr_sp={:#010x} usr_lr={:#010x}",
        banked.svc_sp, banked.svc_lr, banked.irq_sp, banked.irq_lr, banked.usr_sp, banked.usr_lr
    );

    match fault_info(kind) {
        Some(info) => println!(
            "{}: status={:#010x} class={} address={:#010x}",
            kind.as_str(),
            info.status,
            info.class,
            info.address
        ),
        None => {
            if let cpu::ExceptionKind::Unknown(raw) = kind {
                println!("unknown exception raw={}", raw);
            }
        }
    }

    println!("scheduler snapshot:");
    handler.dump_tasks_summary();
    println!("recent scheduler log:");
    handler.dump_recent_logs();

    loop {
        aarch32_cpu::asm::wfi();
    }
}

pub fn print_panic_diagnostics<H: KernelTrapHandler>(handler: &H) {
    println!(
        "panic context: task={} pid={} cpsr={:#010x} last_syscall={}",
        handler.current_task_name(),
        handler.current_task_pid(),
        cpu::cpsr(),
        handler.current_task_last_syscall()
    );
    handler.dump_tasks_summary();
    handler.dump_recent_logs();
}

fn fault_info(kind: cpu::ExceptionKind) -> Option<FaultInfo> {
    match kind {
        cpu::ExceptionKind::DataAbort => {
            let status = cpu::dfsr();
            Some(FaultInfo {
                status,
                address: cpu::dfar(),
                class: decode_fault_status(status),
            })
        }
        cpu::ExceptionKind::PrefetchAbort => {
            let status = cpu::ifsr();
            Some(FaultInfo {
                status,
                address: cpu::ifar(),
                class: decode_fault_status(status),
            })
        }
        _ => None,
    }
}

fn decode_fault_status(status: u32) -> &'static str {
    let fs = ((status >> 6) & 0x10) | (status & 0x0f);
    match fs {
        0b00001 => "alignment",
        0b00100 => "instruction-cache-maintenance",
        0b01100 => "sync-external-abort",
        0b01110 => "sync-external-abort-translation",
        0b00101 => "translation-section",
        0b00111 => "translation-page",
        0b00011 => "access-flag-section",
        0b00110 => "access-flag-page",
        0b01001 => "domain-section",
        0b01011 => "domain-page",
        0b01101 => "permission-section",
        0b01111 => "permission-page",
        0b00010 => "debug-event",
        0b01000 => "sync-parity",
        0b10000 => "tlb-conflict",
        0b10100 => "lockdown",
        0b10110 => "async-external-abort",
        0b11000 => "async-parity",
        _ => "unknown",
    }
}
