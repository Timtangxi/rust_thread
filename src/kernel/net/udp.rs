use crate::kernel::net::{Ipv4Address, NetError, checksum16};

pub const UDP_HEADER_LEN: usize = 8;

#[derive(Clone, Copy)]
pub struct UdpPacket<'a> {
    pub src_port: u16,
    pub dst_port: u16,
    pub payload: &'a [u8],
}

pub fn parse(bytes: &[u8]) -> Result<UdpPacket<'_>, NetError> {
    if bytes.len() < UDP_HEADER_LEN {
        return Err(NetError::Truncated);
    }
    let len = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    if len < UDP_HEADER_LEN || len > bytes.len() {
        return Err(NetError::Truncated);
    }
    Ok(UdpPacket {
        src_port: u16::from_be_bytes([bytes[0], bytes[1]]),
        dst_port: u16::from_be_bytes([bytes[2], bytes[3]]),
        payload: &bytes[UDP_HEADER_LEN..len],
    })
}

pub fn write(
    bytes: &mut [u8],
    src: Ipv4Address,
    dst: Ipv4Address,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Result<usize, NetError> {
    let len = UDP_HEADER_LEN + payload.len();
    if bytes.len() < len || len > u16::MAX as usize {
        return Err(NetError::TooLarge);
    }
    bytes[..len].fill(0);
    bytes[0..2].copy_from_slice(&src_port.to_be_bytes());
    bytes[2..4].copy_from_slice(&dst_port.to_be_bytes());
    bytes[4..6].copy_from_slice(&(len as u16).to_be_bytes());
    bytes[8..len].copy_from_slice(payload);

    let checksum = udp_checksum(src, dst, &bytes[..len]);
    bytes[6..8].copy_from_slice(&checksum.to_be_bytes());
    Ok(len)
}

fn udp_checksum(src: Ipv4Address, dst: Ipv4Address, udp: &[u8]) -> u16 {
    let mut pseudo = [0u8; 12 + 1536];
    pseudo[0..4].copy_from_slice(&src.as_bytes());
    pseudo[4..8].copy_from_slice(&dst.as_bytes());
    pseudo[8] = 0;
    pseudo[9] = 17;
    pseudo[10..12].copy_from_slice(&(udp.len() as u16).to_be_bytes());
    pseudo[12..12 + udp.len()].copy_from_slice(udp);
    let checksum = checksum16(&pseudo[..12 + udp.len()]);
    if checksum == 0 { 0xffff } else { checksum }
}
