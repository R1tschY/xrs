use crate::PI;

/// Document Type Definition
#[derive(Clone, Debug, PartialEq)]
pub struct DocTypeDecl<'a> {
    root_element_name: &'a str,
    external_id: Option<ExternalId<'a>>,
    int_subset: Option<IntSubset<'a>>,
}

impl<'a> DocTypeDecl<'a> {
    pub fn new(
        root_element_name: &'a str,
        external_id: Option<ExternalId<'a>>,
        int_subset: Option<IntSubset<'a>>,
    ) -> Self {
        Self {
            root_element_name,
            external_id,
            int_subset,
        }
    }

    pub fn root_element_name(&self) -> &'a str {
        self.root_element_name
    }

    pub fn external_id(&self) -> Option<ExternalId<'a>> {
        self.external_id.clone()
    }
    pub fn int_subset(&self) -> &Option<IntSubset<'a>> {
        &self.int_subset
    }
}

/// External ID
#[derive(Clone, Debug, PartialEq)]
pub enum ExternalId<'a> {
    System { system: &'a str },
    Public { pub_id: &'a str, system: &'a str },
}

/// Internal Subset
#[derive(Clone, Debug, PartialEq)]
pub struct IntSubset<'a> {
    decls: Vec<MarkupDeclEntry<'a>>,
}

impl<'a> IntSubset<'a> {
    pub fn new(decls: Vec<MarkupDeclEntry<'a>>) -> Self {
        Self { decls }
    }

    pub fn decls(&self) -> &[MarkupDeclEntry<'a>] {
        &self.decls
    }
}

/// Entry of Markup Declaration
#[derive(Clone, Debug, PartialEq)]
pub enum MarkupDeclEntry<'a> {
    Element(&'a str),
    AttList(&'a str),
    Entity(&'a str),
    Notation(&'a str),
    PI(PI<'a>),
    Comment(&'a str),
    PEReference(&'a str),
}
