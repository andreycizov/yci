extern crate nom;

use nom::*;
use std::str;

fn is_symbol(chr: u8) -> bool {
    (chr >= 32 && chr <= 47) ||
    (chr >= 58 && chr <= 64) ||
    (chr >= 91 && chr <= 96) ||
    (chr >= 123 && chr <= 126)
}

fn is_string(chr: u8) -> bool {
    chr != b'\\' && chr != b'\'' &&
        (
            is_alphanumeric(chr) || is_space(chr) || is_symbol(chr)
        )
}

fn is_ident(chr: u8) -> bool {
    is_alphanumeric(chr) || chr == b'_'
}

fn is_not_eol(chr: u8) -> bool {
    chr == b'\n'
}

type Input<'a> = &'a [u8];

pub fn quoted_string(input: Input) -> IResult<Input, &str> {
    delimited!(
        input,
        complete!(char!('\'')),
        map_res!(
          escaped!(take_while1!(is_string), '\\', one_of!("\"n\\")),
          str::from_utf8
        ),
        complete!(char!('\''))
    )
}

pub fn string(input: Input) -> IResult<Input, &str> {
    alt_complete!(
        input,
        quoted_string |
        identifier
    )
}


pub fn opt_multispace(input: Input) -> IResult<Input, Option<Input>> {
    opt!(input, complete!(space1))
}

pub fn identifier(input: Input) -> IResult<Input, &str> {
    map_res!(
        input,
        alt_complete!(
            take_while1!(is_ident) |
            terminated!(alphanumeric, peek!(alt!(eof!() | eol)))
        ),
        str::from_utf8
    )
}

pub fn coderef(input: Input) -> IResult<Input, &str> {
    do_parse!(
        input,
        complete!(tag!("@")) >>
        id: complete!(identifier) >>
        (id)
    )
}

pub fn ctxref(input: Input) -> IResult<Input, &str> {
    do_parse!(
        input,
        tag!("$") >>
        id: identifier >>
        (id)
    )
}

pub fn label(input: Input) -> IResult<Input, &str> {
    map_res!(
        input,
        do_parse!(
            lbl: take_while!(is_ident) >>
            tag!(":") >>
            (lbl)
        ),
        str::from_utf8
    )
}

pub fn ir_arg(input: Input) -> IResult<Input, IRArg> {
    alt_complete!( input,
        string => { |x| IRArg::Const(String::from(x)) } |
        ctxref => { |x| IRArg::Ref(String::from(x)) }
    )
}

pub fn ir_command(input: Input) -> IResult<Input, (String, Vec<IRArg>)> {
    do_parse!(
        input,
        label: complete!(label) >>
           opt_multispace >>
        args: separated_list_complete!( complete!(opt_multispace), ir_arg )>>
           opt_multispace >>
           line_ending >>
            ( (label.to_string(), args) )
    )
}

pub fn ir_comment(input: Input) -> IResult<Input, String> {
    do_parse!(
        input,
        a: map_res!(
            do_parse!(
                tag!("#") >>
                a: alt_complete!(
                    take_until!("\r\n") |
                    take_until!("\n")
                 ) >>
                 line_ending >>
                ( a )
            ),
            str::from_utf8
        ) >>
        ( String::from(a) )
    )
}

pub fn ir_empty(input: Input) -> IResult<Input, ()> {
    do_parse!(
        input,
        opt_multispace >>
        line_ending >>
        ( () )
    )
}

pub fn ir_file(input: Input) -> IResult<Input, Vec<IRLine>> {
    complete!(
        input,
        many0!(
            alt_complete!(
                ir_comment => { |x| IRLine::Comment(x) } |
                ir_empty => { |_| IRLine::Empty } |
                ir_command => { |x| {
                        let (label, args) = x;
                        IRLine::Command(label, args)
                    }
                }
            )
        )
    )
}

#[derive(Debug)]
pub enum IRArg {
    Const(String),
    Ref(String),
}

#[derive(Debug)]
pub enum IRLine {
    Command(String, Vec<IRArg>),
    Comment(String),
    Empty,
}
