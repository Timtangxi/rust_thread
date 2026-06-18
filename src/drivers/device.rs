#![allow(dead_code)]

pub type DeviceResult<T> = Result<T, DeviceError>;

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
