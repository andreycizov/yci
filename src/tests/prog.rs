static TEST_SIMPLE: &str = "./etc/ir/commented_test.ir";
static TEST_ALGO: &str = "./etc/ir/from_docs.ir";
static TEST_INCORRECT: &str = "./etc/ir/missing_opcode.ir";

#[cfg(test)]
mod tests {
    use crate::prog::*;
    use std::str;
    use std::fs::File;
    use std::io::Read;

    fn load_ir_file<'a>(name: &str) -> IRFile {
        let mut file = File::open(name).unwrap();
        let mut contents = String::new();

        file.read_to_string(&mut contents).unwrap();

        let ret = ir_input(&contents);

        ir_file(ret).unwrap().1
    }

    #[test]
    fn test_microcode_one() {
        //dbg!(string(ir_input("abcdef\n")));
        //dbg!(ir_command(ir_input("1:ld $a echo 2\r\n")));
        //dbg!(ir_command(ir_input("3: \'echo\' \'b\' \'3\'\n")));
        //dbg!(super::TEST_SIMPLE);
        let x = load_ir_file(super::TEST_SIMPLE);
        //dbg!(str::from_utf8(x.0.fragment).unwrap());
       // dbg!(x.1);
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
        let x = load_ir_file(super::TEST_ALGO);
        //dbg!(x.1);
    }

    #[test]
    fn test_ir_loader() {
        let x = load_ir_file(super::TEST_INCORRECT);
        //let y = x.unwrap();
        dbg!(x);
        //let y = ir_load(x);
        //dbg!(x.1);
    }
}