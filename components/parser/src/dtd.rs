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
    pub fn internal_subset(&self) -> &Option<IntSubset> {
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

/// Element Type Declaration
///
/// Section 3.2
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub name: String,
    pub content_spec: ContentSpec,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContentSpec {
    Empty,
    Any,
    Mixed(Vec<String>),
    PCData,
    Children(ContentParticle),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContentParticle {
    pub entry: ContentParticleEntry,
    pub repetition: Repetition,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContentParticleEntry {
    Name(String),
    Choice(Vec<ContentParticle>),
    Seq(Vec<ContentParticle>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Repetition {
    One,
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntityDef {
    Internal(String),
    External {
        external_id: ExternalId,
        ndata: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GEDecl {
    pub name: String,
    pub def: EntityDef,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PEDef {
    Internal(String),
    External(ExternalId),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PEDecl {
    pub name: String,
    pub def: PEDef,
}

/// Entry of Markup Declaration
#[derive(Clone, Debug, PartialEq)]
pub enum MarkupDeclEntry {
    Element(Element),
    AttList(String),
    GeneralEntity(GEDecl),
    ParameterEntity(PEDecl),
    Notation(String),
    PI(PI<'static>),
    Comment(String),
    PEReference(String),
}

impl MarkupDeclEntry {
    pub fn new_element(name: String, content: ContentSpec) -> Self {
        Self::Element(Element {
            name,
            content_spec: content,
        })
    }

    pub fn new_entity(name: String, def: EntityDef) -> Self {
        Self::GeneralEntity(GEDecl { name, def })
    }
}
