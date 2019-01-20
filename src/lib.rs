extern crate uuid;
extern crate rand;

use std::collections::HashMap;

pub mod obj;
pub mod prog;
pub mod daemon;
pub mod pubsub;

//pub use obj;
//pub use microcode;

#[cfg(test)]
mod tests;

pub fn main() {
    let y1 = "a";
    let y2 = String::from("bcde");
    let y3 = String::from("c");

    let x01 = String::from("a");
    let x02 = String::from("b");
    let x03 = String::from("c");
    let x04 = String::from("d");
    let x11 = String::from("1");
    let x12 = String::from("2");
    let x13 = String::from("3");
    let x14 = String::from("4");

    let c = {
        let a = obj::Command::create(
            2.to_string(),
            obj::CommandArgument::Ref(y1.into()),
            vec![
                obj::CommandArgument::Const(y2),
                obj::CommandArgument::Ref(y3),
            ]
        );

        let mut values = HashMap::<obj::ContextIdent, obj::ContextValue>::default();
        values.insert(x01, x11);
        values.insert(x02, x12);
        values.insert(x03, x13);
        values.insert(x04, x14);

        let b = obj::Context::create(3, values);
        a.interpolate(&b)
    };


    println!("{:?}", c);
}
