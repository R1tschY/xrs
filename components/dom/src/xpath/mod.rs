use crate::dom::{Document, Element};
use crate::error::{Error, Reason, Result};
use crate::Span;
use std::borrow::Cow;

mod selectors;
mod types;

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

struct XPath {
    steps: Vec<LocationStep>,
}

pub(crate) trait XPathDomExt {
    fn text_value<'a>(&self, doc: &'a Document) -> Result<Cow<'a, str>>;
}

impl XPathDomExt for Element {
    fn text_value<'a>(&self, doc: &'a Document) -> Result<Cow<'a, str>> {
        let mut result = Cow::from(self.text(doc)?);
        for child in self.children() {
            if child.has_tail() {
                result.to_mut().push_str(child.tail(doc)?);
            }
        }
        Ok(result)
    }
}
