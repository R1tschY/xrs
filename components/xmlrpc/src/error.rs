use crate::de::DeError;
use std::fmt::{Debug, Display, Formatter, Write};
use std::ops::Deref;

#[derive(Debug)]
enum Repr {
    Io(std::io::Error),
    Ser(String),
    De(DeError),
    Fault { code: i32, string: String },
    StatusCode { code: u16, string: String },
    ContentType { string: String },
    WrongType { param: u16, expected: String },
}

impl Debug for XmlRpcError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Display for XmlRpcError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0.deref() {
            Repr::Io(err) => f.write_fmt(format_args!("i/o error: {}", err)),
            Repr::De(msg) => f.write_fmt(format_args!("deserialization error: {}", msg)),
            Repr::Ser(msg) => f.write_fmt(format_args!("serialization error: {}", msg)),
            Repr::Fault { code, string } => {
                f.write_fmt(format_args!("XML-RPC fault {}: {}", code, string))
            }
            Repr::WrongType { param, expected } => f.write_fmt(format_args!(
                "Wrong type of parameter {}: {}",
                param, expected
            )),
            Repr::StatusCode { code, string } => f.write_fmt(format_args!(
                "Unexpected HTTP status code {}: {}",
                code, string
            )),
            Repr::ContentType { string } => {
                f.write_fmt(format_args!("Unexpected HTTP content type: {}", string))
            }
        }
    }
}

impl std::error::Error for XmlRpcError {}

pub struct XmlRpcError(Box<Repr>);

impl XmlRpcError {
    pub fn new_ser(message: impl Into<String>) -> Self {
        Self(Box::new(Repr::Ser(message.into())))
    }

    pub fn new_fault(code: i32, string: impl Into<String>) -> Self {
        Self(Box::new(Repr::Fault {
            code,
            string: string.into(),
        }))
    }

    pub fn new_status_code(code: u16, string: impl Into<String>) -> Self {
        Self(Box::new(Repr::StatusCode {
            code,
            string: string.into(),
        }))
    }

    pub fn new_content_type(string: impl Into<String>) -> Self {
        Self(Box::new(Repr::ContentType {
            string: string.into(),
        }))
    }
}

impl From<std::io::Error> for XmlRpcError {
    fn from(value: std::io::Error) -> Self {
        Self(Box::new(Repr::Io(value)))
    }
}

impl From<DeError> for XmlRpcError {
    fn from(err: DeError) -> Self {
        Self(Box::new(Repr::De(err)))
    }
}
