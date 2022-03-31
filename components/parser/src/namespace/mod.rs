use std::borrow::Cow;
use std::fmt;
use std::str::{from_utf8, FromStr, ParseBoolError};
use std::sync::Arc;

use crate::{DocTypeDecl, XmlDecl, XmlError, PI};

pub mod parser;
pub mod stack;

/// Qualified Name
///
/// Name with namespace prefix and local part
#[derive(Clone, Debug, PartialEq)]
pub struct QName<'a> {
    pub prefix: Option<Cow<'a, str>>,
    pub local_part: Cow<'a, str>,
}

impl<'a> QName<'a> {
    pub fn from_cow(input: Cow<'a, str>) -> Result<Self, XmlError> {
        match input {
            Cow::Borrowed(borrowed) => Self::from_str(borrowed),
            Cow::Owned(owned) => Self::from_string(owned),
        }
    }

    pub fn from_string(mut input: String) -> Result<Self, XmlError> {
        let mut spliter = input.split(|c| c == ':');
        if let Some(first) = spliter.next() {
            if let Some(second) = spliter.next() {
                if spliter.next().is_some() {
                    Err(XmlError::IllegalName {
                        name: input.to_string(),
                    })
                } else {
                    let prefix_bytes = first.len();
                    let local_part = second.to_string();
                    input.truncate(prefix_bytes);
                    Ok(QName {
                        prefix: Some(input.into()),
                        local_part: local_part.into(),
                    })
                }
            } else {
                Ok(QName {
                    prefix: None,
                    local_part: input.into(),
                })
            }
        } else {
            Err(XmlError::IllegalName {
                name: String::new(),
            })
        }
    }

    pub fn from_str(input: &'a str) -> Result<Self, XmlError> {
        // TODO: split_once faster?
        let mut spliter = input.split(|c| c == ':');
        if let Some(first) = spliter.next() {
            if let Some(second) = spliter.next() {
                if spliter.next().is_some() {
                    Err(XmlError::IllegalName {
                        name: input.to_string(),
                    })
                } else {
                    Ok(QName {
                        prefix: Some(Cow::Borrowed(first)),
                        local_part: Cow::Borrowed(second),
                    })
                }
            } else {
                Ok(QName {
                    prefix: None,
                    local_part: Cow::Borrowed(first),
                })
            }
        } else {
            Err(XmlError::IllegalName {
                name: String::new(),
            })
        }
    }
}

pub type Namespace = Arc<NamespaceDecl>;

pub struct NamespaceDecl {
    prefix: Option<String>,
    uri: String,
}

impl NamespaceDecl {
    fn new(prefix: Option<String>, uri: String) -> Self {
        Self { prefix, uri }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NsSTag<'a> {
    pub qname: QName<'a>,
    pub empty: bool,
}

#[derive(Clone, PartialEq)]
pub struct NsAttribute<'a> {
    pub qname: QName<'a>,
    pub value: Cow<'a, str>,
}

impl<'a> NsAttribute<'a> {
    pub fn new(qname: QName<'a>, value: impl Into<Cow<'a, str>>) -> Self {
        Self {
            qname,
            value: value.into(),
        }
    }

    pub fn value(&self) -> &str {
        self.value.as_ref()
    }

    pub fn qname(&self) -> QName<'a> {
        self.qname.clone()
    }
}

impl<'a> fmt::Debug for NsAttribute<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Attribute")
            .field("name", &self.qname)
            .field("value", &self.value)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct STagEnd<'a> {
    pub qname: QName<'a>,
}

impl<'a> STagEnd<'a> {
    pub fn qname(&self) -> QName<'a> {
        self.qname.clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NsETag<'a> {
    qname: QName<'a>,
}

impl<'a> NsETag<'a> {
    pub fn name(&self) -> QName<'a> {
        self.qname.clone()
    }
}

/// XML event with namespace parsing
#[derive(Clone, Debug, PartialEq)]
pub enum XmlNsEvent<'a> {
    XmlDecl(XmlDecl),
    Dtd(Box<DocTypeDecl>),
    STag(NsSTag<'a>),
    ETag(NsETag<'a>),
    Characters(Cow<'a, str>),
    PI(PI<'a>),
    Comment(Cow<'a, str>),
}
