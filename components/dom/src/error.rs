use std::fmt::Formatter;
use std::str::Utf8Error;
use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    offset: usize,
    length: usize,
    pub(crate) reason: Reason,
}

impl Error {
    pub fn new(offset: usize, length: usize, reason: Reason) -> Self {
        Self {
            offset,
            length,
            reason,
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match &self.reason {
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
        };
        f.debug_struct("Error")
            .field("offset", &self.offset)
            .field("message", &message)
            .finish()
    }
}

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
}
