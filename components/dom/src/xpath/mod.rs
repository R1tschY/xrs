use crate::dom::{Document, Element};
use crate::error::{Error, Reason, Result};
use crate::Span;

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

trait Selector {
    fn select<'a, 'b, 'c, T: Iterator<Item = &'a Element> + 'a>(
        &'a self,
        iter: T,
        doc: &'b Document<'c>,
    ) -> Box<dyn Iterator<Item = Result<&'a Element>> + 'a>
    where
        'b: 'a;
}

struct AnyChild;

impl Selector for AnyChild {
    fn select<'a, 'b, 'c, T: Iterator<Item = &'a Element> + 'a>(
        &'a self,
        iter: T,
        doc: &'b Document<'c>,
    ) -> Box<dyn Iterator<Item = Result<&'a Element>> + 'a>
    where
        'b: 'a,
    {
        Box::new(iter.flat_map(|elem| elem.children().iter().map(Ok)))
    }
}

struct ChildWithName(String);

impl Selector for ChildWithName {
    fn select<'a, 'b, 'c, T: Iterator<Item = &'a Element> + 'a>(
        &'a self,
        iter: T,
        doc: &'b Document<'c>,
    ) -> Box<dyn Iterator<Item = Result<&'a Element>> + 'a>
    where
        'b: 'a,
    {
        Box::new(iter.flat_map(move |elem| {
            elem.children()
                .iter()
                .filter_map(move |child| match child.tag(doc) {
                    Ok(tag) => {
                        if tag == self.0 {
                            Some(Ok(child))
                        } else {
                            None
                        }
                    }
                    Err(err) => Some(Err(Error::new(child.tag_span(), Reason::Utf8(err)))),
                })
        }))
    }
}
