struct A {
    a: String
}

impl A {
    fn e(&self) -> String {
        self.a.clone()
    }

    fn h(&mut self) {
        self.a = "as".to_string();
    }

    fn g(&self) {
        println!("{}", self.e());
        self.a();
        println!("{}", self.e());
    }
}