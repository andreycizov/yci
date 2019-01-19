#[cfg(test)]
mod tests {
    use crate::daemon::*;
    use crate::obj::*;

    fn create_machine() {
        let mut dpu = DPU::default();
        dpu.load(
            &vec![
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
            ]
        );

        let ctx = dpu.put_context(None);

        let thread = dpu.put_thread(
            "0".to_string(),
            ctx
        );
    }

    fn create_machine_err() {
        let mut dpu = DPU::default();
        dpu.load(
            &vec![
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
            ]
        );

        let ctx = dpu.put_context(None);

        let thread = dpu.put_thread(
            "0".to_string(),
            ctx
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