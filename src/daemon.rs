use rand::prelude::*;

use std::collections::HashMap;
use std::collections::VecDeque;

use super::obj::*;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub(crate) struct Thread {
    // Identity
    pub(crate) id: ThreadId,
    // For every opcode executed, goes up by one
    pub(crate) step: StepId,
    // Instruction pointer
    pub(crate) ip: CommandId,
    // Context pointer
    pub(crate) ctx: ContextId,
    //
    pub(crate) state: ThreadState,

    // where to jump if exception occurs
    pub(crate) eip: Option<CommandId>,
    // which context to set if exception occurs
}

#[derive(Debug, Clone)]
pub enum ThreadError {
    Fetch { id: CommandId },
    Context { id: ContextId },
    Interpolate { err: String },
}

#[derive(Debug, Clone)]
pub(crate) enum ThreadState {
    Created,
    Fetching(CommandId),
    Fetched(Command),
    Interpolating(Command),
    Interpolated(InterpolatedCommand),
    Queued(InterpolatedCommand),
    // Running(InterpolatedCommand, LockId),
    Done,

    Error(ThreadError),

    // Waiting
    Paused,
    Exited(Result<(), ThreadError>),
}

struct DPU {
    commands: HashMap<CommandId, Command>,
    contexts: HashMap<ContextId, Context>,
    threads: HashMap<ThreadId, Thread>,

    queues: HashMap<ContextValue, VecDeque<ThreadId>>,

    queues_workers: HashMap<String, HashSet<WorkerId>>,
    workers: HashMap<WorkerId, Worker>,

    rng: ThreadRng,
}

#[derive(Debug, Clone)]
enum ExecOp {
    ContextCreate { id: ContextIdent },
    ContextCopy { id: ContextIdent, ident: ContextIdent, val: ContextIdent },
    ContextSet { ident: ContextIdent, val: ContextValue },
    ContextRemove { id: ContextIdent },
    ThreadCreate { id: ContextIdent, ip: ContextIdent, ctx: ContextIdent },
    ThreadRemove { id: ContextIdent },
    SetNIP { id: CommandId },
    SetContext { id: ContextId },
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

impl Thread {
    pub fn create(id: ThreadId, ip: CommandId, ctx: ContextId) -> Self {
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

            queues: HashMap::<ContextValue, VecDeque<ThreadId>>::default(),

            queues_workers: HashMap::<String, HashSet<WorkerId>>::default(),
            workers: HashMap::<WorkerId, Worker>::default(),

            rng: ThreadRng::default(),
        }
    }
}


impl DPU {
    pub fn put(&mut self) {}

    pub fn load(&mut self, commands: &Vec<Command>) {
        for command in commands {
            self.commands.insert(command.id, command.clone());
        }
    }

    fn proceed(&mut self, id: &ThreadId) {
        loop {
            let mut thread = match self.threads.get_mut(id) {
                Some(x) => x,
                None => return
            };

            match &thread.state {
                ThreadState::Created => {
                    thread.state = ThreadState::Fetching(thread.ip);
                }
                ThreadState::Done => {
                    thread.state = ThreadState::Fetching(thread.ip)
                }
                ThreadState::Fetching(ip) => {
                    match self.commands.get(&ip) {
                        Some(x) => {
                            thread.state = ThreadState::Fetched(x.clone());
                        }
                        None => {
                            thread.state = ThreadState::Error(ThreadError::Fetch { id: *ip })
                        }
                    }
                }
                ThreadState::Fetched(command) => {
                    thread.state = ThreadState::Interpolating(command.clone());
                }
                ThreadState::Interpolating(command) => {
                    match self.contexts.get(&thread.ctx) {
                        Some(ctx) => {
                            match command.interpolate(ctx) {
                                Ok(x) => {
                                    thread.state = ThreadState::Interpolated(x)
                                }
                                Err(x) => {
                                    thread.state = ThreadState::Error(ThreadError::Interpolate { err: x })
                                }
                            }
                        }
                        None => {
                            thread.state = ThreadState::Error(ThreadError::Context { id: thread.ctx.clone() })
                        }
                    }
                }
                ThreadState::Interpolated(command) => {
                    let queue: &mut VecDeque<ThreadId> = {
                        let queue_name = &command.opcode.value();

                        if !self.queues.contains_key(queue_name) {
                            self.queues.insert(queue_name.clone(), VecDeque::<ThreadId>::default());
                        }

                        match self.queues.get_mut(queue_name) {
                            Some(x) => x,
                            None => {
                                panic!("Should never happen")
                            }
                        }
                    };

                    queue.push_back(thread.id.clone());

                    thread.state = ThreadState::Queued(command.clone());
                }
//                ThreadState::Queued(command) => {
//                    command.opcode.value()
//                }
            };
        }
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
                ExecOp::ContextCreate { id: ident } => {
                    let id: u128 = self.rng.gen();

                    self.contexts.insert(id, Context::empty(id));
                    context.vals.insert(ident.clone(), id.to_string());
                }
                ExecOp::ContextCopy { id: ident, ident: name, val: value } => {
                    let ident = context_get(&ident)?;
                    let value = context_get(&value)?;

                    let ident_int = parse_id(&ident, ExecErrReason::ContextRefInvalid { ident: ident.clone() })?;

                    match self.contexts.get_mut(&ident_int) {
                        Some(x) => x.vals.insert(name.clone(), value),
                        None => return context_err(i, ExecErrReason::ContextDoesNotExist { id: ident_int })
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

                    let ip = parse_id(&ip, ExecErrReason::CommandRefInvalid { ident: ip.clone() })?;
                    let ctx = parse_id(&ctx, ExecErrReason::ContextRefInvalid { ident: ctx.clone() })?;

                    self.threads.insert(id, Thread::create(id, ip, ctx));
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

        self.threads.insert(thread.id, thread);
        self.contexts.insert(context.id, context);

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