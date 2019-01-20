extern crate nom;

use nom::*;
use nom_locate::{position, LocatedSpan};
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

type RawInput<'a> = &'a [u8];
type Input<'a> = LocatedSpan<RawInput<'a>>;
type StrOutput<'a> = LocatedSpan<&'a str>;

pub fn quoted_string(input: Input) -> IResult<Input, StrOutput> {
    delimited!(
        input,
        complete!(char!('\'')),
        map_res!(
          escaped!(take_while1!(is_string), '\\', one_of!("\"n\\")),
          |x| located_span_map_res(x, str::from_utf8)
        ),
        complete!(char!('\''))
    )
}

pub fn string(input: Input) -> IResult<Input, StrOutput> {
    alt_complete!(
        input,
        quoted_string |
        identifier
    )
}


pub fn opt_multispace(input: Input) -> IResult<Input, Option<Input>> {
    opt!(input, complete!(space1))
}

pub fn identifier(input: Input) -> IResult<Input, StrOutput> {
    map_res!(
        input,
        alt_complete!(
            take_while1!(is_ident) |
            terminated!(alphanumeric, peek!(alt!(eof!() | eol)))
        ),
        |x| located_span_map_res(x, str::from_utf8)
    )
}

pub fn coderef(input: Input) -> IResult<Input, StrOutput> {
    do_parse!(
        input,
        complete!(tag!("@")) >>
        id: complete!(identifier) >>
        (id)
    )
}

pub fn ctxref(input: Input) -> IResult<Input, StrOutput> {
    do_parse!(
        input,
        tag!("$") >>
        id: identifier >>
        (id)
    )
}

//fn from_utf(x: LocatedSpan<RawInput>) -> Result<LocatedSpan<&str>, Utf8Error> {
//    let a = str::from_utf8(x.fragment)?;
//    LocatedSpan {
//        line: x.line,
//        offset: x.offset,
//        fragment: a
//    }
//}

fn located_span_from<T, R>(x: LocatedSpan<T>, b: R) -> LocatedSpan<R>
{
    LocatedSpan {
        line: x.line,
        offset: x.offset,
        fragment: b
    }
}

fn located_span_map<T, F, R>(x: LocatedSpan<T>, b: F) -> LocatedSpan<R>
    where F: Fn(T) -> R
{
    LocatedSpan {
        line: x.line,
        offset: x.offset,
        fragment: b(x.fragment)
    }
}

fn located_span_map_res<I, O, E, Fun>(x: LocatedSpan<I>, b: Fun) -> Result<LocatedSpan<O>, E>
    where Fun: Fn(I) -> Result<O, E>
{
    Ok(LocatedSpan {
        line: x.line,
        offset: x.offset,
        fragment: b(x.fragment)?
    })
}

fn located_span_copy<T, A>(x: LocatedSpan<T>, val: LocatedSpan<A>) -> LocatedSpan<A> {
    LocatedSpan {
        line: x.line,
        offset: x.offset,
        fragment: val.fragment
    }
}

pub fn label(input: Input) -> IResult<Input, LocatedSpan<&str>> {
    map_res!(
        input,
        do_parse!(
            pos: position!() >>
            lbl: take_while!(is_ident) >>
            tag!(":") >>
            (located_span_copy(pos, lbl))
        ),
        |x| located_span_map_res(x, str::from_utf8)
    )
}

pub fn ir_arg(input: Input) -> IResult<Input, Located<IRArg>> {
    alt_complete!( input,
        string => { |x| Located::from_span(x).map(|x| IRArg::Const(String::from(x))) } |
        ctxref => { |x| Located::from_span(x).map(|x| IRArg::Ref(String::from(x))) }
    )
}

pub fn ir_command(input: Input) -> IResult<Input, LocatedSpan<(Located<String>, Vec<Located<IRArg>>)>> {
    do_parse!(
        input,
        pos: position!() >>
        label: complete!(label) >>
           opt_multispace >>
        args: separated_list_complete!( complete!(opt_multispace), ir_arg )>>
           opt_multispace >>
           line_ending >>
            ( located_span_from( pos, (Located::from_span(label).map(|x| x.to_string()), args) ) )
    )
}

pub fn ir_comment(input: Input) -> IResult<Input, LocatedSpan<String>> {
    do_parse!(
        input,
        a: map_res!(
            do_parse!(
                pos: position!() >>
                tag!("#") >>
                a: alt_complete!(
                    take_until!("\r\n") |
                    take_until!("\n")
                 ) >>
                 line_ending >>
                ( located_span_copy(pos, a) )
            ),
            |x| located_span_map_res(x, str::from_utf8)
        ) >>
        ( located_span_map(a, String::from) )
    )
}

pub fn ir_empty(input: Input) -> IResult<Input, LocatedSpan<&str>> {
    do_parse!(
        input,
        pos: position!() >>
        opt_multispace >>
        line_ending >>
        ( located_span_copy( pos, LocatedSpan::new( "" ) ) )
    )
}

pub fn ir_file(input: Input) -> IResult<Input, Vec<Located<IRLine>>> {
    complete!(
        input,
        many0!(
            alt_complete!(
                ir_comment => { |x| Located::from_span(x).map(|x| IRLine::Comment(x)) } |
                ir_empty => { |x| Located::from_span(x).map(|_| IRLine::Empty ) } |
                ir_command => { |x| Located::from_span(x).map(|x| {
                        let (label, args) = x;
                        IRLine::Command(label, args)
                    })
                }
            )
        )
    )
}

pub fn ir_input(input: &str) -> Input {
    Input::new(input.as_bytes())
}

#[derive(Debug, Clone, Copy)]
pub struct Location {
    offset: usize,
    line: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Located<T> {
    location: Location,
    item: T,
}

impl <T>Located<T> {
    fn from_span(span: LocatedSpan<T>) -> Located<T> {
        Located {
            location: Location {
                offset: span.offset,
                line: span.line,
            },
            item: span.fragment
        }
    }

    fn map<R, Fun>(self, val: Fun) -> Located<R>
        where Fun: Fn(T) -> R {
        Located {
            location: self.location,
            item: val(self.item)
        }
    }

    fn with_val<V>(self, val: V) -> Located<V> {
        Located {
            location: self.location,
            item: val
        }
    }
}

#[derive(Debug)]
pub enum IRArg {
    Const(String),
    Ref(String),
}

#[derive(Debug)]
pub enum IRLine {
    Command(Located<String>, Vec<Located<IRArg>>),
    Comment(String),
    Empty,
}
