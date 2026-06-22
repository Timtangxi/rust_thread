use crate::kernel::net::{Ipv4Address, NetError, checksum16};

pub const IPV4_HEADER_LEN: usize = 20;
pub const IP_PROTO_ICMP: u8 = 1;
pub const IP_PROTO_TCP: u8 = 6;
pub const IP_PROTO_UDP: u8 = 17;

#[derive(Clone, Copy)]
pub struct Ipv4Packet<'a> {
    pub src: Ipv4Address,
    pub dst: Ipv4Address,
    pub protocol: u8,
    pub ttl: u8,
    pub identification: u16,
    pub payload: &'a [u8],
}

pub fn parse(bytes: &[u8]) -> Result<Ipv4Packet<'_>, NetError> {
    if bytes.len() < IPV4_HEADER_LEN {
        return Err(NetError::Truncated);
    }
    let version = bytes[0] >> 4;
    let ihl = (bytes[0] & 0x0f) as usize * 4;
    if version != 4 || ihl < IPV4_HEADER_LEN || bytes.len() < ihl {
        return Err(NetError::Invalid);
    }
    let total_len = u16::from_be_bytes([bytes[2], bytes[3]]) as usize;
    if total_len < ihl || total_len > bytes.len() {
        return Err(NetError::Truncated);
    }
    if checksum16(&bytes[..ihl]) != 0 {
        return Err(NetError::Invalid);
    }
    Ok(Ipv4Packet {
        src: Ipv4Address(bytes[12..16].try_into().unwrap()),
        dst: Ipv4Address(bytes[16..20].try_into().unwrap()),
        protocol: bytes[9],
        ttl: bytes[8],
        identification: u16::from_be_bytes([bytes[4], bytes[5]]),
        payload: &bytes[ihl..total_len],
    })
}

pub fn write_header(
    bytes: &mut [u8],
    src: Ipv4Address,
    dst: Ipv4Address,
    protocol: u8,
    payload_len: usize,
    identification: u16,
) -> Result<&mut [u8], NetError> {
    let total_len = IPV4_HEADER_LEN + payload_len;
    if bytes.len() < total_len || total_len > u16::MAX as usize {
        return Err(NetError::TooLarge);
    }
    bytes[..IPV4_HEADER_LEN].fill(0);
    bytes[0] = (4 << 4) | 5;
    bytes[1] = 0;
    bytes[2..4].copy_from_slice(&(total_len as u16).to_be_bytes());
    bytes[4..6].copy_from_slice(&identification.to_be_bytes());
    bytes[6..8].copy_from_slice(&0u16.to_be_bytes());
    bytes[8] = 64;
    bytes[9] = protocol;
    bytes[12..16].copy_from_slice(&src.as_bytes());
    bytes[16..20].copy_from_slice(&dst.as_bytes());
    let checksum = checksum16(&bytes[..IPV4_HEADER_LEN]);
    bytes[10..12].copy_from_slice(&checksum.to_be_bytes());
    Ok(&mut bytes[IPV4_HEADER_LEN..total_len])
}
