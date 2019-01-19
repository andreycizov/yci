#[cfg(test)]
mod tests {
    use crate::daemon::*;
    use crate::obj::*;

    fn create_machine() {
        let mut dpu = DPU::default();
        dpu.load(
            &vec![
                Command::create(
                    0,
                    CommandArgument::Const("nop".to_string()),
                    vec![],
                ),
                Command::create(
                    1,
                    CommandArgument::Const("ld".to_string()),
                    vec![
                        CommandArgument::Const("addr".to_string()),
                        CommandArgument::Const("0".to_string()),
                    ],
                ),
                Command::create(
                    2,
                    CommandArgument::Const("jmp".to_string()),
                    vec![
                        CommandArgument::Ref("addr".to_string())
                    ],
                )
            ]
        );

        let ctx = dpu.put_context(None);

        let thread = dpu.put_thread(
            0,
            ctx
        );
    }

    #[test]
    fn test_opcode() {
        create_machine();
    }
}