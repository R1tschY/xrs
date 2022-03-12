pub struct DtdTypeDecl<'a> {
    name: &'a str,
}

impl<'a> DtdTypeDecl<'a> {
    pub fn new(name: &'a str) -> Self {
        Self { name }
    }

    pub fn name(&self) -> &'a str {
        self.name
    }
}
