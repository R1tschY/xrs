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

    pub fn tag<'a>(&self, doc: &'a Document) -> Result<&'a str, Utf8Error> {
        from_utf8(self.tag.to_slice(doc.bytes))
    }

    pub(crate) fn tag_bytes<'a>(&self, doc: &'a [u8]) -> &'a [u8] {
        self.tag.to_slice(doc)
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
    pub fn text<'a>(&self, doc: &'a Document) -> Result<&'a str, Utf8Error> {
        self.text.to_str(doc.bytes)
    }

    #[inline]
    pub fn tail<'a>(&self, doc: &'a Document) -> Result<&'a str, Utf8Error> {
        self.tail.to_str(doc.bytes)
    }

    #[inline]
    pub fn text_from_docbytes<'a>(&self, doc: &'a [u8]) -> Result<&'a str, Utf8Error> {
        self.text.to_str(doc)
    }

    #[inline]
    pub fn tail_from_docbytes<'a>(&self, doc: &'a [u8]) -> Result<&'a str, Utf8Error> {
        self.tail.to_str(doc)
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
        self.text = span; // TODO
    }
}
