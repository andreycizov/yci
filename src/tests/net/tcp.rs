use crate::net::tcp::*;
use nom::Needed::*;
use nom::Err::*;
use crate::net::parser::*;
use crate::net::util::*;
use serde_json;
use mio_extras::channel::channel;
use crate::daemon::DaemonRequest;
use std::net::SocketAddr;
use std::collections::VecDeque;
use crate::daemon::State;
use crate::daemon::MQ;
use crate::daemon::WS;
use crate::tests::prog::LoadIRFile;
use crate::daemon::Ass;
use crate::tests::prog::TEST_ALGO;
use crate::daemon::DPU;
use crate::obj::XCmd;
use crate::obj::XCmdArg;
use crate::daemon::ThreadState;
use crate::obj::XCtxRef;
use crate::obj::XCtxNs;
use std::thread::sleep;
use std::time::Duration;
use crate::tests::worker::FirstExecutor;
use crate::net::tcp::StreamForwarder;
use mio::net::TcpStream;

#[test]
fn test_tcp_parser_a() {
    dbg!(serde_json::to_string(&ClientBkRq::Result(1, Ok(vec![]))));
    assert_eq!(
        parse_packet_bytes(b"\x18\x00{\"Result\":[1,{\"Ok\":[]}]}"),
        Ok((b"".as_ref(), ClientBkRq::Result(1, Ok(vec![]))))
    );
}

#[test]
fn test_tcp_parser_b() {
    assert_eq!(
        parse_packet_bytes(b"\x18\x00{\"Result\":[1,{\"Ok\":[]}]}b"),
        Ok((b"b".as_ref(), ClientBkRq::Result(1, Ok(vec![]))))
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


use serde_json::Error as SerdeError;
use std::io;
use mio_extras::channel::Receiver;
use mio_extras::channel::Sender;
use std::thread::spawn;
use crate::daemon::DaemonWorker;

struct WorkerTcp {
    rx: Receiver<ClientBkRp>,
    tx: Sender<ClientBkRq>,
    chan: StreamForwarder<TcpStream, ClientBkRp, ClientBkRq, SerdeError>,
}

impl FirstExecutor for WorkerTcp {

}

impl WorkerTcp {
    pub fn new(
        addr: &SocketAddr,
    ) -> Result<Self, io::Error> {
        let sock = TcpStream::connect(addr)?;

        sock.set_nodelay(true)?;
        sock.set_keepalive(Some(Duration::from_secs(1)))?;

        let (rx, tx, fw) = StreamForwarder::<TcpStream, ClientBkRp, ClientBkRq, SerdeError>::new(sock)?;

        Ok(WorkerTcp { rx, tx, chan: fw })
    }

    pub fn header(&mut self) {
        self.tx.send(ClientBkRq::Header(None, vec![
            "push".into(),
            "list_create".into(),
            "list_length".into(),
            "db_user_list".into(),
            "set".into(),
            "icmp".into(),
            "if".into(),
        ])).unwrap();

        self.chan.tx_loop().expect("a");
        self.chan.rx_loop().expect("b");
    }

    pub fn run(&mut self) -> usize {
        let mut i = 0;

        self.chan.rx_loop().expect("a");

        while let Ok(x) = self.rx.try_recv() {
            match x {
                ClientBkRp::Request(idx, cmd) => {
                    let ret = self.exec(&cmd);

                    self.tx.send(ClientBkRq::Result(idx, ret));
                    self.chan.tx_loop().expect("b");
                }
            }
        }
        i
    }
}



#[test]
fn test_client_local() {
    let mut state = State::default();
    let mut assignment_queue = VecDeque::<Ass>::default();
    let mut multi_queue = MQ::default();
    let mut workers = WS::default();


    let ir = LoadIRFile::new(TEST_ALGO);
    let ir = ir.load().unwrap();

    state.insert_commands(ir.iter());

    //let (tx, rx) = channel::<DaemonRequest>();

    let thread_id = DPU::job_add(
        "ep".into(),
        None,
        &mut state,
        &mut assignment_queue,
        &mut multi_queue,
    );
    //let (wtx, wrx) = channel();

    let (master_tx, master_rx) = channel::<DaemonRequest>();
    //let (listener_tx, listener_rx) = channel::<ListenerRq>();

    // todo create a tcp stream here.

    // 1. client negotiates capacity
    // 2. client announces itself to the master
    // 3. client renounces themselves from the master

    let addr = "127.0.0.1:45000";
    let addr: SocketAddr = addr.parse().unwrap();

    let listener = TCPWorkerAdapter::new(
        &addr,
        master_tx.clone(),
    ).unwrap();

    let mut w = WorkerTcp::new(&addr).unwrap();

    w.header();

    for i in 0..100 {
        DPU::process_assignments(
            &mut state,
            &mut assignment_queue,
            &mut workers,
        );

        DPU::process_channel(
            &master_rx,
            &mut state,
            &mut assignment_queue,
            &mut workers,
            &mut multi_queue,
        );
        w.run();
        sleep(Duration::from_millis(1));
    }

    assert_eq!(
        state.threads.get(&thread_id).unwrap().state,
        ThreadState::Queued(
            XCmd::create("07".into(), "list_get".into(), vec![
                XCmdArg::Ref(XCtxRef(XCtxNs::Curr, "users".into()), Some("foo@bar.com,zeta@beta.org,culinary@sky.net".into())),
                XCmdArg::Ref(XCtxRef(XCtxNs::Curr, "i".into()), Some("0".into())),
                XCmdArg::Const("user_id".into()),
                XCmdArg::Const("08".into()),
            ]),
        ),
    );

}

