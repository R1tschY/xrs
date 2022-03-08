use std::fmt;
use std::str::from_utf8;
use std::sync::Arc;

pub mod parser;
pub mod stack;

#[derive(Clone, Debug, PartialEq)]
pub struct QName<'a> {
    prefix: Option<&'a str>,
    local_part: &'a str,
}

pub type Namespace = Arc<NamespaceDecl>;

pub struct NamespaceDecl {
    prefix: String,
    uri: String,
}

impl NamespaceDecl {
    fn new(prefix: String, uri: String) -> Self {
        Self { prefix, uri }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NsSTagStart<'a> {
    qname: QName<'a>,
}

#[derive(Clone, PartialEq)]
pub struct NsAttribute<'a> {
    qname: QName<'a>,
    raw_value: &'a str,
}

impl<'a> NsAttribute<'a> {
    pub fn raw_value(&self) -> &str {
        self.raw_value
    }

    pub fn qname(&self) -> QName<'a> {
        self.qname.clone()
    }
}

impl<'a> fmt::Debug for NsAttribute<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Attribute")
            .field("name", &self.qname)
            .field("value", &self.raw_value)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct STagEnd<'a> {
    qname: QName<'a>,
}

impl<'a> STagEnd<'a> {
    pub fn qname(&self) -> QName<'a> {
        self.qname.clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NsETag<'a> {
    qname: QName<'a>,
}

impl<'a> NsETag<'a> {
    pub fn name(&self) -> QName<'a> {
        self.qname.clone()
    }
}

/// XML event with namespace parsing
#[derive(Clone, Debug, PartialEq)]
pub enum XmlNsEvent<'a> {
    STagStart(NsSTagStart<'a>),
    Attribute(NsAttribute<'a>),
    STagEnd,
    ETag(NsETag<'a>),
    STagEndEmpty,
    Characters(&'a str),
}
