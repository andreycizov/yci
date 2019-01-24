use std::collections::{HashMap, VecDeque};
use crate::daemon::*;
use crate::obj::*;
use crate::pubsub::*;
use crate::tests::prog::*;
use crate::worker::*;
use std::sync::mpsc::channel;

struct W1 {}

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

    fn exec(&mut self, command: &InterpolatedCommand) -> WorkerResult {
        let nip = || {
            let next_ip = command.args.last();

            let next_ip = match next_ip {
                Some(x) => Ok(x),
                None => Err(
                    WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                )
            };

            next_ip
        };

        match command.opcode.value().as_ref() {
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
                            RValue::Local(RValueLocal::Const(next_ip.value())),
                        ),
                    ]
                )
            }
            "list_create" => {
                let var_name = command.args.first().ok_or(
                    WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                )?;

                let next_ip = nip()?;

                Ok(
                    vec![
                        Op::ContextSet(
                            var_name.value(),
                            RValueLocal::Const("".into()),
                        ),
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(next_ip.value())),
                        ),
                    ]
                )
            }
            "list_length" => {
                let var_val = command.args.first().ok_or(
                    WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                )?;
                let var_var = command.args.get(1).ok_or(
                    WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                )?;
                let cnt = var_val.value().split(",").count();
                let next_ip = nip()?;
                Ok(
                    vec![
                        Op::ContextSet(
                            var_var.value(),
                            RValueLocal::Const(cnt.to_string()),
                        ),
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(next_ip.value())),
                        ),
                    ])
            }
            "db_user_list" => {
                let var_name = command.args.first().ok_or(
                    WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                )?;

                let var_name = match var_name {
                    InterpolatedCommandArgument::Const(_) => Err(WorkerErr::Default(OpErrReason::InvalidArg(0))),
                    InterpolatedCommandArgument::Ref(ident, _) => Ok(ident)
                }?;

                let next_ip = nip()?;

                Ok(
                    vec![
                        Op::ContextSet(
                            var_name.clone(),
                            RValueLocal::Const("foo@bar.com,zeta@beta.org,culinary@sky.net".into()),
                        ),
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(next_ip.value())),
                        ),
                    ]
                )
            }
            "set" => {
                let mut iter = command.args.iter();

                let mut nip: Option<_> = None;
                let mut ret = Vec::<Op>::with_capacity(command.args.len() + 1 / 2);

                loop {
                    let a = iter.next().ok_or(
                        WorkerErr::Default(OpErrReason::ContextRefInvalid { ident: "".into() })
                    )?;

                    if let Some(b) = iter.next() {
                        ret.push(
                            Op::ContextSet(
                                a.value(),
                                RValueLocal::Const(b.value()),
                            )
                        )
                    } else {
                        nip = Some(a);
                        break;
                    }
                }

                ret.push(
                    Op::LocalSet(
                        LOCAL_NIP.into(),
                        RValue::Local(RValueLocal::Const(nip.unwrap().value())),
                    )
                );

                Ok(
                    ret
                )
            }
            "icmp" => {
                let mut iter = command.args.iter();

                let a = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(0))
                )?;

                let a = a.value().parse::<u128>().map_err(
                    |_| WorkerErr::Default(OpErrReason::InvalidArg(0))
                )?;

                let op = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(1))
                )?;

                let b = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(2))
                )?;

                let b = b.value().parse::<u128>().map_err(
                    |_| WorkerErr::Default(OpErrReason::InvalidArg(2))
                )?;

                let d = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(3))
                )?;

                let cmp_fn: fn(u128, u128) -> bool = match op.value().as_ref() {
                    "<" => |a, b| a < b,
                    ">" => |a, b| a > b,
                    "=" => |a, b| a > b,
                    _ => return Err(WorkerErr::Default(OpErrReason::InvalidArg(1)))
                };

                let res = cmp_fn(a,b).to_string();

                let next_ip = nip()?;

                Ok(
                    vec![
                        Op::ContextSet(
                            d.value().clone(),
                            RValueLocal::Const(res),
                        ),
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(next_ip.value())),
                        ),
                    ]
                )

            }
            "if" => {
                let mut iter = command.args.iter();
                let a = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(0))
                )?;

                let a = a.value().parse::<bool>().map_err(
                    |_| WorkerErr::Default(OpErrReason::InvalidArg(0))
                )?;

                let b = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(1))
                )?;

                let c = iter.next().ok_or(
                    WorkerErr::Default(OpErrReason::MissingArg(2))
                )?;

                let b = b.value();
                let c = c.value();

                Ok(
                    vec![
                        Op::LocalSet(
                            LOCAL_NIP.into(),
                            RValue::Local(RValueLocal::Const(if a { b } else {c})),
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

#[test]
fn test_worker_a() {


    let mut state = State::default();
    let mut assignment_queue = VecDeque::<Ass>::default();
    let mut multi_queue = MQ::default();
    let mut workers = HashMap::<WorkerId, &mut Worker>::default();


    let ir = LoadIRFile::new(TEST_ALGO);
    let ir = ir.load().unwrap();

    state.insert_commands(ir.iter());

    let thread_id = DPU::job_add(
        "ep".into(),
        None,
        &mut state,
        &mut assignment_queue,
        &mut multi_queue,
    );

    let mut wo = W1 {};

    let (tx, rx) = channel::<DaemonRequest>();


    DPU::worker_add(
        &"1".into(),
        &mut wo as &mut Worker,
        &mut workers,
        &mut multi_queue,
        &mut assignment_queue,
    );


    for i in 0..100 {
        DPU::process_assignments(
            tx.clone(),
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
    }

    assert_eq!(
        state.threads.get(&thread_id).unwrap().state,
        ThreadState::Queued(
            InterpolatedCommand::create("07".into(), InterpolatedCommandArgument::Const("list_get".into()), vec![
                InterpolatedCommandArgument::Ref("users".into(), "foo@bar.com,zeta@beta.org,culinary@sky.net".into()),
                InterpolatedCommandArgument::Ref("i".into(), "0".into()),
                InterpolatedCommandArgument::Const("user_id".into()),
                InterpolatedCommandArgument::Const("08".into()),
            ]),
        ),
    );
}
