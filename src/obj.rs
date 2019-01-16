use std::collections::HashMap;

type Id = u128;
type ThreadId = Id;
type LockId = Id;
type ContextId = Id;
type CommandId = Id;
type SignalId = Id;

pub type ContextIdent<'a> = &'a str;
pub type ContextValue<'a> = &'a str;

struct Thread {
    id: ThreadId,
    ip: CommandId,
    ctx: ContextId,
}

enum ThreadState<'a> {
    Started,
    Fetching(CommandId),
    Fetched(Command<'a>),
    Interpolating(Command<'a>),
    Queued(InterpolatedCommand<'a>),
    //Running(InterpolatedCommand, LockId),
    Done(CommandId),
    Signal(SignalId),
    Paused,
    Exited,
}

#[derive(Debug)]
pub struct Context<'a> {
    id: ContextId,
    vals: HashMap<ContextIdent<'a>, ContextValue<'a>>,
}

#[derive(Debug, Clone)]
pub struct Command<'a> {
    id: CommandId,
    opcode: CommandArgument<'a>,
    args: Vec<CommandArgument<'a>>,
}

#[derive(Debug, Clone)]
pub struct InterpolatedCommand<'a> {
    id: CommandId,
    opcode: InterpolatedCommandArgument<'a>,
    args: Vec<InterpolatedCommandArgument<'a>>,
}

#[derive(Debug, Clone)]
pub enum CommandArgument<'a> {
    // value
    Const(ContextValue<'a>),
    // name of the ctx variable that has the value
    Ref(ContextIdent<'a>),
}

#[derive(Debug, Clone)]
pub enum InterpolatedCommandArgument<'a> {
    Const(ContextValue<'a>),
    Ref(ContextIdent<'a>, ContextValue<'a>),
}

impl<'a> Context<'a> {
    pub fn create<'c>(
        id: ContextId,
        vals: HashMap<ContextIdent<'c>, ContextValue<'c>>,
    ) -> Context<'c> {
        Context {
            id,
            vals,
        }
    }
}

impl<'a> Command<'a> {
    pub fn create<'c>(
        id: CommandId,
        opcode: CommandArgument<'c>,
        args: Vec<CommandArgument<'c>>,
    ) -> Command<'c> {
        Command {
            id,
            opcode,
            args,
        }
    }

    pub fn interpolate<'inp, 'ret, 'ctx>(&'inp self, ctx: &'ctx Context<'ctx>) -> Result<InterpolatedCommand<'ret>, &'ret str> {
        let args = {
            self.args.iter().map(|&x| match x {
                CommandArgument::Const(v) => Ok(InterpolatedCommandArgument::Const::<'ret>((*v).into())),
                CommandArgument::Ref(k) => match &ctx.vals.get(k) {
                    Some(v) => Ok(InterpolatedCommandArgument::Ref::<'ret>((*k).into(), (**v).into())),
                    None => Err((*k).into())
                },
            }).collect()
        };

        let a: Result<Vec<InterpolatedCommandArgument<'ret>>, &'ret str> = args;

        let opcode = {
            match &self.opcode {
                CommandArgument::Const(v) => Ok(InterpolatedCommandArgument::Const::<'ret>((*v).into())),
                CommandArgument::Ref(k) => match &ctx.vals.get(k) {
                    Some(v) => Ok(InterpolatedCommandArgument::Ref((*k).into(), (**v).into())),
                    None => Err((*k).into())
                },
            }
        };

        Ok(InterpolatedCommand::<'ret> {
            id: self.id,
            opcode: opcode?,
            args: a?,
        })
    }
}
//
//
//struct DPU<'a> {
//    commands: HashMap<CommandId, Command<'a>>,
//    contexts: HashMap<ContextId, Context<'a>>,
//    threads: HashMap<ThreadId, Thread>,
//}
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
