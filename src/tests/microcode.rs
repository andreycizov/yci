//use super::microcode;

static TEST_SIMPLE: &str = "# first line
1: ld a 'echo' 2
2'
2: echo 'b' 3
";

#[cfg(test)]
mod tests {
    use crate::microcode::*;

    #[test]
    fn test_microcode_one() {
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
        dbg!(ir_command(b"1: $aasd $bdsf\n2: 'asda'"));
        dbg!(ir_file(b"1: $aasd $bdsf\n2: 'asda' $sir 'asd'\n"));
//        dbg!(low_level_line(b"1: $asd $basd\n"));
//        dbg!(low_level_line(b"1: \"asd\" $basd\n"));
//        dbg!(low_level_line(b"1: \"asd\" $basd\n1: \"asd\" $basd\n"));
//        dbg!(low_level_line(b"1: $asd $basd"));
//        dbg!(low_level_line(b"1: \"asd\" $basd"));
    }
}