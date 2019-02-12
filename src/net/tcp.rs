use std::thread::spawn;
use mio_extras::channel::{Sender, Receiver, channel};
use mio::net::TcpListener;
use std::io::Error;
use mio::Poll;
use mio::Ready;
use mio::Token;
use mio::Events;
use mio::Evented;
use mio::PollOpt;
use std::net::SocketAddr;
use std::net::AddrParseError;
use mio::tcp::TcpStream;

use std::io::{Write, Read};
use std::collections::HashMap;
use std::time::Duration;
use std::io::ErrorKind;

use bytes;


use crate::worker::*;
use crate::obj::*;
use crate::daemon::*;
use crate::net::parser::*;
use crate::net::util::*;

use std::slice::SliceIndex;

use serde_derive::{Serialize, Deserialize};
use serde_json::Error as SerdeError;
use std::sync::mpsc::TryRecvError;
use std::fmt::Debug;
use bytes::BigEndian;

use nom::Err as NomErr;
use std::io;
use slab::Slab;
use std::mem;

const TA: Token = Token(0);
const TB: Token = Token(1);
const TC: Token = Token(2);

const CLIENT_CAPACITY: usize = 100;
const CLIENT_BUFFER: usize = 65535 + 4 * 5;


#[derive(Serialize, Deserialize, Debug)]
pub enum ClientBkRp {
    Request(usize, XCmd)
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum ClientBkRq {
    Header(Option<usize>, Vec<CommandId>),
    Result(usize, WorkerResult),
}

impl StreamReadable for ClientBkRq {
    fn read<'a>(buffer: &'a [u8]) -> Result<(&'a [u8], Self), NomErr<&'a [u8], u32>> {
        use nom::{named, map_opt, error_position, map, do_parse, take, call, le_u16};

        pub fn parse_packet(buff: Vec<u8>) -> Option<ClientBkRq> {
            let x: &[u8] = buff.as_ref();
            serde_json::from_slice::<ClientBkRq>(&x).ok()
        }

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

        parse_packet_bytes(buffer)
    }
}

impl StreamWritable<serde_json::Error> for ClientBkRq {
    fn write(&self) -> Result<Vec<u8>, serde_json::Error> {
        use bytes::buf::BufMut;

        let string = serde_json::to_string(self)?;

        let string = string.into_bytes();
        let mut buf = bytes::BytesMut::with_capacity(string.len() + 2);

        buf.put_u16_be(string.len() as u16);
        buf.put(string);
        Ok(buf.to_owned().to_vec())
    }
}

impl StreamReadable for ClientBkRp {
    fn read<'a>(buffer: &'a [u8]) -> Result<(&'a [u8], Self), NomErr<&'a [u8], u32>> {
        use nom::{named, map_opt, error_position, map, do_parse, take, call, le_u16};

        pub fn parse_packet(buff: Vec<u8>) -> Option<ClientBkRp> {
            let x: &[u8] = buff.as_ref();
            serde_json::from_slice::<ClientBkRp>(&x).ok()
        }

        named!(
            pub parse_packet_bytes<ClientBkRp>,
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

        parse_packet_bytes(buffer)
    }
}


impl StreamWritable<serde_json::Error> for ClientBkRp {
    fn write(&self) -> Result<Vec<u8>, serde_json::Error> {
        use bytes::buf::BufMut;

        let string = serde_json::to_string(self)?;

        let string = string.into_bytes();
        let mut buf = bytes::BytesMut::with_capacity(string.len() + 2);

        buf.put_u16_be(string.len() as u16);
        buf.put(string);
        Ok(buf.to_owned().to_vec())
    }
}

pub fn err_sink<Err: Debug, R, F>(f: F) -> Result<R, Err>
    where F: FnOnce() -> Result<R, Err> {
    match f() {
        Ok(x) => Ok(x),
        Err(err) => {
            Err(dbg!(err))
        }
    }
}


struct StreamForwarder<S: Read + Write + Evented, I: StreamReadable + Debug, O: Debug + StreamWritable<E>, E> {
    //stream: S,

    pub(crate) bk: ParsingStream<S, I, O, E>,
    tx: Sender<I>,
    should_tx: bool,
    pub(crate) rx: Receiver<O>,
    should_rx: bool,

}

pub enum StreamForwarderErr {
    TxDisconnected,
    RxDisconnected,
    Io(io::Error),
}

impl From<io::Error> for StreamForwarderErr {
    fn from(x: io::Error) -> Self {
        StreamForwarderErr::Io(x)
    }
}

impl<S: Read + Write + Evented, I: StreamReadable + Debug, O: Debug + StreamWritable<E>, E>
StreamForwarder<S, I, O, E> {
    /// we must map one behaviour to one thing (?)


    pub fn new(
        stream: S,
    ) -> Result<(Receiver<I>, Sender<O>, Self), Error> {
        let (itx, irx) = channel::<I>();
        let (otx, orx) = channel::<O>();

        let buffer = StreamingBuffer::new(CLIENT_BUFFER);
        let bk = ParsingStream::new(
            stream,
            buffer,
        );

        let rx = irx;
        let tx = otx;

        Ok(
            (
                rx,
                tx,
                StreamForwarder {
                    bk,
                    tx: itx,
                    should_tx: true,
                    rx: orx,
                    should_rx: true,
                }
            )
        )
    }

    pub fn run(&mut self) -> Result<(), StreamForwarderErr> {
        let poll = Poll::new()?;

        poll.register(&self.bk, Token(0), Ready::readable(), PollOpt::edge())?;
        poll.register(&self.rx, Token(1), Ready::readable(), PollOpt::edge())?;


        self.tx_loop()?;
        self.rx_loop()?;
        Ok(())
    }

    pub fn tx_loop(&mut self) -> Result<(), StreamForwarderErr> {
        loop {
            match self.should_tx {
                true => match self.bk.try_recv() {
                    Ok(x) => {
                        match self.tx.send(x) {
                            Ok(_) => continue,
                            Err(_) => {
                                self.should_tx = false;
                                return Err(StreamForwarderErr::RxDisconnected);
                            }
                        }
                    }
                    Err(x) => match x {
                        TryRecvError::Empty => {
                            return Ok(());
                        }
                        TryRecvError::Disconnected => {
                            return Err(StreamForwarderErr::TxDisconnected);
                        }
                    }
                },
                false => return Err(StreamForwarderErr::TxDisconnected)
            }
        }
    }

    pub fn rx_loop(&mut self) -> Result<(), StreamForwarderErr> {
        loop {
            match self.should_rx {
                true => match self.rx.try_recv() {
                    Ok(x) => {
                        match self.bk.send(&x) {
                            Ok(_) => continue,
                            Err(_) => {
                                self.should_rx = false;
                                continue;
                            }
                        }
                    }
                    Err(x) => match x {
                        TryRecvError::Empty => {
                            return Ok(());
                        }
                        TryRecvError::Disconnected => {
                            return Err(StreamForwarderErr::RxDisconnected);
                        }
                    }
                },
                false => return Err(StreamForwarderErr::RxDisconnected)
            }
        }
    }
}

type AssignedCommands = Slab<(ThreadId, StepId, CommandId)>;

pub struct TcpClient {
    address: SocketAddr,
    state: ClientState,
    rx: Receiver<ClientBkRq>,
    tx: Sender<ClientBkRp>,
    rrx: Receiver<DaemonWorker>,
    chan: StreamForwarder<TcpStream, ClientBkRq, ClientBkRp, SerdeError>,
}

pub enum ClientState {
    Waiting(Sender<DaemonWorker>),
    Assigned,
    Operating(WorkerId, AssignedCommands),
}

pub enum ListenerRq {
    Kill,
}

struct Listener {
    listener: TcpListener,
    master_tx: Sender<DaemonRequest>,
    poll: Poll,
    l_rcvr: Receiver<ListenerRq>,
    tok_ctr: usize,
    clients: HashMap<usize, TcpClient>,
}

#[derive(Debug)]
pub enum TcpClientErr {
    Rx(usize),
    Tx(usize),
    Nx(usize),
}

const TOK_PER_BLOCK: usize = 5;

impl Listener {
    pub fn new(
        addr: SocketAddr,
        master_tx: Sender<DaemonRequest>,
        l_rcvr: Receiver<ListenerRq>,
    ) -> Result<Listener, Error> {
        let listener = TcpListener::bind(&addr)?;

        let poll = Poll::new()?;


        poll.register(&listener, Token(0), Ready::readable(), PollOpt::edge())?;
        poll.register(&l_rcvr, Token(1), Ready::readable(), PollOpt::edge())?;

        return Ok(
            Listener {
                listener,
                master_tx,
                poll,
                l_rcvr,
                tok_ctr: 1,
                clients: HashMap::<usize, TcpClient>::new(),
            }
        );
    }

    fn register(&mut self, client: TcpClient) -> Result<(), io::Error> {
        let idx = self.tok_ctr + 1;
        self.tok_ctr = self.tok_ctr.wrapping_add(1);

        let tok_begin = idx * TOK_PER_BLOCK;

        self.poll.register(&client.chan.bk, Token(tok_begin), Ready::readable(), PollOpt::edge())?;
        self.poll.register(&client.chan.rx, Token(tok_begin + 1), Ready::readable(), PollOpt::edge())?;
        self.poll.register(&client.rx, Token(tok_begin + 2), Ready::readable(), PollOpt::edge())?;
        self.poll.register(&client.rrx, Token(tok_begin + 3), Ready::readable(), PollOpt::edge())?;
        //self.poll.register(&client.tx, Token(tok_begin + 3), Ready::readable(), PollOpt::edge())?;

        self.clients.insert(
            idx,
            client,
        );

        Ok(())
    }

    fn unregister(&mut self, idx: usize) -> bool {
        if let Some(client) = self.clients.remove(&idx) {
            self.poll.deregister(&client.chan.bk).unwrap();
            self.poll.deregister(&client.chan.rx).unwrap();
            self.poll.deregister(&client.rx).unwrap();
            self.poll.deregister(&client.rrx).unwrap();
            true
        } else {
            false
        }
    }

    pub fn process_client(&mut self, client_idx: usize, event_idx: usize) -> Result<(), TcpClientErr> {
        let mut client = match self.clients.get_mut(&client_idx) {
            Some(x) => x,
            None => return Err(TcpClientErr::Nx(0))
        };

        match event_idx {
            0 => {
                client.chan.rx_loop().map_err(|_| TcpClientErr::Rx(1000))?;
            }
            1 => {
                client.chan.tx_loop().map_err(|_| TcpClientErr::Rx(1001))?;
            }
            2 => loop {
                match client.rx.try_recv() {
                    Ok(x) => {
                        match &mut client.state {
                            ClientState::Waiting(atx) => {
                                let atx = atx.clone();
                                mem::replace(&mut client.state, ClientState::Assigned);

                                match x {
                                    ClientBkRq::Header(capacity, queues) => {
                                        self.master_tx.send(DaemonRequest::WorkerAdd(WorkerInfo(capacity, queues), atx)).map_err(|_| TcpClientErr::Rx(108))?;
                                    }
                                    _ => {
                                        return Err(TcpClientErr::Rx(0));
                                    }
                                }
                            }
                            ClientState::Operating(wid, assigned_commands) => {
                                match x {
                                    ClientBkRq::Result(idx, wres) => {
                                        if assigned_commands.contains(idx) {
                                            let (a, b, c) = assigned_commands.remove(idx);

                                            self.master_tx.send(DaemonRequest::Finished(wid.clone(), a, b, c, wres)).map_err(|_| TcpClientErr::Rx(99))?;
                                        } else {
                                            return Err(TcpClientErr::Rx(1));
                                        }
                                    }
                                    _ => {
                                        return Err(TcpClientErr::Rx(444));
                                    }
                                }
                            }
                            _ => {
                                return Err(TcpClientErr::Rx(2));
                            }
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(_) => {
                        return Err(TcpClientErr::Rx(4));
                    }
                }
            },
            3 => loop {
                match client.rrx.try_recv() {
                    Ok(pkt) => {
                        match &mut client.state {
                            ClientState::Assigned => {
                                match pkt {
                                    DaemonWorker::WorkerCreated(wid) => {
                                        client.state = ClientState::Operating(
                                            wid,
                                            AssignedCommands::with_capacity(100),
                                        );
                                    }
                                    _ => {
                                        return Err(TcpClientErr::Tx(3435));
                                    }
                                }
                            }
                            ClientState::Operating(wid, acmds) => {
                                match pkt {
                                    DaemonWorker::JobAssigned(a, b, c, d) => {
                                        let idx = acmds.insert((a.clone(), b.clone(), c.clone()));
                                        client.tx.send(ClientBkRp::Request(idx, d)).map_err(|_| TcpClientErr::Tx(99))?;
                                    }
                                    _ => {
                                        return Err(TcpClientErr::Tx(2342));
                                    }
                                }
                            }
                            _ => {
                                return Err(TcpClientErr::Tx(23565));
                            }
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(_) => {
                        return Err(TcpClientErr::Tx(0));
                    }
                }
            },
            _ => unreachable!()
        };

        Ok(())
    }

    pub fn run(&mut self) -> Result<bool, Error> {
        // right now, the issue is that we need to pick one of the sequential communicators.
        let mut events = Events::with_capacity(1024);
        loop {
            self.poll.poll(&mut events, None)?;

            for event in events.iter() {
                match event.token() {
                    Token(0) => {
                        loop {
                            let (connected, address) = match self.listener.accept() {
                                Ok(x) => x,
                                Err(x) => match x.kind() {
                                    io::ErrorKind::WouldBlock => continue,
                                    _ => return Err(x)
                                }
                            };

                            let (rx, tx, fw) = StreamForwarder::<TcpStream, ClientBkRq, ClientBkRp, SerdeError>::new(connected)?;

                            let (atx, arx) = channel::<DaemonWorker>();

                            let client = TcpClient { address, state: ClientState::Waiting(atx), chan: fw, rx, tx, rrx: arx };

                            self.register(client).unwrap();
                            //spawn(move || err_sink(|| c.run()));
                            // todo how do we manage error conditions spawned in threads?
                            // todo somehow manage the error condition here, possibly needs to go upstream
                        }
                    }
                    Token(1) => {
                        // The server just shuts down the socket, let's just exit
                        // from our event loop.

                        // todo we need to tell all of the threads that are still running to shut down.
                        // todo although instead the Master could tell that to them.

                        return Ok(false);
                    }
                    Token(x) => {
                        let client_idx = x / TOK_PER_BLOCK;
                        let event_idx = x % TOK_PER_BLOCK;

                        match self.process_client(client_idx, event_idx) {
                            _ => {}
                            Err(x) => {
                                let success = self.unregister(client_idx);
                                dbg!(x);

                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum TCPWorkerAdapterError {
    Address(AddrParseError),
    IO(Error),
    Str(String),
}

impl From<&str> for TCPWorkerAdapterError {
    fn from(x: &str) -> Self {
        TCPWorkerAdapterError::Str(x.to_string())
    }
}

impl From<Error> for TCPWorkerAdapterError {
    fn from(x: Error) -> Self {
        TCPWorkerAdapterError::IO(x)
    }
}

pub struct TCPWorkerAdapter {
    /// create new workers as they are received on the channel?
    pub listener: Sender<ListenerRq>,
}

impl TCPWorkerAdapter {
    /// Should own the WorkerForwarders (they will go away with it).

    pub fn new(addr: &str, master_tx: Sender<DaemonRequest>) -> Result<Self, TCPWorkerAdapterError> {
        let (l_sndr, l_rcvr) = channel::<ListenerRq>();

        let addr: SocketAddr = addr.parse().map_err(|x| TCPWorkerAdapterError::Address(x))?;

        // todo who owns the workers created by the ListenerThread ?

        let mut listener = Listener::new(addr, master_tx, l_rcvr)?;

        spawn(move || err_sink(|| listener.run()));

        Ok(TCPWorkerAdapter {
            listener: l_sndr.clone(),
        })
    }
}

impl Drop for TCPWorkerAdapter {
    fn drop(&mut self) {
        self.listener.send(ListenerRq::Kill);
    }
}
