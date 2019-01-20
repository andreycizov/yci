//use super::microcode;

static TEST_SIMPLE: &str = "# first line
# second line
1:ld $a echo 2
# third line
2: 'asd'


4: 'asd'
# third line

5: 'echo' 'b' '3'
# fourth line

6:'ld' $a 'echo' '2'
7:'ld' $a 'echo' '2'
";

static TEST_ALGO: &str = "
# [http_hdlr_root]
01: list_create $users 02
02: db_user_list $users 03
03: list_length $users $cnt 04
04: set $i 0 05
05: cmp $i '<' $cnt $check 06
06: if $check 07 10
07: list_get $users $i $user_id 08
08: db_user_activate $user_id 09
09: add $i 1 05
10: usr_op_x 11
# should we actually file a new thread here ?
# and set a timer
# then if the timer is done, then the worker had timed out
# and we can kill the replier thread.
11: http_rep 200 OK $req_handle


# [http_hdlr_rep]
50: http_load_handler $req_handle $srvr_queue 51
51: $srvr_queue
";

#[cfg(test)]
mod tests {
    use crate::microcode::*;
    use std::str;

    #[test]
    fn test_microcode_one() {
        dbg!(string(ir_input("abcdef\n")));
        dbg!(ir_command(ir_input("1:ld $a echo 2\r\n")));
        dbg!(ir_command(ir_input("3: \'echo\' \'b\' \'3\'\n")));
        dbg!(super::TEST_SIMPLE);
        let x = ir_file(ir_input(super::TEST_SIMPLE)).unwrap();
        dbg!(str::from_utf8(x.0.fragment).unwrap());
        dbg!(x.1);
//        dbg!(prefixed(b"hello     "));
//        dbg!(string(b"\"beauty\\\"sad\""));
//        dbg!(string(b"\"beauty \""));
//        dbg!(string(b"\"beauty_\""));
//        dbg!(coderef(b"@asd"));
//        dbg!(label(b":"));
//        dbg!(label(b"1:"));
//        dbg!(low_level(b"1: \"a\" \"b\"\n"));
//        dbg!(low_level(b"1: $a $b\n"));
//        dbg!(low_level(b"1: $a $b\n1: $a $b \n"));
//        dbg!(ir_command(b"1: $aasd $bdsf\n2: 'asda'"));
//        dbg!(ir_file(b"1: $aasd $bdsf\n2: 'asda' $sir 'asd'\n"));
//        dbg!(low_level_line(b"1: $asd $basd\n"));
//        dbg!(low_level_line(b"1: \"asd\" $basd\n"));
//        dbg!(low_level_line(b"1: \"asd\" $basd\n1: \"asd\" $basd\n"));
//        dbg!(low_level_line(b"1: $asd $basd"));
//        dbg!(low_level_line(b"1: \"asd\" $basd"));

    }

    #[test]
    fn test_microcode_algo() {
        let x = ir_file(ir_input(super::TEST_ALGO)).unwrap();
        dbg!(x.1);
    }
}