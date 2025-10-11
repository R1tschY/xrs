#![allow(unused)]

use std::borrow::Cow;
use std::error::Error;
use std::fmt::Write;
use std::fmt::{Display, Formatter};
use std::{fmt, io};

pub use namespace::parser::*;
pub use namespace::*;
use parser::cursor::Cursor;
pub use reader::Reader;
use xrs_chars::XmlAsciiChar;
use xrs_chars::XmlChar;

use crate::dtd::DocTypeDecl;

pub(crate) mod cow;
mod dtd;
#[cfg(feature = "encoding")]
pub mod encoding;
mod namespace;
pub mod parser;
mod reader;
pub mod simple;

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
    pub data: Option<Cow<'a, str>>,
}

impl<'a> PI<'a> {
    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn data(&self) -> Option<&str> {
        self.data.as_ref().map(|s| s as &str)
    }

    pub fn into_owned(self) -> PI<'static> {
        PI {
            target: Cow::Owned(self.target.into_owned()),
            data: self.data.map(|d| Cow::Owned(d.into_owned())),
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

    pub fn pi(target: impl Into<Cow<'a, str>>, data: Option<Cow<'a, str>>) -> Self {
        XmlEvent::PI(PI {
            target: target.into(),
            data,
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

#[derive(Debug, PartialEq)]
pub enum XmlErrorAtom {
    XmlDecl,
    CData,
    Comment,
    PI,
    Markup,
    Element,
    Whitespace,
}

/// Fatal parsing error
#[derive(Debug, PartialEq)]
pub enum XmlError {
    IllegalNameStartChar(char),
    IllegalChar(char),
    ExpectedElementStart,
    ExpectedElementEnd,
    ExpectedAttrName,
    ExpectedEquals,
    ExpectedDocumentEnd,
    Expected(Box<[XmlErrorAtom]>),
    ExpectedWhitespace,
    WrongETagName {
        expected_name: String,
    },
    UnexpectedEof,
    IllegalCDataSectionEnd,
    UnexpectedDtdEntry,
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
    IllegalNamespaceUri(String),
    Io(String),
    Decoding(String),
    UnsupportedVersion(String),
}

impl From<io::Error> for XmlError {
    fn from(value: io::Error) -> Self {
        XmlError::Io(value.to_string())
    }
}

impl Display for XmlError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            XmlError::IllegalNameStartChar(c) => write!(f, "Illegal name start char: {c}"),
            XmlError::IllegalChar(c) => write!(f, "Illegal char: {c}"),
            XmlError::ExpectedElementStart => write!(f, "Expected element start"),
            XmlError::ExpectedElementEnd => write!(f, "Expected element end"),
            XmlError::ExpectedAttrName => write!(f, "Expected attribute name"),
            XmlError::ExpectedEquals => write!(f, "Expected '='"),
            XmlError::ExpectedDocumentEnd => write!(f, "Expected document end"),
            XmlError::Expected(expected) => {
                f.write_str("Expected one of: ")?;
                for (i, e) in expected.iter().enumerate() {
                    if i != 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                Ok(())
            }
            XmlError::ExpectedWhitespace => write!(f, "Expected whitespace"),
            XmlError::WrongETagName { expected_name } => {
                write!(f, "Wrong end tag name, expected '{}'", expected_name)
            }
            XmlError::UnexpectedEof => write!(f, "Unexpected end of file"),
            XmlError::IllegalCDataSectionEnd => write!(f, "Illegal CDATA section end"),
            XmlError::UnexpectedDtdEntry => write!(f, "Unexpected DTD entry"),
            XmlError::ETagAfterRootElement => write!(f, "End tag after root element"),
            XmlError::OpenElementAtEof => write!(f, "Open element at end of file"),
            XmlError::NonUniqueAttribute { attribute } => {
                write!(f, "Non-unique attribute '{}'", attribute)
            }
            XmlError::IllegalName { name } => write!(f, "Illegal name '{}'", name),
            XmlError::InvalidCharacterReference(s) => {
                write!(f, "Invalid character reference '{}'", s)
            }
            XmlError::InvalidCharacter(c) => write!(f, "Invalid character '{}'", c),
            XmlError::IllegalReference => write!(f, "Illegal reference"),
            XmlError::UnknownEntity(s) => write!(f, "Unknown entity '{}'", s),
            XmlError::ExpectToken(token) => write!(f, "Expected token '{}'", token),
            XmlError::IllegalAttributeValue(msg) => write!(f, "Illegal attribute value: {}", msg),
            XmlError::UnsupportedEncoding(enc) => write!(f, "Unsupported encoding '{}'", enc),
            XmlError::DtdError(e) => write!(f, "DTD error: {}", e),
            XmlError::InvalidPITarget => write!(f, "Invalid processing instruction target 'xml'"),
            XmlError::UnexpectedCharacter(c) => write!(f, "Unexpected character '{}'", c),
            XmlError::CommentColonColon => write!(f, "Illegal '--' in comment"),
            XmlError::UnknownNamespacePrefix(p) => write!(f, "Unknown namespace prefix '{}'", p),
            XmlError::IllegalNamespaceUri(uri) => write!(f, "Illegal namespace URI '{}'", uri),
            XmlError::Io(e) => write!(f, "I/O error: {}", e),
            XmlError::Decoding(e) => write!(f, "Decoding error: {}", e),
            XmlError::UnsupportedVersion(v) => write!(f, "Unsupported version '{}'", v),
        }
    }
}

impl Error for XmlError {}

/// Fatal DTD parsing error
#[derive(Debug, PartialEq)]
pub enum XmlDtdError {
    SyntaxError,
    Unsupported,
}

impl Display for XmlErrorAtom {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            XmlErrorAtom::XmlDecl => write!(f, "XML declaration"),
            XmlErrorAtom::CData => write!(f, "CDATA"),
            XmlErrorAtom::Comment => write!(f, "comment"),
            XmlErrorAtom::PI => write!(f, "processing instruction"),
            XmlErrorAtom::Markup => write!(f, "markup"),
            XmlErrorAtom::Element => write!(f, "element"),
            XmlErrorAtom::Whitespace => write!(f, "whitespace"),
        }
    }
}

impl Display for XmlDtdError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            XmlDtdError::SyntaxError => write!(f, "DTD syntax error"),
            XmlDtdError::Unsupported => write!(f, "Unsupported DTD feature"),
        }
    }
}
