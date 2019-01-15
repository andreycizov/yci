use std::collections::HashMap;

type Id = u128;
type ThreadId = Id;
type LockId = Id;
type ContextId = Id;
type CommandId = Id;

type ContextIdent = str;
type ContextValue = str;

struct Thread {
    id: ThreadId,
    ip: CommandId,
    
    // why do we need to lock it?
    locked_by: Optional<u128>,
}

enum ThreadState {
    Started,
    Fetching(CommandId),
    Fetched(Command),
    Interpolating(Command),
    Queued(InterpolatedCommand),
    Locked(InterpolatedCommand, LockId),
    Done(CommandId),
    Exited,
}

struct Context {
    id: ContextId,
    vals: HashMap<ContextIdent, ContextValue>,
}


pub struct Command {
    id: CommandId,
    args: Vec<CommandArgument>,
}

struct InterpolatedCommand {
    id: CommandId,
    args: Vec<InterpolatedCommandArgument>,
}

enum CommandArgument {
    // value
    Const(ContextValue),
    // name of the ctx variable that has the value
    ContextRef(ContextIdent),
}

enum InterpolatedCommandArgument {
    Const(ContextValue),
    ContextRef(ContextIdent, ContextValue),
}

impl Command for Command {
    fn interpolate(self, ctx: Context) -> Result<InterpolatedCommand, &'static str> {
        let a : Result<Vec<InterpolatedCommandArgument>, &'static str> = self.args.iter().map(|x| match x {
            CommandArgument::Const(v) => Ok(InterpolatedCommandArgument::Const(*v)),
            CommandArgument::ContextRef(k) => match ctx.vals.get(&k) {
                Some(v) => Ok(InterpolatedCommandArgument::ContextRef(*k, *v)),
                None => Err(*k)
            },
        }).collect();
        
        Ok(InterpolatedCommand {
            id: self.id,
            args: a?,
        })
    }
}


struct DPU {
    commands: HashMap<CommandId, Command>,
    contexts: HashMap<ContextId, Context>,
    threads: HashMap<ThreadId, Thread>,
}

enum ExecOp {
    ContextCreate(ContextId),
    ContextSet(ContextIdent, ContextValue),
    ContextRemove(ContextId),
    ThreadCreate(ThreadId),
    ThreadRemove(ThreadId),
}

struct ExecOpX {
    thread_id: u128,
}


