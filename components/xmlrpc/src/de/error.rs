use std::char::ParseCharError;
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};
use std::result::Result as StdResult;

use xrs_parser::XmlError;

/// Alias for a `Result` with the error type `serde_explicit_xml::Error`.
pub type DeResult<T> = StdResult<T, DeError>;

/// (De)serialization error
pub struct DeError(Box<DeErrorImpl>);

impl DeError {
    pub(crate) fn new(reason: DeReason, offset: usize) -> Self {
        Self(Box::new(DeErrorImpl { offset, reason }))
    }

    pub(crate) fn set_offset(&mut self, offset: usize) {
        self.0.offset = offset;
    }

    pub fn offset(&self) -> usize {
        self.0.offset
    }
}

/// struct to reduce size of `Error`
struct DeErrorImpl {
    offset: usize,
    reason: DeReason,
}

pub(crate) enum DeReason {
    /// Serde custom error
    Message(String),
    /// Cannot parse to integer
    Int(ParseIntError),
    /// Cannot parse to float
    Double(ParseFloatError),
    /// Xml parsing error
    Xml(XmlError),
    /// Unexpected end of file
    Eof,
    /// Invalid value for a boolean
    InvalidBoolean(String),
    /// Invalid value for character
    InvalidChar(ParseCharError),
    /// Invalid value for NIL
    InvalidNil(String),
    #[cfg(feature = "base64")]
    InvalidBase64(base64::DecodeError),
    #[cfg(feature = "datetime")]
    InvalidDateTime(time::error::Parse),
    /// Expecting only characters
    NoMarkupExpected,
    /// Expected XML-RPC value
    ValueExpected,
    /// Expecting Start event
    Start,
    /// Expecting End event
    ExpectedEndElement(&'static str),
    ExpectedElement(&'static str),
    /// Expecting struct or tuple as root object
    RootStruct,
    /// Unknown type element
    UnknownType(String),
    /// Wrong type element
    WrongType(&'static str, String),
}

impl fmt::Display for DeReason {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeReason::Message(s) => write!(f, "{}", s),
            DeReason::Xml(e) => match e {
                XmlError::Io(err) => write!(f, "I/O error: {}", err),
                XmlError::Decoding(err) => write!(f, "Decoding error: {}", err),
                _ => write!(f, "XML syntax error: {:?}", e),
            },
            DeReason::Int(e) => write!(f, "Invalid integer: {}", e),
            DeReason::Double(e) => write!(f, "Invalid float: {}", e),
            DeReason::Eof => write!(f, "Unexpected end of file"),
            DeReason::InvalidBoolean(v) => write!(f, "Invalid boolean value '{}'", v),
            DeReason::InvalidNil(v) => {
                write!(f, "Invalid NIL value '{}', expected empty string", v)
            }
            DeReason::InvalidChar(v) => write!(f, "Invalid char: {}", v),
            DeReason::Start => write!(f, "Expecting Start event"),
            DeReason::ExpectedEndElement(name) => write!(f, "Expecting end of element `{}`", name),
            DeReason::NoMarkupExpected => write!(f, "Expecting only characters in scalar"),
            DeReason::ValueExpected => write!(f, "Expecting XML-RPC value"),
            DeReason::RootStruct => write!(f, "Can only deserialize struct on root level"),
            DeReason::UnknownType(ty) => write!(f, "Unknown type element <{}>", ty),
            DeReason::WrongType(expected, actual) => {
                write!(f, "Expected type {}, but got {}", expected, actual)
            }
            DeReason::ExpectedElement(elem) => {
                write!(f, "Expecting element: {}", elem)
            }
            DeReason::InvalidBase64(base64) => {
                write!(f, "Invalid base64 string: {}", base64)
            }
            DeReason::InvalidDateTime(date_time) => {
                write!(f, "Invalid date time: {}", date_time)
            }
        }
    }
}

impl fmt::Display for DeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.offset == 0 {
            write!(f, "{}", self.0.reason)
        } else {
            write!(f, "{} at offset {}", self.0.reason, self.0.offset)
        }
    }
}

impl fmt::Debug for DeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Error")
            .field("message", &self.0.reason.to_string())
            .field("offset", &self.0.offset)
            .finish()
    }
}

impl ::std::error::Error for DeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0.reason {
            DeReason::Int(e) => Some(e),
            DeReason::Double(e) => Some(e),
            DeReason::Xml(e) => Some(e),
            _ => None,
        }
    }
}

impl serde::de::Error for DeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        // TODO: parse error message for offset
        DeError::new(DeReason::Message(msg.to_string()), 0)
    }
}

impl serde::ser::Error for DeError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        // TODO: parse error message for offset
        DeError::new(DeReason::Message(msg.to_string()), 0)
    }
}

impl From<std::io::Error> for DeError {
    fn from(e: std::io::Error) -> Self {
        DeError::new(DeReason::Xml(XmlError::Io(e.to_string())), 0)
    }
}

impl From<XmlError> for DeError {
    fn from(e: XmlError) -> Self {
        let offset: usize = match &e {
            _ => 0,
        };

        DeError::new(DeReason::Xml(e), offset)
    }
}

impl From<ParseIntError> for DeError {
    fn from(e: ParseIntError) -> Self {
        DeError::new(DeReason::Int(e), 0)
    }
}

impl From<ParseFloatError> for DeError {
    fn from(e: ParseFloatError) -> Self {
        DeError::new(DeReason::Double(e), 0)
    }
}

impl From<ParseCharError> for DeError {
    fn from(e: ParseCharError) -> Self {
        DeError::new(DeReason::InvalidChar(e), 0)
    }
}

#[cfg(feature = "base64")]
impl From<base64::DecodeError> for DeError {
    fn from(e: base64::DecodeError) -> Self {
        DeError::new(DeReason::InvalidBase64(e), 0)
    }
}

#[cfg(feature = "datetime")]
impl From<time::error::Parse> for DeError {
    fn from(e: time::error::Parse) -> Self {
        DeError::new(DeReason::InvalidDateTime(e), 0)
    }
}
