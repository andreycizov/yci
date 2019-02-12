pub(crate) static TEST_SIMPLE: &str = "./etc/ir/commented_test.ir";
pub(crate) static TEST_ALGO: &str = "./etc/ir/from_docs.ir";
pub(crate) static TEST_INCORRECT: &str = "./etc/ir/missing_opcode.ir";
pub(crate) static TEST_PAR_REF: &str = "./etc/ir/parent_ref.ir";

use crate::prog::*;
use std::str;
use std::fs::File;
use std::io::Read;

use crate::prog::parser::*;

use IRErrType::*;
use crate::obj::*;
use crate::obj::CtxNs::Curr;
use nom_locate::LocatedSpan;

#[derive(Debug)]
pub(crate) struct LoadIRFile<'a> {
    filename: &'a str,
    contents: String,
}

impl<'a> LoadIRFile<'a> {
    pub fn new(name: &'a str) -> Self {
        let mut new = LoadIRFile { filename: name, contents: String::new() };

        let mut file = File::open(name).unwrap();

        file.read_to_string(&mut new.contents).unwrap();

        new
    }

    pub fn load(&self) -> Result<IRMap, IRErr<'a, u32>> {
        ir_load::<u32>(ir_file(ir_input(&self.contents)))
    }
}

#[test]
fn test_microcode_one() {
    let x = LoadIRFile::new(TEST_SIMPLE);

    assert_eq!(
        Ok(vec![Cmd {
            id: "1".into(),
            opcode: CmdArg::Const("ld".into()),
            args: vec![CmdArg::Ref(CtxRef(Curr,
                                          "a".into())),
                       CmdArg::Const("echo".into()),
                       CmdArg::Const("2".into())],
        },
                Cmd {
                    id: "2".into(),
                    opcode: CmdArg::Const("asd".into()),
                    args: vec![],
                },
                Cmd {
                    id: "4".into(),
                    opcode: CmdArg::Const("asd".into()),
                    args: vec![],
                },
                Cmd {
                    id: "5".into(),
                    opcode: CmdArg::Const("echo".into()),
                    args: vec![CmdArg::Const("b".into()),
                               CmdArg::Const("3".into())],
                },
                Cmd {
                    id: "6".into(),
                    opcode: CmdArg::Const("ld".into()),
                    args: vec![CmdArg::Ref(CtxRef(Curr,
                                                  "a".into())),
                               CmdArg::Const("echo".into()),
                               CmdArg::Const("2".into())],
                },
                Cmd {
                    id: "7".into(),
                    opcode: CmdArg::Const("ld".into()),
                    args: vec![CmdArg::Ref(CtxRef(Curr,
                                                  "a".into())),
                               CmdArg::Const("echo".into()),
                               CmdArg::Const("2".into())],
                }]),
        x.load()
    );
}

#[test]
fn test_microcode_algo() {
    let x = LoadIRFile::new(TEST_ALGO);

    assert_eq!(
        Ok(vec![
            Cmd {
                id: "ep".into(),
                opcode: CmdArg::Const("push".into()),
                args: vec![CmdArg::Const("01".into())],
            },
            Cmd {
                id: "01".into(),
                opcode: CmdArg::Const("list_create".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                              "users".into())),
                           CmdArg::Const("02".into())],
            },
            Cmd {
                id: "02".into(),
                opcode: CmdArg::Const("db_user_list".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                              "users".into())),
                           CmdArg::Const("03".into())],
            },
            Cmd {
                id: "03".into(),
                opcode: CmdArg::Const("list_length".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                              "users".into())),
                           CmdArg::Ref(CtxRef(Curr,
                                              "cnt".into())),
                           CmdArg::Const("04".into())],
            },
            Cmd {
                id: "04".into(),
                opcode: CmdArg::Const("set".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                              "i".into())),
                           CmdArg::Const("0".into()),
                           CmdArg::Const("05".into())],
            },
            Cmd {
                id: "05".into(),
                opcode: CmdArg::Const("icmp".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                              "i".into())),
                           CmdArg::Const("<".into()),
                           CmdArg::Ref(CtxRef(Curr,
                                              "cnt".into())),
                           CmdArg::Ref(CtxRef(Curr,
                                              "check".into())),
                           CmdArg::Const("06".into())],
            },
            Cmd {
                id: "06".into(),
                opcode: CmdArg::Const("if".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                              "check".into())),
                           CmdArg::Const("07".into()),
                           CmdArg::Const("10".into())],
            },
            Cmd {
                id: "07".into(),
                opcode: CmdArg::Const("list_get".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                              "users".into())),
                           CmdArg::Ref(CtxRef(Curr,
                                              "i".into())),
                           CmdArg::Const("user_id".into()),
                           CmdArg::Const("08".into())],
            },
            Cmd {
                id: "08".into(),
                opcode: CmdArg::Const("db_user_activate".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                              "user_id".into())),
                           CmdArg::Const("09".into())],
            },
            Cmd {
                id: "09".into(),
                opcode: CmdArg::Const("iadd".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                              "i".into())),
                           CmdArg::Const("1".into()),
                           CmdArg::Const("05".into())],
            },
            Cmd {
                id: "10".into(),
                opcode: CmdArg::Const("usr_op_x".into()),
                args: vec![CmdArg::Const("11".into())],
            },
            Cmd {
                id: "11".into(),
                opcode: CmdArg::Const("http_rep".into()),
                args: vec![CmdArg::Const("200".into()),
                           CmdArg::Const("OK".into()),
                           CmdArg::Ref(CtxRef(Curr,
                                              "req_handle".into()))],
            },
            Cmd {
                id: "50".into(),
                opcode: CmdArg::Const("http_load_handler".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                              "req_handle".into())),
                           CmdArg::Ref(CtxRef(Curr,
                                              "srvr_queue".into())),
                           CmdArg::Const("51".into())],
            },
            Cmd {
                id: "51".into(),
                opcode: CmdArg::Ref(CtxRef(Curr,
                                           "srvr_queue".into())),
                args: vec![],
            }]),
        x.load()
    );
}

#[test]
fn test_xxx() {
    let r = ctxxref(LocatedSpan::new(b"$asd.zxd "));
    assert_eq!(
        r,
        Ok((LocatedSpan { offset: 8, line: 1, fragment: [32].as_ref() }, (LocatedSpan { offset: 1, line: 1, fragment: "asd" }, LocatedSpan { offset: 5, line: 1, fragment: "zxd" })))
    );
}

#[test]
fn test_parent_ref() {
    let x = LoadIRFile::new(TEST_PAR_REF);

    assert_eq!(
        Ok(vec![Cmd {
            id: "ep".into(),
            opcode: CmdArg::Const("push".into()),
            args: vec![CmdArg::Const("01".into()),
                CmdArg::Const("a".into())],
        },
            Cmd {
                id: "ep".into(),
                opcode: CmdArg::Ref(CtxRef(CtxNs::Ref("a".into()),
                                           "b".into())),
                args: vec![CmdArg::Const("01".into()),
                    CmdArg::Const("a".into())],
            },
            Cmd {
                id: "01".into(),
                opcode: CmdArg::Const("set".into()),
                args: vec![CmdArg::Ref(CtxRef(Curr,
                                          "ag".into())),
                    CmdArg::Const("1".into()),
                    CmdArg::Const("02".into())],
            },
            Cmd {
                id: "02".into(),
                opcode: CmdArg::Const("set".into()),
                args: vec![CmdArg::Const("asdasd".into()),
                    CmdArg::Ref(CtxRef(Curr,
                                       "g".into())),
                    CmdArg::Const("asdasdasd".into()),
                    CmdArg::Ref(CtxRef(CtxNs::Ref("a".into()),
                                       "b".into())),
                    CmdArg::Const("asd".into())],
            }]),
        x.load()
    );
}

#[test]
fn test_ir_loader() {
    let x = LoadIRFile::new(TEST_INCORRECT);

    assert_eq!(
        Err(IRErr { location: Location { offset: 46, line: 4 }, code: OpcodeMissing }),
        x.load()
    );
}
