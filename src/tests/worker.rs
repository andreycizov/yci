use std::collections::{HashMap, VecDeque};
use crate::daemon::*;
use crate::obj::*;
use crate::tests::prog::*;
use crate::worker::*;
use mio_extras::channel::{Sender, Receiver, channel};

struct W1 {
    rx: Receiver<DaemonWorker>,
    tx: Sender<DaemonWorker>,
    rep: Sender<DaemonRequest>,
    wid: Option<WorkerId>,
}

pub trait FirstExecutor {
    fn exec(&mut self, command: &XCmd) -> WorkerResult {
        let nip = || {
            let next_ip = command.args.last();

            let next_ip = match next_ip {
                Some(x) => match x.value() {
                    Some(y) => Ok(y),
                    None => Err(
                        WorkerErr::Default(OpErrReason::InvalidArg(command.args.len() - 1))
                    )
                }
                None => Err(
                    WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                )
            };

            next_ip
        };

        match command.opcode.as_ref() {
            "push" => {
                let next_ip = nip()?;

                Ok(
                    vec![
                        Op::LocalSet(
                            "new_ctx".into(),
                            RValue::Extern(RValueExtern::ContextCreate),
                        ),
                        Op::LocalSet(
                            LOCAL_CTX.into(),
                            RValue::Local(RValueLocal::Ref("new_ctx".into())),
                        ),
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(next_ip)),
                        ),
                    ]
                )
            }
            "list_create" => {
                let var_name = command.args.first().ok_or(
                    WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                ).and_then(|a| a.ident().ok_or_else(
                    || WorkerErr::Default(OpErrReason::InvalidArg(0))
                ))?;

                let next_ip = nip()?;

                Ok(
                    vec![
                        var_name.set(
                            RValueLocal::Const("".into()),
                        ),
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(next_ip)),
                        ),
                    ]
                )
            }
            "list_length" => {
                let var_val = command.args.first().ok_or(
                    WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                )?.value().ok_or(
                    WorkerErr::Default(OpErrReason::InvalidArg(0))
                )?;

                let var_var = command.args.get(1).ok_or(
                    WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                )?.ident().ok_or(
                    WorkerErr::Default(OpErrReason::InvalidArg(0))
                )?;

                let cnt = var_val.split(",").count();
                let next_ip = nip()?;
                Ok(
                    vec![
                        var_var.set(RValueLocal::Const(cnt.to_string())),
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(next_ip)),
                        ),
                    ])
            }
            "db_user_list" => {
                let var_name = command.args.first().ok_or(
                    WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                )?;

                let var_name = var_name.ident().ok_or(
                    WorkerErr::Default(OpErrReason::InvalidArg(0))
                )?;

                let next_ip = nip()?;

                Ok(
                    vec![
                        var_name.set(RValueLocal::Const("foo@bar.com,zeta@beta.org,culinary@sky.net".into())),
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(next_ip)),
                        ),
                    ]
                )
            }
            "set" => {
                let mut iter = command.args.iter();

                let mut ret = Vec::<Op>::with_capacity(command.args.len() + 1 / 2);


                let mut argidx = 0;

                loop {
                    let a = iter.next().ok_or(
                        WorkerErr::Default(OpErrReason::InvalidArg(argidx))
                    )?;

                    if let Some(b) = iter.next() {
                        let a = a.ident().ok_or(
                            WorkerErr::Default(OpErrReason::InvalidArg(argidx))
                        )?;

                        argidx += 1;

                        let b = b.value().ok_or(
                            WorkerErr::Default(OpErrReason::InvalidArg(argidx))
                        )?;

                        ret.push(
                            a.set(
                                RValueLocal::Const(b),
                            )
                        )
                    } else {
                        let nip = a.value().ok_or(
                            WorkerErr::Default(OpErrReason::InvalidArg(argidx))
                        )?;
                        ret.push(
                            Op::LocalSet(
                                LOCAL_NIP.into(),
                                RValue::Local(RValueLocal::Const(nip)),
                            )
                        );
                        break;
                    }
                }


                Ok(
                    ret
                )
            }
            "icmp" => {
                let mut iter = command.args.iter();

                let a = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(0))
                ).and_then(|a| a.value().ok_or_else(
                    || WorkerErr::Default(OpErrReason::InvalidArg(0))
                ))?;

                let a = a.parse::<u128>().map_err(
                    |_| WorkerErr::Default(OpErrReason::InvalidArg(0))
                )?;

                let op = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(1))
                )?.value().ok_or_else(
                    || WorkerErr::Default(OpErrReason::InvalidArg(1))
                )?;

                let b = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(2))
                ).and_then(|a| a.value().ok_or_else(
                    || WorkerErr::Default(OpErrReason::InvalidArg(2))
                ))?;

                let b = b.parse::<u128>().map_err(
                    |_| WorkerErr::Default(OpErrReason::InvalidArg(2))
                )?;

                let ctxref = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(3))
                )?.ident().ok_or_else(
                    || WorkerErr::Default(OpErrReason::InvalidArg(3))
                )?;

                let cmp_fn: fn(u128, u128) -> bool = match op.as_ref() {
                    "<" => |a, b| a < b,
                    ">" => |a, b| a > b,
                    "=" => |a, b| a > b,
                    _ => return Err(WorkerErr::Default(OpErrReason::InvalidArg(1)))
                };

                let res = cmp_fn(a, b).to_string();

                let next_ip = nip()?;

                Ok(
                    vec![
                        ctxref.set(RValueLocal::Const(res)),
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(next_ip)),
                        ),
                    ]
                )
            }
            "if" => {
                let mut iter = command.args.iter();
                let a = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(0))
                ).and_then(|a| a.value().ok_or_else(
                    || WorkerErr::Default(OpErrReason::InvalidArg(0))
                ))?;

                let a = a.parse::<bool>().map_err(
                    |_| WorkerErr::Default(OpErrReason::InvalidArg(0))
                )?;

                let b = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(1))
                )?;

                let c = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(2))
                )?;

                let b = b.value().ok_or_else(
                    || WorkerErr::Default(OpErrReason::InvalidArg(1))
                )?;
                let c = c.value().ok_or_else(
                    || WorkerErr::Default(OpErrReason::InvalidArg(2))
                )?;

                Ok(
                    vec![
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(if a { b } else { c })),
                        ),
                    ]
                )
            }
            _ => {
                panic!("Unknown opcode")
            }
        }
    }


}

impl FirstExecutor for W1 {

}

impl Worker for W1 {
    fn capacity(&self) -> Option<usize> {
        None
    }

    fn queues(&self) -> Vec<ContextValue> {
        vec![
            "push".into(),
            "list_create".into(),
            "list_length".into(),
            "db_user_list".into(),
            "set".into(),
            "icmp".into(),
            "if".into(),
        ]
    }

    fn put(&mut self, command: &XCmd, result_cb: WorkerReplier) {
        result_cb.clone().reply(self.exec(command))
    }


}

impl W1  {
    fn run(&mut self) -> usize {
        let mut i = 0;
        while let Ok(x) = self.rx.try_recv() {
            println!("{:?}", x);
            match x {
                DaemonWorker::WorkerCreated(wid) => {
                    self.wid = Some(dbg!(wid));
                }
                DaemonWorker::JobAssigned(tid, sid, cid, cmd) => {
                    let ret = self.exec(&cmd);

                    self.rep.send(DaemonRequest::Finished(self.wid.clone().unwrap(), tid, sid, cid, ret));
                    i += 1;
                }
            }
        }
        i
    }
}

#[test]
fn test_worker_a() {
    let mut state = State::default();
    let mut assignment_queue = VecDeque::<Ass>::default();
    let mut multi_queue = MQ::default();
    let mut workers = WS::default();


    let ir = LoadIRFile::new(TEST_ALGO);
    let ir = ir.load().unwrap();

    state.insert_commands(ir.iter());

    let (tx, rx) = channel::<DaemonRequest>();

    let thread_id = DPU::job_add(
        "ep".into(),
        None,
        &mut state,
        &mut assignment_queue,
        &mut multi_queue,
    );
    let (wtx, wrx) = channel();

    let mut wo = W1 {
        rx: wrx,
        tx: wtx,
        rep: tx.clone(),
        wid: None,
    };

    DPU::worker_add(
        &"1".into(),
        &WorkerInfo(
            wo.capacity(),
            wo.queues(),
        ),
        &wo.tx,
        &mut workers,
        &mut multi_queue,
        &mut assignment_queue,
    );


    for i in 0..100 {
        DPU::process_assignments(
            &mut state,
            &mut assignment_queue,
            &mut workers,
        );

        DPU::process_channel(
            &rx,
            &mut state,
            &mut assignment_queue,
            &mut workers,
            &mut multi_queue,
        );
        for i in 0..100 {
            wo.run();
        }
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
