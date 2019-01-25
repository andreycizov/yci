use std::thread::spawn;
use mio_extras::channel::{Sender, Receiver, channel};
use mio::net::TcpListener;
use std::io::Error;
use mio::Poll;
use mio::Ready;
use mio::Token;
use mio::Events;
use mio::PollOpt;
use std::net::SocketAddr;
use std::net::AddrParseError;
use mio::tcp::TcpStream;

use crate::worker::*;
use crate::obj::*;
use crate::daemon::*;
use std::collections::HashMap;
use std::time::Duration;
use std::io::ErrorKind;

use nom::*;

const TA: Token = Token(0);
const TB: Token = Token(1);
const TC: Token = Token(2);

const CLIENT_CAPACITY: usize = 100;
const CLIENT_BUFFER: usize = 65535 + 4;

enum ClientRq {
    Assign(InterpolatedCommand, WorkerReplier),
    Close,
}

struct Client {
    poll: Poll,
    stream: TcpStream,
    rcvr: Receiver<ClientRq>,
    next_idx: usize,
    waiting: HashMap<usize, WorkerReplier>,
    events: Events,
    buffer: [u8; CLIENT_BUFFER],
}

enum ClientRecv {
    Timeout,
    Exit,
    Err,
    Messages,
}

named!(pub parse_packet_bytes,
    do_parse!(
           ty: be_u16
        >> len: be_u16 // len includes the padding
        >> data: take!(len)
        >> (
            (data)
        ))
);

impl Client {
    pub fn new(
        stream: TcpStream,
    ) -> Result<(Sender<ClientRq>, Self), Error> {
        let (cs, cr) = channel::<ClientRq>();

        let poll = Poll::new()?;

        poll.register(&stream, TA, Ready::all(), PollOpt::edge())?;
        poll.register(&cr, TB, Ready::readable(), PollOpt::edge())?;

        Ok((
            cs,
            Client {
                poll,
                stream,
                rcvr: cr,
                next_idx: 0,
                waiting: HashMap::<usize, WorkerReplier>::default(),
                events: Events::with_capacity(CLIENT_CAPACITY),
                buffer: [0; CLIENT_BUFFER],
            }
        ))
    }

    fn recv(&mut self, timeout: Option<Duration>) -> ClientRecv {
        // WouldBlock
        // TimedOut
        match self.poll.poll(&mut self.events, timeout) {
            Ok(_) => {},
            Err(err) => match err.kind() {
                    ErrorKind::TimedOut => {
                        return ClientRecv::Timeout
                    }
                    _ => return ClientRecv::Err
            }
        };

        for event in self.events.iter() {
            match event.token() {
                TA => {
                    //let connected = self.stream.read();



                    // todo somehow manage the error condition here, possibly needs to go upstream
                }
                TB => {
                    // The server just shuts down the socket, let's just exit
                    // from our event loop.
                    return ClientRecv::Exit;
                }
                _ => unreachable!(),
            }
        }

        return ClientRecv::Messages;
    }

    pub fn run(&mut self, l_sndr: Sender<ListenerRq>) {
        // todo client negotiate with the client <capacity, vec<string>>, send it to TCPListener


    }
}

struct ClientFw {
    sndr: Sender<ClientRq>,
    capacity: Option<usize>,
    queues: Vec<CommandId>,
}

impl Worker for ClientFw {
    fn capacity(&self) -> Option<usize> {
        self.capacity
    }

    fn queues(&self) -> Vec<CommandId> {
        self.queues.clone()
    }

    fn put(&mut self, command: &InterpolatedCommand, result_cb: WorkerReplier) {
        self.sndr.send(ClientRq::Assign(command.clone(), result_cb));
    }
}

enum ListenerRq {
    Negotiated(Sender<ClientRq>, Option<usize>, Vec<CommandId>),
    Kill
}

struct Listener {
    listener: TcpListener,
    poll: Poll,
    d_sndr: Sender<TCPWorkerAdapterRq>,
    l_rcvr: Receiver<ListenerRq>,
    events: Events,
}

impl Listener {
    fn new(addr: SocketAddr, d_sndr: Sender<TCPWorkerAdapterRq>, l_rcvr: Receiver<ListenerRq>) -> Result<Listener, Error> {
        let listener = TcpListener::bind(&addr)?;

        let poll = Poll::new()?;

        let tok_listener = Token(0);
        let tok_exit = Token(1);

        poll.register(&listener, tok_listener, Ready::readable(), PollOpt::edge())?;
        poll.register(&l_rcvr, tok_exit, Ready::readable(), PollOpt::edge())?;

        return Ok(
            Listener {
                listener,
                poll,
                d_sndr,
                l_rcvr,
                events: Events::with_capacity(1024),
            }
        );
    }

    pub fn run(
        addr: SocketAddr,
        d_sndr: Sender<TCPWorkerAdapterRq>,
        l_rcvr: Receiver<ListenerRq>,
        ep_sndr: Sender<Option<Error>>,
    ) {
        let mut listener = match Listener::new(addr, d_sndr, l_rcvr) {
            Ok(x) => x,
            Err(err) => {
                ep_sndr.send(Some(err));
                return;
            }
        };

        ep_sndr.send(None);

        while listener.once() {}
    }

    pub fn once(&mut self) -> bool {
        self.poll.poll(&mut self.events, None).unwrap();

        for event in self.events.iter() {
            match event.token() {
                tok_listener => {
                    let connected = self.listener.accept();

                    // todo somehow manage the error condition here, possibly needs to go upstream
                }
                tok_exit => {
                    // The server just shuts down the socket, let's just exit
                    // from our event loop.
                    return false;
                }
                _ => unreachable!(),
            }
        }

        return true;
    }
}

#[derive(Debug)]
pub enum TCPWorkerAdapterError {
    Address(AddrParseError),
    IO(Error),
}

pub enum TCPWorkerAdapterRq {
    Connected(Option<usize>, Vec<String>, Receiver<WorkerId>),
    Disconnected(WorkerId, Option<usize>),
    Exit,
}

pub struct TCPWorkerAdapter {
    /// create new workers as they are received on the channel?
    daemon: Sender<DaemonRequest>,
    listener: Sender<ListenerRq>,
}

impl TCPWorkerAdapter {
    /// Should own the WorkerForwarders (they will go away with it).

    pub fn new(addr: &str, daemon_chan: Sender<DaemonRequest>) -> Result<Self, TCPWorkerAdapterError> {
        let (l_sndr, l_rcvr) = channel::<ListenerRq>();
        let (d_sndr, d_rcvr) = channel::<TCPWorkerAdapterRq>();

        let (ep_sndr, ep_rcvr) = channel::<Option<Error>>();

        let addr: SocketAddr = addr.parse().map_err(|x| TCPWorkerAdapterError::Address(x))?;

        // todo who owns the workers created by the ListenerThread ?

        spawn(move || { Listener::run(addr, d_sndr, l_rcvr, ep_sndr) });

        let poll = Poll::new().unwrap();
        let mut events = Events::with_capacity(1);

        poll.register(&ep_rcvr, Token(0), Ready::readable(), PollOpt::edge()).unwrap();

        let n = poll.poll(&mut events, None).unwrap();

        assert_eq!(n, 1);

        match ep_rcvr.try_recv().unwrap() {
            Some(err) => {
                return Err(TCPWorkerAdapterError::IO(err));
            }
            None => {}
        };

        Ok(TCPWorkerAdapter {
            daemon: daemon_chan,
            listener: l_sndr.clone(),
        })
    }
}

impl Drop for TCPWorkerAdapter {
    fn drop(&mut self) {
        self.listener.send(ListenerRq::Kill);
    }
}
