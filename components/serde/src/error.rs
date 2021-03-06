use std::result::Result as StdResult;
use std::str::Utf8Error;
use std::{fmt, io};

use crate::Deserializer;
use serde::Serializer;
use std::num::{ParseFloatError, ParseIntError};
use xrs_parser::XmlError;

/// Alias for a `Result` with the error type `serde_explicit_xml::Error`.
pub type Result<T> = StdResult<T, Error>;

/// (De)serialization error
pub struct Error(Box<ErrorImpl>);

impl Error {
    pub(crate) fn new(reason: Reason, offset: usize) -> Self {
        Self(Box::new(ErrorImpl { offset, reason }))
    }

    pub(crate) fn fix_position(self, f: impl FnOnce(Reason) -> Error) -> Self {
        if self.0.offset == 0 {
            f(self.0.reason)
        } else {
            self
        }
    }

    pub fn offset(&self) -> usize {
        self.0.offset
    }
}

/// struct to reduce size of `Error`
struct ErrorImpl {
    offset: usize,
    reason: Reason,
}

pub(crate) enum Reason {
    /// Serde custom error
    Message(String),
    /// Cannot parse to integer
    Int(std::num::ParseIntError),
    /// Cannot parse to float
    Float(std::num::ParseFloatError),
    /// Xml parsing error
    Xml(XmlError),
    /// Unexpected end of attributes
    EndOfAttributes,
    /// Unexpected end of file
    Eof,
    /// Invalid value for a boolean
    InvalidBoolean(String),
    /// Invalid unit value
    InvalidUnit(String),
    /// Invalid event for Enum
    InvalidEnum(String),
    /// Expecting only characters
    NoMarkupExpected,
    /// Unexpected characters
    MarkupExpected,
    /// Expecting Text event
    Text,
    /// Expecting Start event
    Start,
    /// Expecting End event
    End,
    /// Unsupported operation
    Unsupported(&'static str),
    /// Expecting struct as root object
    RootStruct,
    /// Expecting tag name
    Tag(&'static str),
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Reason::Message(s) => write!(f, "{}", s),
            Reason::Xml(e) => match e {
                XmlError::Io(err) => write!(f, "I/O error: {}", err),
                XmlError::Decoding(err) => write!(f, "Decoding error: {}", err),
                _ => write!(f, "XML syntax error: {:?}", e),
            },
            Reason::Int(e) => write!(f, "Invalid integer: {}", e),
            Reason::Float(e) => write!(f, "Invalid float: {}", e),
            Reason::EndOfAttributes => write!(f, "Unexpected end of attributes"),
            Reason::Eof => write!(f, "Unexpected end of file"),
            Reason::InvalidBoolean(v) => write!(f, "Invalid boolean value '{}'", v),
            Reason::InvalidUnit(v) => {
                write!(f, "Invalid unit value '{}', expected empty string", v)
            }
            Reason::InvalidEnum(e) => write!(
                f,
                "Invalid event for Enum, expecting Text or Start, got: {:?}",
                e
            ),
            Reason::Text => write!(f, "Expecting Text event"),
            Reason::Start => write!(f, "Expecting Start event"),
            Reason::End => write!(f, "Expecting End event"),
            Reason::Unsupported(s) => write!(f, "Unsupported operation: {}", s),
            Reason::NoMarkupExpected => write!(f, "Expecting only characters"),
            Reason::MarkupExpected => write!(f, "Expecting only markup"),
            Reason::RootStruct => write!(f, "Can only deserialize struct on root level"),
            Reason::Tag(tag) => write!(f, "Expecting start tag '{}'", tag),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.offset == 0 {
            write!(f, "{}", self.0.reason)
        } else {
            write!(f, "{} at offset {}", self.0.reason, self.0.offset)
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Error")
            .field("message", &self.0.reason.to_string())
            .field("offset", &self.0.offset)
            .finish()
    }
}

impl ::std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0.reason {
            Reason::Int(e) => Some(e),
            Reason::Float(e) => Some(e),
            Reason::Xml(e) => Some(e),
            _ => None,
        }
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        // TODO: parse error message for offset
        Error::new(Reason::Message(msg.to_string()), 0)
    }
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        // TODO: parse error message for offset
        Error::new(Reason::Message(msg.to_string()), 0)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::new(Reason::Xml(XmlError::Io(e.to_string())), 0)
    }
}

impl From<XmlError> for Error {
    fn from(e: XmlError) -> Self {
        let offset: usize = match &e {
            _ => 0,
        };

        Error::new(Reason::Xml(e), offset)
    }
}

pub(crate) trait ResultExt<T> {
    fn at_offset(self, offset: usize) -> Result<T>;
}

impl<T> ResultExt<T> for StdResult<T, ParseIntError> {
    fn at_offset(self, offset: usize) -> Result<T> {
        self.map_err(|err| Error::new(Reason::Int(err), offset))
    }
}

impl<T> ResultExt<T> for StdResult<T, ParseFloatError> {
    fn at_offset(self, offset: usize) -> Result<T> {
        self.map_err(|err| Error::new(Reason::Float(err), offset))
    }
}
