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
                    InterpolatedCommandArgument::Const(_) => Err(WorkerErr::Default(OpErrReason::InvalidArg {idx:0})),
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

    let (tx, rx) = channel::<DPUComm>();

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

    DPU::worker_add(
        1,
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
            &mut workers
        );

        DPU::process_channel(
            &rx,
            &mut state,
            &mut assignment_queue,
            &mut multi_queue
        );
    }


//    let z = dbg!(z);

    assert_eq!(
        state.threads.get(&thread_id).unwrap().state, ThreadState::Assigned(
            InterpolatedCommand::create("01".into(), InterpolatedCommandArgument::Const("02".into()), vec![]),
            005,
        ),
    );
//    assert_eq!(z, vec![]);
//    assert_eq!(z.len(), 1);
}
