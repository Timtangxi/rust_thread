use crate::kernel::net::{Ipv4Address, MacAddress, NetError};

pub const ARP_PACKET_LEN: usize = 28;
pub const ARP_HTYPE_ETHERNET: u16 = 1;
pub const ARP_PTYPE_IPV4: u16 = 0x0800;
pub const ARP_REQUEST: u16 = 1;
pub const ARP_REPLY: u16 = 2;

#[derive(Clone, Copy)]
pub struct ArpPacket {
    pub operation: u16,
    pub sender_mac: MacAddress,
    pub sender_ip: Ipv4Address,
    pub target_mac: MacAddress,
    pub target_ip: Ipv4Address,
}

pub fn parse(bytes: &[u8]) -> Result<ArpPacket, NetError> {
    if bytes.len() < ARP_PACKET_LEN {
        return Err(NetError::Truncated);
    }
    let htype = u16::from_be_bytes([bytes[0], bytes[1]]);
    let ptype = u16::from_be_bytes([bytes[2], bytes[3]]);
    let hlen = bytes[4];
    let plen = bytes[5];
    if htype != ARP_HTYPE_ETHERNET || ptype != ARP_PTYPE_IPV4 || hlen != 6 || plen != 4 {
        return Err(NetError::Unsupported);
    }
    Ok(ArpPacket {
        operation: u16::from_be_bytes([bytes[6], bytes[7]]),
        sender_mac: MacAddress(bytes[8..14].try_into().unwrap()),
        sender_ip: Ipv4Address(bytes[14..18].try_into().unwrap()),
        target_mac: MacAddress(bytes[18..24].try_into().unwrap()),
        target_ip: Ipv4Address(bytes[24..28].try_into().unwrap()),
    })
}

pub fn write(bytes: &mut [u8], packet: ArpPacket) -> Result<usize, NetError> {
    if bytes.len() < ARP_PACKET_LEN {
        return Err(NetError::TooLarge);
    }
    bytes[0..2].copy_from_slice(&ARP_HTYPE_ETHERNET.to_be_bytes());
    bytes[2..4].copy_from_slice(&ARP_PTYPE_IPV4.to_be_bytes());
    bytes[4] = 6;
    bytes[5] = 4;
    bytes[6..8].copy_from_slice(&packet.operation.to_be_bytes());
    bytes[8..14].copy_from_slice(&packet.sender_mac.as_bytes());
    bytes[14..18].copy_from_slice(&packet.sender_ip.as_bytes());
    bytes[18..24].copy_from_slice(&packet.target_mac.as_bytes());
    bytes[24..28].copy_from_slice(&packet.target_ip.as_bytes());
    Ok(ARP_PACKET_LEN)
}
