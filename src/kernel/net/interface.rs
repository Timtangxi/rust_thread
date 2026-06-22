use crate::kernel::net::{Ipv4Address, MAX_PACKET_SIZE, MacAddress, NetError, PacketBuffer};

pub const MAX_NET_INTERFACES: usize = 4;
pub const RX_QUEUE_LEN: usize = 8;
pub const TX_QUEUE_LEN: usize = 8;
const NET_WAIT_BASE: u32 = 0x4e45_0000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NetIfState {
    Down,
    Up,
}

#[derive(Clone, Copy)]
pub struct NetStats {
    pub rx_packets: u64,
    pub rx_bytes: u64,
    pub rx_errors: u64,
    pub rx_dropped: u64,
    pub tx_packets: u64,
    pub tx_bytes: u64,
    pub tx_errors: u64,
    pub tx_dropped: u64,
    pub arp_packets: u64,
    pub icmp_packets: u64,
    pub udp_packets: u64,
    pub tcp_packets: u64,
}

impl NetStats {
    pub const fn empty() -> Self {
        Self {
            rx_packets: 0,
            rx_bytes: 0,
            rx_errors: 0,
            rx_dropped: 0,
            tx_packets: 0,
            tx_bytes: 0,
            tx_errors: 0,
            tx_dropped: 0,
            arp_packets: 0,
            icmp_packets: 0,
            udp_packets: 0,
            tcp_packets: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct NetInterface {
    pub used: bool,
    pub name: [u8; 16],
    pub name_len: usize,
    pub mac: MacAddress,
    pub ipv4: Ipv4Address,
    pub state: NetIfState,
    pub mtu: usize,
    pub rx_head: usize,
    pub rx_len: usize,
    pub rx: [PacketBuffer; RX_QUEUE_LEN],
    pub tx_head: usize,
    pub tx_len: usize,
    pub tx: [PacketBuffer; TX_QUEUE_LEN],
    pub stats: NetStats,
}

impl NetInterface {
    pub const fn empty() -> Self {
        Self {
            used: false,
            name: [0; 16],
            name_len: 0,
            mac: MacAddress::ZERO,
            ipv4: Ipv4Address::ZERO,
            state: NetIfState::Down,
            mtu: 1500,
            rx_head: 0,
            rx_len: 0,
            rx: [const { PacketBuffer::empty() }; RX_QUEUE_LEN],
            tx_head: 0,
            tx_len: 0,
            tx: [const { PacketBuffer::empty() }; TX_QUEUE_LEN],
            stats: NetStats::empty(),
        }
    }

    pub fn name_str(&self) -> &str {
        core::str::from_utf8(&self.name[..self.name_len]).unwrap_or("")
    }
}

static mut INTERFACES: [NetInterface; MAX_NET_INTERFACES] =
    [const { NetInterface::empty() }; MAX_NET_INTERFACES];

pub fn register(name: &str, mac: MacAddress) -> Result<usize, NetError> {
    unsafe {
        for index in 0..MAX_NET_INTERFACES {
            if INTERFACES[index].used {
                continue;
            }
            let mut iface = NetInterface::empty();
            iface.used = true;
            iface.name_len = name.len().min(iface.name.len());
            iface.name[..iface.name_len].copy_from_slice(&name.as_bytes()[..iface.name_len]);
            iface.mac = mac;
            iface.state = NetIfState::Up;
            INTERFACES[index] = iface;
            return Ok(index);
        }
    }
    Err(NetError::QueueFull)
}

pub fn set_ipv4(index: usize, ipv4: Ipv4Address) -> Result<(), NetError> {
    let iface = interface_mut(index)?;
    iface.ipv4 = ipv4;
    Ok(())
}

pub fn push_rx(index: usize, bytes: &[u8]) -> Result<(), NetError> {
    if bytes.len() > MAX_PACKET_SIZE {
        return Err(NetError::TooLarge);
    }
    let iface = interface_mut(index)?;
    if iface.rx_len >= RX_QUEUE_LEN {
        return Err(NetError::QueueFull);
    }
    let tail = (iface.rx_head + iface.rx_len) % RX_QUEUE_LEN;
    iface.rx[tail].resize(bytes.len())?;
    iface.rx[tail].as_mut_slice().copy_from_slice(bytes);
    iface.rx_len += 1;
    Ok(())
}

pub fn queue_tx(index: usize, bytes: &[u8]) -> Result<(), NetError> {
    if bytes.len() > MAX_PACKET_SIZE {
        record_tx_error(index);
        return Err(NetError::TooLarge);
    }

    let iface = interface_mut(index)?;
    if iface.tx_len >= TX_QUEUE_LEN {
        iface.stats.tx_dropped = iface.stats.tx_dropped.wrapping_add(1);
        return Err(NetError::QueueFull);
    }

    let tail = (iface.tx_head + iface.tx_len) % TX_QUEUE_LEN;
    iface.tx[tail].resize(bytes.len())?;
    iface.tx[tail].as_mut_slice().copy_from_slice(bytes);
    iface.tx_len += 1;
    iface.stats.tx_packets = iface.stats.tx_packets.wrapping_add(1);
    iface.stats.tx_bytes = iface.stats.tx_bytes.wrapping_add(bytes.len() as u64);
    Ok(())
}

pub fn pop_rx(index: usize, out: &mut [u8]) -> Result<usize, NetError> {
    let iface = interface_mut(index)?;
    if iface.rx_len == 0 {
        return Err(NetError::QueueEmpty);
    }
    let packet = iface.rx[iface.rx_head];
    let count = out.len().min(packet.len);
    out[..count].copy_from_slice(&packet.as_slice()[..count]);
    iface.rx[iface.rx_head] = PacketBuffer::empty();
    iface.rx_head = (iface.rx_head + 1) % RX_QUEUE_LEN;
    iface.rx_len -= 1;
    Ok(count)
}

pub fn pop_tx(index: usize, out: &mut [u8]) -> Result<usize, NetError> {
    let iface = interface_mut(index)?;
    if iface.tx_len == 0 {
        return Err(NetError::QueueEmpty);
    }
    let packet = iface.tx[iface.tx_head];
    let count = out.len().min(packet.len);
    out[..count].copy_from_slice(&packet.as_slice()[..count]);
    iface.tx[iface.tx_head] = PacketBuffer::empty();
    iface.tx_head = (iface.tx_head + 1) % TX_QUEUE_LEN;
    iface.tx_len -= 1;
    Ok(count)
}

pub fn record_rx(index: usize, len: usize) {
    if let Ok(iface) = interface_mut(index) {
        iface.stats.rx_packets = iface.stats.rx_packets.wrapping_add(1);
        iface.stats.rx_bytes = iface.stats.rx_bytes.wrapping_add(len as u64);
    }
}

pub fn record_rx_error(index: usize) {
    if let Ok(iface) = interface_mut(index) {
        iface.stats.rx_errors = iface.stats.rx_errors.wrapping_add(1);
    }
}

pub fn record_rx_drop(index: usize) {
    if let Ok(iface) = interface_mut(index) {
        iface.stats.rx_dropped = iface.stats.rx_dropped.wrapping_add(1);
    }
}

pub fn record_tx_error(index: usize) {
    if let Ok(iface) = interface_mut(index) {
        iface.stats.tx_errors = iface.stats.tx_errors.wrapping_add(1);
    }
}

pub fn record_arp(index: usize) {
    if let Ok(iface) = interface_mut(index) {
        iface.stats.arp_packets = iface.stats.arp_packets.wrapping_add(1);
    }
}

pub fn record_icmp(index: usize) {
    if let Ok(iface) = interface_mut(index) {
        iface.stats.icmp_packets = iface.stats.icmp_packets.wrapping_add(1);
    }
}

pub fn record_udp(index: usize) {
    if let Ok(iface) = interface_mut(index) {
        iface.stats.udp_packets = iface.stats.udp_packets.wrapping_add(1);
    }
}

pub fn record_tcp(index: usize) {
    if let Ok(iface) = interface_mut(index) {
        iface.stats.tcp_packets = iface.stats.tcp_packets.wrapping_add(1);
    }
}

pub fn count() -> usize {
    let mut count = 0usize;
    for index in 0..MAX_NET_INTERFACES {
        unsafe {
            let iface = &raw const INTERFACES[index];
            if (*iface).used {
                count += 1;
            }
        }
    }
    count
}

pub fn get(index: usize) -> Option<NetInterface> {
    if index >= MAX_NET_INTERFACES {
        return None;
    }
    unsafe {
        let iface = &raw const INTERFACES[index];
        (*iface).used.then_some(*iface)
    }
}

pub fn wait_channel(index: usize) -> u32 {
    NET_WAIT_BASE | (index as u32 & 0xffff)
}

fn interface_mut(index: usize) -> Result<&'static mut NetInterface, NetError> {
    if index >= MAX_NET_INTERFACES {
        return Err(NetError::Invalid);
    }
    unsafe {
        let iface = &raw mut INTERFACES[index];
        if (*iface).used {
            Ok(&mut *iface)
        } else {
            Err(NetError::Invalid)
        }
    }
}
