use rand::prelude::*;

use std::collections::HashMap;

use super::obj::*;
use super::pubsub::*;

#[derive(Debug, Clone)]
pub(crate) struct Thread {
    // Identity
    pub(crate) id: ThreadId,
    // For every opcode executed, goes up by one
    pub(crate) step: StepId,
    // Instruction pointer
    pub(crate) ip: CommandId,
    // Context pointer
    pub(crate) ctx: Option<ContextId>,
    //
    pub(crate) state: ThreadState,

    // where to jump if exception occurs
    pub(crate) eip: Option<CommandId>,
    // which context to set if exception occurs
}

#[derive(Debug, Clone)]
pub enum ThreadError {
    Fetch { id: CommandId },
    Context { id: Option<ContextId> },
    Interpolate { err: InterpolationError },
}

#[derive(Debug, Clone)]
pub(crate) enum ThreadState {
    Created,
    Fetching(CommandId),
    Fetched(Command),
    Interpolating(Command),
    Interpolated(InterpolatedCommand),
    Queued(InterpolatedCommand),
    Assigned(InterpolatedCommand, WorkerId),
    // Running(InterpolatedCommand, LockId),
    Done,

    Err(ThreadError),

    // Waiting
    Paused,
    Exited(Result<(), ThreadError>),
}

pub struct DPU {
    commands: HashMap<CommandId, Command>,
    contexts: HashMap<ContextId, Context>,
    threads: HashMap<ThreadId, Thread>,

    //queues: HashMap<ContextValue, VecDeque<ThreadId>>,

    multi_queue: MultiQueue<WorkerId, ContextValue, ThreadId>,
    workers: HashMap<WorkerId, Worker>,

    rng: ThreadRng,
}

#[derive(Debug, Clone)]
pub enum ROp {
    ContextCreate,
    ThreadCreate,
    Ref(ContextIdent),
    Const(ContextValue),
}

#[derive(Debug, Clone)]
pub enum ExecOp2 {
    LocalSet(ContextIdent, ROp),
    ContextSet(ContextIdent, ROp),

    ContextCopy(ROp, ROp, ROp),
    ContextRemove(ROp),

    ThreadRemove(ROp),

    ThreadContextSet(ROp),
    ThredNipSet(ROp),
}

#[derive(Debug, Clone)]
pub enum ExecOp {
    ContextCreate { id: ContextIdent }, //
    ContextCopy { id: ContextIdent, ident: ContextIdent, val: ContextIdent },
    ContextSet { ident: ContextIdent, val: ContextValue }, //
    ContextRemove { id: ContextIdent },
    ThreadCreate { id: ContextIdent, ip: ContextIdent, ctx: ContextIdent },
    ThreadRemove { id: ContextIdent },
    SetNIP { id: CommandId },
    SetContext { id: Option<ContextId> },
}

#[derive(Debug, Clone)]
pub enum ExecErrReason {
    ContextDoesNotExist { id: ContextId },
    ThreadDoesNotExist { id: ThreadId },
    ContextRefInvalid { ident: ContextValue },
    ThreadRefInvalid { ident: ContextValue },
    CommandRefInvalid { ident: ContextValue },
    PostStepped { current: StepId, selected: StepId },
    UnknownOp,
}

#[derive(Debug, Clone)]
pub struct ExecErr {
    op_index: Option<usize>,
    op_reason: ExecErrReason,
}

pub enum WorkerExec {
    // what worker returns when executed.

}

impl Thread {
    pub fn create(id: ThreadId, ip: CommandId, ctx: Option<ContextId>) -> Self {
        Thread {
            id: id,
            step: 0,
            ip: ip,
            ctx: ctx,
            state: ThreadState::Created,
            eip: None,
        }
    }
}

impl Default for DPU {
    fn default() -> Self {
        DPU {
            commands: HashMap::<CommandId, Command>::default(),
            contexts: HashMap::<ContextId, Context>::default(),
            threads: HashMap::<ThreadId, Thread>::default(),

            //queues: HashMap::<ContextValue, VecDeque<ThreadId>>::default(),

            multi_queue: MultiQueue::<WorkerId, ContextValue, ThreadId>::default(),
            workers: HashMap::<WorkerId, Worker>::default(),

            rng: ThreadRng::default(),
        }
    }
}


impl DPU {
    pub fn put_thread(&mut self, ip: CommandId, ctx: Option<ContextId>) -> ThreadId {
        let id = self.id_create();

        let thread = Thread::create(id, ip, ctx);

        self.threads.insert(id, thread);

        self.proceed(&id);

        id
    }

    pub fn put_context(&mut self, vals: Option<HashMap<ContextIdent, ContextValue>>) -> ContextId {
        let id = self.id_create();
        let vals = vals.unwrap_or_else(|| {
            HashMap::<ContextIdent, ContextValue>::default()
        });
        let context = Context::create(id, vals);

        self.contexts.insert(id, context);

        id
    }

    pub fn load(&mut self, commands: &Vec<Command>) {
        for command in commands {
            self.commands.insert(command.id.clone(), command.clone());
        }
    }

    fn proceed(&mut self, id: &ThreadId) {
        loop {
            // whenever we re-store

            let mut thread = match self.threads.get(id) {
                Some(x) => x.clone(),
                None => return
            };

            let new_state: Option<ThreadState> = match &thread.state {
                ThreadState::Created => {
                    Some(ThreadState::Fetching(thread.ip.clone()))
                }
                ThreadState::Done => {
                    thread.step += 1;
                    Some(ThreadState::Fetching(thread.ip.clone()))
                }
                ThreadState::Fetching(ip) => {
                    match self.commands.get(ip) {
                        Some(x) => {
                            Some(ThreadState::Fetched(x.clone()))
                        }
                        None => {
                            Some(ThreadState::Err(ThreadError::Fetch { id: ip.clone() }))
                        }
                    }
                }
                ThreadState::Fetched(command) => {
                    Some(ThreadState::Interpolating(command.clone()))
                }
                ThreadState::Interpolating(command) => {
                    let mut ctx: Option<&Context> = None;

                    if let Some(x) = &thread.ctx {
                            match self.contexts.get(x) {
                                Some(fetched_ctx) => {
                                    ctx = Some(fetched_ctx);
                                }
                                None => {
                                    Some(ThreadState::Err(ThreadError::Context { id: thread.ctx.clone() }))
                                }
                            }
                    }

                    match command.interpolate(ctx) {
                        Ok(x) => {
                            Some(ThreadState::Interpolated(x))
                        }
                        Err(x) => {
                            Some(ThreadState::Err(ThreadError::Interpolate { err: x }))
                        }
                    }
                }
                ThreadState::Interpolated(command) => {
                    let assignment = self.multi_queue.job_create(&command.opcode.value(), &thread.id);

                    match assignment.first() {
                        Some(x) => {
                            Some(ThreadState::Assigned(command.clone(), x.worker_key))
                        },
                        None => {
                            Some(ThreadState::Queued(command.clone()))
                        }
                    }
                }
                ThreadState::Queued(command) => {
                    None
                }
                ThreadState::Assigned(command, worker_id) => {
                    None
                }
                ThreadState::Paused => {
                    None
                }
                ThreadState::Err(error) => {
                    match thread.eip {
                        Some(eip) => {
                            thread.ip = eip;
                            thread.eip = None;
                            // todo ... we need to somehow pass the error to the thread back
                            // todo should it go to the context or should it be handled as part of the
                            // todo thread object?

                            // todo should it instead go into a separate context that may later on be
                            // todo disposed of by the thread?
                            // todo this way threads can easily copy the exception back anywhere.

                            let id = self.context_create();
                            let err_str = format!("{:?}", error);

                            self.context_mut(&id).unwrap().vals.insert(
                                "exc".to_string(),
                                err_str
                            );
                            Some(ThreadState::Done)
                        }
                        None => {
                            Some(ThreadState::Exited(Err(error.clone())))
                        }
                    }
                }
                ThreadState::Exited(res) => {
                    match res {
                        Ok(_) => {
                            None
                        }
                        Err(err) => {
                            None
                        }
                    }
                }
            };

            let mut should_break = false;

            if let Some(state) = new_state {
                thread.state = state;
            } else {
                should_break = true;
            }

            self.threads.insert(id.clone(), thread);

            if should_break {
                return
            }
        }
    }

    fn id_create(&mut self) -> u128{
        self.rng.gen()
    }

    fn context_create(&mut self) -> ContextId {
        let id = self.id_create();

        self.contexts.insert(id, Context::empty(id));

        id
    }

    fn context_mut(&mut self, key: &ContextId) -> Option<&mut Context> {
        self.contexts.get_mut(key)
    }

    fn exec(&mut self, id: &ThreadId, ops: &Vec<ExecOp>) -> Result<(), ExecErr> {
        let mut thread = match self.threads.get(id) {
            Some(x) => x,
            None => return Err(ExecErr { op_index: None, op_reason: ExecErrReason::ThreadDoesNotExist { id: *id } })
        }.clone();

        let mut context = match self.contexts.get(&thread.ctx) {
            Some(x) => x,
            None => return Err(ExecErr { op_index: None, op_reason: ExecErrReason::ContextDoesNotExist { id: thread.ctx } })
        }.clone();

        for (i, op) in ops.iter().enumerate() {
            fn context_err<A>(idx: usize, reason: ExecErrReason) -> Result<A, ExecErr> {
                Err(ExecErr { op_index: Some(idx), op_reason: reason })
            }

            let context_get = |ident| {
                match context.get(ident) {
                    Some(x) => Ok(x),
                    None => context_err(i, ExecErrReason::ContextRefInvalid { ident: ident.clone() })
                }
            };

            let parse_id = |id_str: &String, err: ExecErrReason| {
                match id_str.parse::<u128>() {
                    Ok(x) => Ok(x),
                    Err(_) => context_err(i, err)
                }
            };


            match op {
                // TODO: /////////////////////////////////////////////
                // can we only allow the contexts to be created in a certain way?
                // e.g. by copying the context from A -> B but not otherwise
                // [if that is the case, then we will be unable to write proper algorithms]
                // [with

                // the issue is that all of these operations are defined against the context and do not support
                // contextless operation (Excluding SetNIP, SetContext)
                // thus, we can not create a context without having a thread context
                // todo: /////////////////////////////////////////////


                ExecOp::ContextCreate { id: ident } => {
                    let id = self.context_create();
                    context.vals.insert(ident.clone(), id.to_string());
                }
                ExecOp::ContextCopy { id: ident, ident: name, val: value } => {
                    let ident = context_get(&ident)?;
                    let value = context_get(&value)?;

                    let ident_int = parse_id(&ident, ExecErrReason::ContextRefInvalid { ident: ident.clone() })?;

                    if let Some(x) = self.contexts.get_mut(&ident_int) {
                        x.vals.insert(name.clone(), value);
                    } else {
                        return context_err(i, ExecErrReason::ContextDoesNotExist { id: ident_int })
                    };

                }
                ExecOp::ContextSet { ident, val } => {
                    context.vals.insert(ident.clone(), val.clone());
                }
                ExecOp::ContextRemove { id: ident } => {
                    let ident = parse_id(&context_get(ident)?, ExecErrReason::ContextRefInvalid { ident: ident.clone() })?;

                    match self.contexts.remove(&ident) {
                        Some(_) => {}
                        None => return context_err(i, ExecErrReason::ContextDoesNotExist { id: ident.clone() })
                    }
                }
                ExecOp::ThreadCreate { id, ip, ctx } => {
                    let id: u128 = self.rng.gen();

                    let ip = context_get(&ip)?;
                    let ctx = context_get(&ip)?;

                    //let ip = parse_id(&ip, ExecErrReason::CommandRefInvalid { ident: ip.clone() })?;
                    let ctx = parse_id(&ctx, ExecErrReason::ContextRefInvalid { ident: ctx.clone() })?;

                    self.threads.insert(id, Thread::create(id, ip, Some(ctx)));
                }
                ExecOp::ThreadRemove { id: ident } => {
                    let ident = parse_id(&context_get(ident)?, ExecErrReason::ThreadRefInvalid { ident: ident.clone() })?;

                    match self.contexts.remove(&ident) {
                        Some(_) => {}
                        None => return context_err(i, ExecErrReason::ThreadDoesNotExist { id: ident.clone() })
                    }
                }
                ExecOp::SetNIP { id } => {
                    thread.ip = id.clone()
                }
                ExecOp::SetContext { id } => {
                    thread.ctx = id.clone()
                }
                //_ => return context_err(i, ExecErrReason::UnknownOp)
            }
        }

        thread.step.wrapping_add(1);

        let thread_id = thread.id.clone();

        self.threads.insert(thread.id, thread);
        self.contexts.insert(context.id, context);

        self.proceed(&thread_id);

        Ok(())
    }

    pub fn done(&mut self, id: &ThreadId, step: StepId, ops: &Vec<ExecOp>) -> Result<(), ExecErr> {
        if let Some(x) = self.threads.get(id) {
            if x.step == step {
                return Ok(self.exec(id, ops)?);
            } else {
                return Err(ExecErr { op_index: None, op_reason: ExecErrReason::PostStepped { current: x.step, selected: step } });
            }
        } else {
            return Err(ExecErr { op_index: None, op_reason: ExecErrReason::ThreadDoesNotExist { id: *id } });
        };
    }
}