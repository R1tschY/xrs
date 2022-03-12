#![allow(unused)]

use parser::cursor::Cursor;
use std::fmt::Formatter;
use std::fs::read_to_string;
use std::str::from_utf8;
use std::{fmt, io};
use xml_chars::XmlAsciiChar;
use xml_chars::XmlChar;

use crate::XmlError::{ExpectedElementEnd, ExpectedName};

mod dtd;
mod namespace;
pub mod parser;
mod reader;
mod shufti;

#[derive(Clone, Debug, PartialEq)]
pub struct XmlDecl<'a> {
    version: &'a str,
    encoding: Option<&'a str>,
    standalone: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct STag<'a> {
    name: &'a str,
    empty: bool,
}

impl<'a> STag<'a> {
    pub fn name(&self) -> &'a str {
        self.name
    }
}

#[derive(Clone, PartialEq)]
pub struct Attribute<'a> {
    name: &'a str,
    raw_value: &'a str,
}

impl<'a> Attribute<'a> {
    pub fn new(name: &'a str, raw_value: &'a str) -> Self {
        Self { name, raw_value }
    }

    pub fn raw_value(&self) -> &'a str {
        self.raw_value
    }

    pub fn name(&self) -> &'a str {
        self.name
    }
}

impl<'a> fmt::Debug for Attribute<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Attribute")
            .field("name", &self.name)
            .field("value", &self.raw_value)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ETag<'a> {
    name: &'a str,
}

impl<'a> ETag<'a> {
    pub fn name(&self) -> &'a str {
        self.name
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum XmlEvent<'a> {
    XmlDecl(XmlDecl<'a>),
    STag(STag<'a>),
    ETag(ETag<'a>),
    Characters(&'a str),
}

impl<'a> XmlEvent<'a> {
    pub fn decl(version: &'a str, encoding: Option<&'a str>, standalone: Option<bool>) -> Self {
        XmlEvent::XmlDecl(XmlDecl {
            version,
            encoding,
            standalone,
        })
    }

    pub fn stag(name: &'a str, empty: bool) -> Self {
        XmlEvent::STag(STag { name, empty })
    }

    pub fn etag(name: &'a str) -> Self {
        XmlEvent::ETag(ETag { name })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum XmlDtdError {}

#[derive(Clone, Debug, PartialEq)]
pub enum XmlError {
    ExpectedName,
    ExpectedElementStart,
    ExpectedElementEnd,
    ExpectedAttrName,
    ExpectedAttrValue,
    ExpectedEquals,
    ExpectedDocumentEnd,
    UnexpectedEof,
    OpenElementAtEof,
    NonUniqueAttribute { attribute: String },
    IllegalName { name: String },
    ExpectToken(&'static str),
    IllegalAttributeValue(&'static str),
    UnsupportedEncoding(String),
    DtdError(XmlDtdError),
}
