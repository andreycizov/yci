#[cfg(test)]
mod tests {
    use crate::daemon::*;
    use crate::obj::*;

    fn create_machine() {
        let mut dpu = DPU::default();
        let mut state = dpu.get_state_mut();
        state.insert_commands(
            vec![
                Command::create(
                    "0".to_string(),
                    CommandArgument::Const("nop".to_string()),
                    vec![],
                ),
                Command::create(
                    "1".to_string(),
                    CommandArgument::Const("ld".to_string()),
                    vec![
                        CommandArgument::Const("addr".to_string()),
                        CommandArgument::Const("0".to_string()),
                    ],
                ),
                Command::create(
                    "2".to_string(),
                    CommandArgument::Const("jmp".to_string()),
                    vec![
                        CommandArgument::Ref("addr".to_string())
                    ],
                )
            ].iter()
        );
        
        let state = dpu.get_state_mut();
        let ctx = Context::empty(state.create_id());
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
                Command::create(
                    "0".to_string(),
                    CommandArgument::Ref("nop".to_string()),
                    vec![],
                ),
                Command::create(
                    "1".to_string(),
                    CommandArgument::Const("ld".to_string()),
                    vec![
                        CommandArgument::Ref("addr".to_string()),
                        CommandArgument::Const("0".to_string()),
                    ],
                ),
                Command::create(
                    "2".to_string(),
                    CommandArgument::Const("jmp".to_string()),
                    vec![
                        CommandArgument::Ref("addr".to_string())
                    ],
                )
            ].iter()
        );
    
        let state = dpu.get_state_mut();
        let ctx = Context::empty(state.create_id());
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
}