use crate::kernel::net::{MacAddress, NetError};

pub const ETH_HEADER_LEN: usize = 14;
pub const ETH_TYPE_IPV4: u16 = 0x0800;
pub const ETH_TYPE_ARP: u16 = 0x0806;

#[derive(Clone, Copy)]
pub struct EthernetFrame<'a> {
    pub dst: MacAddress,
    pub src: MacAddress,
    pub ethertype: u16,
    pub payload: &'a [u8],
}

pub fn parse(bytes: &[u8]) -> Result<EthernetFrame<'_>, NetError> {
    if bytes.len() < ETH_HEADER_LEN {
        return Err(NetError::Truncated);
    }
    Ok(EthernetFrame {
        dst: MacAddress(bytes[0..6].try_into().unwrap()),
        src: MacAddress(bytes[6..12].try_into().unwrap()),
        ethertype: u16::from_be_bytes([bytes[12], bytes[13]]),
        payload: &bytes[ETH_HEADER_LEN..],
    })
}

pub fn write_header(
    bytes: &mut [u8],
    dst: MacAddress,
    src: MacAddress,
    ethertype: u16,
) -> Result<&mut [u8], NetError> {
    if bytes.len() < ETH_HEADER_LEN {
        return Err(NetError::TooLarge);
    }
    bytes[0..6].copy_from_slice(&dst.as_bytes());
    bytes[6..12].copy_from_slice(&src.as_bytes());
    bytes[12..14].copy_from_slice(&ethertype.to_be_bytes());
    Ok(&mut bytes[ETH_HEADER_LEN..])
}
