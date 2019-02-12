use std::collections::HashMap;
use serde_derive::{Serialize, Deserialize};
use crate::daemon::LOCAL_CTX;
use crate::daemon::Op;
use crate::daemon::RValueLocal;

pub type Id = u128;
pub type GenId = String;
pub type ThreadId = GenId;
pub type StepId = Id;
pub type ContextId = GenId;
pub type CommandId = GenId;
pub type WorkerId = GenId;
pub type PauseId = GenId;

pub type ContextIdent = GenId;
pub type ContextValue = GenId;

// todo null values for context variables
// todo ability to access both current context and a context referred to by a variable from the current context
// todo

#[derive(Debug, Clone)]
pub struct WorkerStatus {
    capacity: u64,
    queues: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Ctx {
    pub(crate) id: ContextId,
    pub(crate) vals: HashMap<ContextIdent, ContextValue>,
}

impl Ctx {
    pub fn get(&self, ident: &ContextIdent) -> Option<ContextValue> {
        match self.vals.get(ident) {
            Some(x) => Some(x.clone()),
            None => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CtxNs {
    Curr,
    Ref(ContextIdent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CtxRef(
    pub CtxNs,
    pub ContextIdent,
);


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cmd {
    pub(crate) id: CommandId,
    pub(crate) opcode: CmdArg,
    pub(crate) args: Vec<CmdArg>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CmdArg {
    // value
    Const(ContextValue),
    // name of the ctx variable that has the value
    Ref(CtxRef),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum XCtxNs {
    Curr,
    Ref(ContextId),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XCtxRef(
    pub XCtxNs,
    pub ContextIdent,
);

impl XCtxRef {
    pub fn set(&self, val: RValueLocal) -> Op {
        let XCtxRef(ns, var) = &self;
        Op::ContextSet(
            match ns {
                XCtxNs::Curr => RValueLocal::Ref(LOCAL_CTX.to_string()),
                XCtxNs::Ref(id) => RValueLocal::Const(id.clone()),
            },
            RValueLocal::Const(var.clone()),
            val,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct XCmd {
    pub id: CommandId,
    pub opcode: ContextValue,
    pub args: Vec<XCmdArg>,
}

impl XCmd {
    pub fn create(
        id: CommandId,
        opcode: ContextValue,
        args: Vec<XCmdArg>,
    ) -> Self {
        XCmd {
            id,
            opcode,
            args,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum XCmdArg {
    Const(ContextValue),
    Ref(XCtxRef, Option<ContextValue>),
}

impl XCmdArg {
    pub fn ident(&self) -> Option<XCtxRef> {
        match self {
            XCmdArg::Const(_) => None,
            XCmdArg::Ref(xref, _) => Some(xref.clone())
        }
    }

    pub fn value(&self) -> Option<ContextValue> {
        match self {
            XCmdArg::Const(x) => Some(x.clone()),
            XCmdArg::Ref(_, x) => x.clone(),
        }
    }
}

impl Ctx {
    pub fn empty(
        id: ContextId,
    ) -> Self {
        Ctx::create(id, HashMap::<ContextIdent, ContextValue>::default())
    }

    pub fn create(
        id: ContextId,
        vals: HashMap<ContextIdent, ContextValue>,
    ) -> Self {
        Ctx {
            id,
            vals,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterpolationError {
    CtxNull,
    CtxMiss(ContextId),
    CmdNull,
    Ref(CtxRef),
}

impl Cmd {
    pub fn create(
        id: CommandId,
        opcode: CmdArg,
        args: Vec<CmdArg>,
    ) -> Cmd {
        Cmd {
            id,
            opcode,
            args,
        }
    }


}
