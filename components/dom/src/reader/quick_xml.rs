use std::io::Cursor;
use std::str::from_utf8;

use quick_xml::events::{BytesDecl, BytesStart, BytesText, Event};
use quick_xml::Error as XmlError;

use crate::chars::XmlBytesExt;
use crate::dom::{Document, Element};
use crate::error::{Error, Reason, Result};
use crate::reader::DomReader;
use crate::Span;

pub struct QuickXmlDomReader<'r, V> {
    bytes: &'r [u8],
    reader: quick_xml::Reader<Cursor<&'r [u8]>>,
    last_offset: usize,
    offset: usize,
    validator: V,
}

impl<'r, V> QuickXmlDomReader<'r, V> {
    pub fn new(bytes: &'r [u8], validator: V) -> Self {
        Self {
            bytes,
            reader: quick_xml::Reader::from_reader(Cursor::new(bytes)),
            last_offset: 0,
            offset: 0,
            validator,
        }
    }

    pub fn create_element(&self, start: BytesStart) -> Element {
        Element::new(
            self.last_offset + 1,
            start.name().len(),
            start.attributes_raw().len(),
        )
    }

    pub fn error(&self, reason: Reason) -> Error {
        Error::new(self.reader.buffer_position(), 0, reason)
    }

    pub fn conv_utf8<'a>(&self, s: &'a [u8]) -> Result<&'a str> {
        from_utf8(s).map_err(|err| self.error(Reason::Utf8(err)))
    }

    fn xml_error(err: quick_xml::Error, offset: usize) -> Error {
        match err {
            quick_xml::Error::Io(err) => Error::new(offset, 0, Reason::Io(err)),
            quick_xml::Error::Utf8(err) => Error::new(offset, 0, Reason::Utf8(err)),
            quick_xml::Error::UnexpectedEof(_) => Error::new(offset, 0, Reason::UnexpectedEof),
            quick_xml::Error::EndEventMismatch { expected, found } => {
                Error::new(offset, 0, Reason::EndEventMismatch { expected, found })
            }
            quick_xml::Error::UnexpectedToken(token) => {
                Error::new(offset, 0, Reason::UnexpectedToken(token))
            }
            quick_xml::Error::UnexpectedBang => Error::new(offset, 0, Reason::InvalidBang),
            quick_xml::Error::TextNotFound => unreachable!(),
            quick_xml::Error::XmlDeclWithoutVersion(_) => {
                Error::new(offset, 0, Reason::XmlDeclWithoutVersion)
            }
            quick_xml::Error::NameWithQuote(pos) => {
                Error::new(offset + pos, 0, Reason::NameWithQuote)
            }
            quick_xml::Error::NoEqAfterName(pos) => {
                Error::new(offset + pos, 0, Reason::NoEqAfterName)
            }
            quick_xml::Error::UnquotedValue(pos) => {
                Error::new(offset + pos, 0, Reason::UnquotedValue)
            }
            quick_xml::Error::DuplicatedAttribute(pos1, pos2) => {
                Error::new(offset + pos1, 0, Reason::DuplicatedAttribute(offset + pos2))
            }
            quick_xml::Error::EscapeError(_) => Error::new(offset, 0, Reason::InvalidEntity),
        }
    }

    fn read_event<'a>(&mut self, buffer: &'a mut Vec<u8>) -> Result<quick_xml::events::Event<'a>> {
        self.last_offset = self.offset;
        let evt = self
            .reader
            .read_event(buffer)
            .map_err(|err| Self::xml_error(err, self.reader.buffer_position()))?;
        self.offset = self.reader.buffer_position();
        Ok(evt)
    }

    fn parse_inner_xml(&mut self, mut buffer: &mut Vec<u8>, start: Element) -> Result<Element> {
        let mut stack: Vec<Element> = Vec::with_capacity(16);
        stack.push(start);

        loop {
            match self.read_event(buffer)? {
                Event::Start(start) => {
                    stack.push(self.create_element(start));
                }
                Event::End(end) => {
                    if let Some(element) = stack.pop() {
                        let start_tag = element.tag_bytes(self.bytes);
                        if end.name() != start_tag {
                            return Err(self.error(Reason::EndEventMismatch {
                                expected: self.conv_utf8(start_tag)?.to_string(),
                                found: self.conv_utf8(end.name())?.to_string(),
                            }));
                        }

                        let stack_len = stack.len();
                        if stack_len == 0 {
                            return Ok(element);
                        } else {
                            stack[stack_len - 1].push_child(element);
                        }
                    } else {
                        unreachable!()
                    }
                }
                Event::Empty(start) => {
                    let element = self.create_element(start);
                    let stack_len = stack.len();
                    if stack_len == 0 {
                        return Ok(element);
                    } else {
                        stack[stack_len - 1].push_child(element);
                    }
                }
                Event::Text(text) => {
                    if text.len() > 0 {
                        let stack_len = stack.len();
                        let mut top = &mut stack[stack_len - 1];
                        let span = Span::new(self.last_offset, text.len());
                        let children_len = top.children().len();
                        if children_len == 0 {
                            debug_assert!(
                                !top.has_text(),
                                "tried to reassign text {:?} with {:?}",
                                top.text_from_docbytes(self.bytes).unwrap(),
                                from_utf8(span.to_slice(self.bytes)).unwrap()
                            );
                            top.push_text(span);
                        } else {
                            debug_assert!(
                                !top.children()[children_len - 1].has_tail(),
                                "tried to reassign tail {:?} with {:?}",
                                top.tail_from_docbytes(self.bytes).unwrap(),
                                from_utf8(span.to_slice(self.bytes)).unwrap()
                            );
                            top.children_mut()[children_len - 1].push_tail(span);
                        }
                    }
                }
                Event::DocType(_) => return Err(self.error(Reason::UnexpectedDocType)),
                Event::Decl(_) => return Err(self.error(Reason::UnexpectedDecl)),
                Event::Comment(_) | Event::PI(_) => {} // ignore
                Event::CData(_) => unimplemented!(),
                Event::Eof => return Err(self.error(Reason::UnexpectedEof)),
            }
        }
    }
}

impl<'r, V> DomReader<'r> for QuickXmlDomReader<'r, V> {
    type Error = Error;

    fn parse(mut self) -> Result<Document<'r>> {
        self.reader.check_comments(false);
        self.reader.check_end_names(false);
        self.reader.trim_text(false);
        self.reader.trim_markup_names_in_closing_tags(true); // TODO: \x0c is not xml whitespace

        let mut buffer: Vec<u8> = Vec::with_capacity(1024);

        // prolog
        let mut doc_decl: Option<BytesDecl<'static>> = None;
        let mut doc_doctype: Option<BytesText<'static>> = None;

        let root: Element = loop {
            match self.read_event(&mut buffer)? {
                Event::Start(start) => {
                    let root = self.create_element(start);
                    break self.parse_inner_xml(&mut buffer, root)?;
                }
                Event::Empty(start) => {
                    break self.create_element(start);
                }
                Event::Text(text) if text.as_ref().only_xml_whitespace() => continue,
                Event::Comment(_) | Event::PI(_) => continue,
                Event::Decl(decl) => {
                    if doc_doctype.is_some() || doc_decl.is_some() {
                        return Err(self.error(Reason::UnexpectedDecl));
                    }
                    doc_decl = Some(decl.into_owned());
                }
                Event::DocType(doctype) => {
                    if doc_doctype.is_some() {
                        return Err(self.error(Reason::UnexpectedDocType));
                    }
                    doc_doctype = Some(doctype.into_owned());
                }
                Event::End(end) => {
                    return Err(self.error(Reason::EndEventMismatch {
                        expected: "".to_string(),
                        found: self.conv_utf8(end.name())?.to_string(),
                    }))
                }
                Event::Text(_) | Event::CData(_) => {}
                Event::Eof => {}
            }
        };

        // no trailing content
        loop {
            match self.read_event(&mut buffer)? {
                Event::Eof => return Ok(Document::new(self.bytes, root)),
                Event::Text(text) if text.as_ref().only_xml_whitespace() => (),
                _ => return Err(self.error(Reason::TrailingContent)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quick_xml::Error as XmlError;

    use crate::reader::quick_xml::QuickXmlDomReader;
    use crate::reader::DomReader;
    use crate::validate::NonValidator;

    #[test]
    fn only_root() {
        let reader = QuickXmlDomReader::new(b"<root></root>", NonValidator);
        let doc = reader.parse().unwrap();
        assert_eq!("root", doc.get_root().tag(&doc).unwrap());
        assert_eq!("", doc.get_root().text(&doc).unwrap());
        assert_eq!(0, doc.get_root().children().len());
    }

    #[test]
    fn empty() {
        let reader = QuickXmlDomReader::new(b"<root />", NonValidator);
        let doc = reader.parse().unwrap();
        assert_eq!("root", doc.get_root().tag(&doc).unwrap());
        assert_eq!("", doc.get_root().text(&doc).unwrap());
        assert_eq!(0, doc.get_root().children().len());
    }

    #[test]
    fn whitespace() {
        let reader = QuickXmlDomReader::new(b"<root >\n\r</root\t>\n\r  ", NonValidator);
        let doc = reader.parse().unwrap();
        assert_eq!("root", doc.get_root().tag(&doc).unwrap());
        assert_eq!("\n\r", doc.get_root().text(&doc).unwrap());
        assert_eq!(0, doc.get_root().children().len());
    }

    #[test]
    fn tail() {
        let reader = QuickXmlDomReader::new(b"<root>text<elem/>tail</root>", NonValidator);
        let doc = reader.parse().unwrap();
        assert_eq!("text", doc.get_root().text(&doc).unwrap());
        assert_eq!("tail", doc.get_root().children()[0].tail(&doc).unwrap());
    }

    #[test]
    fn pre() {
        let reader = QuickXmlDomReader::new(
            b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
            <!DOCTYPE root SYSTEM \"scheme.dtd\">
            <!-- Comment -->
            <root></root>",
            NonValidator,
        );
        let doc = reader.parse().unwrap();
        assert_eq!("root", doc.get_root().tag(&doc).unwrap());
    }

    mod structure_fails {
        use super::*;
        use crate::error::Reason;

        #[test]
        fn pre_text() {
            let reader = QuickXmlDomReader::new(b"sdsf<root></roo>", NonValidator);
            assert!(matches!(
                reader.parse().map_err(|err| err.reason),
                Err(Reason::UnexpectedToken(_))
            ));
        }

        #[test]
        fn wrong_end() {
            let reader = QuickXmlDomReader::new(b"<root></roo>", NonValidator);
            assert!(matches!(
                reader.parse().map_err(|err| err.reason),
                Err(Reason::EndEventMismatch { expected, found })
                if expected == "root" && found == "roo"));
        }

        #[test]
        fn eof() {
            let reader = QuickXmlDomReader::new(b"<root>", NonValidator);
            assert!(matches!(
                reader.parse().map_err(|err| err.reason),
                Err(Reason::UnexpectedEof)
            ));
        }

        #[test]
        fn double_decl() {
            let reader = QuickXmlDomReader::new(
                b"<?xml version=\"1.0\" ?><?xml version=\"1.0\" ?><root></root>",
                NonValidator,
            );
            assert!(matches!(
                reader.parse().map_err(|err| err.reason),
                Err(Reason::UnexpectedDecl)
            ));
        }

        #[test]
        fn doctype_before_decl() {
            let reader = QuickXmlDomReader::new(
                b"<!DOCTYPE root SYSTEM \"scheme.dtd\"><?xml version=\"1.0\" ?><root></root>",
                NonValidator,
            );
            assert!(matches!(
                reader.parse().map_err(|err| err.reason),
                Err(Reason::UnexpectedDecl)
            ));
        }

        #[test]
        fn double_doctype() {
            let reader = QuickXmlDomReader::new(
                b"<!DOCTYPE root SYSTEM \"scheme.dtd\"><!DOCTYPE root SYSTEM \"scheme.dtd\"><root></root>",
                NonValidator,
            );
            assert!(matches!(
                reader.parse().map_err(|err| err.reason),
                Err(Reason::UnexpectedDocType)
            ));
        }
    }
}
