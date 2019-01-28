use crate::net::tcp::*;
use nom::Needed::*;
use nom::Err::*;
use crate::net::parser::*;
use crate::net::util::*;

#[test]
fn test_tcp_parser_a() {
    assert_eq!(
        parse_packet_bytes(b"\x01\x00a"),
        Ok((b"".as_ref(), b"a".to_vec()))
    );
}

#[test]
fn test_tcp_parser_b() {
    assert_eq!(
        parse_packet_bytes(b"\x01\x00ab"),
        Ok((b"b".as_ref(), b"a".to_vec()))
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
    let mut b = StreamingBuffer::new(parse_packet_bytes, 100);

    b.buf()[0] = 1;
    b.buf()[2] = b'\x66';

    let x = b.try_parse_buffer();

    assert_eq!(x, None);


    b.proceed(6);

    let x = b.try_parse_buffer();

    assert_eq!(x, Some(vec![b'\x66']));

    let x = b.try_parse_buffer();

    assert_eq!(x, Some(vec![]));

    b.buf()[0] = 2;
    b.buf()[2] = b'\x66';
    b.buf()[3] = b'\x66';

    b.proceed(10);

    let x = b.try_parse_buffer();

    assert_eq!(x, Some(vec![b'\x66', b'\x66']));
}
//struct StreamingBuffer<'a> {
//    b: &'a [u8],
//    up: usize,
//    down: usize,
//}
//
//impl <'a>StreamingBuffer<'a> {
//    pub fn new(size: usize) -> Self {
//        StreamingBuffer {
//            b: &[0; size],
//            up: 0,
//            down: 0,
//        }
//    }
//
//    pub fn buf(&mut self) -> &mut [u8] {
//        self.b[up..]
//    }
//
//    pub fn proceed(&mut self, count: usize) {
//        self.up += count;
//
//        assert_eq!(self.up < SIZE, true);
//    }
//
//    pub fn try_parse_buffer<F>(&mut self, parser: F) -> Option<&[u8]>
//        where F: FnOnce(&[u8]) -> Result<(&[u8], &[u8]), Err<&[u8], u32>>
//    {
//        // Incomplete(Needed),
//        if let Some((other, found)) = match parser(&self.b[self.down..self.up]) {
//            Ok(x) => Some(x),
//            Err(err) => match err {
//                Needed::Size(x) => {
//                    if x > SIZE - self.up {
//                        unsafe {
//                            use std::ptr;
//
//                            ptr::copy_nonoverlapping(&self.b[self.down], *mut self.b, self.up - self.down);
//                        }
//
//                        self.up = self.up - self.down;
//                        self.down = 0;
//                    }
//                    None
//                },
//                Unknown => {
//                    None
//                }
//            }
//        } {
//            self.down += other.len();
//            Some(found)
//        } else {
//            None
//        }
//    }
//}
