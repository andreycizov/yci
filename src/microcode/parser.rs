extern crate nom;

use nom::*;
use nom::types::{CompleteStr, CompleteByteSlice};
use std::str;

//escaped![];

named!(pub prefixed, preceded!(tag!("hello"), take!(5)));

fn is_string(chr: u8) -> bool {
    is_alphanumeric(chr) || is_space(chr) || chr == b'_'
}

fn is_ident(chr: u8) -> bool {
    is_alphanumeric(chr) || chr == b'_' //is_alphanumeric(chr) || chr == b'_'
}

named!(
  pub string<&str>,

  delimited!(
    complete!(char!('"')),
    //map_res!(escaped!(call!(alphanumeric), '\\', is_a!("\"n\\")), str::from_utf8),
    map_res!(
      escaped!(take_while1!(is_string), '\\', one_of!("\"n\\")),
      str::from_utf8
    ),
    complete!(char!('"'))
  )
);


fn check_ident(i: u8) -> Option<u8> {
   if is_ident(i) {
       Some(i)
   } else {
       None
   }
}


named!(pub opt_multispace<&[u8], Option<&[u8]>>,
       opt!(complete!(space1))
);

pub fn end_of_line(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt!(input, eof!() | eol)
}

pub fn read_line(input: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated!(input, alphanumeric, peek!(end_of_line))
}

named!(
    pub identifier<&str>,
    map_res!(
        alt_complete!(
            take_while1!(is_ident) |
            read_line
        )
        ,
        str::from_utf8
    )
);

named!(
    pub coderef<&str>,
    do_parse!(
        complete!(tag!("@")) >>
        id: complete!(identifier) >>
        (id)
    )
);

named!(
    pub ctxref<&str>,
    do_parse!(
        tag!("$") >>
        id: identifier >>
        (id)
    )
);


named!(
    pub label<&str>,
    map_res!(
        do_parse!(
            lbl: take_until1!(":") >>
            tag!(":") >>
            (lbl)
        ),
        str::from_utf8
    )
);

use std;

// this works but no EOF
named!(
    pub low_level_line<(&str, &str, std::vec::Vec<&str>)>,
    do_parse!(
        a: complete!(label) >>
        opt_multispace >>
        b: alt_complete!( string | ctxref )  >>
        opt_multispace >>
        c: separated_list_complete!( complete!(multispace), alt_complete!( string | ctxref ) )>>
        opt_multispace >>
        opt!(peek!(
            line_ending
        )) >>
        (a, b, c)
    )

);

named!(
    pub low_level_line_b<(&str, &str)>,
    ws!(
        do_parse!(
            a: label >>
            c: ctxref >>

            (a, c)
        )
    )
);

#[derive(Debug)]
pub enum Line<'a>{
    Line(&'a str, &'a str, std::vec::Vec<&'a str>),
    Empty
}

named!(
    pub low_level<std::vec::Vec<Line>>,
    do_parse!(
        mn: separated_list_complete!(
            complete!(line_ending),
            do_parse!(
                a: alt_complete!(
                    do_parse!(
                        lll: low_level_line >>
                        (Line::Line(lll.0, lll.1, lll.2))
                    ) |
                    do_parse!(
                        opt_multispace >>
                        (Line::Empty)
                    )
                ) >>
                (a)
            )
        ) >>
        (mn)
    )
);
