use crate::Span;
use std::fmt::Formatter;
use std::str::Utf8Error;
use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    span: Span,
    pub(crate) reason: Reason,
}

impl Error {
    pub fn new(span: Span, reason: Reason) -> Self {
        Self { span, reason }
    }

    /// Return whether error is caused by not well formed XML
    pub fn is_not_wf(&self) -> bool {
        match self.reason {
            Reason::Io(_) => false,
            _ => true,
        }
    }

    fn message(&self) -> String {
        match &self.reason {
            Reason::Io(err) => format!("I/O error: {:?}", err),
            Reason::Utf8(err) => format!("UTF-8 error: {:?}", err),
            Reason::UnexpectedEof => "unexpected end of file".to_string(),
            Reason::EndEventMismatch { expected, found } => {
                format!("expected </{}> but got </{}>", expected, found)
            }
            Reason::UnexpectedToken(token) => format!("unexpected token: {:?}", token),
            Reason::InvalidBang => "invalid bang".to_string(),
            Reason::XmlDeclWithoutVersion => {
                "first attribute of xml decl must be the version".to_string()
            }
            Reason::NameWithQuote => "attribute name contains quote".to_string(),
            Reason::NoEqAfterName => "missing `=` after attribute name".to_string(),
            Reason::UnquotedValue => "missing `\"` around attribute value".to_string(),
            Reason::DuplicatedAttribute(other) => format!("attribute already exists at {}", other),
            Reason::InvalidEntity => "unknown or invalid entity".to_string(),
            Reason::UnexpectedDocType => "unexpected doctype".to_string(),
            Reason::UnexpectedDecl => "xml decl not at start of file".to_string(),
            Reason::TrailingContent => "trailing content".to_string(),
            Reason::IllegalPatternInComment => "`--` not allowed in comment".to_string(),
            Reason::PrologCharacters => "non-whitespace characters in prolog".to_string(),
            Reason::InvalidName => "invalid XML name".to_string(),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Error")
            .field("offset", &self.span.start)
            .field("length", &self.span.len)
            .field("message", &self.message())
            .finish()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at offset {} with length {}",
            self.message(),
            self.span.start,
            self.span.len
        )
    }
}

impl std::error::Error for Error {}

pub enum Reason {
    // general
    Io(io::Error),

    // not-wf
    Utf8(Utf8Error),
    UnexpectedEof,
    EndEventMismatch { expected: String, found: String },
    UnexpectedToken(String),
    InvalidBang,
    XmlDeclWithoutVersion,
    NameWithQuote,
    NoEqAfterName,
    UnquotedValue,
    DuplicatedAttribute(usize),
    InvalidEntity,
    UnexpectedDocType,
    UnexpectedDecl,
    TrailingContent,
    PrologCharacters,
    IllegalPatternInComment,
    InvalidName,
}
