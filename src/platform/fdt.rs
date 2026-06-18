#![allow(dead_code)]

use core::ptr::read_volatile;

use crate::platform::qemu_virt;

unsafe extern "C" {
    static boot_fdt_ptr: u32;
}

pub const MAX_VIRTIO_MMIO: usize = 8;

const FDT_MAGIC: u32 = 0xd00d_feed;
const FDT_BEGIN_NODE: u32 = 1;
const FDT_END_NODE: u32 = 2;
const FDT_PROP: u32 = 3;
const FDT_NOP: u32 = 4;
const FDT_END: u32 = 9;

const DEFAULT_ADDRESS_CELLS: u32 = 2;
const DEFAULT_SIZE_CELLS: u32 = 1;
const DEFAULT_INTERRUPT_CELLS: u32 = 3;

const ROOT_COMPAT_LEN: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BootSource {
    Static,
    Fdt,
    FdtInvalid,
}

impl BootSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Static => "qemu-virt-static",
            Self::Fdt => "fdt",
            Self::FdtInvalid => "qemu-virt-static+fdt-invalid",
        }
    }
}

#[derive(Clone, Copy)]
pub struct MemRegion {
    pub start: usize,
    pub size: usize,
}

impl MemRegion {
    pub const fn empty() -> Self {
        Self { start: 0, size: 0 }
    }

    pub const fn new(start: usize, size: usize) -> Self {
        Self { start, size }
    }

    pub const fn end(self) -> usize {
        self.start + self.size
    }

    pub const fn is_present(self) -> bool {
        self.size != 0
    }
}

#[derive(Clone, Copy)]
pub struct IrqSpec {
    pub irq: u32,
    pub flags: u32,
}

impl IrqSpec {
    pub const fn none() -> Self {
        Self { irq: 0, flags: 0 }
    }

    pub const fn new(irq: u32, flags: u32) -> Self {
        Self { irq, flags }
    }

    pub const fn is_present(self) -> bool {
        self.irq != 0
    }
}

#[derive(Clone, Copy)]
pub struct Pl011Device {
    pub reg: MemRegion,
    pub irq: IrqSpec,
}

impl Pl011Device {
    pub const fn fallback() -> Self {
        Self {
            reg: MemRegion::new(qemu_virt::UART0_BASE, 0x1000),
            irq: IrqSpec::new(qemu_virt::UART0_IRQ, 0),
        }
    }
}

#[derive(Clone, Copy)]
pub struct GicDevice {
    pub distributor: MemRegion,
    pub cpu_interface: MemRegion,
    pub interrupt_cells: u32,
}

impl GicDevice {
    pub const fn fallback() -> Self {
        Self {
            distributor: MemRegion::new(qemu_virt::GICD_BASE, 0x1000),
            cpu_interface: MemRegion::new(qemu_virt::GICC_BASE, 0x2000),
            interrupt_cells: DEFAULT_INTERRUPT_CELLS,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TimerDevice {
    pub physical_irq: IrqSpec,
}

impl TimerDevice {
    pub const fn fallback() -> Self {
        Self {
            physical_irq: IrqSpec::new(qemu_virt::GENERIC_TIMER_PHYS_IRQ, 0),
        }
    }
}

#[derive(Clone, Copy)]
pub enum VirtioKind {
    Unknown,
    Block,
}

impl VirtioKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Block => "block",
        }
    }
}

#[derive(Clone, Copy)]
pub struct VirtioMmioDevice {
    pub reg: MemRegion,
    pub irq: IrqSpec,
    pub kind: VirtioKind,
}

impl VirtioMmioDevice {
    pub const fn empty() -> Self {
        Self {
            reg: MemRegion::empty(),
            irq: IrqSpec::none(),
            kind: VirtioKind::Unknown,
        }
    }

    pub const fn is_present(self) -> bool {
        self.reg.is_present()
    }
}

#[derive(Clone, Copy)]
pub struct PlatformDevices {
    pub uart0: Pl011Device,
    pub gic: GicDevice,
    pub timer: TimerDevice,
    pub virtio_mmio: [VirtioMmioDevice; MAX_VIRTIO_MMIO],
    pub virtio_mmio_count: usize,
}

impl PlatformDevices {
    pub const fn fallback() -> Self {
        Self {
            uart0: Pl011Device::fallback(),
            gic: GicDevice::fallback(),
            timer: TimerDevice::fallback(),
            virtio_mmio: [const { VirtioMmioDevice::empty() }; MAX_VIRTIO_MMIO],
            virtio_mmio_count: 0,
        }
    }

    fn push_virtio_mmio(&mut self, device: VirtioMmioDevice) {
        if self.virtio_mmio_count >= MAX_VIRTIO_MMIO {
            return;
        }

        self.virtio_mmio[self.virtio_mmio_count] = device;
        self.virtio_mmio_count += 1;
    }
}

#[derive(Clone, Copy)]
pub struct BootInfo {
    pub fdt_ptr: usize,
    pub source: BootSource,
    pub ram_start: usize,
    pub ram_end: usize,
    pub mmio_start: usize,
    pub mmio_end: usize,
    pub root_compatible: [u8; ROOT_COMPAT_LEN],
    pub root_compatible_len: usize,
    pub devices: PlatformDevices,
}

impl BootInfo {
    pub const fn fallback_with_source(fdt_ptr: usize, source: BootSource) -> Self {
        Self {
            fdt_ptr,
            source,
            ram_start: qemu_virt::RAM_START,
            ram_end: qemu_virt::RAM_END,
            mmio_start: qemu_virt::MMIO_START,
            mmio_end: qemu_virt::MMIO_END,
            root_compatible: [0; ROOT_COMPAT_LEN],
            root_compatible_len: 0,
            devices: PlatformDevices::fallback(),
        }
    }

    pub const fn source_str(self) -> &'static str {
        self.source.as_str()
    }

    pub fn root_compatible_str(&self) -> &str {
        core::str::from_utf8(&self.root_compatible[..self.root_compatible_len]).unwrap_or("")
    }
}

static mut BOOT_INFO: BootInfo = BootInfo::fallback_with_source(0, BootSource::Static);
static mut INITIALIZED: bool = false;

pub fn init() -> BootInfo {
    let fdt_ptr = detect_fdt_ptr();
    let info = if fdt_ptr == 0 {
        BootInfo::fallback_with_source(0, BootSource::Static)
    } else {
        match unsafe { Parser::new(fdt_ptr) }.and_then(|mut parser| parser.parse()) {
            Some(mut info) => {
                info.fdt_ptr = fdt_ptr;
                info.source = BootSource::Fdt;
                info
            }
            None => BootInfo::fallback_with_source(fdt_ptr, BootSource::FdtInvalid),
        }
    };

    unsafe {
        BOOT_INFO = info;
        INITIALIZED = true;
    }
    info
}

pub fn boot_info() -> BootInfo {
    unsafe {
        if INITIALIZED {
            BOOT_INFO
        } else {
            BootInfo::fallback_with_source(detect_fdt_ptr(), BootSource::Static)
        }
    }
}

pub fn devices() -> PlatformDevices {
    boot_info().devices
}

pub fn uart0() -> Pl011Device {
    devices().uart0
}

pub fn gic() -> GicDevice {
    devices().gic
}

pub fn timer() -> TimerDevice {
    devices().timer
}

pub fn virtio_mmio(index: usize) -> Option<VirtioMmioDevice> {
    let devices = devices();
    if index < devices.virtio_mmio_count {
        Some(devices.virtio_mmio[index])
    } else {
        None
    }
}

fn detect_fdt_ptr() -> usize {
    let register_ptr = unsafe { core::ptr::addr_of!(boot_fdt_ptr).read_volatile() as usize };
    if is_valid_fdt_header(register_ptr) {
        return register_ptr;
    }

    scan_fdt(qemu_virt::FDT_SCAN_START, qemu_virt::FDT_SCAN_END).unwrap_or(0)
}

fn scan_fdt(start: usize, end: usize) -> Option<usize> {
    let mut addr = (start + 7) & !7;
    while addr + 40 <= end {
        if is_valid_fdt_header(addr) {
            return Some(addr);
        }
        addr += 8;
    }
    None
}

fn is_valid_fdt_header(addr: usize) -> bool {
    if addr == 0 || addr & 0x3 != 0 {
        return false;
    }

    let Some(magic) = (unsafe { read_be32(addr) }) else {
        return false;
    };
    if magic != FDT_MAGIC {
        return false;
    }

    let Some(total) = (unsafe { read_be32(addr + 4) }) else {
        return false;
    };
    let total = total as usize;
    (40..=qemu_virt::RAM_SIZE).contains(&total)
}

#[derive(Clone, Copy)]
struct FdtHeader {
    totalsize: usize,
    off_dt_struct: usize,
    off_dt_strings: usize,
    size_dt_struct: usize,
    size_dt_strings: usize,
}

struct Parser {
    base: usize,
    header: FdtHeader,
    info: BootInfo,
    depth: usize,
    nodes: [NodeState; 16],
    address_cells: [u32; 16],
    size_cells: [u32; 16],
    root_address_cells: u32,
    root_size_cells: u32,
    interrupt_cells: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NodeKind {
    Root,
    Memory,
    Uart,
    Gic,
    Timer,
    VirtioMmio,
    Other,
}

#[derive(Clone, Copy)]
struct NodeState {
    kind: NodeKind,
    first_reg: Option<MemRegion>,
    second_reg: Option<MemRegion>,
    first_irq: IrqSpec,
    timer_irq: IrqSpec,
    interrupt_cells: u32,
}

impl NodeState {
    const fn empty() -> Self {
        Self {
            kind: NodeKind::Other,
            first_reg: None,
            second_reg: None,
            first_irq: IrqSpec::none(),
            timer_irq: IrqSpec::none(),
            interrupt_cells: 0,
        }
    }
}

#[derive(Clone, Copy)]
enum PropAction {
    None,
    AddressCells(u32),
    SizeCells(u32),
    InterruptCells(u32),
    Kind(NodeKind),
    RootCompatible {
        compatible: [u8; ROOT_COMPAT_LEN],
        len: usize,
    },
    Reg {
        first: MemRegion,
        second: Option<MemRegion>,
    },
    Interrupts {
        first: Option<IrqSpec>,
        timer: Option<IrqSpec>,
    },
}

impl Parser {
    unsafe fn new(base: usize) -> Option<Self> {
        if base & 0x3 != 0 {
            return None;
        }

        let magic = unsafe { read_be32(base)? };
        if magic != FDT_MAGIC {
            return None;
        }

        let totalsize = unsafe { read_be32(base + 4)? as usize };
        let off_dt_struct = unsafe { read_be32(base + 8)? as usize };
        let off_dt_strings = unsafe { read_be32(base + 12)? as usize };
        let _off_mem_rsvmap = unsafe { read_be32(base + 16)? as usize };
        let _version = unsafe { read_be32(base + 20)? };
        let _last_comp_version = unsafe { read_be32(base + 24)? };
        let _boot_cpuid_phys = unsafe { read_be32(base + 28)? };
        let size_dt_strings = unsafe { read_be32(base + 32)? as usize };
        let size_dt_struct = unsafe { read_be32(base + 36)? as usize };

        if totalsize < 40
            || off_dt_struct >= totalsize
            || off_dt_strings >= totalsize
            || off_dt_struct + size_dt_struct > totalsize
            || off_dt_strings + size_dt_strings > totalsize
        {
            return None;
        }

        let mut info = BootInfo::fallback_with_source(base, BootSource::Fdt);
        info.mmio_start = usize::MAX;
        info.mmio_end = 0;
        info.devices = PlatformDevices::fallback();

        Some(Self {
            base,
            header: FdtHeader {
                totalsize,
                off_dt_struct,
                off_dt_strings,
                size_dt_struct,
                size_dt_strings,
            },
            info,
            depth: 0,
            nodes: [const { NodeState::empty() }; 16],
            address_cells: [DEFAULT_ADDRESS_CELLS; 16],
            size_cells: [DEFAULT_SIZE_CELLS; 16],
            root_address_cells: DEFAULT_ADDRESS_CELLS,
            root_size_cells: DEFAULT_SIZE_CELLS,
            interrupt_cells: DEFAULT_INTERRUPT_CELLS,
        })
    }

    fn parse(&mut self) -> Option<BootInfo> {
        let mut off = self.header.off_dt_struct;
        let end = self.header.off_dt_struct + self.header.size_dt_struct;

        while off + 4 <= end {
            let token = self.read_struct_be32(off)?;
            off += 4;

            match token {
                FDT_BEGIN_NODE => {
                    let name_start = off;
                    let name_len = self.node_name_len(name_start, end)?;
                    self.begin_node(name_start, name_len);
                    off = align4(name_start + name_len + 1);
                }
                FDT_END_NODE => {
                    if self.depth == 0 {
                        return None;
                    }
                    self.finish_node();
                    self.depth -= 1;
                }
                FDT_PROP => {
                    if off + 8 > end {
                        return None;
                    }
                    let len = self.read_struct_be32(off)? as usize;
                    let nameoff = self.read_struct_be32(off + 4)? as usize;
                    off += 8;
                    self.property(nameoff, off, len)?;
                    off = align4(off + len);
                }
                FDT_NOP => {}
                FDT_END => {
                    if self.info.mmio_start == usize::MAX {
                        self.info.mmio_start = qemu_virt::MMIO_START;
                        self.info.mmio_end = qemu_virt::MMIO_END;
                    }
                    return Some(self.info);
                }
                _ => return None,
            }
        }

        None
    }

    fn begin_node(&mut self, name_off: usize, name_len: usize) {
        self.depth += 1;
        let parent_depth = self.depth.saturating_sub(1);
        if self.depth < self.address_cells.len() {
            self.address_cells[self.depth] = self.address_cells[parent_depth];
            self.size_cells[self.depth] = self.size_cells[parent_depth];
        }

        let name = self.struct_slice(name_off, name_len).unwrap_or(&[]);
        let kind = if self.depth == 1 {
            NodeKind::Root
        } else if node_name_eq(name, b"memory") {
            NodeKind::Memory
        } else if node_name_eq(name, b"timer") {
            NodeKind::Timer
        } else {
            NodeKind::Other
        };
        let index = self.node_index();
        self.nodes[index] = NodeState {
            kind,
            ..NodeState::empty()
        };
    }

    fn finish_node(&mut self) {
        let state = self.nodes[self.node_index()];
        match state.kind {
            NodeKind::Memory => {
                if let Some(region) = state.first_reg {
                    self.info.ram_start = region.start;
                    self.info.ram_end = region.end();
                }
            }
            NodeKind::Uart => {
                if let Some(region) = state.first_reg {
                    self.info.devices.uart0.reg = region;
                    self.include_mmio(region);
                }
                if state.first_irq.is_present() {
                    self.info.devices.uart0.irq = state.first_irq;
                }
            }
            NodeKind::Gic => {
                if let Some(region) = state.first_reg {
                    self.info.devices.gic.distributor = region;
                    self.include_mmio(region);
                }
                if let Some(region) = state.second_reg {
                    self.info.devices.gic.cpu_interface = region;
                    self.include_mmio(region);
                }
                if state.interrupt_cells != 0 {
                    self.interrupt_cells = state.interrupt_cells;
                    self.info.devices.gic.interrupt_cells = state.interrupt_cells;
                }
            }
            NodeKind::Timer => {
                let irq = if state.timer_irq.is_present() {
                    state.timer_irq
                } else {
                    state.first_irq
                };
                if irq.is_present() {
                    self.info.devices.timer.physical_irq = irq;
                }
            }
            NodeKind::VirtioMmio => {
                if let Some(region) = state.first_reg {
                    let device = VirtioMmioDevice {
                        reg: region,
                        irq: state.first_irq,
                        kind: VirtioKind::Unknown,
                    };
                    self.info.devices.push_virtio_mmio(device);
                    self.include_mmio(region);
                }
            }
            NodeKind::Root | NodeKind::Other => {}
        }
    }

    fn property(&mut self, nameoff: usize, value_off: usize, value_len: usize) -> Option<()> {
        let name = self.string_at(nameoff)?;
        let value = self.struct_slice(value_off, value_len)?;
        let mut action = PropAction::None;

        if bytes_eq(name, b"#address-cells") {
            if let Some(cells) = prop_u32(value) {
                action = PropAction::AddressCells(cells);
            }
        } else if bytes_eq(name, b"#size-cells") {
            if let Some(cells) = prop_u32(value) {
                action = PropAction::SizeCells(cells);
            }
        } else if bytes_eq(name, b"#interrupt-cells") {
            if let Some(cells) = prop_u32(value) {
                action = PropAction::InterruptCells(cells);
            }
        } else if bytes_eq(name, b"compatible") {
            action = self.compatible_action(value);
        } else if bytes_eq(name, b"device_type") {
            if value_streq(value, b"memory") {
                action = PropAction::Kind(NodeKind::Memory);
            }
        } else if bytes_eq(name, b"reg") {
            action = self.reg_action(value);
        } else if bytes_eq(name, b"interrupts") {
            action = self.interrupts_action(value);
        }

        let _ = name;
        let _ = value;
        self.apply_action(action);
        Some(())
    }

    fn compatible_action(&self, value: &[u8]) -> PropAction {
        if self.current_state().kind == NodeKind::Root {
            let mut compatible = [0u8; ROOT_COMPAT_LEN];
            let len = copy_root_compatible(&mut compatible, value, ROOT_COMPAT_LEN);
            return PropAction::RootCompatible { compatible, len };
        }

        if compatible_contains(value, b"arm,pl011") {
            PropAction::Kind(NodeKind::Uart)
        } else if compatible_contains(value, b"arm,cortex-a15-gic")
            || compatible_contains(value, b"arm,gic-400")
            || compatible_contains(value, b"arm,cortex-a9-gic")
        {
            PropAction::Kind(NodeKind::Gic)
        } else if compatible_contains(value, b"arm,armv7-timer")
            || compatible_contains(value, b"arm,armv8-timer")
        {
            PropAction::Kind(NodeKind::Timer)
        } else if compatible_contains(value, b"virtio,mmio") {
            PropAction::Kind(NodeKind::VirtioMmio)
        } else {
            PropAction::None
        }
    }

    fn reg_action(&self, value: &[u8]) -> PropAction {
        let addr_cells = self.parent_address_cells().clamp(1, 2);
        let size_cells = self.parent_size_cells().clamp(1, 2);
        let stride = (addr_cells + size_cells) as usize * 4;
        if value.len() < stride {
            return PropAction::None;
        }

        let Some(first) = parse_reg_entry(value, addr_cells, size_cells, 0) else {
            return PropAction::None;
        };

        let second = parse_reg_entry(value, addr_cells, size_cells, 1);
        PropAction::Reg { first, second }
    }

    fn interrupts_action(&self, value: &[u8]) -> PropAction {
        let cells = self.interrupt_cells.max(1) as usize;
        if value.len() < cells * 4 {
            return PropAction::None;
        }

        let first = parse_irq(value, 0, cells);
        let timer = choose_timer_irq(value, cells);
        PropAction::Interrupts { first, timer }
    }

    fn apply_action(&mut self, action: PropAction) {
        match action {
            PropAction::None => {}
            PropAction::AddressCells(cells) => self.set_address_cells(cells),
            PropAction::SizeCells(cells) => self.set_size_cells(cells),
            PropAction::InterruptCells(cells) => {
                self.current_state_mut().interrupt_cells = cells;
            }
            PropAction::Kind(kind) => {
                self.current_state_mut().kind = kind;
            }
            PropAction::RootCompatible { compatible, len } => {
                self.info.root_compatible = compatible;
                self.info.root_compatible_len = len;
            }
            PropAction::Reg { first, second } => {
                let state = self.current_state_mut();
                state.first_reg = Some(first);
                state.second_reg = second;
            }
            PropAction::Interrupts { first, timer } => {
                let state = self.current_state_mut();
                if let Some(irq) = first {
                    state.first_irq = irq;
                }
                if let Some(irq) = timer {
                    state.timer_irq = irq;
                }
            }
        }
    }

    fn set_address_cells(&mut self, cells: u32) {
        if self.depth == 1 {
            self.root_address_cells = cells;
        }
        if self.depth < self.address_cells.len() {
            self.address_cells[self.depth] = cells;
        }
    }

    fn set_size_cells(&mut self, cells: u32) {
        if self.depth == 1 {
            self.root_size_cells = cells;
        }
        if self.depth < self.size_cells.len() {
            self.size_cells[self.depth] = cells;
        }
    }

    fn parent_address_cells(&self) -> u32 {
        let parent = self.depth.saturating_sub(1);
        self.address_cells
            .get(parent)
            .copied()
            .unwrap_or(self.root_address_cells)
    }

    fn parent_size_cells(&self) -> u32 {
        let parent = self.depth.saturating_sub(1);
        self.size_cells
            .get(parent)
            .copied()
            .unwrap_or(self.root_size_cells)
    }

    fn include_mmio(&mut self, region: MemRegion) {
        if !region.is_present() {
            return;
        }

        self.info.mmio_start = self.info.mmio_start.min(region.start);
        self.info.mmio_end = self.info.mmio_end.max(region.end());
    }

    fn current_state(&self) -> NodeState {
        self.nodes[self.node_index()]
    }

    fn current_state_mut(&mut self) -> &mut NodeState {
        let index = self.node_index();
        &mut self.nodes[index]
    }

    fn node_index(&self) -> usize {
        self.depth.min(self.nodes.len() - 1)
    }

    fn read_struct_be32(&self, off: usize) -> Option<u32> {
        if off + 4 > self.header.off_dt_struct + self.header.size_dt_struct {
            return None;
        }
        unsafe { read_be32(self.base + off) }
    }

    fn struct_slice(&self, off: usize, len: usize) -> Option<&[u8]> {
        if off + len > self.header.off_dt_struct + self.header.size_dt_struct {
            return None;
        }
        Some(unsafe { core::slice::from_raw_parts((self.base + off) as *const u8, len) })
    }

    fn string_at(&self, nameoff: usize) -> Option<&[u8]> {
        if nameoff >= self.header.size_dt_strings {
            return None;
        }

        let start = self.header.off_dt_strings + nameoff;
        let end = self.header.off_dt_strings + self.header.size_dt_strings;
        let mut off = start;
        while off < end {
            let byte = unsafe { read_volatile((self.base + off) as *const u8) };
            if byte == 0 {
                return Some(unsafe {
                    core::slice::from_raw_parts((self.base + start) as *const u8, off - start)
                });
            }
            off += 1;
        }
        None
    }

    fn node_name_len(&self, off: usize, end: usize) -> Option<usize> {
        let mut len = 0;
        while off + len < end {
            let byte = unsafe { read_volatile((self.base + off + len) as *const u8) };
            if byte == 0 {
                return Some(len);
            }
            len += 1;
        }
        None
    }
}

fn parse_reg_entry(
    value: &[u8],
    addr_cells: u32,
    size_cells: u32,
    index: usize,
) -> Option<MemRegion> {
    let cells_per_entry = (addr_cells + size_cells) as usize;
    let start = index.checked_mul(cells_per_entry)?.checked_mul(4)?;
    let end = start.checked_add(cells_per_entry * 4)?;
    if end > value.len() {
        return None;
    }

    let addr = read_cells(value, start, addr_cells)?;
    let size = read_cells(value, start + addr_cells as usize * 4, size_cells)?;
    if addr > usize::MAX as u64 || size > usize::MAX as u64 {
        return None;
    }

    Some(MemRegion::new(addr as usize, size as usize))
}

fn choose_timer_irq(value: &[u8], cells: usize) -> Option<IrqSpec> {
    let count = value.len() / (cells * 4);
    let mut first = None;
    let mut preferred = None;

    for index in 0..count {
        let irq = parse_irq(value, index, cells)?;
        if first.is_none() {
            first = Some(irq);
        }
        if irq.irq == qemu_virt::GENERIC_TIMER_PHYS_IRQ {
            preferred = Some(irq);
        }
    }

    preferred.or(first)
}

fn parse_irq(value: &[u8], index: usize, cells: usize) -> Option<IrqSpec> {
    let start = index.checked_mul(cells)?.checked_mul(4)?;
    if start + cells * 4 > value.len() {
        return None;
    }

    if cells >= 3 {
        let irq_type = read_prop_be32(value, start)?;
        let number = read_prop_be32(value, start + 4)?;
        let flags = read_prop_be32(value, start + 8).unwrap_or(0);
        let irq = match irq_type {
            0 => number + 32,
            1 => number + 16,
            _ => number,
        };
        Some(IrqSpec::new(irq, flags))
    } else {
        let irq = read_prop_be32(value, start)?;
        Some(IrqSpec::new(irq, 0))
    }
}

fn read_cells(value: &[u8], mut off: usize, cells: u32) -> Option<u64> {
    if cells == 0 || cells > 2 {
        return None;
    }

    let mut result = 0u64;
    for _ in 0..cells {
        result = (result << 32) | u64::from(read_prop_be32(value, off)?);
        off += 4;
    }
    Some(result)
}

fn prop_u32(value: &[u8]) -> Option<u32> {
    read_prop_be32(value, 0)
}

fn read_prop_be32(value: &[u8], off: usize) -> Option<u32> {
    if off + 4 > value.len() {
        return None;
    }
    Some(u32::from_be_bytes([
        value[off],
        value[off + 1],
        value[off + 2],
        value[off + 3],
    ]))
}

unsafe fn read_be32(addr: usize) -> Option<u32> {
    let b0 = unsafe { read_volatile(addr as *const u8) };
    let b1 = unsafe { read_volatile((addr + 1) as *const u8) };
    let b2 = unsafe { read_volatile((addr + 2) as *const u8) };
    let b3 = unsafe { read_volatile((addr + 3) as *const u8) };
    Some(u32::from_be_bytes([b0, b1, b2, b3]))
}

fn compatible_contains(value: &[u8], needle: &[u8]) -> bool {
    let mut start = 0;
    while start < value.len() {
        let mut end = start;
        while end < value.len() && value[end] != 0 {
            end += 1;
        }

        if bytes_eq(&value[start..end], needle) {
            return true;
        }

        start = end + 1;
    }
    false
}

fn copy_root_compatible(dst: &mut [u8; ROOT_COMPAT_LEN], value: &[u8], max: usize) -> usize {
    let mut len = 0;
    while len < value.len() && len < max {
        let byte = value[len];
        dst[len] = if byte == 0 { b',' } else { byte };
        len += 1;
    }

    if len != 0 && dst[len - 1] == b',' {
        len -= 1;
    }
    len
}

fn value_streq(value: &[u8], text: &[u8]) -> bool {
    let len = value
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(value.len());
    bytes_eq(&value[..len], text)
}

fn node_name_eq(name: &[u8], prefix: &[u8]) -> bool {
    if name.len() < prefix.len() {
        return false;
    }

    if !bytes_eq(&name[..prefix.len()], prefix) {
        return false;
    }

    name.len() == prefix.len() || name[prefix.len()] == b'@'
}

fn bytes_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len() && left.iter().zip(right.iter()).all(|(l, r)| l == r)
}

const fn align4(value: usize) -> usize {
    (value + 3) & !3
}
