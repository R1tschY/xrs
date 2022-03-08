use crate::namespace::NamespaceDecl;

pub struct NamespaceStack {
    namespaces: Vec<NamespaceDecl>,
    stack: Vec<usize>,
}

impl NamespaceStack {
    pub fn new() -> Self {
        Self {
            namespaces: vec![],
            stack: vec![],
        }
    }

    pub fn begin_scope(&mut self) -> NamespaceStackScopeBuilder {
        NamespaceStackScopeBuilder {
            stack: self,
            size: 0,
        }
    }
}

pub struct NamespaceStackScopeBuilder<'a> {
    stack: &'a mut NamespaceStack,
    size: usize,
}

impl<'a> NamespaceStackScopeBuilder<'a> {
    pub fn add(&mut self, namespace: NamespaceDecl) {
        self.stack.namespaces.push(namespace);
        self.size += 1;
    }

    pub fn add_prefix(&mut self, prefix: impl Into<String>, uri: impl Into<String>) {
        self.add(NamespaceDecl::new(prefix.into(), uri.into()));
    }

    pub fn end(self) -> &'a mut NamespaceStack {
        self.stack.stack.push(self.size);
        self.stack
    }
}

impl Default for NamespaceStack {
    fn default() -> Self {
        Self::new()
    }
}
