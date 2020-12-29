use crate::error::{Error, Reason, Result};
use crate::Span;
use std::str::{from_utf8, Utf8Error};

pub struct Document<'a> {
    bytes: &'a [u8],
    root: Element,
}

impl<'a> Document<'a> {
    pub fn new(bytes: &'a [u8], root: Element) -> Self {
        Self { bytes, root }
    }

    pub fn get_root(&self) -> &Element {
        &self.root
    }
}

pub struct Element {
    offset: usize,
    tag: Span,
    text: Span,
    tail: Span,
    children: Vec<Element>,
    attributes: Span,
    namespaces: Option<Vec<(Span, Span)>>,
}

fn decode(span: Span, doc: &[u8]) -> Result<&str> {
    span.to_str(doc)
        .map_err(|err| Error::new(span, Reason::Utf8(err)))
}

impl Element {
    // new

    pub fn new(offset: usize, tag_len: usize, attrs_len: usize) -> Self {
        Self {
            offset,
            tag: Span::new(offset + 1, tag_len),
            text: Span::empty(),
            tail: Span::empty(),
            children: vec![],
            attributes: Span::new(offset + 1 + tag_len, attrs_len),
            namespaces: None,
        }
    }

    // tag

    pub fn tag<'a>(&self, doc: &Document<'a>) -> Result<&'a str> {
        decode(self.tag, doc.bytes)
    }

    pub fn tag_span(&self) -> Span {
        self.tag
    }

    // children

    pub fn children(&self) -> &[Element] {
        &self.children
    }

    pub fn children_mut(&mut self) -> &mut [Element] {
        &mut self.children
    }

    // text / tail

    #[inline]
    pub fn text<'a>(&self, doc: &Document<'a>) -> Result<&'a str> {
        decode(self.text, doc.bytes)
    }

    #[inline]
    pub fn tail<'a>(&self, doc: &Document<'a>) -> Result<&'a str> {
        decode(self.tail, doc.bytes)
    }

    #[inline]
    pub fn text_from_docbytes<'a>(&self, doc: &'a [u8]) -> Result<&'a str> {
        decode(self.text, doc)
    }

    #[inline]
    pub fn tail_from_docbytes<'a>(&self, doc: &'a [u8]) -> Result<&'a str> {
        decode(self.tail, doc)
    }

    pub fn has_text(&self) -> bool {
        !self.text.is_empty()
    }

    pub fn has_tail(&self) -> bool {
        !self.tail.is_empty()
    }

    // manipulators

    pub fn push_child(&mut self, element: Element) {
        self.children.push(element);
    }

    pub fn push_text(&mut self, span: Span) {
        self.text = span; // TODO
    }

    pub fn push_tail(&mut self, span: Span) {
        self.tail = span; // TODO
    }
}
