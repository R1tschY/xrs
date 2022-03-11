use crate::namespace::stack::NamespaceStack;
use crate::namespace::{NsAttribute, NsETag, NsSTag, QName, XmlNsEvent};
use crate::reader::Reader;
use crate::{ETag, STag, XmlError, XmlEvent};

pub struct NsReader<'a> {
    reader: Reader<'a>,
    namespaces: NamespaceStack,
    empty_element: bool,
    attributes: Vec<NsAttribute<'a>>,
}

impl<'a> NsReader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            reader: Reader::new(input),
            namespaces: NamespaceStack::default(),
            empty_element: false,
            attributes: Vec::with_capacity(4),
        }
    }

    pub fn next(&mut self) -> Result<Option<XmlNsEvent<'a>>, XmlError> {
        if self.empty_element {
            self.namespaces.pop_scope();
            self.empty_element = false;
        }
        self.attributes.clear();

        match self.reader.next()? {
            None => Ok(None),
            Some(XmlEvent::STag(stag)) => self.parse_stag(stag),
            Some(XmlEvent::ETag(etag)) => self.parse_etag(etag),
            Some(XmlEvent::Characters(chars)) => Ok(Some(XmlNsEvent::Characters(chars))),
            Some(XmlEvent::XmlDecl(decl)) => Ok(Some(XmlNsEvent::XmlDecl(decl))),
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

        self.empty_element = stag.empty;

        Ok(Some(XmlNsEvent::STag(NsSTag {
            qname: QName::from_str(stag.name())?,
            empty: stag.empty,
        })))
    }

    fn parse_etag(&mut self, etag: ETag<'a>) -> Result<Option<XmlNsEvent<'a>>, XmlError> {
        self.namespaces.pop_scope();
        Ok(Some(XmlNsEvent::ETag(NsETag {
            qname: QName::from_str(etag.name())?,
        })))
    }
}
