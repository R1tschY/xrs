use crate::namespace::stack::NamespaceStack;
use crate::namespace::{NsAttribute, NsETag, NsSTagStart, QName, XmlNsEvent};
use crate::{Reader, XmlError, XmlEvent};

pub struct NsReader<'a> {
    reader: Reader<'a>,
    namespaces: NamespaceStack,
}

impl<'a> NsReader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            reader: Reader::new(input),
            namespaces: NamespaceStack::default(),
        }
    }

    pub fn next(&mut self) -> Result<Option<XmlNsEvent<'a>>, XmlError> {
        match self.reader.next()? {
            None => Ok(None),
            Some(XmlEvent::STagStart(stag)) => {
                self.namespaces.begin_scope();
                Ok(Some(XmlNsEvent::STagStart(NsSTagStart {
                    qname: Self::parse_qname(stag.name())?,
                })))
            }
            Some(XmlEvent::Attribute(attr)) => {
                let qname = Self::parse_qname(attr.name())?;
                if qname.prefix == Some("xmlns") {
                    // TODO: self.namespaces.add(qname.prefix, attr.value);
                }

                Ok(Some(XmlNsEvent::Attribute(NsAttribute {
                    qname,
                    raw_value: attr.raw_value(),
                })))
            }
            Some(XmlEvent::STagEnd) => {
                // TODO: self.namespaces.end_stag();
                Ok(Some(XmlNsEvent::STagEnd))
            }
            Some(XmlEvent::ETag(etag)) => {
                // TODO: self.namespaces.etag();
                Ok(Some(XmlNsEvent::ETag(NsETag {
                    qname: Self::parse_qname(etag.name())?,
                })))
            }
            Some(XmlEvent::STagEndEmpty) => {
                // TODO: self.namespaces.end_stag();
                // TODO: self.namespaces.etag();
                Ok(Some(XmlNsEvent::STagEndEmpty))
            }
            Some(XmlEvent::Characters(chars)) => Ok(Some(XmlNsEvent::Characters(chars))),
        }
    }

    fn parse_qname(input: &str) -> Result<QName, XmlError> {
        let mut spliter = input.split(|c| c == ':');
        if let Some(first) = spliter.next() {
            if let Some(second) = spliter.next() {
                if let Some(_) = spliter.next() {
                    Err(XmlError::IllegalName {
                        name: input.to_string(),
                    })
                } else {
                    Ok(QName {
                        prefix: Some(first),
                        local_part: second,
                    })
                }
            } else {
                Ok(QName {
                    prefix: None,
                    local_part: first,
                })
            }
        } else {
            Err(XmlError::IllegalName {
                name: String::new(),
            })
        }
    }
}
