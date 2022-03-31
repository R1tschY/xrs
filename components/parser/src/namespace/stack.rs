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

    pub fn resolve(&self, prefix: &str) -> Option<&str> {
        self.namespaces
            .iter()
            .rev()
            .find(|ns| matches!(ns.prefix.as_ref(), Some(prefix)))
            .map(|ns| &ns.uri as &str)
    }

    pub fn resolve_default(&self) -> Option<&str> {
        self.namespaces
            .iter()
            .rev()
            .find(|ns| ns.prefix.is_none())
            .map(|ns| &ns.uri as &str)
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

    pub fn add_prefix(&mut self, prefix: Option<String>, uri: String) {
        self.add(NamespaceDecl::new(prefix, uri));
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
