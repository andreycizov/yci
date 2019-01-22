#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use crate::obj::*;
    use std::collections::HashMap;

    fn create_context() -> Context {
        let mut values = HashMap::<ContextIdent, ContextValue>::default();

        for (i, c) in "abcdef".chars().enumerate() {
            values.insert(c.to_string(), i.to_string());
        }

        Context::create(0, values)
    }

    #[test]
    fn test_opcode() {
        let ctx = create_context();

        let cmd = Command::create("0".to_string(), CommandArgument::Ref("b".to_string()), vec![]);

        assert_eq!(
            cmd.interpolate(Some(&ctx)),
            Ok(InterpolatedCommand::create(
                "0".to_string(),
                InterpolatedCommandArgument::Ref("b".into(), "1".into()),
                vec![]
            ))
        )
    }

    #[test]
    fn test_opcode_err() {
        let ctx = create_context();

        let cmd = Command::create("0".to_string(), CommandArgument::Ref("g".to_string()), vec![]);

        assert_eq!(
            cmd.interpolate(Some(&ctx)),
            Err(InterpolationError::Ref("g".into()))
        )
    }
}