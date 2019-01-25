use nom::Err;

pub struct StreamingBuffer {
    b: Vec<u8>,
    p: usize,
    c: usize,
}

impl StreamingBuffer {
    pub fn new(capacity: usize) -> Self {
        StreamingBuffer {
            b: vec![0; capacity],
            p: 0,
            c: capacity,
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

    pub fn try_parse_buffer<F>(&mut self, parser: F) -> Option<Vec<u8>>
        where F: FnOnce(&[u8]) -> Result<(&[u8], &[u8]), Err<&[u8], u32>>
    {
        let rtn = if let Some((other, found)) = match parser(&self.b[..self.p]) {
            Ok(x) => Some(x),

            // todo implement More vs actual parser error here
            // todo in this scenario parser can't cause an error ever
            Err(err) => None
        } {
            let (other, found) = dbg!((other, found));
            let len = self.p - other.len();
            Some((len, found.to_vec()))
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