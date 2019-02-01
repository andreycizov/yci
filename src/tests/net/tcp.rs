use crate::net::tcp::*;
use nom::Needed::*;
use nom::Err::*;
use crate::net::parser::*;
use crate::net::util::*;
use serde_json;

#[test]
fn test_tcp_parser_a() {
    dbg!(serde_json::to_string(&ClientBkRq::Result("1".into(), Ok(vec![]))));
    assert_eq!(
        parse_packet_bytes(b"\x1A\x00{\"Result\":[\"1\",{\"Ok\":[]}]}"),
        Ok((b"".as_ref(), ClientBkRq::Result("1".into(), Ok(vec![]))))
    );
}

#[test]
fn test_tcp_parser_b() {
    assert_eq!(
        parse_packet_bytes(b"\x1A\x00{\"Result\":[\"1\",{\"Ok\":[]}]}b"),
        Ok((b"b".as_ref(), ClientBkRq::Result("1".into(), Ok(vec![]))))
    );
}

#[test]
fn test_tcp_parser_c() {
    assert_eq!(
        parse_packet_bytes(b"\x25\x00"),
        Err(Incomplete(Size(37)))
    );
}

#[test]
fn test_streaming_buffer() {
//    let mut b = StreamingBuffer::new(parse_packet_bytes, 100);
//
//    b.buf()[0] = 31;
//    b.buf()[2..2+31] = *b"{\"ClientBkRq\":[\"1\", {\"Ok\": [}]}";
//
//    let x = b.try_parse_buffer();
//
//    assert_eq!(x, Err(StreamingBufferError::ParserError));
//
//
//    b.proceed(6);
//
//    let x = b.try_parse_buffer();
//
//    assert_eq!(x, Some(vec![b'\x66']));
//
//    let x = b.try_parse_buffer();
//
//    assert_eq!(x, Some(vec![]));
//
//    b.buf()[0] = 2;
//    b.buf()[2] = b'\x66';
//    b.buf()[3] = b'\x66';
//
//    b.proceed(10);
//
//    let x = b.try_parse_buffer();
//
//    assert_eq!(x, Some(vec![b'\x66', b'\x66']));
}


#[test]
fn test_client_local() {
    Client::new(

        
    )
}

