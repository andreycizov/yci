pub use std::str;
pub use std::slice::*;

pub use crate::prog::ir_loader::*;
pub use crate::prog::parser::{located_span_map, located_span_map_res, Input};
use std::str::Utf8Error;
use std::cmp::max;
use std::fmt::Debug;

fn build_offsets(items: &Vec<&str>) -> Vec<usize> {
    let mut ret = Vec::<usize>::with_capacity(items.len());
    let mut offset: usize = 0;
    for it in items.iter() {
        ret.push(
            offset
        );
        offset += it.len() + 1;
    }
    ret
}

fn find_matching_offset(items: &Vec<usize>, offset: usize) -> Option<usize> {
    let mut prev_offset: Option<usize> = None;

    for (idx, x) in items.iter().enumerate() {
        if x > &offset {
            break
        }

        prev_offset = Some(idx);
    }
    prev_offset
}

pub fn format_error<EP: Debug>(input: &Input, err: &IRErr<EP>) -> Result<String, Utf8Error> {
    let input = located_span_map_res(*input, str::from_utf8)?;
    let prog = input.to_string();

    let offset = err.location.offset;

    let items = prog.split("\n").collect();

    let item_offsets = build_offsets(&items);

    let matching_idx = find_matching_offset(&item_offsets, offset).unwrap();

    let match_idx_offset = item_offsets[matching_idx];

    let items: Vec<(usize, &&str)> = items.iter().enumerate().collect();

    let pre = 3;
    let post = 3;

    let idx_start = matching_idx.saturating_sub(pre);
    let idx_end = max(items.len(), matching_idx.saturating_add(post));

    let mut ret: Vec<String> = Vec::<String>::with_capacity(idx_end - idx_start + 1);

    for idx in idx_start..idx_end {
        let lineno = idx + 1;
        let sepa = ": ";
        let lineno_str=  format!("{:4}", lineno) + sepa;
        let lineno_str_len = lineno_str.len();
        let new = lineno_str + items[idx].1;
        ret.push( new);
        if idx == matching_idx {
            let diff = offset - match_idx_offset;

            let spaces = " ".repeat(diff + lineno_str_len);
            let string = spaces.clone() + "^" + "==================================";

            ret.push(string);


            let err_fmtd = spaces + &format!("{:?}", err.code);
            ret.push(err_fmtd);
        }
    }


    Ok(ret.join("\n"))
}