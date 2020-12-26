use quick_xml::events::{BytesDecl, BytesStart, BytesText, Event};
use quick_xml::{Error as XmlError, Error};
use std::borrow::Cow;
use std::io::Cursor;
use std::str::{from_utf8, Utf8Error};

use self::chars::*;

mod chars;

#[derive(Copy, Clone, Eq, PartialEq)]
struct Span {
    start: usize,
    len: usize,
}

impl Span {
    pub fn new(start: usize, len: usize) -> Self {
        Self { start, len }
    }

    pub fn empty() -> Self {
        Self::new(0, 0)
    }

    pub fn to_slice<'a>(&self, bytes: &'a [u8]) -> &'a [u8] {
        &bytes[self.start..self.start + self.len]
    }

    pub fn is_null(&self) -> bool {
        self.start == 0 && self.len == 0
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

enum CowSpan {
    Borrowed(Span),
    Owned(Vec<u8>),
}

struct QName {
    namespace: Option<CowSpan>,
    prefix: Span,
    local_name: Span,
}

impl QName {
    pub fn new(namespace: Option<CowSpan>, prefix: Span, local_name: Span) -> Self {
        Self {
            namespace,
            prefix,
            local_name,
        }
    }
}

pub struct Document<'a> {
    bytes: &'a [u8],
    root: Element,
}

impl<'a> Document<'a> {
    pub fn get_root(&self) -> &Element {
        &self.root
    }
}

pub struct Element {
    offset: usize,
    tag: Span,
    text: Span,
    tail: Span,
    children: Vec<Element>,
    attributes: Span,
    namespaces: Option<Vec<(Span, Span)>>,
}

impl Element {
    pub fn tag<'a>(&self, doc: &'a Document) -> Result<&'a str, Utf8Error> {
        from_utf8(self.tag.to_slice(doc.bytes))
    }

    pub fn children(&self) -> &[Element] {
        &self.children
    }

    pub fn text<'a>(&self, doc: &'a Document) -> Result<&'a str, Utf8Error> {
        from_utf8(self.text.to_slice(doc.bytes))
    }

    pub fn tail<'a>(&self, doc: &'a Document) -> Result<&'a str, Utf8Error> {
        from_utf8(self.tail.to_slice(doc.bytes))
    }
}

pub trait XmlValidator {
    fn validate_tag(&self, tag: &[u8]) -> Result<(), XmlError>;
    fn validate_comment(&self, comment: &[u8]) -> Result<(), XmlError>;
    fn validate_text(&self, text: &[u8]) -> Result<(), XmlError>;
}

pub struct NonValidator;

impl XmlValidator for NonValidator {
    fn validate_tag(&self, _tag: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_comment(&self, _comment: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_text(&self, _text: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

pub trait DomReader<'r> {
    type Error;

    fn parse(self) -> Result<Document<'r>, Self::Error>;
}

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
        let attrs_len = start.attributes_raw().len();
        let offset = self.last_offset + 1;

        Element {
            offset,
            tag: Span::new(offset, start.name().len()),
            text: Span::empty(),
            tail: Span::empty(),
            children: vec![],
            attributes: Span::new(offset + start.name().len(), attrs_len),
            namespaces: None,
        }
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
                Event::Text(text) if text.as_ref().only_xml_whitespace() =>
                {
                    continue
                }
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
                        let start_tag = element.tag.to_slice(self.bytes);
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
                            stack[stack_len - 1].children.push(element);
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
                        stack[stack_len - 1].children.push(element);
                    }
                }
                Event::Text(text) => {
                    if text.len() > 0 {
                        let stack_len = stack.len();
                        let mut top = &mut stack[stack_len - 1];
                        let span = Span::new(self.last_offset, text.len());
                        if top.children.len() == 0 {
                            debug_assert!(
                                top.text.is_null(),
                                "tried to reassign text {:?} with {:?}",
                                from_utf8(top.text.to_slice(self.bytes)).unwrap(),
                                from_utf8(span.to_slice(self.bytes)).unwrap()
                            );
                            top.text = span;
                        } else {
                            let children_len = top.children.len();
                            debug_assert!(
                                top.children[children_len - 1].tail.is_null(),
                                "tried to reassign tail {:?} with {:?}",
                                from_utf8(top.text.to_slice(self.bytes)).unwrap(),
                                from_utf8(span.to_slice(self.bytes)).unwrap()
                            );
                            top.children[children_len - 1].tail = span;
                        }
                    }
                }
                Event::Comment(_) | Event::PI(_) => {} // ignore
                Event::CData(_) => unimplemented!(),
                Event::Eof => return Err(XmlError::UnexpectedEof("missing root end".to_string())),
            }
        }

        let doc = if let Some(root) = root {
            Document {
                bytes: self.bytes,
                root,
            }
        } else {
            unreachable!()
        };

        // no trailing content
        loop {
            match self.reader.read_event(&mut buffer)? {
                Event::Eof => return Ok(doc),
                Event::Text(text) if text.as_ref().only_xml_whitespace() => (),
                _ => return Err(XmlError::UnexpectedToken("trailing content".to_string())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
