use std::collections::HashMap;

pub type Id = u128;
pub type GenId = String;
pub type ThreadId = GenId;
pub type StepId = Id;
pub type ContextId = GenId;
pub type CommandId = GenId;
pub type WorkerId = Id;

pub type ContextIdent = GenId;
pub type ContextValue = GenId;


#[derive(Debug, Clone)]
pub struct WorkerStatus {
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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub enum InterpolationError {
    EmptyContext,
    Ref(String)
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

    pub fn interpolate(&self, ctx: Option<&Context>) -> Result<InterpolatedCommand, InterpolationError> {
        let match_arg = |x: &CommandArgument| match x {
            CommandArgument::Const(v) => Ok(InterpolatedCommandArgument::Const(v.clone())),
            CommandArgument::Ref(k) => ctx.ok_or(InterpolationError::EmptyContext).and_then(|ctx|{
                match ctx.vals.get(k) {
                    Some(v) => Ok(InterpolatedCommandArgument::Ref(k.clone(), v.clone())),
                    None => Err(InterpolationError::Ref(k.clone()))
                }
            })
        };

        let a: Result<Vec<InterpolatedCommandArgument>, InterpolationError> = self.args.iter().map(match_arg).collect();

        let opcode = match_arg(&self.opcode);

        Ok(InterpolatedCommand {
            id: self.id.clone(),
            opcode: opcode?,
            args: a?,
        })
    }
}




