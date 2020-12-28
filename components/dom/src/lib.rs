use quick_xml::events::{BytesDecl, BytesStart, BytesText, Event};
use quick_xml::{Error as XmlError, Error};
use std::borrow::Cow;
use std::io::Cursor;
use std::str::{from_utf8, Utf8Error};

use self::chars::*;
use dom::{Document, Element};

pub mod chars;
pub mod dom;
pub mod error;
pub mod reader;
pub mod validate;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Span {
    start: usize,
    len: usize,
}

impl Span {
    pub fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }

    pub fn empty() -> Self {
        Self::new(0, 0)
    }

    pub fn to_slice<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        &bytes[self.start..self.start + self.len]
    }

    pub fn to_str<'a>(&self, bytes: &'a [u8]) -> Result<&'a str, Utf8Error> {
        from_utf8(self.to_slice(bytes))
    }

    pub fn is_null(&self) -> bool {
        self.start == 0 && self.len == 0
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

pub struct QName {
    namespace: Option<Span>,
    prefix: Span,
    local_name: Span,
}

impl QName {
    pub fn new(namespace: Option<Span>, prefix: Span, local_name: Span) -> Self {
        Self {
            namespace,
            prefix,
            local_name,
        }
    }
}
