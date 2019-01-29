use nom::{named, map_opt, error_position, map, do_parse, take, call, le_u16};

use crate::net::tcp::*;
use bytes::buf::BufMut;

named!(
    pub parse_packet_bytes<ClientBkRq>,
    map_opt!(
        map!(
            do_parse!(
               ty: le_u16
                >> data: take!(ty)
                >> (
                    data
                )
            ),
            Vec::from
        ),
        parse_packet
    )
);

pub fn parse_packet(buff: Vec<u8>) -> Option<ClientBkRq> {
    let x: &[u8] = buff.as_ref();
    serde_json::from_slice::<ClientBkRq>(&x).ok()
}

pub fn unparse_packet_bytes(x: &ClientBkRp) -> Result<Vec<u8>, serde_json::Error> {
    let string = serde_json::to_string(x)?;

    let string = string.into_bytes();
    let mut buf = bytes::BytesMut::with_capacity(string.len() + 2);

    buf.put_u16_be(string.len() as u16);
    buf.put(string);
    Ok(buf.to_owned().to_vec())
}