use std::fs::File;
use std::io::prelude::*;
use std::io;

use yci;
use yci::prog::*;


static TEST_SIMPLE: &str = "./etc/ir/commented_test.ir";
static TEST_ALGO: &str = "./etc/ir/from_docs.ir";
static TEST_INCORRECT: &str = "./etc/ir/missing_opcode.ir";

fn main() {
    let mut file = File::open(TEST_INCORRECT).unwrap();
    let mut contents = String::new();

    file.read_to_string(&mut contents);

    dbg!(&contents);

    let contents = ir_input(&contents);

    let x = ir_file(contents);
    let y = ir_load(x);

    let stdout = io::stderr();
    let mut handle = stdout.lock();


    match y {
        Ok(x) => {
            dbg!(x);
        }
        Err(x) => {
            let err: String = format_error(&contents, &x).unwrap();
            //err.encode;

            handle.write(err.as_bytes()).unwrap();
        }
    }
}
