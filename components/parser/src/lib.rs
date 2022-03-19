#![allow(unused)]

use std::borrow::Cow;
use std::fmt::Formatter;
use std::fs::read_to_string;
use std::str::from_utf8;
use std::{fmt, io};

use parser::cursor::Cursor;
pub use reader::Reader;
use xml_chars::XmlAsciiChar;
use xml_chars::XmlChar;

use crate::dtd::DocTypeDecl;
use crate::XmlError::{ExpectedElementEnd, IllegalNameStartChar};

mod dtd;
mod namespace;
pub mod parser;
mod reader;
mod shufti;

/// XML Declaration
#[derive(Clone, Debug, PartialEq)]
pub struct XmlDecl<'a> {
    version: &'a str,
    encoding: Option<&'a str>,
    standalone: Option<bool>,
}

impl<'a> XmlDecl<'a> {
    pub fn version(&self) -> &'a str {
        self.version
    }

    pub fn encoding(&self) -> Option<&'a str> {
        self.encoding
    }

    pub fn standalone(&self) -> Option<bool> {
        self.standalone
    }
}

/// Start tag
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

/// Attribute
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

/// End tag
#[derive(Clone, Debug, PartialEq)]
pub struct ETag<'a> {
    name: &'a str,
}

impl<'a> ETag<'a> {
    pub fn name(&self) -> &'a str {
        self.name
    }
}

/// Processing Instruction
#[derive(Clone, Debug, PartialEq)]
pub struct PI<'a> {
    target: &'a str,
    data: &'a str,
}

impl<'a> PI<'a> {
    pub fn target(&self) -> &'a str {
        self.target
    }

    pub fn data(&self) -> &'a str {
        self.data
    }
}

/// Event of Pull Parser
#[derive(Clone, Debug, PartialEq)]
pub enum XmlEvent<'a> {
    XmlDecl(XmlDecl<'a>),
    Dtd(DocTypeDecl<'a>),
    STag(STag<'a>),
    ETag(ETag<'a>),
    Characters(Cow<'a, str>),
    PI(PI<'a>),
    Comment(&'a str),
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

    pub fn characters(chars: impl Into<Cow<'a, str>>) -> Self {
        XmlEvent::Characters(chars.into())
    }

    pub fn etag(name: &'a str) -> Self {
        XmlEvent::ETag(ETag { name })
    }

    pub fn comment(comment: &'a str) -> Self {
        XmlEvent::Comment(comment)
    }

    pub fn pi(target: &'a str, data: &'a str) -> Self {
        XmlEvent::PI(PI { target, data })
    }
}

/// Fatal parsing error
#[derive(Clone, Debug, PartialEq)]
pub enum XmlError {
    IllegalNameStartChar(char),
    IllegalChar(char),
    ExpectedElementStart,
    ExpectedElementEnd,
    ExpectedAttrName,
    ExpectedAttrValue,
    ExpectedEquals,
    ExpectedDocumentEnd,
    WrongETagName {
        expected_name: String,
    },
    UnexpectedEof,
    CDataEndInContent,
    ETagAfterRootElement,
    OpenElementAtEof,
    NonUniqueAttribute {
        attribute: String,
    },
    IllegalName {
        name: String,
    },
    InvalidCharacterReference(String),
    IllegalReference,
    ExpectToken(&'static str),
    IllegalAttributeValue(&'static str),
    UnsupportedEncoding(String),
    DtdError(XmlDtdError),
    /// Processing Instruction target should not be `xml` (case-insensitive)
    InvalidPITarget,
    UnexpectedCharacter(char),
    CommentColonColon,
}

/// Fatal DTD parsing error
#[derive(Clone, Debug, PartialEq)]
pub enum XmlDtdError {}
