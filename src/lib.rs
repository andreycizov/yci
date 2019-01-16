use std::collections::HashMap;

pub mod obj;

pub fn main() {
    let c = {
        let a = obj::Command::create(
            2,
            obj::CommandArgument::Ref("a"),
            vec![
                obj::CommandArgument::Const("bcde"),
                obj::CommandArgument::Ref("c"),
            ]
        );

        let mut values = HashMap::<obj::ContextIdent, obj::ContextValue>::default();
        values.insert("a", "1");
        values.insert("b", "2");
        values.insert("c", "3");
        values.insert("d", "4");

        let b = obj::Context::create(3, values);
        a.interpolate(&b)
    };


    println!("{:?}", c);
}
