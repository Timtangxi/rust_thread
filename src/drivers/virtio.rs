#![allow(dead_code)]

use core::ptr::{read_volatile, write_volatile};

use aarch32_cpu::asm;

use crate::drivers::device::{
    self, DeviceClass, DeviceError, DeviceId, DeviceIo, DeviceNode, DeviceResult, DeviceState,
    IrqStatus, KernelDriver,
};
use crate::drivers::gic;
use crate::kernel::net::{self, MacAddress};
use crate::platform::fdt::{self, MAX_VIRTIO_MMIO, VirtioKind, VirtioMmioDevice};

const VIRTIO_MAGIC: u32 = 0x7472_6976;
const VIRTIO_VERSION_LEGACY: u32 = 1;
const VIRTIO_VERSION_MODERN: u32 = 2;
const VIRTIO_DEVICE_NETWORK: u32 = 1;
const VIRTIO_DEVICE_BLOCK: u32 = 2;

const REG_MAGIC: usize = 0x000;
const REG_VERSION: usize = 0x004;
const REG_DEVICE_ID: usize = 0x008;
const REG_VENDOR_ID: usize = 0x00c;
const REG_DEVICE_FEATURES: usize = 0x010;
const REG_DEVICE_FEATURES_SEL: usize = 0x014;
const REG_DRIVER_FEATURES: usize = 0x020;
const REG_DRIVER_FEATURES_SEL: usize = 0x024;
const REG_GUEST_PAGE_SIZE: usize = 0x028;
const REG_QUEUE_SEL: usize = 0x030;
const REG_QUEUE_NUM_MAX: usize = 0x034;
const REG_QUEUE_NUM: usize = 0x038;
const REG_QUEUE_ALIGN: usize = 0x03c;
const REG_QUEUE_PFN: usize = 0x040;
const REG_QUEUE_READY: usize = 0x044;
const REG_QUEUE_NOTIFY: usize = 0x050;
const REG_STATUS: usize = 0x070;
const REG_INTERRUPT_STATUS: usize = 0x060;
const REG_INTERRUPT_ACK: usize = 0x064;
const REG_QUEUE_DESC_LOW: usize = 0x080;
const REG_QUEUE_DESC_HIGH: usize = 0x084;
const REG_QUEUE_AVAIL_LOW: usize = 0x090;
const REG_QUEUE_AVAIL_HIGH: usize = 0x094;
const REG_QUEUE_USED_LOW: usize = 0x0a0;
const REG_QUEUE_USED_HIGH: usize = 0x0a4;
const REG_CONFIG: usize = 0x100;

const STATUS_ACKNOWLEDGE: u32 = 1;
const STATUS_DRIVER: u32 = 2;
const STATUS_DRIVER_OK: u32 = 4;
const STATUS_FEATURES_OK: u32 = 8;
const STATUS_FAILED: u32 = 0x80;

const VIRTIO_F_VERSION_1: u32 = 1;
const VIRTIO_NET_F_MAC: u32 = 1 << 5;
const MAX_VIRTIO_NET_DEVICES: usize = 4;
const MAX_VIRTIO_BLOCK_DEVICES: usize = 1;
const VIRTIO_BLK_SECTOR_SIZE: usize = 512;
const VIRTQUEUE_SIZE: u16 = 8;
const VIRTQUEUE_PAGES: usize = 2;
const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;

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
        match self.device_id {
            VIRTIO_DEVICE_NETWORK => VirtioKind::Network,
            VIRTIO_DEVICE_BLOCK => VirtioKind::Block,
            _ => VirtioKind::Unknown,
        }
    }

    pub const fn is_block(self) -> bool {
        self.device_id == VIRTIO_DEVICE_BLOCK && self.supported
    }

    pub const fn is_network(self) -> bool {
        self.device_id == VIRTIO_DEVICE_NETWORK && self.supported
    }
}

pub struct VirtioBlock {
    probe: VirtioProbe,
    initialized: bool,
    queue: VirtQueue,
    queue_mem: *mut u8,
    avail_idx: u16,
    last_used_idx: u16,
}

impl VirtioBlock {
    pub const fn new(probe: VirtioProbe) -> Self {
        Self {
            probe,
            initialized: false,
            queue: VirtQueue::empty(),
            queue_mem: core::ptr::null_mut(),
            avail_idx: 0,
            last_used_idx: 0,
        }
    }

    pub const fn probe(&self) -> VirtioProbe {
        self.probe
    }

    pub const fn queue(&self) -> VirtQueue {
        self.queue
    }
}

impl Default for VirtioBlock {
    fn default() -> Self {
        Self::new(VirtioProbe::empty())
    }
}

#[derive(Clone, Copy)]
pub struct VirtioNet {
    probe: VirtioProbe,
    initialized: bool,
    present: bool,
    interface: usize,
    mac: MacAddress,
    rx_queue: VirtQueue,
    tx_queue: VirtQueue,
}

impl VirtioNet {
    pub const fn empty() -> Self {
        Self {
            probe: VirtioProbe::empty(),
            initialized: false,
            present: false,
            interface: usize::MAX,
            mac: MacAddress::ZERO,
            rx_queue: VirtQueue::empty(),
            tx_queue: VirtQueue::empty(),
        }
    }

    pub const fn new(probe: VirtioProbe) -> Self {
        Self {
            probe,
            initialized: false,
            present: true,
            interface: usize::MAX,
            mac: MacAddress::ZERO,
            rx_queue: VirtQueue::empty(),
            tx_queue: VirtQueue::empty(),
        }
    }

    pub const fn is_present(self) -> bool {
        self.present
    }

    pub const fn interface(self) -> Option<usize> {
        if self.initialized && self.interface != usize::MAX {
            Some(self.interface)
        } else {
            None
        }
    }

    fn bind(&mut self, slot: usize) -> DeviceResult<()> {
        if !self.probe.is_network() {
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

            let driver_features = self.probe.features0 & VIRTIO_NET_F_MAC;
            write_reg(
                self.probe.device.reg.start,
                REG_DRIVER_FEATURES,
                driver_features,
            );
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

            self.rx_queue = probe_queue(self.probe.device.reg.start, 0);
            self.tx_queue = probe_queue(self.probe.device.reg.start, 1);
            if self.rx_queue.size == 0 || self.tx_queue.size == 0 {
                write_reg(self.probe.device.reg.start, REG_STATUS, STATUS_FAILED);
                return Err(DeviceError::Invalid);
            }

            self.mac = if driver_features & VIRTIO_NET_F_MAC != 0 {
                read_mac_config(self.probe.device.reg.start)
            } else {
                fallback_mac(self.probe.device.reg.start, self.probe.device.irq.irq)
            };
        }

        let name = netdev_name(slot);
        let interface = net::interface::register(name, self.mac).map_err(|_| DeviceError::Busy)?;
        let _ = net::interface::set_ipv4(interface, net::DEFAULT_IPV4);
        self.interface = interface;

        if self.probe.device.irq.is_present() {
            gic::enable_device_irq(self.probe.device.irq.irq);
        }

        unsafe {
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
}

#[derive(Clone, Copy)]
pub struct VirtQueue {
    pub index: u16,
    pub size: u16,
    pub align: u32,
    pub pfn: u32,
    pub ready: bool,
}

impl VirtQueue {
    pub const fn empty() -> Self {
        Self {
            index: 0,
            size: 0,
            align: 4096,
            pfn: 0,
            ready: false,
        }
    }

    pub const fn is_ready(self) -> bool {
        self.ready
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

#[repr(C)]
struct VirtqAvail {
    flags: u16,
    idx: u16,
    ring: [u16; VIRTQUEUE_SIZE as usize],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtqUsedElem {
    id: u32,
    len: u32,
}

#[repr(C)]
struct VirtqUsed {
    flags: u16,
    idx: u16,
    ring: [VirtqUsedElem; VIRTQUEUE_SIZE as usize],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VirtioBlkRequestHeader {
    pub request_type: u32,
    pub reserved: u32,
    pub sector: u64,
}

pub const VIRTIO_BLK_T_IN: u32 = 0;
pub const VIRTIO_BLK_T_OUT: u32 = 1;
pub const VIRTIO_BLK_S_OK: u8 = 0;
pub const VIRTIO_BLK_S_IOERR: u8 = 1;
pub const VIRTIO_BLK_S_UNSUPP: u8 = 2;

static mut VIRTIO_NET_DEVICES: [VirtioNet; MAX_VIRTIO_NET_DEVICES] =
    [VirtioNet::empty(); MAX_VIRTIO_NET_DEVICES];
static mut VIRTIO_BLOCK_DEVICES: [VirtioBlock; MAX_VIRTIO_BLOCK_DEVICES] =
    [const { VirtioBlock::new(VirtioProbe::empty()) }; MAX_VIRTIO_BLOCK_DEVICES];

impl KernelDriver for VirtioBlock {
    fn name(&self) -> &'static str {
        "virtio-blk"
    }

    fn init(&mut self) -> DeviceResult<()> {
        if !self.probe.is_block() {
            return Err(DeviceError::Unsupported);
        }

        unsafe {
            let queue_mem =
                crate::kernel::memory::alloc_pages(VIRTQUEUE_PAGES).ok_or(DeviceError::Busy)?;

            write_reg(self.probe.device.reg.start, REG_STATUS, 0);
            asm::dsb();
            write_reg(self.probe.device.reg.start, REG_STATUS, STATUS_ACKNOWLEDGE);
            write_reg(
                self.probe.device.reg.start,
                REG_STATUS,
                STATUS_ACKNOWLEDGE | STATUS_DRIVER,
            );
            write_reg(self.probe.device.reg.start, REG_DRIVER_FEATURES_SEL, 0);
            write_reg(self.probe.device.reg.start, REG_DRIVER_FEATURES, 0);
            if self.probe.version == VIRTIO_VERSION_MODERN {
                write_reg(self.probe.device.reg.start, REG_DRIVER_FEATURES_SEL, 1);
                write_reg(
                    self.probe.device.reg.start,
                    REG_DRIVER_FEATURES,
                    VIRTIO_F_VERSION_1,
                );
                write_reg(self.probe.device.reg.start, REG_DRIVER_FEATURES_SEL, 0);
            }
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

            write_reg(self.probe.device.reg.start, REG_QUEUE_SEL, 0);
            asm::dsb();
            let queue_max = read_reg(self.probe.device.reg.start, REG_QUEUE_NUM_MAX);
            if queue_max == 0 {
                write_reg(self.probe.device.reg.start, REG_STATUS, STATUS_FAILED);
                return Err(DeviceError::Invalid);
            }
            let queue_size = queue_max.min(u32::from(VIRTQUEUE_SIZE));
            write_reg(self.probe.device.reg.start, REG_QUEUE_NUM, queue_size);

            let desc = queue_mem as usize;
            let avail = desc + core::mem::size_of::<VirtqDesc>() * queue_size as usize;
            let used = crate::kernel::address::align_up(
                avail + 4 + core::mem::size_of::<u16>() * queue_size as usize,
                4096,
            );

            for idx in 0..(VIRTQUEUE_PAGES * crate::kernel::memory::PAGE_SIZE) {
                queue_mem.add(idx).write_volatile(0);
            }

            if self.probe.version == VIRTIO_VERSION_MODERN {
                write_reg(self.probe.device.reg.start, REG_QUEUE_DESC_LOW, desc as u32);
                write_reg(self.probe.device.reg.start, REG_QUEUE_DESC_HIGH, 0);
                write_reg(
                    self.probe.device.reg.start,
                    REG_QUEUE_AVAIL_LOW,
                    avail as u32,
                );
                write_reg(self.probe.device.reg.start, REG_QUEUE_AVAIL_HIGH, 0);
                write_reg(self.probe.device.reg.start, REG_QUEUE_USED_LOW, used as u32);
                write_reg(self.probe.device.reg.start, REG_QUEUE_USED_HIGH, 0);
                write_reg(self.probe.device.reg.start, REG_QUEUE_READY, 1);
            } else {
                write_reg(self.probe.device.reg.start, REG_GUEST_PAGE_SIZE, 4096);
                write_reg(self.probe.device.reg.start, REG_QUEUE_ALIGN, 4096);
                write_reg(
                    self.probe.device.reg.start,
                    REG_QUEUE_PFN,
                    (desc >> 12) as u32,
                );
            }
            self.queue = VirtQueue {
                index: 0,
                size: queue_size as u16,
                align: 4096,
                pfn: (desc >> 12) as u32,
                ready: true,
            };
            self.queue_mem = queue_mem;
            self.avail_idx = 0;
            self.last_used_idx = 0;

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

impl VirtioBlock {
    fn read_sector(
        &mut self,
        sector: u64,
        dst: &mut [u8; VIRTIO_BLK_SECTOR_SIZE],
    ) -> DeviceResult<()> {
        if !self.initialized || !self.queue.ready || self.queue_mem.is_null() {
            return Err(DeviceError::NotPresent);
        }

        let mut header = VirtioBlkRequestHeader {
            request_type: VIRTIO_BLK_T_IN,
            reserved: 0,
            sector,
        };
        let mut status = VIRTIO_BLK_S_IOERR;

        unsafe {
            let desc = self.desc_ptr();
            desc.add(0).write_volatile(VirtqDesc {
                addr: (&mut header as *mut VirtioBlkRequestHeader) as u64,
                len: core::mem::size_of::<VirtioBlkRequestHeader>() as u32,
                flags: VIRTQ_DESC_F_NEXT,
                next: 1,
            });
            desc.add(1).write_volatile(VirtqDesc {
                addr: dst.as_mut_ptr() as u64,
                len: VIRTIO_BLK_SECTOR_SIZE as u32,
                flags: VIRTQ_DESC_F_NEXT | VIRTQ_DESC_F_WRITE,
                next: 2,
            });
            desc.add(2).write_volatile(VirtqDesc {
                addr: (&mut status as *mut u8) as u64,
                len: 1,
                flags: VIRTQ_DESC_F_WRITE,
                next: 0,
            });

            let avail = self.avail_ptr();
            (*avail).ring[(self.avail_idx as usize) % self.queue.size as usize] = 0;
            asm::dsb();
            self.avail_idx = self.avail_idx.wrapping_add(1);
            core::ptr::addr_of_mut!((*avail).idx).write_volatile(self.avail_idx);
            asm::dsb();
            write_reg(
                self.probe.device.reg.start,
                REG_QUEUE_NOTIFY,
                u32::from(self.queue.index),
            );

            let used = self.used_ptr();
            let mut spins = 0usize;
            while core::ptr::addr_of!((*used).idx).read_volatile() == self.last_used_idx {
                spins = spins.wrapping_add(1);
                if spins > 20_000_000 {
                    return Err(DeviceError::Io);
                }
                core::hint::spin_loop();
            }
            self.last_used_idx = self.last_used_idx.wrapping_add(1);

            let irq_status = read_reg(self.probe.device.reg.start, REG_INTERRUPT_STATUS);
            if irq_status != 0 {
                write_reg(self.probe.device.reg.start, REG_INTERRUPT_ACK, irq_status);
            }
            asm::dsb();
        }

        if status == VIRTIO_BLK_S_OK {
            Ok(())
        } else {
            Err(DeviceError::Io)
        }
    }

    unsafe fn desc_ptr(&self) -> *mut VirtqDesc {
        self.queue_mem as *mut VirtqDesc
    }

    unsafe fn avail_ptr(&self) -> *mut VirtqAvail {
        unsafe {
            self.queue_mem
                .add(core::mem::size_of::<VirtqDesc>() * self.queue.size as usize)
                as *mut VirtqAvail
        }
    }

    unsafe fn used_ptr(&self) -> *mut VirtqUsed {
        let avail =
            self.queue_mem as usize + core::mem::size_of::<VirtqDesc>() * self.queue.size as usize;
        crate::kernel::address::align_up(
            avail + 4 + core::mem::size_of::<u16>() * self.queue.size as usize,
            4096,
        ) as *mut VirtqUsed
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

pub fn register_probes(probes: &[VirtioProbe]) -> usize {
    let mut registered = 0usize;
    let mut net_slot = 0usize;
    let mut block_slot = 0usize;
    for probe in probes {
        if !probe.supported {
            continue;
        }
        let class = match probe.kind() {
            VirtioKind::Block => DeviceClass::Block,
            VirtioKind::Network => DeviceClass::Network,
            VirtioKind::Unknown => DeviceClass::Bus,
        };
        let node = DeviceNode {
            id: DeviceId::new(usize::MAX),
            name: match probe.kind() {
                VirtioKind::Block => "virtio-blk",
                VirtioKind::Network => "virtio-net",
                VirtioKind::Unknown => "virtio-mmio",
            },
            driver: "virtio-mmio",
            class,
            state: DeviceState::Discovered,
            mmio_base: probe.device.reg.start,
            mmio_size: probe.device.reg.size,
            irq: probe.device.irq.irq,
            major: match probe.kind() {
                VirtioKind::Block => 254,
                VirtioKind::Network => 255,
                VirtioKind::Unknown => 0,
            },
            minor: registered as u16,
            wait_channel: probe
                .device
                .irq
                .is_present()
                .then_some(0x5652_0000 | (probe.device.irq.irq & 0xffff))
                .unwrap_or(0),
        };

        let Ok(id) = device::register(node) else {
            continue;
        };

        match probe.kind() {
            VirtioKind::Block if crate::config::CONFIG_VIRTIO_BLK => {
                if register_block_probe(*probe, block_slot).is_ok() {
                    let _ = device::mark_ready(id);
                    block_slot += 1;
                } else {
                    let _ = device::mark_failed(id);
                }
            }
            VirtioKind::Network if crate::config::CONFIG_VIRTIO_NET => {
                if register_net_probe(*probe, net_slot).is_ok() {
                    let _ = device::mark_ready(id);
                    net_slot += 1;
                } else {
                    let _ = device::mark_failed(id);
                }
            }
            _ => {}
        }

        registered += 1;
    }
    registered
}

pub fn block_read_sector(sector: u64, dst: &mut [u8; VIRTIO_BLK_SECTOR_SIZE]) -> DeviceResult<()> {
    unsafe {
        let device = &raw mut VIRTIO_BLOCK_DEVICES[0];
        (*device).read_sector(sector, dst)
    }
}

pub fn block_read_at(offset: usize, dst: &mut [u8]) -> DeviceResult<usize> {
    if dst.is_empty() {
        return Ok(0);
    }

    let mut done = 0usize;
    let mut sector_buf = [0u8; VIRTIO_BLK_SECTOR_SIZE];
    while done < dst.len() {
        let absolute = offset + done;
        let sector = (absolute / VIRTIO_BLK_SECTOR_SIZE) as u64;
        let sector_offset = absolute % VIRTIO_BLK_SECTOR_SIZE;
        block_read_sector(sector, &mut sector_buf)?;
        let count = (VIRTIO_BLK_SECTOR_SIZE - sector_offset).min(dst.len() - done);
        dst[done..done + count].copy_from_slice(&sector_buf[sector_offset..sector_offset + count]);
        done += count;
    }
    Ok(done)
}

pub fn handle_irq(irq: u32) -> IrqStatus {
    unsafe {
        for index in 0..MAX_VIRTIO_NET_DEVICES {
            let device = &raw mut VIRTIO_NET_DEVICES[index];
            if !(*device).initialized || (*device).probe.device.irq.irq != irq {
                continue;
            }

            let status = read_reg((*device).probe.device.reg.start, REG_INTERRUPT_STATUS);
            if status == 0 {
                return IrqStatus::Unhandled;
            }
            write_reg((*device).probe.device.reg.start, REG_INTERRUPT_ACK, status);
            asm::dsb();

            if let Some(interface) = (*device).interface() {
                return IrqStatus::Wake {
                    channel: net::interface::wait_channel(interface),
                };
            }
            return IrqStatus::Handled;
        }
    }

    IrqStatus::Unhandled
}

pub fn owns_irq(irq: u32) -> bool {
    unsafe {
        for index in 0..MAX_VIRTIO_NET_DEVICES {
            let device = &raw const VIRTIO_NET_DEVICES[index];
            if (*device).initialized && (*device).probe.device.irq.irq == irq {
                return true;
            }
        }
    }
    false
}

pub fn network_device_count() -> usize {
    let mut count = 0usize;
    unsafe {
        for index in 0..MAX_VIRTIO_NET_DEVICES {
            let device = &raw const VIRTIO_NET_DEVICES[index];
            if (*device).initialized {
                count += 1;
            }
        }
    }
    count
}

fn register_net_probe(probe: VirtioProbe, slot: usize) -> DeviceResult<()> {
    if slot >= MAX_VIRTIO_NET_DEVICES {
        return Err(DeviceError::Busy);
    }

    unsafe {
        let device = &raw mut VIRTIO_NET_DEVICES[slot];
        *device = VirtioNet::new(probe);
        match (*device).bind(slot) {
            Ok(()) => Ok(()),
            Err(err) => {
                *device = VirtioNet::empty();
                Err(err)
            }
        }
    }
}

fn register_block_probe(probe: VirtioProbe, slot: usize) -> DeviceResult<()> {
    if slot >= MAX_VIRTIO_BLOCK_DEVICES {
        return Err(DeviceError::Busy);
    }

    unsafe {
        let device = &raw mut VIRTIO_BLOCK_DEVICES[slot];
        *device = VirtioBlock::new(probe);
        match (*device).init() {
            Ok(()) => Ok(()),
            Err(err) => {
                *device = VirtioBlock::new(VirtioProbe::empty());
                Err(err)
            }
        }
    }
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
                kind: match device_id {
                    VIRTIO_DEVICE_NETWORK => VirtioKind::Network,
                    VIRTIO_DEVICE_BLOCK => VirtioKind::Block,
                    _ => device.kind,
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
                write_reg(base, REG_DEVICE_FEATURES_SEL, 0);
                read_reg(base, REG_DEVICE_FEATURES)
            } else {
                0
            },
            supported,
        }
    }
}

unsafe fn probe_queue(base: usize, index: u16) -> VirtQueue {
    unsafe {
        write_reg(base, REG_QUEUE_SEL, u32::from(index));
        asm::dsb();
        let queue_max = read_reg(base, REG_QUEUE_NUM_MAX);
        if queue_max == 0 {
            return VirtQueue {
                index,
                size: 0,
                align: 4096,
                pfn: 0,
                ready: false,
            };
        }

        let queue_size = queue_max.min(8);
        write_reg(base, REG_QUEUE_NUM, queue_size);
        write_reg(base, REG_QUEUE_ALIGN, 4096);
        write_reg(base, REG_QUEUE_PFN, 0);
        VirtQueue {
            index,
            size: queue_size as u16,
            align: 4096,
            pfn: 0,
            ready: false,
        }
    }
}

unsafe fn read_mac_config(base: usize) -> MacAddress {
    let mut bytes = [0u8; 6];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        *byte = unsafe { read_config_u8(base, offset) };
    }
    if bytes == MacAddress::ZERO.as_bytes() {
        fallback_mac(base, 0)
    } else {
        MacAddress::new(bytes)
    }
}

fn fallback_mac(base: usize, irq: u32) -> MacAddress {
    MacAddress::new([
        0x52,
        0x54,
        0x00,
        ((base >> 12) & 0xff) as u8,
        (irq & 0xff) as u8,
        0x01,
    ])
}

const fn netdev_name(slot: usize) -> &'static str {
    match slot {
        0 => "eth0",
        1 => "eth1",
        2 => "eth2",
        _ => "eth3",
    }
}

unsafe fn read_reg(base: usize, offset: usize) -> u32 {
    unsafe { read_volatile((base + offset) as *const u32) }
}

unsafe fn read_config_u8(base: usize, offset: usize) -> u8 {
    unsafe { read_volatile((base + REG_CONFIG + offset) as *const u8) }
}

unsafe fn write_reg(base: usize, offset: usize, value: u32) {
    unsafe {
        write_volatile((base + offset) as *mut u32, value);
    }
}
