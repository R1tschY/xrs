use crate::dom::{Document, Element};
use crate::error;
use crate::error::{Error, Reason};

pub trait Selector {
    fn select<'a, 'b, 'c, T: Iterator<Item = &'a Element> + 'a>(
        &'a self,
        iter: T,
        doc: &'b Document<'c>,
    ) -> Box<dyn Iterator<Item = error::Result<&'a Element>> + 'a>
    where
        'b: 'a;
}

pub struct AnyChild;

impl Selector for AnyChild {
    fn select<'a, 'b, 'c, T: Iterator<Item = &'a Element> + 'a>(
        &'a self,
        iter: T,
        doc: &'b Document<'c>,
    ) -> Box<dyn Iterator<Item = error::Result<&'a Element>> + 'a>
    where
        'b: 'a,
    {
        Box::new(iter.flat_map(|elem| elem.children().iter().map(Ok)))
    }
}

pub struct ChildWithName(String);

impl Selector for ChildWithName {
    fn select<'a, 'b, 'c, T: Iterator<Item = &'a Element> + 'a>(
        &'a self,
        iter: T,
        doc: &'b Document<'c>,
    ) -> Box<dyn Iterator<Item = error::Result<&'a Element>> + 'a>
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
                    Err(err) => Some(Err(err)),
                })
        }))
    }
}
