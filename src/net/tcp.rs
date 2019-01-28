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

use std::io::Read;
use std::collections::HashMap;
use std::time::Duration;
use std::io::ErrorKind;


use crate::worker::*;
use crate::obj::*;
use crate::daemon::*;
use crate::net::parser::*;
use crate::net::util::*;

use std::slice::SliceIndex;

use serde_derive::{Serialize, Deserialize};
use serde_json::Error as SerdeError;
use std::sync::mpsc::TryRecvError;

const TA: Token = Token(0);
const TB: Token = Token(1);
const TC: Token = Token(2);

const CLIENT_CAPACITY: usize = 100;
const CLIENT_BUFFER: usize = 65535 + 4 * 5;

enum ClientRq {
    Assign(InterpolatedCommand, WorkerReplier),
    Close,
}

#[derive(Serialize, Deserialize, Debug)]
enum ClientBkRp {
    Request(String, InterpolatedCommand)
}

#[derive(Serialize, Deserialize, Debug)]
enum ClientBkRq {
    Header(Option<usize>, Vec<CommandId>),
    Result(String, WorkerResult),
}

struct Client {
    poll: Poll,
    stream: TcpStream,
    rcvr: Receiver<ClientRq>,
    next_idx: usize,
    waiting: HashMap<usize, WorkerReplier>,
    events: Events,
    buffer: StreamingBuffer<Vec<u8>>,
}

enum ClientRecv {
    Timeout,
    Exit,
    Empty,
    Disconnected,
    Err(ErrorKind),
    ErrJson(SerdeError),
    Backend(ClientBkRq),
    Frontend(ClientRq),
}

impl Client {
    pub fn new(
        stream: TcpStream,
    ) -> Result<(Sender<ClientRq>, Self), Error> {
        let (cs, cr) = channel::<ClientRq>();

        let poll = Poll::new()?;

        poll.register(&stream, TA, Ready::all(), PollOpt::level())?;
        poll.register(&cr, TB, Ready::readable(), PollOpt::level())?;

        let buffer = StreamingBuffer::new(parse_packet_bytes, CLIENT_BUFFER);
        // todo client actually needs to register first by creating Box<ClientFw>
        Ok((
            cs,
            Client {
                poll,
                stream,
                rcvr: cr,
                next_idx: 0,
                waiting: HashMap::<usize, WorkerReplier>::default(),
                events: Events::with_capacity(CLIENT_CAPACITY),
                buffer: buffer,
            }
        ))
    }

    fn recv(&mut self, timeout: Option<Duration>) -> ClientRecv {
        /// timeout, error, disconnected

        loop {
            match self.poll.poll(&mut self.events, timeout) {
                Ok(_) => {}
                Err(err) => match err.kind() {
                    ErrorKind::TimedOut => {
                        return ClientRecv::Timeout;
                    }
                    x => return ClientRecv::Err(x)
                }
            };

            for event in self.events.iter() {
                match event.token() {
                    TA => {
                        match self.stream.parse_read(&mut self.buffer) {
                            Ok(x) => {
                                if x == 0 {
                                    return ClientRecv::Disconnected;
                                }
                                self.buffer.proceed(x);
                            }
                            Err(err) => match err.kind() {
                                ErrorKind::WouldBlock => {
                                    continue;
                                }
                                x => {
                                    return ClientRecv::Err(x);
                                }
                            }
                        }

                        match self.buffer.try_parse_buffer() {
                            Some(x) => {
                                let pkt = match serde_json::from_slice(x.as_ref()) {
                                    Ok(x) => x,
                                    Err(err) => return ClientRecv::ErrJson(err),
                                };

                                return ClientRecv::Backend(pkt);
                            }
                            None => {}
                        }
                    }
                    TB => {
                        match self.rcvr.try_recv() {
                            Ok(x) => {
                                return ClientRecv::Frontend(x);
                            }
                            Err(x) => match x {
                                TryRecvError::Empty => {
                                    continue;
                                }
                                TryRecvError::Disconnected => {
                                    return ClientRecv::Disconnected;
                                }
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }


//        return ClientRecv::Messages;
        return ClientRecv::Exit;
    }

    pub fn run(&mut self) {
        // todo client negotiate with the client <capacity, vec<string>>, send it to TCPListener

        self.recv(Some(Duration::new(1, 0)));
    }
}

struct ClientFw {
    // whenever this is dropped, the Client needs to be dropped too
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
    Kill,
}

struct Listener {
    listener: TcpListener,
    master_tx: Sender<DaemonRequest>,
    poll: Poll,
    d_sndr: Sender<TCPWorkerAdapterRq>,
    l_rcvr: Receiver<ListenerRq>,
    events: Events,
}

const TOKEN_LISTENER: Token = Token(0);
const TOKEN_EXIT: Token = Token(1);

impl Listener {
    fn new(
        addr: SocketAddr,
        master_tx: Sender<DaemonRequest>,
        d_sndr: Sender<TCPWorkerAdapterRq>,
        l_rcvr: Receiver<ListenerRq>,
    ) -> Result<Listener, Error> {
        let listener = TcpListener::bind(&addr)?;

        let poll = Poll::new()?;



        poll.register(&listener, TOKEN_LISTENER, Ready::readable(), PollOpt::edge())?;
        poll.register(&l_rcvr, TOKEN_EXIT, Ready::readable(), PollOpt::edge())?;

        return Ok(
            Listener {
                listener,
                master_tx,
                poll,
                d_sndr,
                l_rcvr,
                events: Events::with_capacity(1024),
            }
        );
    }

    pub fn run(&mut self) {
        while match self.once() {
            Ok(x) => { x },
            Err(x) => {
                dbg!(x);
                false
            }
        } {}
    }

    pub fn once(&mut self) -> Result<bool, Error> {
        self.poll.poll(&mut self.events, None).unwrap();

        for event in self.events.iter() {
            match event.token() {
                TOKEN_LISTENER => {
                    let (connected, _) = self.listener.accept().unwrap();
                    let (client_reply_channel, mut c) = Client::new(connected).unwrap();

                    spawn(move || { c.run() });
                    // todo how do we manage error conditions spawned in threads?
                    // todo somehow manage the error condition here, possibly needs to go upstream
                }
                TOKEN_EXIT => {
                    // The server just shuts down the socket, let's just exit
                    // from our event loop.
                    return Ok(false);
                }
                _ => unreachable!(),
            }
        }

        return Ok(true);
    }
}

#[derive(Debug)]
pub enum TCPWorkerAdapterError {
    Address(AddrParseError),
    IO(Error),
}

impl From<Error> for TCPWorkerAdapterError {
    fn from(x: Error) -> Self {
        TCPWorkerAdapterError::IO(x)
    }
}

pub enum TCPWorkerAdapterRq {
    Connected(Option<usize>, Vec<String>, Receiver<WorkerId>),
    Disconnected(WorkerId, Option<usize>),
    Exit,
}

pub struct TCPWorkerAdapter {
    /// create new workers as they are received on the channel?
    listener: Sender<ListenerRq>,
}

impl TCPWorkerAdapter {
    /// Should own the WorkerForwarders (they will go away with it).

    pub fn new(addr: &str, master_tx: Sender<DaemonRequest>) -> Result<Self, TCPWorkerAdapterError> {
        let (l_sndr, l_rcvr) = channel::<ListenerRq>();
        let (d_sndr, d_rcvr) = channel::<TCPWorkerAdapterRq>();

        let (ep_sndr, ep_rcvr) = channel::<Option<Error>>();

        let addr: SocketAddr = addr.parse().map_err(|x| TCPWorkerAdapterError::Address(x))?;

        // todo who owns the workers created by the ListenerThread ?

        let mut listener = Listener::new(addr, master_tx, d_sndr, l_rcvr)?;

        spawn(move || { listener.run() });

        let poll = Poll::new().unwrap();
        let mut events = Events::with_capacity(1);

        poll.register(&ep_rcvr, Token(0), Ready::readable(), PollOpt::edge())?;

        // we wait only one second for the thread to start, otherwise it doesn't make much sense to wait.
        let n = poll.poll(
            &mut events,
            Some(Duration::from_secs(1)),
        ).unwrap();

        assert_eq!(n, 1);

        match ep_rcvr.try_recv().unwrap() {
            Some(err) => {
                return Err(TCPWorkerAdapterError::IO(err));
            }
            None => {}
        };

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
