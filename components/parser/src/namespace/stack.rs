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

    pub fn build_scope(&mut self) -> NamespaceStackScopeBuilder {
        NamespaceStackScopeBuilder {
            stack: self,
            size: 0,
        }
    }

    pub fn pop_scope(&mut self) {
        let scope_namespaces = self.stack.pop().expect("TODO: stack underflow");
        self.namespaces
            .truncate(self.namespaces.len() - scope_namespaces);
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

    pub fn finish(self) -> &'a mut NamespaceStack {
        self.stack.stack.push(self.size);
        self.stack
    }
}

impl Default for NamespaceStack {
    fn default() -> Self {
        Self::new()
    }
}
