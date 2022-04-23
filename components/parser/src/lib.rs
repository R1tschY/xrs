#![allow(unused)]

use std::borrow::Cow;
use std::fmt::Formatter;
use std::fs::read_to_string;
use std::str::from_utf8;
use std::{fmt, io};

pub use namespace::parser::*;
pub use namespace::*;
use parser::cursor::Cursor;
pub use reader::Reader;
use xrs_chars::XmlAsciiChar;
use xrs_chars::XmlChar;

use crate::dtd::DocTypeDecl;
use crate::XmlError::{ExpectedElementEnd, IllegalNameStartChar};

mod dtd;
mod namespace;
pub mod parser;
mod reader;
mod shufti;

/// XML Declaration
#[derive(Clone, Debug, PartialEq)]
pub struct XmlDecl {
    version: String,
    encoding: Option<String>,
    standalone: Option<bool>,
}

impl XmlDecl {
    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn encoding(&self) -> Option<&str> {
        self.encoding.as_ref().map(|encoding| encoding as &str)
    }

    pub fn standalone(&self) -> Option<bool> {
        self.standalone
    }
}

/// Start tag
#[derive(Clone, Debug, PartialEq)]
pub struct STag<'a> {
    pub name: Cow<'a, str>,
    pub empty: bool,
    pub attrs: Vec<Attribute<'a>>,
}

impl<'a> STag<'a> {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn empty(&self) -> bool {
        self.empty
    }

    pub fn attributes(&self) -> &[Attribute<'a>] {
        &self.attrs
    }

    pub fn into_owned(self) -> STag<'static> {
        STag {
            name: self.name.into_owned().into(),
            empty: false,
            attrs: self
                .attrs
                .into_iter()
                .map(|attr| attr.into_owned())
                .collect(),
        }
    }
}

/// Attribute
#[derive(Clone, PartialEq)]
pub struct Attribute<'a> {
    pub name: Cow<'a, str>,
    pub value: Cow<'a, str>,
}

impl<'a> Attribute<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>, value: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn value(&self) -> &str {
        self.value.as_ref()
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn value_cow(&self) -> Cow<'a, str> {
        self.value.clone()
    }

    pub fn name_cow(&self) -> Cow<'a, str> {
        self.name.clone()
    }

    pub fn into_owned(self) -> Attribute<'static> {
        Attribute {
            name: self.name.into_owned().into(),
            value: self.value.into_owned().into(),
        }
    }
}

impl<'a> fmt::Debug for Attribute<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Attribute")
            .field("name", &self.name)
            .field("value", &self.value)
            .finish()
    }
}

/// End tag
#[derive(Clone, Debug, PartialEq)]
pub struct ETag<'a> {
    pub name: Cow<'a, str>,
}

impl<'a> ETag<'a> {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn into_owned(self) -> ETag<'static> {
        ETag {
            name: self.name.into_owned().into(),
        }
    }
}

/// Processing Instruction
#[derive(Clone, Debug, PartialEq)]
pub struct PI<'a> {
    pub target: Cow<'a, str>,
    pub data: Cow<'a, str>,
}

impl<'a> PI<'a> {
    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn data(&self) -> &str {
        &self.data
    }

    pub fn into_owned(self) -> PI<'static> {
        PI {
            target: self.target.into_owned().into(),
            data: self.data.into_owned().into(),
        }
    }
}

/// Event of Pull Parser
#[derive(Clone, Debug, PartialEq)]
pub enum XmlEvent<'a> {
    XmlDecl(XmlDecl),
    Dtd(Box<DocTypeDecl>),
    STag(STag<'a>),
    ETag(ETag<'a>),
    Characters(Cow<'a, str>),
    PI(PI<'a>),
    Comment(Cow<'a, str>),
}

impl<'a> XmlEvent<'a> {
    pub fn decl(version: &'a str, encoding: Option<&'a str>, standalone: Option<bool>) -> Self {
        XmlEvent::XmlDecl(XmlDecl {
            version: version.to_string(),
            encoding: encoding.map(|enc| enc.to_string()),
            standalone,
        })
    }

    pub fn stag(name: impl Into<Cow<'a, str>>, empty: bool) -> Self {
        XmlEvent::STag(STag {
            name: name.into(),
            empty,
            attrs: vec![],
        })
    }

    pub fn stag_with_attrs(
        name: impl Into<Cow<'a, str>>,
        empty: bool,
        attrs: impl Into<Vec<Attribute<'a>>>,
    ) -> Self {
        XmlEvent::STag(STag {
            name: name.into(),
            empty,
            attrs: attrs.into(),
        })
    }

    pub fn characters(chars: impl Into<Cow<'a, str>>) -> Self {
        XmlEvent::Characters(chars.into())
    }

    pub fn etag(name: impl Into<Cow<'a, str>>) -> Self {
        XmlEvent::ETag(ETag { name: name.into() })
    }

    pub fn comment(comment: impl Into<Cow<'a, str>>) -> Self {
        XmlEvent::Comment(comment.into())
    }

    pub fn pi(target: impl Into<Cow<'a, str>>, data: impl Into<Cow<'a, str>>) -> Self {
        XmlEvent::PI(PI {
            target: target.into(),
            data: data.into(),
        })
    }

    pub fn into_owned(self) -> XmlEvent<'static> {
        match self {
            XmlEvent::XmlDecl(v) => XmlEvent::XmlDecl(v),
            XmlEvent::Dtd(v) => XmlEvent::Dtd(v),
            XmlEvent::STag(v) => XmlEvent::STag(v.into_owned()),
            XmlEvent::ETag(v) => XmlEvent::ETag(v.into_owned()),
            XmlEvent::Characters(v) => XmlEvent::Characters(v.into_owned().into()),
            XmlEvent::PI(v) => XmlEvent::PI(v.into_owned()),
            XmlEvent::Comment(v) => XmlEvent::Comment(v.into_owned().into()),
        }
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
    ExpectedWhitespace,
    WrongETagName {
        expected_name: String,
    },
    UnexpectedEof,
    UnexpectedDtdEntry,
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
    InvalidCharacter(char),
    IllegalReference,
    UnknownEntity(String),
    ExpectToken(&'static str),
    IllegalAttributeValue(&'static str),
    UnsupportedEncoding(String),
    DtdError(XmlDtdError),
    /// Processing Instruction target should not be `xml` (case-insensitive)
    InvalidPITarget,
    UnexpectedCharacter(char),
    CommentColonColon,
    UnknownNamespacePrefix(String),
}

/// Fatal DTD parsing error
#[derive(Clone, Debug, PartialEq)]
pub enum XmlDtdError {
    SyntaxError,
}
