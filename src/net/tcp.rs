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

const TA: Token = Token(0);
const TB: Token = Token(1);
const TC: Token = Token(2);

const CLIENT_CAPACITY: usize = 100;
const CLIENT_BUFFER: usize = 65535 + 4 * 5;


#[derive(Serialize, Deserialize, Debug)]
pub enum ClientBkRp {
    Request(String, InterpolatedCommand)
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum ClientBkRq {
    Header(Option<usize>, Vec<CommandId>),
    Result(String, WorkerResult),
}

struct Client {
    poll: Poll,
    master_tx: Sender<DaemonRequest>,
    bk: ParsingStream<TcpStream, ClientBkRq, ClientBkRp, SerdeError>,
    tx: Sender<DaemonWorker>,
    rx: Receiver<DaemonWorker>,
    next_idx: usize,
    waiting: HashMap<usize, WorkerReplier>,
    events: Events,
}

enum ClientRecv {
    Timeout,
    Disconnected,
    Err(ErrorKind),
    ErrJson(SerdeError),
    Backend(ClientBkRq),
    Frontend(DaemonWorker),
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

impl Client {
    pub fn new(
        master_tx: Sender<DaemonRequest>,
        stream: TcpStream,
    ) -> Result<Self, Error> {
        let (tx, rx) = channel::<DaemonWorker>();

        let poll = Poll::new()?;

        poll.register(&stream, TA, Ready::all(), PollOpt::level())?;
        poll.register(&rx, TB, Ready::readable(), PollOpt::level())?;

        let buffer = StreamingBuffer::new(parse_packet_bytes, CLIENT_BUFFER);
        let bk = ParsingStream::new(
            stream,
            buffer,
            unparse_packet_bytes
        );
        // todo client actually needs to register first by creating Box<ClientFw>
        Ok(
            Client {
                poll,
                master_tx,
                bk,
                tx,
                rx,
                next_idx: 0,
                waiting: HashMap::<usize, WorkerReplier>::default(),
                events: Events::with_capacity(CLIENT_CAPACITY),
            }
        )
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
                    x => return ClientRecv::Disconnected,
                }
            };

            for event in self.events.iter() {
                match event.token() {
                    TA => {
                        match self.bk.try_recv() {
                            Ok(x) => return ClientRecv::Backend(x),
                            Err(err) => return ClientRecv::Disconnected,
                        }
                    }
                    TB => {
                        match self.rx.try_recv() {
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
    }

    fn run_loop(&mut self, key: &String) -> Result<bool, TCPWorkerAdapterError> {
        let mut ctr = 0;
        let mut commands = HashMap::<String, (ThreadId, StepId, CommandId)>::default();

        loop {
            match self.recv(None) {
                ClientRecv::Backend(x) => {
                    match x {
                        ClientBkRq::Result(cmd_id, res) => {
                            if let Some((tid, sid, cid)) = commands.remove(&cmd_id) {
                                self.master_tx.send(
                                    DaemonRequest::Finished(
                                        key.clone(),
                                        tid,
                                        sid,
                                        cid,
                                        res,
                                    )
                                );
                            } else {
                                return Err(TCPWorkerAdapterError::from("unknown command response"));
                            }
                        }
                        _ => return Err(TCPWorkerAdapterError::from("bk: unknown message"))
                    }
                }
                ClientRecv::Frontend(x) => {
                    match x {
                        DaemonWorker::JobAssigned(tid, sid, cid, cmd) => {
                            let cmd_key = ctr.to_string();
                            commands.insert(
                                cmd_key.clone(),
                                (tid, sid, cid)
                            );

                            let req = &ClientBkRp::Request(
                                cmd_key.clone(),
                                cmd,
                            );

                            self.bk.send(req).map_err(|_| "could not send buf")?;

                            ctr += 1;
                        }
                        _ => return Err(TCPWorkerAdapterError::from("fr: unknown message"))
                    }
                }
                ClientRecv::Disconnected => {
                    return Ok(true)
                }
                _ => return Err(TCPWorkerAdapterError::from("both: unexpected"))
            }
        }
    }

    pub fn run(&mut self) -> Result<bool, TCPWorkerAdapterError> {
        // todo client negotiate with the client <capacity, vec<string>>, send it to TCPListener

        match self.recv(Some(Duration::new(1, 0))) {
            ClientRecv::Backend(x) => {
                match x {
                    ClientBkRq::Header(capacity, queues) => {
                        self.master_tx.send(
                            DaemonRequest::WorkerAdd(
                                WorkerInfo { capacity, queues },
                                self.tx.clone(),
                            )
                        ).map_err(|_| "could not reach master")?;
                    }
                    _ => return Err(TCPWorkerAdapterError::from("header not received in time"))
                }
            }
            _ => return Err(TCPWorkerAdapterError::from("protocol error"))
        };

        let key = match self.recv(None) {
            ClientRecv::Frontend(x) => {
                match x {
                    DaemonWorker::WorkerCreated(key) => { key }
                    _ => return Err(TCPWorkerAdapterError::from("worker could not be created"))
                }
            }
            _ => return Err(TCPWorkerAdapterError::from("protocol error"))
        };

        let res = self.run_loop(&key);

        self.master_tx.send(DaemonRequest::WorkerRemove(key));

        res
    }
}

enum ListenerRq {
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

    pub fn run(&mut self) -> Result<bool, Error> {
        self.poll.poll(&mut self.events, None).unwrap();

        loop {
            for event in self.events.iter() {
                match event.token() {
                    TOKEN_LISTENER => {
                        let (connected, _) = self.listener.accept()?;
                        let mut c = Client::new(self.master_tx.clone(), connected)?;

                        spawn(move || err_sink(|| c.run()));
                        // todo how do we manage error conditions spawned in threads?
                        // todo somehow manage the error condition here, possibly needs to go upstream
                    }
                    TOKEN_EXIT => {
                        // The server just shuts down the socket, let's just exit
                        // from our event loop.

                        // todo we need to tell all of the threads that are still running to shut down.
                        // todo although instead the Master could tell that to them.

                        return Ok(false);
                    }
                    _ => unreachable!(),
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
