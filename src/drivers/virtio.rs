#![allow(dead_code)]

use core::ptr::{read_volatile, write_volatile};

use aarch32_cpu::asm;

use crate::drivers::device::{DeviceError, DeviceIo, DeviceResult, IrqStatus, KernelDriver};
use crate::platform::fdt::{self, MAX_VIRTIO_MMIO, VirtioKind, VirtioMmioDevice};

const VIRTIO_MAGIC: u32 = 0x7472_6976;
const VIRTIO_VERSION_LEGACY: u32 = 1;
const VIRTIO_VERSION_MODERN: u32 = 2;
const VIRTIO_DEVICE_BLOCK: u32 = 2;

const REG_MAGIC: usize = 0x000;
const REG_VERSION: usize = 0x004;
const REG_DEVICE_ID: usize = 0x008;
const REG_VENDOR_ID: usize = 0x00c;
const REG_DEVICE_FEATURES: usize = 0x010;
const REG_DRIVER_FEATURES: usize = 0x020;
const REG_QUEUE_SEL: usize = 0x030;
const REG_QUEUE_NUM_MAX: usize = 0x034;
const REG_STATUS: usize = 0x070;
const REG_INTERRUPT_STATUS: usize = 0x060;
const REG_INTERRUPT_ACK: usize = 0x064;

const STATUS_ACKNOWLEDGE: u32 = 1;
const STATUS_DRIVER: u32 = 2;
const STATUS_DRIVER_OK: u32 = 4;
const STATUS_FEATURES_OK: u32 = 8;
const STATUS_FAILED: u32 = 0x80;

#[derive(Clone, Copy)]
pub struct VirtioProbe {
    pub device: VirtioMmioDevice,
    pub magic: u32,
    pub version: u32,
    pub device_id: u32,
    pub vendor_id: u32,
    pub queue0_max: u32,
    pub features0: u32,
    pub supported: bool,
}

impl VirtioProbe {
    pub const fn empty() -> Self {
        Self {
            device: VirtioMmioDevice::empty(),
            magic: 0,
            version: 0,
            device_id: 0,
            vendor_id: 0,
            queue0_max: 0,
            features0: 0,
            supported: false,
        }
    }

    pub const fn kind(self) -> VirtioKind {
        if self.device_id == VIRTIO_DEVICE_BLOCK {
            VirtioKind::Block
        } else {
            VirtioKind::Unknown
        }
    }

    pub const fn is_block(self) -> bool {
        self.device_id == VIRTIO_DEVICE_BLOCK && self.supported
    }
}

pub struct VirtioBlock {
    probe: VirtioProbe,
    initialized: bool,
}

impl VirtioBlock {
    pub const fn new(probe: VirtioProbe) -> Self {
        Self {
            probe,
            initialized: false,
        }
    }

    pub const fn probe(&self) -> VirtioProbe {
        self.probe
    }
}

impl KernelDriver for VirtioBlock {
    fn name(&self) -> &'static str {
        "virtio-blk"
    }

    fn init(&mut self) -> DeviceResult<()> {
        if !self.probe.is_block() {
            return Err(DeviceError::Unsupported);
        }

        unsafe {
            write_reg(self.probe.device.reg.start, REG_STATUS, 0);
            asm::dsb();
            write_reg(self.probe.device.reg.start, REG_STATUS, STATUS_ACKNOWLEDGE);
            write_reg(
                self.probe.device.reg.start,
                REG_STATUS,
                STATUS_ACKNOWLEDGE | STATUS_DRIVER,
            );
            write_reg(self.probe.device.reg.start, REG_DRIVER_FEATURES, 0);
            write_reg(
                self.probe.device.reg.start,
                REG_STATUS,
                STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK,
            );
            let status = read_reg(self.probe.device.reg.start, REG_STATUS);
            if status & STATUS_FEATURES_OK == 0 {
                write_reg(self.probe.device.reg.start, REG_STATUS, STATUS_FAILED);
                return Err(DeviceError::Unsupported);
            }
            write_reg(
                self.probe.device.reg.start,
                REG_STATUS,
                STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK,
            );
            asm::dsb();
        }

        self.initialized = true;
        Ok(())
    }

    fn irq(&self) -> Option<u32> {
        self.probe
            .device
            .irq
            .is_present()
            .then_some(self.probe.device.irq.irq)
    }

    fn handle_irq(&mut self) -> IrqStatus {
        if !self.initialized {
            return IrqStatus::Unhandled;
        }

        unsafe {
            let status = read_reg(self.probe.device.reg.start, REG_INTERRUPT_STATUS);
            if status == 0 {
                return IrqStatus::Unhandled;
            }
            write_reg(self.probe.device.reg.start, REG_INTERRUPT_ACK, status);
            asm::dsb();
        }
        IrqStatus::Handled
    }

    fn io(&mut self, _request: DeviceIo<'_>) -> DeviceResult<usize> {
        Err(DeviceError::Unsupported)
    }

    fn wait_channel(&self) -> Option<u32> {
        self.irq().map(|irq| 0x5652_0000 | (irq & 0xffff))
    }
}

pub fn probe_all(out: &mut [VirtioProbe; MAX_VIRTIO_MMIO]) -> usize {
    let mut count = 0;
    for index in 0..MAX_VIRTIO_MMIO {
        let Some(device) = fdt::virtio_mmio(index) else {
            break;
        };
        if !device.is_present() {
            continue;
        }
        if count >= out.len() {
            break;
        }
        out[count] = probe(device);
        count += 1;
    }
    count
}

pub fn probe(device: VirtioMmioDevice) -> VirtioProbe {
    unsafe {
        let base = device.reg.start;
        let magic = read_reg(base, REG_MAGIC);
        let version = read_reg(base, REG_VERSION);
        let device_id = read_reg(base, REG_DEVICE_ID);
        let vendor_id = read_reg(base, REG_VENDOR_ID);
        let supported = magic == VIRTIO_MAGIC
            && (version == VIRTIO_VERSION_LEGACY || version == VIRTIO_VERSION_MODERN)
            && device_id != 0;

        if supported {
            write_reg(base, REG_QUEUE_SEL, 0);
            asm::dsb();
        }

        VirtioProbe {
            device: VirtioMmioDevice {
                kind: if device_id == VIRTIO_DEVICE_BLOCK {
                    VirtioKind::Block
                } else {
                    device.kind
                },
                ..device
            },
            magic,
            version,
            device_id,
            vendor_id,
            queue0_max: if supported {
                read_reg(base, REG_QUEUE_NUM_MAX)
            } else {
                0
            },
            features0: if supported {
                read_reg(base, REG_DEVICE_FEATURES)
            } else {
                0
            },
            supported,
        }
    }
}

unsafe fn read_reg(base: usize, offset: usize) -> u32 {
    unsafe { read_volatile((base + offset) as *const u32) }
}

unsafe fn write_reg(base: usize, offset: usize, value: u32) {
    unsafe {
        write_volatile((base + offset) as *mut u32, value);
    }
}
