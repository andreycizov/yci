use core::fmt::Debug;
use std::io::Error;
use nom::Err;

type StreamingFun<O: Debug> = for<'a> fn(&'a [u8]) -> Result<(&'a [u8], O), Err<&'a [u8], u32>>;

pub struct StreamingBuffer<O: Debug> {
    b: Vec<u8>,
    p: usize,
    c: usize,
    parser: StreamingFun<O>,
}

impl<O: Debug> StreamingBuffer<O> {
    pub fn new(parser: StreamingFun<O>, capacity: usize) -> Self {
        StreamingBuffer {
            b: vec![0; capacity],
            p: 0,
            c: capacity,
            parser,
        }
    }

    fn try_extend(&mut self) {
        if self.b.len() - self.p < self.c / 2 {
            self.b.append(&mut vec![0; self.c]);
        }
    }

    pub fn buf(&mut self) -> &mut [u8] {
        &mut self.b
    }

    pub fn proceed(&mut self, size: usize) {
        self.p += size;

        self.try_extend();

        assert_eq!(self.p < self.b.len(), true);
    }

    pub fn try_parse_buffer(&mut self) -> Option<O>
    {


        let rtn = if let Some((other, found)) = match (self.parser)(&self.b[..self.p]) {
            Ok(x) => Some(x),

            // todo implement More vs actual parser error here
            // todo in this scenario parser can't cause an error ever
            Err(err) => {
                // TODO what happens if the buffer is smaller than the message to be decoded?
                if self.p == self.c {
                    panic!("Must implement input overflow");
                }
                // TODO what if it is an actual error ?
                None
            }
        } {
            let (other, found) = dbg!((other, found));
            let len = self.p - other.len();
            Some((len, found))
        } else {
            None
        };

        rtn.map(|(len, rtn)| {
            self.b.drain(..len);

            self.p -= len;

            self.try_extend();

            rtn
        })
    }
}

use std::io::Read;

pub trait ParserStreamer<O: Debug> {
    fn parse_read(&mut self, buffer: &mut StreamingBuffer<O>) -> Result<usize, Error>;
}

impl<T, O: Debug> ParserStreamer<O> for T
where T: Read {
    fn parse_read(&mut self, buffer: &mut StreamingBuffer<O>) -> Result<usize, Error>{
        let read = self.read(buffer.buf())?;

        buffer.proceed(read);

        Ok(read)
    }
}
