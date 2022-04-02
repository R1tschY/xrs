#![allow(dead_code)]

use std::borrow::Cow;

use xml_dom::error::Result as DomResult;
use xml_dom::{Document, Element};

use crate::ast::Expr;

mod ast;
mod token;

pub(crate) mod datamodel;
pub(crate) mod dom;
pub(crate) mod select;

mod functions;

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

#[derive(Debug, PartialEq, Clone)]
pub enum XPathError {
    WrongFunctionArgument(String),
    CallToUndefinedFunction(String),
}

struct XPath {
    expr: Expr,
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
