#![allow(dead_code)]

pub type DeviceResult<T> = Result<T, DeviceError>;

pub const MAX_DEVICES: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DeviceError {
    NotPresent,
    Unsupported,
    Busy,
    Invalid,
    Io,
}

impl DeviceError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotPresent => "not-present",
            Self::Unsupported => "unsupported",
            Self::Busy => "busy",
            Self::Invalid => "invalid",
            Self::Io => "io",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum IrqStatus {
    Unhandled,
    Handled,
    Wake { channel: u32 },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DeviceClass {
    Uart,
    InterruptController,
    Timer,
    Block,
    Input,
    Gpu,
    Network,
    Bus,
    Other,
}

impl DeviceClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Uart => "uart",
            Self::InterruptController => "irqchip",
            Self::Timer => "timer",
            Self::Block => "block",
            Self::Input => "input",
            Self::Gpu => "gpu",
            Self::Network => "net",
            Self::Bus => "bus",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Empty,
    Discovered,
    Bound,
    Ready,
    Failed,
}

impl DeviceState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Discovered => "discovered",
            Self::Bound => "bound",
            Self::Ready => "ready",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy)]
pub struct DeviceId(usize);

impl DeviceId {
    pub const fn new(value: usize) -> Self {
        Self(value)
    }

    pub const fn as_usize(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct DeviceNode {
    pub id: DeviceId,
    pub name: &'static str,
    pub driver: &'static str,
    pub class: DeviceClass,
    pub state: DeviceState,
    pub mmio_base: usize,
    pub mmio_size: usize,
    pub irq: u32,
    pub major: u16,
    pub minor: u16,
    pub wait_channel: u32,
}

impl DeviceNode {
    pub const fn empty() -> Self {
        Self {
            id: DeviceId::new(usize::MAX),
            name: "",
            driver: "",
            class: DeviceClass::Other,
            state: DeviceState::Empty,
            mmio_base: 0,
            mmio_size: 0,
            irq: 0,
            major: 0,
            minor: 0,
            wait_channel: 0,
        }
    }

    pub const fn is_present(self) -> bool {
        !matches!(self.state, DeviceState::Empty)
    }
}

pub struct DeviceManager {
    nodes: [DeviceNode; MAX_DEVICES],
    count: usize,
}

impl DeviceManager {
    pub const fn new() -> Self {
        Self {
            nodes: [const { DeviceNode::empty() }; MAX_DEVICES],
            count: 0,
        }
    }

    pub fn register(&mut self, mut node: DeviceNode) -> DeviceResult<DeviceId> {
        if self.count >= MAX_DEVICES {
            return Err(DeviceError::Busy);
        }

        let id = DeviceId::new(self.count);
        node.id = id;
        if node.state == DeviceState::Empty {
            node.state = DeviceState::Discovered;
        }
        self.nodes[self.count] = node;
        self.count += 1;
        Ok(id)
    }

    pub fn mark_ready(&mut self, id: DeviceId) -> DeviceResult<()> {
        let Some(node) = self.nodes.get_mut(id.as_usize()) else {
            return Err(DeviceError::Invalid);
        };
        if !node.is_present() {
            return Err(DeviceError::NotPresent);
        }
        node.state = DeviceState::Ready;
        Ok(())
    }

    pub fn mark_failed(&mut self, id: DeviceId) -> DeviceResult<()> {
        let Some(node) = self.nodes.get_mut(id.as_usize()) else {
            return Err(DeviceError::Invalid);
        };
        if !node.is_present() {
            return Err(DeviceError::NotPresent);
        }
        node.state = DeviceState::Failed;
        Ok(())
    }

    pub fn find_by_irq(&self, irq: u32) -> Option<DeviceNode> {
        self.nodes
            .iter()
            .take(self.count)
            .copied()
            .find(|node| node.irq == irq && node.is_present())
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn get(&self, index: usize) -> Option<DeviceNode> {
        self.nodes
            .get(index)
            .copied()
            .filter(|node| node.is_present())
    }
}

pub enum DeviceIo<'a> {
    Read(&'a mut [u8]),
    Write(&'a [u8]),
}

pub trait KernelDriver {
    fn name(&self) -> &'static str;
    fn init(&mut self) -> DeviceResult<()>;
    fn irq(&self) -> Option<u32>;
    fn handle_irq(&mut self) -> IrqStatus;
    fn io(&mut self, request: DeviceIo<'_>) -> DeviceResult<usize>;
    fn wait_channel(&self) -> Option<u32>;
}

static mut DEVICE_MANAGER: DeviceManager = DeviceManager::new();

pub fn register(node: DeviceNode) -> DeviceResult<DeviceId> {
    unsafe {
        let manager = &raw mut DEVICE_MANAGER;
        (*manager).register(node)
    }
}

pub fn mark_ready(id: DeviceId) -> DeviceResult<()> {
    unsafe {
        let manager = &raw mut DEVICE_MANAGER;
        (*manager).mark_ready(id)
    }
}

pub fn mark_failed(id: DeviceId) -> DeviceResult<()> {
    unsafe {
        let manager = &raw mut DEVICE_MANAGER;
        (*manager).mark_failed(id)
    }
}

pub fn find_by_irq(irq: u32) -> Option<DeviceNode> {
    unsafe {
        let manager = &raw const DEVICE_MANAGER;
        (*manager).find_by_irq(irq)
    }
}

pub fn count() -> usize {
    unsafe {
        let manager = &raw const DEVICE_MANAGER;
        (*manager).count()
    }
}

pub fn get(index: usize) -> Option<DeviceNode> {
    unsafe {
        let manager = &raw const DEVICE_MANAGER;
        (*manager).get(index)
    }
}
