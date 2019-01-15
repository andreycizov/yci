struct Thread {
    id: u128,
    ip: u128,
    locked_by: Optional<u128>,
}

struct Ctx {
    id: u128,
    vals: HashMap(),
}

enum CmdArg {
    // value
    Static(str),
    // name of the ctx variable that has the value
    CtxRef(str),
}

struct Cmd {
    id: u128,
    args: [CmdArg],
}

struct DPU {
    code: HashMap<u128, Cmd>,
    ctxs: HashMap<u128, Ctx>,
    threads: HashMap<u128, Ctx>
}

enum ExecOp {
    CreateCtx(u128),
    ModCtx(str, str),
    CreateThread()
}

struct ExecOpX {
    thread_id: u128,
}