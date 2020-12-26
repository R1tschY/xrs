use crate::chars::XmlBytesExt;
use crate::dom::{Document, Element};
use crate::reader::DomReader;
use crate::Span;
use quick_xml::events::{BytesDecl, BytesStart, BytesText, Event};
use quick_xml::Error;
use std::io::Cursor;
use std::str::from_utf8;

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
}

impl<'r, V> DomReader<'r> for QuickXmlDomReader<'r, V> {
    type Error = quick_xml::Error;

    fn parse(mut self) -> Result<Document<'r>, Self::Error> {
        self.reader.check_comments(false);
        self.reader.check_end_names(false);
        self.reader.trim_text(false);
        self.reader.trim_markup_names_in_closing_tags(true); // TODO: \x0c is not xml whitespace

        let mut buffer: Vec<u8> = Vec::with_capacity(1024);
        let mut stack: Vec<Element> = Vec::with_capacity(16);
        let mut root: Option<Element> = None;

        // prolog
        let mut doc_decl: Option<BytesDecl<'static>> = None;
        let mut doc_doctype: Option<BytesText<'static>> = None;

        loop {
            self.last_offset = self.offset;
            let evt = self.reader.read_event(&mut buffer)?;
            self.offset = self.reader.buffer_position();
            match evt {
                Event::Start(start) => {
                    stack.push(self.create_element(start));
                    break;
                }
                Event::Empty(start) => {
                    root = Some(self.create_element(start));
                    break;
                }
                Event::Text(text) if text.as_ref().only_xml_whitespace() => continue,
                Event::Comment(_) | Event::PI(_) => continue,
                Event::Decl(decl) => {
                    if let Some(_doctype) = doc_doctype {
                        return Err(quick_xml::Error::UnexpectedToken(
                            "doctype found before decl".to_string(),
                        ));
                    }
                    if let Some(_last_decl) = doc_decl {
                        return Err(quick_xml::Error::UnexpectedToken(
                            "Second decl found".to_string(),
                        ));
                    }
                    doc_decl = Some(decl.into_owned());
                }
                Event::DocType(doctype) => {
                    if let Some(_doctype) = doc_doctype {
                        return Err(quick_xml::Error::UnexpectedToken(
                            "doctype found before decl".to_string(),
                        ));
                    }
                    doc_doctype = Some(doctype.into_owned());
                }
                evt => {
                    return Err(quick_xml::Error::UnexpectedToken(format!(
                        "unexpected event before root element: {:?}",
                        evt
                    )))
                }
            }
        }

        // Inner XML
        while stack.len() != 0 {
            self.last_offset = self.offset;
            let evt = self.reader.read_event(&mut buffer)?;
            self.offset = self.reader.buffer_position();
            match evt {
                Event::Start(start) => {
                    stack.push(self.create_element(start));
                }
                Event::DocType(evt) => {
                    return Err(quick_xml::Error::UnexpectedToken(format!(
                        "unexpected event: {:?}",
                        evt
                    )))
                }
                Event::Decl(evt) => {
                    return Err(quick_xml::Error::UnexpectedToken(format!(
                        "unexpected event: {:?}",
                        evt
                    )))
                }
                Event::End(end) => {
                    if let Some(element) = stack.pop() {
                        let start_tag = element.tag_bytes(self.bytes);
                        if end.name() != start_tag {
                            return Err(quick_xml::Error::EndEventMismatch {
                                expected: from_utf8(start_tag)
                                    .map_err(|err| quick_xml::Error::Utf8(err))?
                                    .to_string(),
                                found: from_utf8(end.name())
                                    .map_err(|err| quick_xml::Error::Utf8(err))?
                                    .to_string(),
                            });
                        }

                        let stack_len = stack.len();
                        if stack_len == 0 {
                            root = Some(element);
                            break;
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
                        root = Some(element);
                        break;
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
                Event::Comment(_) | Event::PI(_) => {} // ignore
                Event::CData(_) => unimplemented!(),
                Event::Eof => return Err(Error::UnexpectedEof("missing root end".to_string())),
            }
        }

        let doc = if let Some(root) = root {
            Document::new(self.bytes, root)
        } else {
            unreachable!()
        };

        // no trailing content
        loop {
            match self.reader.read_event(&mut buffer)? {
                Event::Eof => return Ok(doc),
                Event::Text(text) if text.as_ref().only_xml_whitespace() => (),
                _ => return Err(Error::UnexpectedToken("trailing content".to_string())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::reader::quick_xml::QuickXmlDomReader;
    use crate::reader::DomReader;
    use crate::validate::NonValidator;
    use quick_xml::Error as XmlError;

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

        #[test]
        fn pre_text() {
            let reader = QuickXmlDomReader::new(b"sdsf<root></roo>", NonValidator);
            assert!(matches!(reader.parse(), Err(XmlError::UnexpectedToken(_))));
        }

        #[test]
        fn wrong_end() {
            let reader = QuickXmlDomReader::new(b"<root></roo>", NonValidator);
            assert!(matches!(
                reader.parse(), 
                Err(XmlError::EndEventMismatch { expected, found }) 
                if expected == "root" && found == "roo"));
        }

        #[test]
        fn eof() {
            let reader = QuickXmlDomReader::new(b"<root>", NonValidator);
            assert!(matches!(reader.parse(), Err(XmlError::UnexpectedEof(msg))));
        }

        #[test]
        fn double_decl() {
            let reader = QuickXmlDomReader::new(
                b"<?xml version=\"1.0\" ?><?xml version=\"1.0\" ?><root></root>",
                NonValidator,
            );
            assert!(matches!(reader.parse(), Err(XmlError::UnexpectedToken(_))));
        }

        #[test]
        fn doctype_before_decl() {
            let reader = QuickXmlDomReader::new(
                b"<!DOCTYPE root SYSTEM \"scheme.dtd\"><?xml version=\"1.0\" ?><root></root>",
                NonValidator,
            );
            assert!(matches!(reader.parse(), Err(XmlError::UnexpectedToken(_))));
        }

        #[test]
        fn double_doctype() {
            let reader = QuickXmlDomReader::new(
                b"<!DOCTYPE root SYSTEM \"scheme.dtd\"><!DOCTYPE root SYSTEM \"scheme.dtd\"><root></root>",
                NonValidator,
            );
            assert!(matches!(reader.parse(), Err(XmlError::UnexpectedToken(_))));
        }
    }
}
