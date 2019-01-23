pub(crate) static TEST_SIMPLE: &str = "./etc/ir/commented_test.ir";
pub(crate) static TEST_ALGO: &str = "./etc/ir/from_docs.ir";
pub(crate) static TEST_INCORRECT: &str = "./etc/ir/missing_opcode.ir";

use nom::IResult;
use crate::prog::*;
use std::str;
use std::fs::File;
use std::io::Read;

#[derive(Debug)]
pub(crate) struct LoadIRFile<'a> {
    filename: &'a str,
    contents: String,
}

impl <'a>LoadIRFile<'a> {
    pub fn new(name: &'a str) -> Self {
        let mut new = LoadIRFile { filename: name, contents: String::new() };

        let mut file = File::open(name).unwrap();

        file.read_to_string(&mut new.contents).unwrap();

        new
    }

    pub fn load(&self) -> Result<IRMap, IRErr<'a, u32>>{
        ir_load::<u32>(ir_file(ir_input(&self.contents)))
    }
}

#[test]
fn test_microcode_one() {
    let x = LoadIRFile::new(TEST_SIMPLE);

    dbg!(x.load());
}

#[test]
fn test_microcode_algo() {
    let x = LoadIRFile::new(TEST_ALGO);

    dbg!(x.load());
}

#[test]
fn test_ir_loader() {
    let x = LoadIRFile::new(TEST_INCORRECT);

    dbg!(x.load());
}