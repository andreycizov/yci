use rand::prelude::*;

use std::collections::HashMap;
use std::collections::VecDeque;

type Id = u128;
type ThreadId = Id;
type StepId = Id;
type ContextId = Id;
type CommandId = Id;
type WorkerId = Id;

pub type ContextIdent = String;
pub type ContextValue = String;

use uuid;

#[derive(Debug, Clone)]
struct Thread {
    // Identity
    id: ThreadId,
    // For every opcode executed, goes up by one
    step: StepId,
    // Instruction pointer
    ip: CommandId,
    // Context pointer
    ctx: ContextId,
    //
    state: ThreadState,
}

#[derive(Debug, Clone)]
enum ThreadState {
    Started,
    Fetching(CommandId),
    Fetched(Command),
    Interpolating(Command),
    Queued(InterpolatedCommand),
    //Running(InterpolatedCommand, LockId),
    Done { nip: CommandId },

    // Waiting
    Paused,
    Exited,
}

#[derive(Debug, Clone)]
pub struct Worker {
    capacity: u64,
}

#[derive(Debug, Clone)]
pub struct Context {
    id: ContextId,
    vals: HashMap<ContextIdent, ContextValue>,
}

impl Context {
    pub fn get(&self, ident: &ContextIdent) -> Option<ContextValue> {
        match self.vals.get(ident) {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    id: CommandId,
    opcode: CommandArgument,
    args: Vec<CommandArgument>,
}

#[derive(Debug, Clone)]
pub struct InterpolatedCommand {
    id: CommandId,
    opcode: InterpolatedCommandArgument,
    args: Vec<InterpolatedCommandArgument>,
}

#[derive(Debug, Clone)]
pub enum CommandArgument {
    // value
    Const(ContextValue),
    // name of the ctx variable that has the value
    Ref(ContextIdent),
}

#[derive(Debug, Clone)]
pub enum InterpolatedCommandArgument {
    Const(ContextValue),
    Ref(ContextIdent, ContextValue),
}

impl Context {
    pub fn empty(
        id: ContextId,
    ) -> Self {
        Context::create(id, HashMap::<ContextIdent, ContextValue>::default())
    }

    pub fn create(
        id: ContextId,
        vals: HashMap<ContextIdent, ContextValue>,
    ) -> Self {
        Context {
            id,
            vals,
        }
    }
}

impl Command {
    pub fn create(
        id: CommandId,
        opcode: CommandArgument,
        args: Vec<CommandArgument>,
    ) -> Command {
        Command {
            id,
            opcode,
            args,
        }
    }

    pub fn interpolate(&self, ctx: &Context) -> Result<InterpolatedCommand, String> {
        let a: Result<Vec<InterpolatedCommandArgument>, String> = self.args.iter().map(|x| match x {
            CommandArgument::Const(v) => Ok(InterpolatedCommandArgument::Const(v.clone())),
            CommandArgument::Ref(k) => match ctx.vals.get(k) {
                Some(v) => Ok(InterpolatedCommandArgument::Ref(k.clone(), v.clone())),
                None => Err(k.clone())
            },
        }).collect();

        let opcode = match &self.opcode {
            CommandArgument::Const(v) => Ok(InterpolatedCommandArgument::Const(v.clone())),
            CommandArgument::Ref(k) => match ctx.vals.get(k) {
                Some(v) => Ok(InterpolatedCommandArgument::Ref(k.clone(), v.clone())),
                None => Err(k.clone())
            },
        };

        Ok(InterpolatedCommand {
            id: self.id,
            opcode: opcode?,
            args: a?,
        })
    }
}


struct DPU {
    commands: HashMap<CommandId, Command>,
    contexts: HashMap<ContextId, Context>,
    threads: HashMap<ThreadId, Thread>,

    queues: HashMap<ContextValue, VecDeque<ThreadId>>,
    workers: HashMap<WorkerId, Worker>,

    rng: ThreadRng,
}

impl Default for DPU {
    fn default() -> Self {
        DPU {
            commands: HashMap::<CommandId, Command>::default(),
            contexts: HashMap::<ContextId, Context>::default(),
            threads: HashMap::<ThreadId, Thread>::default(),

            queues: HashMap::<ContextValue, VecDeque<ThreadId>>::default(),
            workers: HashMap::<WorkerId, Worker>::default(),

            rng: ThreadRng::default(),
        }
    }
}

#[derive(Debug, Clone)]
enum ExecOp {
    ContextCreate { id: ContextIdent },
    ContextCopy { id: ContextIdent, ident: ContextIdent, val: ContextIdent },
    ContextSet { ident: ContextIdent, val: ContextValue },
    ContextRemove { id: ContextIdent },
    ThreadCreate { id: ContextIdent, ip: CommandId, ctx: ContextIdent },
    ThreadRemove { id: ContextIdent },
    SetNIP { id: CommandId },
    SetContext { id: ContextId },
}

#[derive(Debug, Clone)]
pub enum ExecErrReason {
    ContextDoesNotExist { id: ContextId },
    ThreadDoesNotExist { id: ContextId },
    ContextRefInvalid { ident: ContextIdent },
    UnknownOp,
}

#[derive(Debug, Clone)]
pub struct ExecErr {
    op_index: Option<usize>,
    op_reason: ExecErrReason,
}

impl DPU {
    pub fn put(&mut self) {}

    pub fn load(&mut self, commands: &Vec<Command>) {
        for command in commands {
            self.commands.insert(command.id, command.clone());
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
                ExecOp::ContextSet { ident: ident, val: val } => {
                    context.vals.insert(ident.clone(), val.clone());
                }
                ExecOp::ContextRemove { id: ident } => {
                    let ident = parse_id(&context_get(ident)?, ExecErrReason::ContextRefInvalid { ident: ident.clone() })?;

                    match self.contexts.remove(&ident) {
                        Some(_) => {}
                        None => return context_err(i, ExecErrReason::ContextDoesNotExist { id: ident.clone() })
                    }
                }
                ExecOp::SetNIP { id: id } => {
                    thread.ip = id.clone()
                }
                ExecOp::SetContext { id: id } => {
                    thread.ctx = id.clone()
                }
                _ => return context_err(i, ExecErrReason::UnknownOp)
            }
        }

        thread.step.wrapping_add(1);

        self.threads.insert(thread.id, thread);
        self.contexts.insert(context.id, context);

        Ok(())
    }

    pub fn done(&mut self, id: &ThreadId, step: StepId, ops: &Vec<ExecOp>) -> Result<(), &str> {
        if let Some(x) = self.threads.get(id) {
            if x.step == step {

                //let &mut thread = x;
                //thread.step += 1;
                return Ok(());
            } else {
                return Err("post-stepped");
            }
        } else {
            return Err("side-stepped");
        };
    }
}