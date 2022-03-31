use crate::namespace::stack::NamespaceStack;
use crate::namespace::{NsAttribute, NsETag, NsSTag, QName, XmlNsEvent};
use crate::reader::Reader;
use crate::{Attribute, ETag, STag, XmlError, XmlEvent};

pub struct NsReader<'a> {
    reader: Reader<'a>,
    namespaces: NamespaceStack,
    attributes: Vec<NsAttribute<'a>>,
}

impl<'a> NsReader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            reader: Reader::new(input),
            namespaces: NamespaceStack::default(),
            attributes: Vec::with_capacity(4),
        }
    }

    pub fn next(&'a mut self) -> Result<Option<XmlNsEvent<'a>>, XmlError> {
        self.attributes.clear();

        let evt = self.reader.next()?;
        match evt {
            None => Ok(None),
            Some(XmlEvent::STag(stag)) => {
                self.attributes.reserve(stag.attributes().len());
                let mut scope = self.namespaces.build_scope();
                for attr in stag.attrs {
                    let qname = QName::from_cow(attr.name)?;
                    if let Some(prefix) = &qname.prefix {
                        if *prefix == "xmlns" {
                            scope.add_prefix(qname.local_part.to_string(), attr.value.to_string())
                        }
                    }

                    self.attributes.push(NsAttribute::new(qname, attr.value));
                }
                scope.finish();

                Ok(Some(XmlNsEvent::STag(NsSTag {
                    qname: QName::from_cow(stag.name)?,
                    empty: stag.empty,
                })))
            }
            Some(XmlEvent::ETag(etag)) => {
                self.namespaces.pop_scope();
                Ok(Some(XmlNsEvent::ETag(NsETag {
                    // TODO: use qname stack
                    qname: QName::from_cow(etag.name)?,
                })))
            }
            Some(XmlEvent::Characters(chars)) => Ok(Some(XmlNsEvent::Characters(chars))),
            Some(XmlEvent::XmlDecl(decl)) => Ok(Some(XmlNsEvent::XmlDecl(decl))),
            Some(XmlEvent::Dtd(dtd)) => Ok(Some(XmlNsEvent::Dtd(dtd))),
            Some(XmlEvent::PI(pi)) => Ok(Some(XmlNsEvent::PI(pi))),
            Some(XmlEvent::Comment(comment)) => Ok(Some(XmlNsEvent::Comment(comment))),
        }
    }
}
