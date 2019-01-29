use core::fmt::Debug;
use std::io::{Error, ErrorKind};
use std::io;
use nom::Err as NomErr;

pub trait StreamReadable
    where Self: Sized {

    fn read<'a>(buffer: &'a [u8]) -> Result<(&'a [u8], Self), NomErr<&'a [u8], u32>>;
}

pub trait StreamWritable<Err>
    where Self: Sized {
    fn write(&self) -> Result<Vec<u8>, Err>;
}

pub struct StreamingBuffer {
    b: Vec<u8>,
    p: usize,
    c: usize,
}

#[derive(Debug)]
pub enum StreamingBufferError {
    BufferOverflow,
    ParserError,
    ShouldWait,
}

impl StreamingBuffer{
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

    pub fn try_read<'a, O: StreamReadable>(&mut self) -> Result<O, StreamingBufferError>
    {
        let (other, found) = match O::read(&self.b[..self.p]) {
            Ok(x) => x,
            Err(err) => match err {
                NomErr::Incomplete(x) => {
                    if self.p == self.c {
                        return Err(StreamingBufferError::BufferOverflow);
                    }
                    return Err(StreamingBufferError::ShouldWait);
                }
                _ => return Err(StreamingBufferError::ParserError)
            }
        };

        let len = self.p - other.len();

        self.b.drain(..len);
        self.p -= len;
        self.try_extend();

        Ok(found)
    }
}

use std::io::Read;
use mio::Evented;
use mio::Poll;
use mio::Token;
use mio::Ready;
use mio::PollOpt;
use std::io::Write;
use mio_extras::channel::{Receiver, Sender};
use std::sync::mpsc;
use mio_extras::channel::channel;
use std::marker::PhantomData;

#[derive(Debug)]
pub enum ParserStreamerError {
    Io(Error),
    Buffer(StreamingBufferError),
}

impl From<Error> for ParserStreamerError {
    fn from(x: Error) -> Self {
        ParserStreamerError::Io(x)
    }
}

impl From<StreamingBufferError> for ParserStreamerError {
    fn from(x: StreamingBufferError) -> Self {
        ParserStreamerError::Buffer(x)
    }
}

type SS<I: Debug, X> = fn(&I) -> Result<Vec<u8>, X>;

pub struct ParsingStream<S: Read + Write + Evented, I: StreamReadable, O, OErr>
{
    stream: S,
    msgs_tx: Sender<Result<I, ParserStreamerError>>,
    msgs_rx: Receiver<Result<I, ParserStreamerError>>,
    enabled: bool,
    buffer: StreamingBuffer,
    po: PhantomData<O>,
    poe: PhantomData<OErr>,
}

impl<S: Read + Write + Evented, I: Debug + StreamReadable, O: StreamWritable<OErr>, OErr> Evented for ParsingStream<S, I, O, OErr> {
    fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        poll.register(&self.stream, token, interest, opts)?;
        poll.register(&self.msgs_rx, token, interest, opts)?;
        Ok(())
    }

    fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()> {
        poll.reregister(&self.stream, token, interest, opts)?;
        poll.reregister(&self.msgs_rx, token, interest, opts)?;
        Ok(())
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        poll.deregister(&self.stream)?;
        poll.deregister(&self.msgs_rx)?;
        Ok(())
    }
}

/// An error returned from the `Sender::send` or `SyncSender::send` function.
pub enum SendError<S> {
    /// An IO error.
    Io(io::Error),

    /// The receiving half of the channel has disconnected.
    Disconnected,
    Serializer(S),
}

impl<X> From<io::Error> for SendError<X> {
    fn from(x: io::Error) -> Self {
        SendError::Io(x)
    }
}

impl<S: Read + Write + Evented, I: StreamReadable, O: StreamWritable<OErr>, OErr> ParsingStream<S, I, O, OErr> {
    pub fn new(
        stream: S,
        buffer: StreamingBuffer,
    ) -> Self {
        let (tx, rx) = channel::<Result<I, ParserStreamerError>>();

        ParsingStream {
            stream,
            msgs_tx: tx,
            msgs_rx: rx,
            enabled: true,
            buffer,
            po: PhantomData,
            poe: PhantomData
        }
    }

    pub fn send(&mut self, t: &O) -> Result<(), SendError<OErr>> {
        if !self.enabled {
            return Err(SendError::Disconnected);
        }

        self.stream.write(t.write(t).map_err(|x| SendError::Serializer(x))?.as_ref())?;

        Ok(())
    }

    pub fn try_recv(&mut self) -> Result<I, mpsc::TryRecvError> {
        if !self.enabled {
            return Err(mpsc::TryRecvError::Disconnected);
        }

        let buffer = self.stream.parse_read(&mut self.buffer);

        for x in buffer {
            self.msgs_tx.send(x).unwrap();
        }

        match self.msgs_rx.try_recv() {
            Ok(x) => match x {
                Ok(y) => Ok(y),
                Err(err) => {
                    self.enabled = false;
                    Err(mpsc::TryRecvError::Disconnected)
                }
            }
            Err(err) => Err(err)
        }
    }
}


pub trait ParserStreamer<O: StreamReadable> {
    fn parse_read(&mut self, buffer: &mut StreamingBuffer) -> Vec<Result<O, ParserStreamerError>>;
}


impl<T, O: StreamReadable> ParserStreamer<O> for T
    where T: Read {
    fn parse_read(&mut self, buffer: &mut StreamingBuffer) -> Vec<Result<O, ParserStreamerError>> {
        let mut rtn = Vec::<_>::with_capacity(10);

        loop {
            let read = match self.read(buffer.buf()) {
                Ok(x) => x,
                Err(err) => match err.kind() {
                    ErrorKind::WouldBlock => {
                        break;
                    }
                    _ => {
                        rtn.push(Err(ParserStreamerError::from(err)));
                        break;
                    }
                }
            };

            buffer.proceed(read);

            let mut should_stop = false;

            while !should_stop {
                let x = buffer.try_read::<O>();

                match x {
                    Ok(x) => {
                        rtn.push(Ok(x));
                    }
                    Err(err) => match err {
                        StreamingBufferError::ShouldWait => {
                            should_stop = true;
                        }
                        _ => {
                            rtn.push(Err(ParserStreamerError::from(err)));
                        }
                    }
                };
            };
        }

        rtn
    }
}
