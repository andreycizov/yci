use rand::prelude::*;

use std::collections::HashMap;

use super::obj::*;
use super::pubsub::*;

#[derive(Debug, Clone)]
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

pub struct State {
    commands: HashMap<CommandId, Command>,
    contexts: HashMap<ContextId, Context>,
    threads: HashMap<ThreadId, Thread>,
    
    rng: ThreadRng,
}

impl State {
    pub fn create_id(&mut self) -> GenId {
        self.rng.gen::<u128>().to_string()
    }
    
    pub fn insert_thread(&mut self, thread: &Thread) {
        self.threads.insert(thread.id.clone(), thread.clone());
    }
    
    pub fn insert_context(&mut self, context: &Context) {
        self.contexts.insert(context.id.clone(), context.clone());
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

pub struct DPU {
    state: State,
    
    multi_queue: MultiQueue<WorkerId, ContextValue, ThreadId>,
    workers: HashMap<WorkerId, Worker>,
}

#[derive(Debug, Clone)]
pub enum RValueLocal {
    Ref(ContextIdent),
    Const(ContextValue),
}

impl RValueLocal {
    pub fn resolve(
        &self,
        locals: &HashMap<ContextIdent, ContextValue>,
    ) -> Result<ContextValue, ExecErrReason> {
        match self {
            RValueLocal::Const(val) => Ok(val.clone()),
            RValueLocal::Ref(ident) => Err(ExecErrReason::LocalRefInvalid { ident: ident.clone() })
        }
    }
}

#[derive(Debug, Clone)]
pub enum RValueExtern {
    ContextCreate,
    ThreadCreate(RValueLocal, Option<RValueLocal>),
}

impl RValueExtern {
    pub fn resolve(
        &self,
        locals: &HashMap<ContextIdent, ContextValue>,
        state: &mut State,
    ) -> Result<ContextValue, ExecErrReason> {
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
                state.insert_thread(&Thread::create(id.clone(), ip, ctx));
                
                Ok(ContextValue::from(id))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum RValue {
    Local(RValueLocal),
    Extern(RValueExtern),
}

impl RValue {
    pub fn resolve(
        &self,
        locals: &HashMap<ContextIdent, ContextValue>,
        state: &mut State,
    ) -> Result<ContextValue, ExecErrReason> {
        match self {
            RValue::Local(x) => x.resolve(locals),
            RValue::Extern(x) => x.resolve(locals, state),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Op {
    ValueSet(ContextIdent, RValue),
    ContextSet(ContextIdent, RValueLocal),
    
    ContextCopy(RValueLocal, RValueLocal, RValueLocal),
    ContextRemove(RValueLocal),
    
    ThreadRemove(RValueLocal),
    
    ThreadContextSet(RValueLocal),
    ThredNipSet(RValueLocal),
}

#[derive(Debug, Clone)]
pub enum ExecErrReason {
    ContextDoesNotExist { id: ContextId },
    ThreadDoesNotExist { id: ThreadId },
    LocalRefInvalid { ident: ContextIdent },
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

pub type WorkerExec<'a> = Result<&'a Vec<Op>, ExecErrReason>;

//pub enum WorkerExec {
//    // what worker returns when executed.
//
//}

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
            state: State::default(),
            
            //queues: HashMap::<ContextValue, VecDeque<ThreadId>>::default(),
            
            multi_queue: MultiQueue::<WorkerId, ContextValue, ThreadId>::default(),
            workers: HashMap::<WorkerId, Worker>::default(),
        }
    }
}


impl DPU {
    pub fn get_state_mut(&mut self) -> &mut State {
        &mut self.state
    }
    
    pub fn load(&mut self, commands: &Vec<Command>) {
        for command in commands {
            self.state.commands.insert(command.id.clone(), command.clone());
        }
    }
    
    fn proceed(&mut self, id: &ThreadId) {
        loop {
            // whenever we re-store
            
            let mut thread = match self.state.threads.get(id) {
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
                    match self.state.commands.get(ip) {
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
                    let ctx = (thread.ctx.clone()).and_then(|x| self.state.contexts.get(&x));
                    
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
                    let assignment = self.multi_queue.job_create(&command.opcode.value(), &thread.id);
                    
                    match assignment.first() {
                        Some(x) => {
                            Some(ThreadState::Assigned(command.clone(), x.worker_key))
                        }
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
                            
                            let id = self.state.create_id().to_string();
                            
                            let mut ctx = Context::empty(id);
                            let err_str = format!("{:?}", error);
                            
                            ctx.vals.insert(
                                // todo serialize err_str as json
                                "exc".to_string(),
                                err_str,
                            );
                            
                            self.state.insert_context(&ctx);
                            
                            thread.ctx = Some(ctx.id);
                            
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
            
            self.state.threads.insert(id.clone(), thread.clone());
            
            if should_break {
                return;
            }
        }
    }
    
    fn exec(
        thread: &mut Thread,
        state: &mut State,
        ops: &Vec<Op>,
    ) -> Result<(), ExecErr> {
        let mut locals = HashMap::<ContextIdent, ContextValue>::default();
        
        for (op_index, op) in ops.iter().enumerate() {
            let map_err_fn = |op_reason| ExecErr { op_index: Some(op_index), op_reason };
            
            match op {
                Op::ValueSet(loc_ident, rval) => {
                    locals.insert(
                        loc_ident.clone(),
                        rval.resolve(&locals, state).map_err(map_err_fn)?,
                    );
                }
                Op::ContextSet(loc_ident, rval) => {
                    locals.insert(
                        loc_ident.clone(),
                        rval.resolve(&locals).map_err(map_err_fn)?,
                    );
                }
                _ => { panic!("Must handle all") }
            }
        }

//        let a = {
//            match op {
//                Ops::ContextCreate { id: ident } => {
//                    let id = self.context_create();
//                    context.vals.insert(ident.clone(), id.to_string());
//                }
//                Ops::ContextCopy { id: ident, ident: name, val: value } => {
//                    let ident = context_get(&ident)?;
//                    let value = context_get(&value)?;
//
//                    let ident_int = parse_id(&ident, ExecErrReason::ContextRefInvalid { ident: ident.clone() })?;
//
//                    if let Some(x) = self.contexts.get_mut(&ident_int) {
//                        x.vals.insert(name.clone(), value);
//                    } else {
//                        return context_err(i, ExecErrReason::ContextDoesNotExist { id: ident_int })
//                    };
//
//                }
//                Ops::ContextSet { ident, val } => {
//                    context.vals.insert(ident.clone(), val.clone());
//                }
//                Ops::ContextRemove { id: ident } => {
//                    let ident = parse_id(&context_get(ident)?, ExecErrReason::ContextRefInvalid { ident: ident.clone() })?;
//
//                    match self.contexts.remove(&ident) {
//                        Some(_) => {}
//                        None => return context_err(i, ExecErrReason::ContextDoesNotExist { id: ident.clone() })
//                    }
//                }
//                Ops::ThreadCreate { id, ip, ctx } => {
//                    let id: u128 = self.rng.gen();
//
//                    let ip = context_get(&ip)?;
//                    let ctx = context_get(&ip)?;
//
//                    //let ip = parse_id(&ip, ExecErrReason::CommandRefInvalid { ident: ip.clone() })?;
//                    let ctx = parse_id(&ctx, ExecErrReason::ContextRefInvalid { ident: ctx.clone() })?;
//
//                    self.threads.insert(id, Thread::create(id, ip, Some(ctx)));
//                }
//                Ops::ThreadRemove { id: ident } => {
//                    let ident = parse_id(&context_get(ident)?, ExecErrReason::ThreadRefInvalid { ident: ident.clone() })?;
//
//                    match self.contexts.remove(&ident) {
//                        Some(_) => {}
//                        None => return context_err(i, ExecErrReason::ThreadDoesNotExist { id: ident.clone() })
//                    }
//                }
//                Ops::SetNIP { id } => {
//                    thread.ip = id.clone()
//                }
//                Ops::SetContext { id } => {
//                    thread.ctx = id.clone()
//                }
//                //_ => return context_err(i, ExecErrReason::UnknownOp)
//            }
//        }
        
        thread.step.wrapping_add(1);

//        let thread_id = thread.id.clone();
//
//        self.threads.insert(thread.id, thread);
//        self.contexts.insert(context.id, context);
//
//        self.proceed(&thread_id);
        
        Ok(())
    }
    
    pub fn done(&mut self, id: &ThreadId, step: StepId, ops: &Vec<Op>) -> Result<(), ExecErr> {
        if let Some(x) = self.state.threads.get(id) {
            if x.step == step {
                let mut thread = x.clone();
                
                let ret = DPU::exec(&mut thread, &mut self.state, ops)?;
                
                self.state.threads.insert(id.clone(), thread);
                
                return Ok(ret);
            } else {
                return Err(ExecErr { op_index: None, op_reason: ExecErrReason::PostStepped { current: x.step, selected: step } });
            }
        } else {
            return Err(ExecErr { op_index: None, op_reason: ExecErrReason::ThreadDoesNotExist { id: id.clone() } });
        };
    }
}