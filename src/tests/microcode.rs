//use super::microcode;

#[cfg(test)]
mod tests {
    use crate::microcode::*;

    #[test]
    fn test_microcode_one() {
        dbg!(prefixed(b"hello     "));
        dbg!(string(b"\"beauty\\\"sad\""));
        dbg!(string(b"\"beauty \""));
        dbg!(string(b"\"beauty_\""));
        dbg!(coderef(b"@asd"));
        dbg!(label(b":"));
        dbg!(label(b"1:"));
        dbg!(low_level(b"1: \"a\" \"b\"\n"));
        dbg!(low_level(b"1: $a $b\n"));
        dbg!(low_level(b"1: $a $b\n1: $a $b \n"));
        dbg!(low_level_line(b"1: $aasd $bdsf\n"));
        dbg!(low_level_line(b"1: $asd $basd\n"));
        dbg!(low_level_line(b"1: \"asd\" $basd\n"));
        dbg!(low_level_line(b"1: \"asd\" $basd\n1: \"asd\" $basd\n"));
        dbg!(low_level_line(b"1: $asd $basd"));
        dbg!(low_level_line(b"1: \"asd\" $basd"));
    }
}