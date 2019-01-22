use nom::{IResult, Err as NomErr, Context as NomContext};

use crate::prog::parser::*;
use crate::obj::*;

type IRMap = Vec<Command>;

#[derive(Debug)]
pub struct IRErr<'a, EP> {
    pub location: Location,

    pub code: IRErrType<'a, EP>,
}


impl<'a, EP> IRErr<'a, EP> {
    pub fn new(location: Location, code: IRErrType<'a, EP>) -> Self {
        IRErr { location, code }
    }
}

#[derive(Debug)]
pub enum IRErrType<'a, EP> {
    // file not fully parsed
    AdditionalData,

    // command contains 0 arguments
    OpcodeMissing,

    //
    ParserError(NomErr<Input<'a>, EP>),
    ParserFailure,
}

pub fn ir_load<'a, EP>(file: IResult<Input<'a>, IRFile, EP>) -> Result<IRMap, IRErr<'a, EP>> {
    let file = match file {
        Ok((prepend, res)) => {
            if prepend.fragment.len() > 0 {
                return Err(
                    IRErr::new(
                        Location::from_span(&prepend),
                        IRErrType::AdditionalData,
                    )
                );
            }

            res
        }
        Err(nom_err) => {
            let null_loc = Location::new(0, 0);

            let loc = match &nom_err {
                NomErr::Incomplete(_) => {
                    null_loc
                }
                NomErr::Error(ctx) => {
                    match ctx {
                        NomContext::Code(prepend, _) => {
                            Location::from_span(&prepend)
                        }
// [verbose_errors_only]
//                        NomContext::List(items) => {
//                            items.first().map(|x| {
//                                let (prepend, b) = x;
//                                Location::from_span(&prepend);
//                            }).unwrap_or(
//                                null_loc
//                            )
//                        }
                    }
                }
                NomErr::Failure(ctx) => {
                    match ctx {
                        NomContext::Code(prepend, _) => {
                            Location::from_span(&prepend)
                        }
// [verbose_errors_only]
//                        NomContext::List(items) => {
//                            items.first().map(|x| {
//                                let (prepend, b) = x;
//                                Location::from_span(&prepend);
//                            }).unwrap_or(
//                                null_loc
//                            )
//                        }
                    }
                }
            };

            return Err(IRErr::new(
                loc,
                IRErrType::ParserError(nom_err),
            ));
        }
    };

    let go = || file.iter().filter(|x| match &x.item {
        IRLine::Command(_, _) => {
            true
        }
        _ => {
            false
        }
    });

    let len = go().count();

    let mut res = IRMap::with_capacity(len);

    for item in go() {
        match &item.item {
            IRLine::Command(key, args) => {
                let map_param = |x: &IRArg| match x {
                    IRArg::Const(z) => CommandArgument::Const(z.clone()),
                    IRArg::Ref(z) => CommandArgument::Ref(z.clone()),
                };

                let opcode = args.first();


                if let Some(opcode_mapped) = opcode {
                    let params = args[1..].iter();
                    let params = params.map(|x| &x.item);
                    let params = params.map(map_param);
                    let params = params.collect();

                    res.push(
                        Command::create(
                            key.item.clone(),
                            map_param((&opcode_mapped.item).clone()),
                            params,
                        )
                    )
                } else {
                    return Err(IRErr {
                        location: item.location.clone(),
                        code: IRErrType::OpcodeMissing,
                    });
                }
            }
            _ => {}
        }
    }

    Ok(res)
}