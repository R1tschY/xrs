use std::borrow::Cow;

use xml_dom::error::Error as DomError;
use xml_dom::error::Result as DomResult;
use xml_dom::{Document, Element};

mod ast;
mod characters;
mod token;

mod selectors;
mod types;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Span {
    lo: usize,
    hi: usize,
}

impl Span {
    pub(crate) fn new(lo: usize, hi: usize) -> Self {
        Self { lo, hi }
    }
}

enum Axis {
    Ancestor,
    AncestorOrSelf,
    Attribute,
    Child,
    Descendant,
    DescendantOrSelf,
    Following,
    FollowingSibling,
    Namespace,
    Parent,
    Preceding,
    PrecedingSibling,
    Self_,
}
enum NodeTest {
    Name(String),
    AnyName,
    AnyNameInNamespace(String),
    Comment,
    Text,
    ProcessingInstruction(Option<String>),
    Node,
}

struct Predicate {}

struct LocationStep {
    axis: Axis,
    node_test: NodeTest,
    predicates: Vec<Predicate>,
}

pub enum XPathError {
    Dom(DomError),
}

impl From<DomError> for XPathError {
    fn from(value: DomError) -> Self {
        Self::Dom(value)
    }
}

struct XPath {
    steps: Vec<LocationStep>,
}

pub(crate) trait XPathDomExt {
    fn text_value<'a>(&self, doc: &'a Document) -> DomResult<Cow<'a, str>>;
}

impl XPathDomExt for Element {
    fn text_value<'a>(&self, doc: &'a Document) -> DomResult<Cow<'a, str>> {
        let mut result = Cow::from(self.text(doc)?);
        for child in self.children() {
            if child.has_tail() {
                result.to_mut().push_str(child.tail(doc)?);
            }
        }
        Ok(result)
    }
}
