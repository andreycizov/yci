#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use crate::obj::*;
    use std::collections::HashMap;

    fn create_context() -> Ctx {
        let mut values = HashMap::<ContextIdent, ContextValue>::default();

        for (i, c) in "abcdef".chars().enumerate() {
            values.insert(c.to_string(), i.to_string());
        }

        Ctx::create(0.to_string(), values)
    }
//
//    #[test]
//    fn test_opcode() {
//        let ctx = create_context();
//
//        let cmd = Cmd::create("0".to_string(), CmdArg::Ref(CtxRef(CtxNs::Curr, "b".to_string())), vec![]);
//
//        assert_eq!(
//            cmd.interpolate(Some(&ctx)),
//            Ok(XCmd::create(
//                "0".to_string(),
//                XCmdArg::Ref(XCtxRef(XCtxNs::Curr, "b".into()), "1".into()),
//                vec![]
//            ))
//        )
//    }
//
//    #[test]
//    fn test_opcode_err() {
//        let ctx = create_context();
//
//        let cmd = Cmd::create("0".to_string(), CmdArg::Ref(CtxRef(CtxNs::Curr, "g".to_string())), vec![]);
//
//        assert_eq!(
//            cmd.interpolate(Some(&ctx)),
//            Err(InterpolationError::Ref(CtxRef(CtxNs::Curr, "g".into())))
//        )
//    }
}