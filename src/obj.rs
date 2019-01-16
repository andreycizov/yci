use std::collections::HashMap;

type Id = u128;
type ThreadId = Id;
type LockId = Id;
type StepId = Id;
type ContextId = Id;
type CommandId = Id;
type SignalId = Id;

pub type ContextIdent = String;
pub type ContextValue = String;

struct Thread {
    // Identity
    id: ThreadId,
    // For every opcode executed, goes up by one
    step: StepId,
    // Instruction pointer
    ip: CommandId,
    // Context pointer
    ctx: ContextId,
}

enum ThreadState {
    Started,
    Fetching(CommandId),
    Fetched(Command),
    Interpolating(Command),
    Queued(InterpolatedCommand),
    //Running(InterpolatedCommand, LockId),
    Done(CommandId),

    // Waiting
    Paused,
    Exited,
}

#[derive(Debug, Clone)]
pub struct Context {
    id: ContextId,
    vals: HashMap<ContextIdent, ContextValue>,
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
    pub fn create(
        id: ContextId,
        vals: HashMap<ContextIdent, ContextValue>,
    ) -> Context {
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
        let a: Result<Vec<InterpolatedCommandArgument>, String> =  self.args.iter().map(|x| match x {
            CommandArgument::Const(v) => Ok(InterpolatedCommandArgument::Const(v.clone())),
            CommandArgument::Ref(k) => match ctx.vals.get(k) {
                Some(v) => Ok(InterpolatedCommandArgument::Ref(k.clone(), v.clone())),
                None => Err(k.clone())
            },
        }).collect();

        let opcode =  match &self.opcode {
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


struct DPU<'a> {
    commands: HashMap<CommandId, Command>,
    contexts: HashMap<ContextId, Context>,
    threads: HashMap<ThreadId, Thread>,
}
//
//enum ExecOp<'a> {
//    ContextCreate(ContextId),
//    ContextSet(ContextIdent<'a>, ContextValue<'a>),
//    ContextRemove(ContextId),
//    ThreadCreate(ThreadId),
//    ThreadRemove(ThreadId),
//
//    // proceed the thread to the next command
//    ThreadNext(ThreadId, LockId, CommandId),
//
//    SetIP(CommandId),
//    SetContext(ContextId),
//}
