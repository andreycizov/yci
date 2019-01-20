extern crate nom;

use nom::*;
use std::str;
use super::super::obj::*;

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

pub fn quoted_string(input: &[u8]) -> IResult<&[u8], &str> {
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

pub fn string(input: &[u8]) -> IResult<&[u8], &str> {
    alt_complete!(
        input,
        quoted_string |
        identifier
    )
}


pub fn opt_multispace(input: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    opt!(input, complete!(space1))
}

pub fn identifier(input: &[u8]) -> IResult<&[u8], &str> {
    map_res!(
        input,
        alt_complete!(
            take_while1!(is_ident) |
            terminated!(alphanumeric, peek!(alt!(eof!() | eol)))
        ),
        str::from_utf8
    )
}

pub fn coderef(input: &[u8]) -> IResult<&[u8], &str> {
    do_parse!(
        input,
        complete!(tag!("@")) >>
        id: complete!(identifier) >>
        (id)
    )
}

pub fn ctxref(input: &[u8]) -> IResult<&[u8], &str> {
    do_parse!(
        input,
        tag!("$") >>
        id: identifier >>
        (id)
    )
}

pub fn label(input: &[u8]) -> IResult<&[u8], &str> {
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

pub fn param(input: &[u8]) -> IResult<&[u8], CommandArgument> {
    alt_complete!( input,
        string => { |x| CommandArgument::Const(ContextValue::from(x)) } |
        ctxref => { |x| CommandArgument::Ref(ContextIdent::from(x)) }
    )
}

pub fn ir_command(input: &[u8]) -> IResult<&[u8], Command> {
    do_parse!(
        input,
        a: complete!(label) >>
           opt_multispace >>
        b: param  >>
           opt_multispace >>
        c: separated_list_complete!( complete!(opt_multispace), param )>>
           opt_multispace >>
           line_ending >>
            ( Command::create(CommandId::from(a), b, c) )
    )
}

pub fn ir_comment(input: &[u8]) -> IResult<&[u8], String> {
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

pub fn ir_empty(input: &[u8]) -> IResult<&[u8], ()> {
    do_parse!(
        input,
        line_ending >>
        ( () )
    )
}

pub fn ir_file(input: &[u8]) -> IResult<&[u8], Vec<IRLine>> {
    complete!(
        input,
        many0!(
            alt_complete!(
                ir_comment => { |x| IRLine::Comment(x) } |
                ir_empty => { |_| IRLine::Empty } |
                ir_command => { |x| IRLine::Command(x) }
            )
        )
    )
}

#[derive(Debug)]
pub enum IRLine {
    Command(Command),
    Comment(String),
    Empty,
}
