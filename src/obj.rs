use std::collections::HashMap;

pub(crate) type Id = u128;
pub(crate) type ThreadId = Id;
pub(crate) type StepId = Id;
pub(crate) type ContextId = Id;
pub(crate) type CommandId = String;
pub(crate) type WorkerId = Id;

pub(crate) type ContextIdent = String;
pub(crate) type ContextValue = String;

//use uuid;



#[derive(Debug, Clone)]
pub(crate) struct Worker {
    capacity: u64,
    queues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub(crate) id: ContextId,
    pub(crate) vals: HashMap<ContextIdent, ContextValue>,
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
    pub(crate) id: CommandId,
    pub(crate) opcode: CommandArgument,
    pub(crate) args: Vec<CommandArgument>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterpolatedCommand {
    pub(crate) id: CommandId,
    pub(crate) opcode: InterpolatedCommandArgument,
    pub(crate) args: Vec<InterpolatedCommandArgument>,
}

impl InterpolatedCommand {
    pub fn create(
        id: CommandId,
        opcode: InterpolatedCommandArgument,
        args: Vec<InterpolatedCommandArgument>,
    ) -> Self {
        InterpolatedCommand {
            id,
            opcode,
            args,
        }

    }
}

#[derive(Debug, Clone)]
pub enum CommandArgument {
    // value
    Const(ContextValue),
    // name of the ctx variable that has the value
    Ref(ContextIdent),
}


#[derive(Debug, Clone, PartialEq)]
pub enum InterpolatedCommandArgument {
    Const(ContextValue),
    Ref(ContextIdent, ContextValue),
}

impl InterpolatedCommandArgument {
    pub fn value(&self) -> ContextValue {
        match self {
            InterpolatedCommandArgument::Const(x) => x.clone(),
            InterpolatedCommandArgument::Ref(_, x) => x.clone(),
        }
    }
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
            id: self.id.clone(),
            opcode: opcode?,
            args: a?,
        })
    }
}




