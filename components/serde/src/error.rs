use std::fmt;
use std::result::Result as StdResult;

use std::num::{ParseFloatError, ParseIntError};
use xrs_parser::XmlError;

/// Alias for a `Result` with the error type `serde_explicit_xml::Error`.
pub type Result<T> = StdResult<T, Error>;

/// (De)serialization error
pub struct Error(Box<ErrorImpl>);

impl Error {
    pub(crate) fn new(reason: Reason, offset: usize) -> Self {
        Self(Box::new(ErrorImpl {
            offset,
            reason,
            hints: vec![],
        }))
    }

    pub(crate) fn with_position(mut self, offset: usize) -> Self {
        self.0.offset = offset;
        self
    }

    pub(crate) fn with_hint(mut self, offset: usize, message: String) -> Self {
        self.0.hints.push((offset, message));
        self
    }

    pub fn offset(&self) -> usize {
        self.0.offset
    }
}

/// struct to reduce size of `Error`
struct ErrorImpl {
    offset: usize,
    reason: Reason,
    hints: Vec<(usize, String)>,
}

pub(crate) enum Reason {
    /// Serde custom error
    Message(String),
    /// Cannot parse to integer
    Int(ParseIntError),
    /// Cannot parse to float
    Float(ParseFloatError),
    /// Xml parsing error
    Xml(XmlError),
    /// Unexpected end of file
    Eof,
    /// Invalid value for a boolean
    InvalidBoolean(String),
    /// Invalid unit value
    InvalidUnit(String),
    /// Expecting only characters
    NoMarkupExpected,
    /// Unexpected characters
    MarkupExpected,
    /// Expecting Start event
    Start,
    /// Expecting End event
    End,
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
            Reason::Eof => write!(f, "Unexpected end of file"),
            Reason::InvalidBoolean(v) => write!(f, "Invalid boolean value '{}'", v),
            Reason::InvalidUnit(v) => {
                write!(f, "Invalid unit value '{}', expected empty string", v)
            }
            Reason::Start => write!(f, "Expecting Start event"),
            Reason::End => write!(f, "Expecting End event"),
            Reason::NoMarkupExpected => write!(f, "Expecting only characters"),
            Reason::MarkupExpected => write!(f, "Expecting only markup"),
            Reason::RootStruct => write!(f, "Can only deserialize struct on root level"),
            Reason::Tag(tag) => write!(f, "Expecting start tag <{} ...>", tag),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.offset == 0 {
            write!(f, "{}", self.0.reason)?;
        } else {
            write!(f, "{} at offset {}", self.0.reason, self.0.offset)?;
        }
        if !self.0.hints.is_empty() {
            write!(f, " (")?;
            for (i, hint) in self.0.hints.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", &hint.1)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Error")
            .field("message", &self.0.reason.to_string())
            .field("offset", &self.0.offset)
            .field("hints", &self.0.hints)
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
