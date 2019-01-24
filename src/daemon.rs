use rand::prelude::*;

use std::collections::HashMap;

use super::obj::*;
use super::pubsub::*;
use super::worker::*;
use std::collections::VecDeque;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;

#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
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

#[derive(Debug, Clone, PartialEq)]
pub enum ThreadError {
    Fetch { id: CommandId },
    Context { id: Option<ContextId> },
    Interpolate { err: InterpolationError },

    WorkerDuring(WorkerErr),
    WorkerPost(OpErr),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ThreadState {
    Created,
    Fetching(CommandId),
    Fetched(Command),
    Interpolating(Command),
    Interpolated(InterpolatedCommand),
    Queued(InterpolatedCommand),
    Assigned(InterpolatedCommand, WorkerId),
    // Running(InterpolatedCommand, LockId),
    Done(WorkerResult),

    Err(ThreadError),

    // Waiting
    Paused,
    Exited(Result<(), ThreadError>),
}

pub struct State {
    commands: HashMap<CommandId, Command>,
    contexts: HashMap<ContextId, Context>,
    pub(crate) threads: HashMap<ThreadId, Thread>,

    rng: ThreadRng,
}

impl State {
    pub fn create_id(&mut self) -> GenId {
        let val = self.rng.gen::<u128>();

        let val = format!("{:X}", val);

        val
    }

    pub fn insert_thread(&mut self, thread: Thread) {
        self.threads.insert(thread.id.clone(), thread);
    }

    pub fn insert_context(&mut self, context: &Context) {
        self.contexts.insert(context.id.clone(), context.clone());
    }

    pub fn insert_commands<'a, I>(&mut self, commands: I)
        where I: Iterator<Item=&'a Command>, {
        for command in commands {
            self.commands.insert(command.id.clone(), command.clone());
        }
    }
}

impl Default for State {
    fn default() -> Self {
        State {
            commands: HashMap::<CommandId, Command>::default(),
            contexts: HashMap::<ContextId, Context>::default(),
            threads: HashMap::<ThreadId, Thread>::default(),
            rng: ThreadRng::default(),
        }
    }
}

pub struct DPU<'a> {
    state: State,

    multi_queue: MQ,
    workers: HashMap<WorkerId, &'a mut Worker>,
    assignment_queue: VecDeque<Ass>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RValueLocal {
    Ref(ContextIdent),
    Const(ContextValue),
}

impl RValueLocal {
    pub fn resolve(
        &self,
        locals: &HashMap<ContextIdent, ContextValue>,
    ) -> Result<ContextValue, OpErrReason> {
        match self {
            RValueLocal::Const(val) => Ok(val.clone()),
            RValueLocal::Ref(ident) => match locals.get(ident) {
                Some(val) => Ok(val.clone()),
                None => Err(OpErrReason::LocalRefInvalid { ident: ident.clone() })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RValueExtern {
    ContextCreate,
    ThreadCreate(RValueLocal, Option<RValueLocal>),
}

impl RValueExtern {
    pub fn resolve(
        &self,
        locals: &HashMap<ContextIdent, ContextValue>,
        state: &mut State,
    ) -> Result<ContextValue, OpErrReason> {
        match self {
            RValueExtern::ContextCreate => {
                let id: ContextId = state.create_id().to_string();
                state.insert_context(&Context::empty(id.clone()));
                Ok(ContextValue::from(id))
            }
            RValueExtern::ThreadCreate(ip, ctx) => {
                let ip = ip.resolve(locals)?;

                let ctx: Option<String> = match ctx {
                    Some(ctx) => Some(ctx.resolve(locals)?),
                    None => None
                };

                let id: ThreadId = state.create_id().to_string();
                state.insert_thread(Thread::create(id.clone(), ip, ctx));

                Ok(ContextValue::from(id))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RValue {
    Local(RValueLocal),
    Extern(RValueExtern),
}

impl RValue {
    pub fn resolve(
        &self,
        locals: &HashMap<ContextIdent, ContextValue>,
        state: &mut State,
    ) -> Result<ContextValue, OpErrReason> {
        match self {
            RValue::Local(x) => x.resolve(locals),
            RValue::Extern(x) => x.resolve(locals, state),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    LocalSet(ContextIdent, RValue),

    ContextSet(ContextIdent, RValueLocal),
    ContextCopy(RValueLocal, RValueLocal, RValueLocal),
    ContextRemove(RValueLocal),

    ThreadRemove(RValueLocal),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpErrReason {
    ContextDoesNotExist { id: ContextId },
    ThreadDoesNotExist { id: ThreadId },
    LocalRefInvalid { ident: ContextIdent },
    ContextRefInvalid { ident: ContextValue },
    InvalidArg(usize),
    MissingArg(usize),
    ThreadRefInvalid { ident: ContextValue },
    CommandRefInvalid { ident: ContextValue },
    PostStepped { current: StepId, selected: StepId },
    UnknownOp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpErr {
    op_index: Option<usize>,
    op_reason: OpErrReason,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerErr {
    Custom(HashMap<String, String>),
    Default(OpErrReason),
}

pub type WorkerResult = Result<Vec<Op>, WorkerErr>;

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

impl<'a> Default for DPU<'a> {
    fn default() -> Self {
        DPU {
            state: State::default(),
            multi_queue: MQ::default(),
            workers: HashMap::<WorkerId, &'a mut Worker>::default(),

            assignment_queue: VecDeque::<Ass>::default(),
        }
    }
}


pub static LOCAL_TID: &str = "$tid";
pub static LOCAL_NIP: &str = "$nip";
pub static LOCAL_EIP: &str = "$eip";
pub static LOCAL_CTX: &str = "$ctx";
pub static LOCAL_PAR_CTX: &str = "^ctx";
pub static LOCAL_PAR_IP: &str = "^ip";

pub(crate) type MQ = MultiQueue<WorkerId, ContextValue, (ThreadId, StepId)>;
pub(crate) type Ass = Assignment<WorkerId, ContextValue, (ThreadId, StepId)>;

pub enum DPUComm {
    Finished(WorkerId, CommandId, ThreadId, StepId, WorkerResult),
}

impl<'a> DPU<'a> {
    pub fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub(crate) fn worker_add(
        key: WorkerId,
        worker: &'a mut Worker,
        workers: &mut HashMap<WorkerId, &'a mut Worker>,
        multi_queue: &mut MQ,
        assignment_queue: &mut VecDeque<Ass>,
    ) {
        let capa = worker.capacity().clone();
        let queues = worker.queues().clone();
        workers.insert(key.clone(), worker);
        for a in multi_queue.worker_add(
            key,
            capa,
            queues,
        ) {
            assignment_queue.push_back(a)
        }
    }

    pub(crate) fn job_add(
        ep: CommandId,
        ctx: Option<ContextId>,
        state: &mut State,
        assignment_queue: &mut VecDeque<Ass>,
        multi_queue: &mut MQ,
    ) -> ThreadId {
        let id = state.create_id();

        let thread = Thread::create(
            id.clone(),
            ep,
            ctx,
        );

        state.insert_thread(
            thread
        );

        DPU::proceed(
            &id,
            state,
            assignment_queue,
            multi_queue,
        );

        id.clone()
    }

    pub(crate) fn process_channel(
        receiver: &Receiver<DPUComm>,
        state: &mut State,
        assignment_queue: &mut VecDeque<Ass>,
        multi_queue: &mut MQ,
    ) {
        while let Ok(pkt) = receiver.try_recv() {
            match pkt {
                DPUComm::Finished(wid, queue_id, thread_id, step_id, res) => {
                    let thread = state.threads.get_mut(&thread_id).unwrap();

                    assert_eq!(step_id, thread.step);

                    thread.state = ThreadState::Done(res);

                    multi_queue.job_finish(&queue_id, &(thread_id.clone(), step_id));

                    DPU::proceed(
                        &thread_id,
                        state,
                        assignment_queue,
                        multi_queue,
                    )
                }
            }
        }
    }

    pub(crate) fn process_assignments(
        sender: Sender<DPUComm>,
        state: &mut State,
        assignment_queue: &mut VecDeque<Ass>,
        workers: &mut HashMap<WorkerId, &'a mut Worker>,
    ) {
        let drained = assignment_queue.drain(..);

        for ass in drained {
            assert_eq!(ass.action, Action::Started);

            let (thread_id, step_id) = ass.job_key;

            let worker = workers.get_mut(&ass.worker_key).unwrap();
            let thread = state.threads.get_mut(&thread_id).unwrap();

            let command = match &thread.state {
                ThreadState::Queued(cmd) => cmd,
                _ => panic!("{:?}", thread.state)
            };

            // there is a situation where the thread was woken up by the

            assert_eq!(step_id, thread.step);

            worker.put(command, WorkerReplier::new(ass.worker_key, ass.queue_key, thread_id, step_id, sender.clone()))
        }
    }

    pub(crate) fn proceed(
        thread_id: &ThreadId,
        state: &mut State,
        assignment_queue: &mut VecDeque<Ass>,
        multi_queue: &mut MQ,
    ) {
        let mut thread = state.threads.get(thread_id).unwrap().clone();

        loop {
            let new_state: Option<ThreadState> = match &thread.state {
                ThreadState::Created => {
                    Some(ThreadState::Fetching(thread.ip.clone()))
                }
                ThreadState::Done(res) => {
                    let res = res.clone();
                    let res =
                        res.map_err(|res| ThreadError::WorkerDuring(res.clone()));
                    let res =
                        res.and_then(
                            |res|
                                DPU::exec(&mut thread, state, &res).map_err(
                                    |err| ThreadError::WorkerPost(err)
                                )
                        );

                    match res {
                        Ok(_) => {
                            Some(ThreadState::Fetching(thread.ip.clone()))
                        }
                        Err(err) => {
                            Some(ThreadState::Err(err))
                        }
                    }
                }
                ThreadState::Fetching(ip) => {
                    match state.commands.get(ip) {
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
                    let ctx = (thread.ctx.clone()).and_then(|x| state.contexts.get(&x));

                    // we ignore the case where the ContextId is nonexistent, but the command never
                    // accesses the context

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
                    thread.step = thread.step.wrapping_add(1);

                    let assignment = multi_queue.job_create(&command.opcode.value(), &(thread.id.clone(), thread.step));

                    for val in assignment {
                        assignment_queue.push_back(val);
                    }


//                    match assignment.first() {
//                        Some(x) => {
//                            Some(ThreadState::Assigned(command.clone(), x.worker_key))
//                        }
//                        None => {
//                            Some(ThreadState::Queued(command.clone()))
//                        }
//                    }

                    Some(ThreadState::Queued(command.clone()))
                }
                ThreadState::Queued(command) => {
                    None
                }
                ThreadState::Assigned(command, worker_id) => {
                    // todo notify the relevant worker_id
                    None
                }
                ThreadState::Paused => {
                    None
                }
                ThreadState::Err(error) => {
                    match &thread.eip {
                        Some(eip) => {
                            let err_str = format!("{:?}", error);

                            Some(ThreadState::Done(Ok(vec![
                                Op::LocalSet(
                                    "exc".into(),
                                    // todo serialize exception value into json string
                                    RValue::Local(RValueLocal::Const(err_str)),
                                ),
                                Op::LocalSet(
                                    "new_ctx".into(),
                                    RValue::Extern(RValueExtern::ContextCreate),
                                ),
                                Op::ContextCopy(
                                    RValueLocal::Ref("new_ctx".into()),
                                    RValueLocal::Const(LOCAL_PAR_CTX.into()),
                                    RValueLocal::Ref(LOCAL_CTX.into()),
                                ),
                                Op::ContextCopy(
                                    RValueLocal::Ref("new_ctx".into()),
                                    RValueLocal::Const(LOCAL_PAR_IP.into()),
                                    RValueLocal::Ref(LOCAL_NIP.into()),
                                ),
                                Op::LocalSet(
                                    LOCAL_CTX.into(),
                                    RValue::Local(RValueLocal::Ref("new_ctx".into())),
                                ),
                                Op::LocalSet(
                                    LOCAL_NIP.into(),
                                    RValue::Local(RValueLocal::Const(eip.clone())),
                                ),
                                Op::LocalSet(
                                    LOCAL_EIP.into(),
                                    RValue::Local(RValueLocal::Const("".into())),
                                )
                            ])))
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
                            // thread had reached the exception stack
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

            if should_break {
                break;
            }
        }

        state.threads.insert(thread_id.clone(), thread);
    }

    fn exec(
        thread: &mut Thread,
        state: &mut State,
        ops: &Vec<Op>,
    ) -> Result<(), OpErr> {
        let mut locals = HashMap::<ContextIdent, ContextValue>::default();

        locals.insert(LOCAL_NIP.to_string(), thread.ip.clone());
        locals.insert(LOCAL_EIP.to_string(), thread.eip.clone().unwrap_or("".to_string()));
        locals.insert(LOCAL_CTX.to_string(), thread.ctx.clone().unwrap_or("".to_string()));

        for (op_index, op) in ops.iter().enumerate() {
            let map_err_fn = |op_reason| OpErr { op_index: Some(op_index), op_reason };

            match op {
                Op::LocalSet(loc_ident, rval) => {
                    locals.insert(
                        loc_ident.clone(),
                        rval.resolve(&locals, state).map_err(map_err_fn)?,
                    );
                }
                Op::ContextSet(loc_ident, rval) => {
                    let ctx_ident = RValueLocal::Ref(LOCAL_CTX.into()).resolve(&locals).map_err(map_err_fn)?;

                    let mut ctx = state.contexts.get_mut(&ctx_ident).ok_or(
                        map_err_fn(OpErrReason::ContextDoesNotExist { id: ctx_ident })
                    )?;

                    ctx.vals.insert(
                        loc_ident.clone(),
                        rval.resolve(&locals).map_err(map_err_fn)?,
                    );
                }
                Op::ContextCopy(ctx_ident, ctx_val_ident, rval) => {
                    let ctx_ident = ctx_ident.resolve(&locals).map_err(map_err_fn)?;
                    let ctx_val_ident = ctx_val_ident.resolve(&locals).map_err(map_err_fn)?;
                    let rval = rval.resolve(&locals).map_err(map_err_fn)?;

                    match state.contexts.get_mut(&ctx_ident) {
                        Some(context) => context.vals.insert(ctx_val_ident, rval),
                        None => {
                            return Err(map_err_fn(OpErrReason::ContextRefInvalid { ident: ctx_ident }));
                        }
                    };
                }
                Op::ContextRemove(rval) => {
                    let rval = rval.resolve(&locals).map_err(map_err_fn)?;

                    match state.contexts.remove(&rval) {
                        Some(_) => {}
                        None => {
                            return Err(map_err_fn(OpErrReason::ContextDoesNotExist { id: rval }));
                        }
                    };
                }

                Op::ThreadRemove(rval) => {
                    let rval = rval.resolve(&locals).map_err(map_err_fn)?;

                    match state.threads.remove(&rval) {
                        Some(_) => {}
                        None => {
                            return Err(map_err_fn(OpErrReason::ThreadDoesNotExist { id: rval }));
                        }
                    };
                }
            }
        }

        thread.ip = locals.get(&LOCAL_NIP.to_string()).unwrap().clone();
        thread.eip = match locals.get(&LOCAL_EIP.to_string()).unwrap().as_ref() {
            "" => None,
            x => Some(x.to_string())
        };
        thread.ctx = match locals.get(&LOCAL_CTX.to_string()).unwrap().as_ref() {
            "" => None,
            x => Some(x.to_string())
        };

        Ok(())
    }
}