use crate::namespace::stack::NamespaceStack;
use crate::namespace::{NsAttribute, NsETag, NsSTag, QName, XmlNsEvent};
use crate::reader::Reader;
use crate::{ETag, STag, XmlError, XmlEvent};

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

    pub fn next(&mut self) -> Result<Option<XmlNsEvent<'a>>, XmlError> {
        self.attributes.clear();

        match self.reader.next()? {
            None => Ok(None),
            Some(XmlEvent::STag(stag)) => self.parse_stag(stag),
            Some(XmlEvent::ETag(etag)) => self.parse_etag(etag),
            Some(XmlEvent::Characters(chars)) => Ok(Some(XmlNsEvent::Characters(chars))),
            Some(XmlEvent::XmlDecl(decl)) => Ok(Some(XmlNsEvent::XmlDecl(decl))),
            Some(XmlEvent::Dtd(dtd)) => Ok(Some(XmlNsEvent::Dtd(dtd))),
            Some(XmlEvent::PI(pi)) => Ok(Some(XmlNsEvent::PI(pi))),
            Some(XmlEvent::Comment(comment)) => Ok(Some(XmlNsEvent::Comment(comment))),
        }
    }

    fn parse_stag(&mut self, stag: STag<'a>) -> Result<Option<XmlNsEvent<'a>>, XmlError> {
        self.attributes.reserve(self.reader.attributes().len());
        let mut scope = self.namespaces.build_scope();
        for attr in self.reader.attributes() {
            let qname = QName::from_str(attr.name())?;
            if let Some(prefix) = qname.prefix {
                if prefix == "xmlns" {
                    scope.add_prefix(qname.local_part, attr.raw_value())
                }
            }

            self.attributes
                .push(NsAttribute::new(qname, attr.raw_value));
        }
        scope.finish();

        Ok(Some(XmlNsEvent::STag(NsSTag {
            qname: QName::from_str(stag.name())?,
            empty: stag.empty,
        })))
    }

    fn parse_etag(&mut self, etag: ETag<'a>) -> Result<Option<XmlNsEvent<'a>>, XmlError> {
        self.namespaces.pop_scope();
        Ok(Some(XmlNsEvent::ETag(NsETag {
            // TODO: use qname stack
            qname: QName::from_str(etag.name())?,
        })))
    }
}
