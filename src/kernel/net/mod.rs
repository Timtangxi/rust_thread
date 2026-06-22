#![allow(dead_code)]

pub mod arp;
pub mod ethernet;
pub mod icmp;
pub mod interface;
pub mod ipv4;
pub mod udp;

pub const MAX_PACKET_SIZE: usize = 1536;
pub const DEFAULT_IPV4: Ipv4Address = Ipv4Address::new(10, 0, 2, 15);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    pub const BROADCAST: Self = Self([0xff; 6]);
    pub const ZERO: Self = Self([0; 6]);

    pub const fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; 6] {
        self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    pub const ZERO: Self = Self([0; 4]);
    pub const BROADCAST: Self = Self([255; 4]);

    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    pub const fn as_bytes(self) -> [u8; 4] {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct PacketBuffer {
    pub bytes: [u8; MAX_PACKET_SIZE],
    pub len: usize,
}

impl PacketBuffer {
    pub const fn empty() -> Self {
        Self {
            bytes: [0; MAX_PACKET_SIZE],
            len: 0,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.bytes[..self.len]
    }

    pub fn resize(&mut self, len: usize) -> Result<(), NetError> {
        if len > MAX_PACKET_SIZE {
            return Err(NetError::TooLarge);
        }
        self.len = len;
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NetError {
    Truncated,
    Invalid,
    Unsupported,
    TooLarge,
    NoRoute,
    QueueFull,
    QueueEmpty,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PacketKind {
    Arp,
    Icmp,
    Udp,
    Tcp,
    Unsupported,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RxDisposition {
    Consumed,
    Ignored,
    QueuedReply,
}

pub fn classify_ethernet(bytes: &[u8]) -> Result<PacketKind, NetError> {
    let frame = ethernet::parse(bytes)?;
    match frame.ethertype {
        ethernet::ETH_TYPE_ARP => {
            let _ = arp::parse(frame.payload)?;
            Ok(PacketKind::Arp)
        }
        ethernet::ETH_TYPE_IPV4 => {
            let packet = ipv4::parse(frame.payload)?;
            match packet.protocol {
                ipv4::IP_PROTO_ICMP => {
                    let _ = icmp::parse_echo(packet.payload)?;
                    Ok(PacketKind::Icmp)
                }
                ipv4::IP_PROTO_UDP => {
                    let _ = udp::parse(packet.payload)?;
                    Ok(PacketKind::Udp)
                }
                ipv4::IP_PROTO_TCP => Ok(PacketKind::Tcp),
                _ => Ok(PacketKind::Unsupported),
            }
        }
        _ => Ok(PacketKind::Unsupported),
    }
}

pub fn handle_rx(interface_index: usize, bytes: &[u8]) -> Result<RxDisposition, NetError> {
    interface::record_rx(interface_index, bytes.len());

    if let Err(err) = interface::push_rx(interface_index, bytes) {
        if err == NetError::QueueFull {
            interface::record_rx_drop(interface_index);
        }
    }

    let Some(iface) = interface::get(interface_index) else {
        return Err(NetError::Invalid);
    };

    let frame = match ethernet::parse(bytes) {
        Ok(frame) => frame,
        Err(err) => {
            interface::record_rx_error(interface_index);
            return Err(err);
        }
    };

    if frame.dst != iface.mac && frame.dst != MacAddress::BROADCAST {
        return Ok(RxDisposition::Ignored);
    }

    match frame.ethertype {
        ethernet::ETH_TYPE_ARP => handle_arp(interface_index, iface, frame),
        ethernet::ETH_TYPE_IPV4 => handle_ipv4(interface_index, iface, frame),
        _ => Ok(RxDisposition::Ignored),
    }
}

fn handle_arp(
    interface_index: usize,
    iface: interface::NetInterface,
    frame: ethernet::EthernetFrame<'_>,
) -> Result<RxDisposition, NetError> {
    interface::record_arp(interface_index);
    let packet = arp::parse(frame.payload)?;
    if packet.operation != arp::ARP_REQUEST
        || packet.target_ip != iface.ipv4
        || iface.ipv4 == Ipv4Address::ZERO
    {
        return Ok(RxDisposition::Consumed);
    }

    let mut out = PacketBuffer::empty();
    out.resize(ethernet::ETH_HEADER_LEN + arp::ARP_PACKET_LEN)?;
    let payload = ethernet::write_header(
        out.as_mut_slice(),
        packet.sender_mac,
        iface.mac,
        ethernet::ETH_TYPE_ARP,
    )?;
    arp::write(
        payload,
        arp::ArpPacket {
            operation: arp::ARP_REPLY,
            sender_mac: iface.mac,
            sender_ip: iface.ipv4,
            target_mac: packet.sender_mac,
            target_ip: packet.sender_ip,
        },
    )?;
    interface::queue_tx(interface_index, out.as_slice())?;
    Ok(RxDisposition::QueuedReply)
}

fn handle_ipv4(
    interface_index: usize,
    iface: interface::NetInterface,
    frame: ethernet::EthernetFrame<'_>,
) -> Result<RxDisposition, NetError> {
    let packet = ipv4::parse(frame.payload)?;
    if packet.dst != iface.ipv4 && packet.dst != Ipv4Address::BROADCAST {
        return Ok(RxDisposition::Ignored);
    }

    match packet.protocol {
        ipv4::IP_PROTO_ICMP => {
            interface::record_icmp(interface_index);
            handle_icmp(interface_index, iface, frame.src, packet)
        }
        ipv4::IP_PROTO_UDP => {
            let _ = udp::parse(packet.payload)?;
            interface::record_udp(interface_index);
            Ok(RxDisposition::Consumed)
        }
        ipv4::IP_PROTO_TCP => {
            interface::record_tcp(interface_index);
            Ok(RxDisposition::Consumed)
        }
        _ => Ok(RxDisposition::Ignored),
    }
}

fn handle_icmp(
    interface_index: usize,
    iface: interface::NetInterface,
    peer_mac: MacAddress,
    packet: ipv4::Ipv4Packet<'_>,
) -> Result<RxDisposition, NetError> {
    let echo = icmp::parse_echo(packet.payload)?;
    if echo.icmp_type != icmp::ICMP_ECHO_REQUEST || echo.code != 0 {
        return Ok(RxDisposition::Consumed);
    }

    let icmp_len = 8 + echo.payload.len();
    let total_len = ethernet::ETH_HEADER_LEN + ipv4::IPV4_HEADER_LEN + icmp_len;
    let mut out = PacketBuffer::empty();
    out.resize(total_len)?;

    let ip_payload = ethernet::write_header(
        out.as_mut_slice(),
        peer_mac,
        iface.mac,
        ethernet::ETH_TYPE_IPV4,
    )?;
    let icmp_payload = ipv4::write_header(
        ip_payload,
        iface.ipv4,
        packet.src,
        ipv4::IP_PROTO_ICMP,
        icmp_len,
        packet.identification,
    )?;
    icmp::write_echo_reply(icmp_payload, echo.identifier, echo.sequence, echo.payload)?;
    interface::queue_tx(interface_index, out.as_slice())?;
    Ok(RxDisposition::QueuedReply)
}

pub fn checksum16(bytes: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut chunks = bytes.chunks_exact(2);
    for chunk in &mut chunks {
        sum = sum.wrapping_add(u16::from_be_bytes([chunk[0], chunk[1]]) as u32);
    }
    if let Some(last) = chunks.remainder().first() {
        sum = sum.wrapping_add(u16::from_be_bytes([*last, 0]) as u32);
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}
