use crate::kernel::net::{NetError, checksum16};

pub const ICMP_ECHO_REPLY: u8 = 0;
pub const ICMP_ECHO_REQUEST: u8 = 8;

#[derive(Clone, Copy)]
pub struct IcmpEcho<'a> {
    pub icmp_type: u8,
    pub code: u8,
    pub identifier: u16,
    pub sequence: u16,
    pub payload: &'a [u8],
}

pub fn parse_echo(bytes: &[u8]) -> Result<IcmpEcho<'_>, NetError> {
    if bytes.len() < 8 {
        return Err(NetError::Truncated);
    }
    if checksum16(bytes) != 0 {
        return Err(NetError::Invalid);
    }
    Ok(IcmpEcho {
        icmp_type: bytes[0],
        code: bytes[1],
        identifier: u16::from_be_bytes([bytes[4], bytes[5]]),
        sequence: u16::from_be_bytes([bytes[6], bytes[7]]),
        payload: &bytes[8..],
    })
}

pub fn write_echo_reply(
    bytes: &mut [u8],
    identifier: u16,
    sequence: u16,
    payload: &[u8],
) -> Result<usize, NetError> {
    let len = 8 + payload.len();
    if bytes.len() < len {
        return Err(NetError::TooLarge);
    }
    bytes[..len].fill(0);
    bytes[0] = ICMP_ECHO_REPLY;
    bytes[1] = 0;
    bytes[4..6].copy_from_slice(&identifier.to_be_bytes());
    bytes[6..8].copy_from_slice(&sequence.to_be_bytes());
    bytes[8..len].copy_from_slice(payload);
    let checksum = checksum16(&bytes[..len]);
    bytes[2..4].copy_from_slice(&checksum.to_be_bytes());
    Ok(len)
}
