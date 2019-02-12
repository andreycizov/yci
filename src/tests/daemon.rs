
use crate::daemon::*;
use crate::obj::*;

fn create_machine() {
    let mut dpu = DPU::default();
    let mut state = dpu.get_state_mut();
    state.insert_commands(
        vec![
            Cmd::create(
                "0".to_string(),
                CmdArg::Const("nop".to_string()),
                vec![],
            ),
            Cmd::create(
                "1".to_string(),
                CmdArg::Const("ld".to_string()),
                vec![
                    CmdArg::Const("addr".to_string()),
                    CmdArg::Const("0".to_string()),
                ],
            ),
            Cmd::create(
                "2".to_string(),
                CmdArg::Const("jmp".to_string()),
                vec![
                    CmdArg::Ref(CtxRef(CtxNs::Curr, "addr".to_string()))
                ],
            )
        ].iter()
    );

    let state = dpu.get_state_mut();
    let ctx = Ctx::empty(state.create_id());
    state.insert_context(&ctx);

    let thr = Thread::create(
        state.create_id(),
        "0".to_string(),
        Some(ctx.id)
    );

}

fn create_machine_err() {
    let mut dpu = DPU::default();
    let mut state = dpu.get_state_mut();
    state.insert_commands(
        vec![
            Cmd::create(
                "0".to_string(),
                CmdArg::Ref(CtxRef(CtxNs::Curr, "nop".to_string())),
                vec![],
            ),
            Cmd::create(
                "1".to_string(),
                CmdArg::Const("ld".to_string()),
                vec![
                    CmdArg::Ref(CtxRef(CtxNs::Curr, "addr".to_string())),
                    CmdArg::Const("0".to_string()),
                ],
            ),
            Cmd::create(
                "2".to_string(),
                CmdArg::Const("jmp".to_string()),
                vec![
                    CmdArg::Ref(CtxRef(CtxNs::Curr, "addr".to_string()))
                ],
            )
        ].iter()
    );

    let state = dpu.get_state_mut();
    let ctx = Ctx::empty(state.create_id());
    state.insert_context(&ctx);

    let thr = Thread::create(
        state.create_id(),
        "0".to_string(),
        Some(ctx.id)
    );
}

#[test]
fn test_machine() {
   // create_machine();
}

#[test]
fn test_machine_err() {
        create_machine_err();
    }
