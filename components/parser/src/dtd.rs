use crate::PI;

/// Document Type Definition
#[derive(Clone, Debug, PartialEq)]
pub struct DocTypeDecl {
    root_element_name: String,
    external_id: Option<ExternalId>,
    int_subset: Option<IntSubset>,
}

impl DocTypeDecl {
    pub fn new(
        root_element_name: String,
        external_id: Option<ExternalId>,
        int_subset: Option<IntSubset>,
    ) -> Self {
        Self {
            root_element_name,
            external_id,
            int_subset,
        }
    }

    pub fn root_element_name(&self) -> &str {
        &self.root_element_name
    }

    pub fn external_id(&self) -> Option<ExternalId> {
        self.external_id.clone()
    }
    pub fn int_subset(&self) -> &Option<IntSubset> {
        &self.int_subset
    }
}

/// External ID
#[derive(Clone, Debug, PartialEq)]
pub enum ExternalId {
    System { system: String },
    Public { pub_id: String, system: String },
}

/// Internal Subset
#[derive(Clone, Debug, PartialEq)]
pub struct IntSubset {
    decls: Vec<MarkupDeclEntry>,
}

impl IntSubset {
    pub fn new(decls: Vec<MarkupDeclEntry>) -> Self {
        Self { decls }
    }

    pub fn decls(&self) -> &[MarkupDeclEntry] {
        &self.decls
    }
}

/// Entry of Markup Declaration
#[derive(Clone, Debug, PartialEq)]
pub enum MarkupDeclEntry {
    Element(String),
    AttList(String),
    Entity(String),
    Notation(String),
    PI(PI<'static>),
    Comment(String),
    PEReference(String),
}
