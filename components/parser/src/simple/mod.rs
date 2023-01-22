use std::borrow::Cow;

use xrs_chars::{XmlAsciiChar, XmlChar};

use crate::parser::core::optional;
use crate::parser::Parser;
use crate::reader::chars::is_ascii_content_char;
use crate::reader::{
    AttValueToken, CDataToken, CharRefToken, CommentToken, EntityRefToken, EntityStrValueResolver,
    EqToken, NameToken, PIToken, SToken, XmlDeclToken,
};
use crate::XmlError::{UnexpectedCharacter, UnexpectedEof};
use crate::{Cursor, XmlDecl, XmlDtdError, XmlError, XmlErrorAtom};

mod namespace;

pub trait SimpleXmlVisitor<'i>: Sized {
    type Value;

    fn visit_start_element<A: AttributeAccess<'i>>(
        self,
        tag: &'i str,
        attrs: A,
    ) -> Result<Self::Value, XmlError>;
    fn visit_end_element(self, tag: &'i str) -> Result<Self::Value, XmlError>;
    fn visit_declaration(self, decl: XmlDecl) -> Result<Self::Value, XmlError>;
    fn visit_characters(self, characters: &'i str) -> Result<Self::Value, XmlError>;
    fn visit_borrowed_characters(self, characters: &str) -> Result<Self::Value, XmlError>;
    fn visit_pi(self, target: &'i str, data: Option<&'i str>) -> Result<Self::Value, XmlError>;
    fn visit_comment(self, comment: &'i str) -> Result<Self::Value, XmlError>;
}

pub trait AttributeAccess<'i>: Sized {
    fn next_entry<K: StrVisitor<'i>, V: StrVisitor<'i>>(
        &mut self,
        key_visitor: K,
        value_visitor: V,
    ) -> Result<Option<(K::Value, V::Value)>, XmlError>;
}

pub trait StrVisitor<'i>: Sized {
    type Value;

    fn visit_str(self, value: &str) -> Result<Self::Value, XmlError>;

    fn visit_borrowed(self, value: &'i str) -> Result<Self::Value, XmlError> {
        self.visit_str(value)
    }

    fn visit_string(self, value: String) -> Result<Self::Value, XmlError> {
        self.visit_str(&value)
    }

    fn visit_cow(self, value: Cow<'i, str>) -> Result<Self::Value, XmlError> {
        match value {
            Cow::Borrowed(borrowed) => self.visit_borrowed(borrowed),
            Cow::Owned(owned) => self.visit_string(owned),
        }
    }
}

pub struct CowVisitor;

impl<'i> StrVisitor<'i> for CowVisitor {
    type Value = Cow<'i, str>;

    fn visit_str(self, value: &str) -> Result<Self::Value, XmlError> {
        Ok(Cow::Owned(value.to_string()))
    }

    fn visit_borrowed(self, value: &'i str) -> Result<Self::Value, XmlError> {
        Ok(Cow::Borrowed(value))
    }

    fn visit_string(self, value: String) -> Result<Self::Value, XmlError> {
        Ok(Cow::Owned(value))
    }

    fn visit_cow(self, value: Cow<'i, str>) -> Result<Self::Value, XmlError> {
        Ok(value)
    }
}

pub struct StringVisitor;

impl<'i> StrVisitor<'i> for StringVisitor {
    type Value = String;

    fn visit_str(self, value: &str) -> Result<Self::Value, XmlError> {
        Ok(value.to_string())
    }

    fn visit_string(self, value: String) -> Result<Self::Value, XmlError> {
        Ok(value)
    }
}

/// Simple XML parser
///
/// Does not support DTDs and only UTF-8 strings.
///
/// Should be sufficient for most modern XML.
pub struct SimpleXmlParser<'i> {
    state: ParserState,
    cursor: Cursor<'i>,
    empty: bool,
    attribute_names: Vec<&'i str>,
    stack: Vec<&'i str>,
    version: Option<String>,
}

pub enum ParserState {
    XmlDecl,
    Prologue,
    Main,
    Epilogue,
}

impl<'i> SimpleXmlParser<'i> {
    pub fn from_str(input: &'i str) -> Self {
        Self {
            state: ParserState::XmlDecl,
            cursor: Cursor::new(input),
            empty: false,
            attribute_names: vec![],
            stack: vec![],
            version: None,
        }
    }

    pub fn cursor_offset(&self) -> usize {
        self.cursor.offset()
    }

    pub fn unparsed(&self) -> &'i str {
        self.cursor.rest()
    }

    pub fn parse_next<V: SimpleXmlVisitor<'i>>(
        &mut self,
        visitor: V,
    ) -> Result<Option<V::Value>, XmlError> {
        Ok(Some(match self.state {
            ParserState::XmlDecl => self.parse_xml_decl(visitor),
            ParserState::Prologue => self.parse_prologue(visitor),
            ParserState::Main => self.parse_root_element(visitor),
            ParserState::Epilogue => return self.parse_epilogue(visitor),
        }?))
    }

    fn parse_xml_decl<V: SimpleXmlVisitor<'i>>(
        &mut self,
        visitor: V,
    ) -> Result<V::Value, XmlError> {
        while let Some(c) = self.cursor.next_byte(0) {
            return match c {
                b'<' => {
                    if let Some(c) = self.cursor.next_byte(1) {
                        if c == b'?' {
                            self.state = ParserState::Prologue;
                            if self.is_decl_start() {
                                self.parse_decl(visitor)
                            } else {
                                self.parse_pi(visitor)
                            }
                        } else if c == b'!' {
                            if self.cursor.has_next_str("<!--") {
                                self.parse_comment(visitor)
                            } else if self.cursor.has_next_str("<!DOCTYPE") {
                                Err(XmlError::DtdError(XmlDtdError::Unsupported))
                            } else {
                                break;
                            }
                        } else {
                            self.cursor = self.cursor.advance(1);
                            self.state = ParserState::Main;
                            self.parse_stag(visitor)
                        }
                    } else {
                        break;
                    }
                }
                _ => {
                    self.consume_whitespace(c)?;
                    continue;
                }
            };
        }

        Err(XmlError::Expected(Box::new([
            XmlErrorAtom::XmlDecl,
            XmlErrorAtom::Comment,
            XmlErrorAtom::PI,
            XmlErrorAtom::Element,
            XmlErrorAtom::Whitespace,
        ])))
    }

    fn is_decl_start(&self) -> bool {
        self.cursor.has_next_str("<?xml")
            && self
                .cursor
                .next_byte(5)
                .map(|c| c.is_xml_whitespace() || !c.is_ascii_alphabetic() && !b":_-.".contains(&c))
                .unwrap_or(false)
    }

    fn parse_prologue<V: SimpleXmlVisitor<'i>>(
        &mut self,
        visitor: V,
    ) -> Result<V::Value, XmlError> {
        while let Some(c) = self.cursor.next_byte(0) {
            return match c {
                b'<' => {
                    if let Some(c) = self.cursor.next_byte(1) {
                        if c == b'?' {
                            self.parse_pi(visitor)
                        } else if c == b'!' {
                            if self.cursor.has_next_str("<!--") {
                                self.parse_comment(visitor)
                            } else if self.cursor.has_next_str("<!DOCTYPE") {
                                Err(XmlError::DtdError(XmlDtdError::Unsupported))
                            } else {
                                break;
                            }
                        } else {
                            self.cursor = self.cursor.advance(1);
                            self.state = ParserState::Main;
                            self.parse_stag(visitor)
                        }
                    } else {
                        break;
                    }
                }
                _ => {
                    self.consume_whitespace(c)?;
                    continue;
                }
            };
        }

        Err(XmlError::Expected(Box::new([
            XmlErrorAtom::Comment,
            XmlErrorAtom::PI,
            XmlErrorAtom::Element,
            XmlErrorAtom::Whitespace,
        ])))
    }

    fn parse_root_element<V: SimpleXmlVisitor<'i>>(
        &mut self,
        visitor: V,
    ) -> Result<V::Value, XmlError> {
        if self.empty {
            self.empty = false;
            if let Some(name) = self.stack.pop() {
                if self.stack.is_empty() {
                    self.state = ParserState::Epilogue;
                }
                return Ok(visitor.visit_end_element(name)?);
            }
            unreachable!()
        }

        while let Some(c) = self.cursor.next_byte(0) {
            return match c {
                b'<' => {
                    if let Some(c) = self.cursor.next_byte(1) {
                        if c == b'/' {
                            self.cursor = self.cursor.advance(2);
                            self.parse_etag(visitor)
                        } else if c == b'!' {
                            if self.cursor.has_next_str("<!--") {
                                self.parse_comment(visitor)
                            } else if self.cursor.has_next_str("<![CDATA[") {
                                self.parse_cdata(visitor)
                            } else if self.cursor.has_next_str("<!DOCTYPE") {
                                Err(XmlError::DtdError(XmlDtdError::Unsupported))
                            } else {
                                Err(XmlError::ExpectedElementStart)
                            }
                        } else if c == b'?' {
                            self.parse_pi(visitor)
                        } else {
                            self.cursor = self.cursor.advance(1);
                            self.parse_stag(visitor)
                        }
                    } else {
                        Err(XmlError::ExpectedElementStart)
                    }
                }
                b'&' => self.parse_reference(visitor),
                b'\r' => {
                    let c = self.cursor.next_byte(1);
                    self.commit(self.cursor.advance(1));
                    if c == Some(b'\n') {
                        continue;
                    } else {
                        visitor.visit_characters("\n")
                    }
                }
                _ => self.parse_characters(visitor),
            };
        }

        Err(XmlError::OpenElementAtEof)
    }

    fn parse_epilogue<V: SimpleXmlVisitor<'i>>(
        &mut self,
        visitor: V,
    ) -> Result<Option<V::Value>, XmlError> {
        while let Some(c) = self.cursor.next_byte(0) {
            match c {
                b'<' => {
                    if let Some(c) = self.cursor.next_byte(1) {
                        if c == b'?' {
                            return Ok(Some(self.parse_pi(visitor)?));
                        } else if self.cursor.has_next_str("<!--") {
                            return Ok(Some(self.parse_comment(visitor)?));
                        }
                    }

                    return Err(XmlError::Expected(Box::new([
                        XmlErrorAtom::Comment,
                        XmlErrorAtom::PI,
                    ])));
                }
                _ => {
                    self.consume_whitespace(c)?;
                    continue;
                }
            };
        }
        Ok(None)
    }

    fn consume_whitespace(&mut self, c: u8) -> Result<(), XmlError> {
        // only white space allowed
        if c.is_xml_whitespace() {
            let (_, cur) = SToken.parse(self.cursor)?;
            self.cursor = cur;
            Ok(())
        } else {
            Err(UnexpectedCharacter(self.cursor.next_char().unwrap()))
        }
    }

    fn parse_stag<V: SimpleXmlVisitor<'i>>(&mut self, visitor: V) -> Result<V::Value, XmlError> {
        let (name, mut cursor) = NameToken.parse(self.cursor)?;
        let mut got_whitespace = if let Ok((_, cur)) = SToken.parse(cursor) {
            self.cursor = cur;
            true
        } else {
            self.cursor = cursor;
            false
        };

        self.stack.push(name);
        self.attribute_names.clear();

        visitor.visit_start_element(
            name,
            SimpleAttributeAccess {
                parser: self,
                got_whitespace,
            },
        )
    }

    fn parse_etag<V: SimpleXmlVisitor<'i>>(&mut self, visitor: V) -> Result<V::Value, XmlError> {
        // TODO: xml_lit(self.stack.pop()) should be faster
        let (name, cursor) = NameToken.parse(self.cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let cursor = expect_byte(cursor, b'>', || XmlError::ExpectedElementEnd)?;
        self.commit(cursor);

        if let Some(expected_name) = self.stack.pop() {
            if expected_name == name {
                if self.stack.is_empty() {
                    self.state = ParserState::Epilogue;
                }
                visitor.visit_end_element(name)
            } else {
                Err(XmlError::WrongETagName {
                    expected_name: expected_name.to_string(),
                })
            }
        } else {
            unreachable!()
        }
    }

    fn parse_decl<V: SimpleXmlVisitor<'i>>(&mut self, visitor: V) -> Result<V::Value, XmlError> {
        let (decl, cursor) = XmlDeclToken.parse(self.cursor)?;

        self.version = Some(decl.version.to_string());

        // TODO: handle encoding better
        // if let Some(encoding) = &decl.encoding {
        //     if !encoding.eq_ignore_ascii_case("UTF-8") {
        //         return Err(XmlError::UnsupportedEncoding(encoding.to_string()));
        //     }
        // }

        self.commit(cursor);
        visitor.visit_declaration(decl)
    }

    fn parse_pi<V: SimpleXmlVisitor<'i>>(&mut self, visitor: V) -> Result<V::Value, XmlError> {
        let (pi, cursor) = PIToken.parse(self.cursor)?;
        self.commit(cursor);
        visitor.visit_pi(pi.0, pi.1)
    }

    fn parse_comment<V: SimpleXmlVisitor<'i>>(&mut self, visitor: V) -> Result<V::Value, XmlError> {
        let (comment, cursor) = CommentToken.parse(self.cursor)?;
        self.commit(cursor);
        visitor.visit_comment(comment)
    }

    fn parse_cdata<V: SimpleXmlVisitor<'i>>(&mut self, visitor: V) -> Result<V::Value, XmlError> {
        let (cdata, cursor) = CDataToken.parse(self.cursor)?;
        self.commit(cursor);
        visitor.visit_characters(cdata.into())
    }

    fn parse_characters<V: SimpleXmlVisitor<'i>>(
        &mut self,
        visitor: V,
    ) -> Result<V::Value, XmlError> {
        if let Some((i, c)) = self
            .cursor
            .rest()
            .char_indices()
            .find(|(_, c)| !is_ascii_content_char(*c))
        {
            if c.is_xml_char() {
                debug_assert!(i > 0);
                let (chars, cursor) = self.cursor.advance2(i);
                self.commit(cursor);
                if chars.contains("]]>") {
                    Err(XmlError::IllegalCDataSectionEnd)
                } else {
                    visitor.visit_characters(chars)
                }
            } else {
                Err(XmlError::InvalidCharacter(c))
            }
        } else {
            Err(XmlError::UnexpectedEof)
        }
    }

    fn parse_reference<V: SimpleXmlVisitor<'i>>(
        &mut self,
        visitor: V,
    ) -> Result<V::Value, XmlError> {
        let cur = self.cursor;
        if let Some(c) = cur.next_byte(1) {
            if c == b'#' {
                let (character, cursor) = CharRefToken.parse(cur)?;
                self.commit(cursor);
                let mut buf = [0; 4];
                visitor.visit_borrowed_characters(character.encode_utf8(&mut buf))
            } else {
                let (entity_ref, cursor) = EntityRefToken.parse(cur)?;
                self.commit(cursor);
                visitor.visit_characters(match entity_ref {
                    "apos" => "\'",
                    "quot" => "\"",
                    "lt" => "<",
                    "gt" => ">",
                    "amp" => "&",
                    _ => return Err(XmlError::UnknownEntity(entity_ref.to_string())),
                })
            }
        } else {
            Err(XmlError::IllegalReference)
        }
    }

    fn consume(&mut self, n: usize) {
        self.cursor = self.cursor.advance(n);
    }

    fn commit(&mut self, cursor: Cursor<'i>) {
        self.cursor = cursor;
    }

    fn exists_attribute_name(&self, attr_name: &str) -> bool {
        self.attribute_names.iter().any(|&name| name == attr_name)
    }
}

struct SimpleEntityStrValueResolver;

impl<'i> EntityStrValueResolver<'i> for SimpleEntityStrValueResolver {}

struct SimpleAttributeAccess<'a, 'i> {
    parser: &'a mut SimpleXmlParser<'i>,
    got_whitespace: bool,
}

impl<'a, 'i> AttributeAccess<'i> for SimpleAttributeAccess<'a, 'i> {
    fn next_entry<K: StrVisitor<'i>, V: StrVisitor<'i>>(
        &mut self,
        key_visitor: K,
        value_visitor: V,
    ) -> Result<Option<(K::Value, V::Value)>, XmlError> {
        if let Some(c) = self.parser.cursor.next_byte(0) {
            // /> empty end
            if c == b'/' {
                return if Some(b'>') == self.parser.cursor.next_byte(1) {
                    self.parser.consume(2);
                    self.parser.empty = true;
                    Ok(None)
                } else {
                    Err(XmlError::ExpectedElementEnd)
                };
            }

            // normal end
            if c == b'>' {
                self.parser.consume(1);
                return Ok(None);
            }

            // attribute
            if !self.got_whitespace {
                return Err(XmlError::ExpectedWhitespace);
            }

            let (attr_name, cur) = NameToken.parse(self.parser.cursor)?;
            let (_, cur) = EqToken.parse(cur)?;
            let (value, cur) = AttValueToken::new(SimpleEntityStrValueResolver).parse(cur)?;
            if let Ok((_, cur)) = SToken.parse(cur) {
                self.parser.commit(cur);
                self.got_whitespace = true;
            } else {
                self.parser.commit(cur);
                self.got_whitespace = false;
            }

            if self.parser.exists_attribute_name(attr_name) {
                return Err(XmlError::NonUniqueAttribute {
                    attribute: attr_name.to_string(),
                });
            }
            self.parser.attribute_names.push(attr_name);

            let key = key_visitor.visit_borrowed(attr_name)?;
            let value = value_visitor.visit_cow(value)?;
            Ok(Some((key, value)))
        } else {
            Err(XmlError::UnexpectedEof)
        }
    }
}

fn expect_byte(cursor: Cursor, c: u8, err: impl Fn() -> XmlError) -> Result<Cursor, XmlError> {
    if cursor.next_byte(0) == Some(c) {
        Ok(cursor.advance(1))
    } else {
        Err((err)())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    macro_rules! assert_evt {
        ($exp:expr, $parser:expr) => {{
            let evt = $parser.parse_next(EventVisitor);
            assert_eq!($exp, evt, "error at {:?}", $parser.unparsed())
        }};
    }

    macro_rules! assert_evt_matches {
        ($exp:pat, $parser:expr) => {
            assert!(
                matches!($parser.parse_next(EventVisitor), $exp),
                "error at {:?}",
                $parser.unparsed()
            )
        };
    }

    fn empty_array<T>() -> &'static [T] {
        &[]
    }

    #[derive(PartialEq, Debug)]
    enum Event<'i> {
        Decl(XmlDecl),
        PI(&'i str, Option<&'i str>),
        Comment(&'i str),
        Start(&'i str, Vec<(Cow<'i, str>, Cow<'i, str>)>),
        End(&'i str),
        Chars(Cow<'i, str>),
    }

    impl<'i> Event<'i> {
        pub fn decl(version: &'i str, encoding: Option<&'i str>, standalone: Option<bool>) -> Self {
            Event::Decl(XmlDecl {
                version: version.to_string(),
                encoding: encoding.map(|enc| enc.to_string()),
                standalone,
            })
        }
    }

    struct EventVisitor;

    impl<'i> SimpleXmlVisitor<'i> for EventVisitor {
        type Value = Event<'i>;

        fn visit_start_element<A: AttributeAccess<'i>>(
            self,
            tag: &'i str,
            mut attrs: A,
        ) -> Result<Self::Value, XmlError> {
            let mut attrs_vec: Vec<(Cow<'i, str>, Cow<'i, str>)> = vec![];
            while let Some((key, value)) = attrs.next_entry(CowVisitor, CowVisitor)? {
                attrs_vec.push((key, value));
            }
            Ok(Event::Start(tag, attrs_vec))
        }

        fn visit_end_element(self, tag: &'i str) -> Result<Self::Value, XmlError> {
            Ok(Event::End(tag))
        }

        fn visit_declaration(self, decl: XmlDecl) -> Result<Self::Value, XmlError> {
            Ok(Event::Decl(decl))
        }

        fn visit_characters(self, characters: &'i str) -> Result<Self::Value, XmlError> {
            Ok(Event::Chars(Cow::Borrowed(characters)))
        }

        fn visit_borrowed_characters(self, characters: &str) -> Result<Self::Value, XmlError> {
            Ok(Event::Chars(Cow::Owned(characters.to_string())))
        }

        fn visit_pi(self, target: &'i str, data: Option<&'i str>) -> Result<Self::Value, XmlError> {
            Ok(Event::PI(target, data))
        }

        fn visit_comment(self, comment: &'i str) -> Result<Self::Value, XmlError> {
            Ok(Event::Comment(comment))
        }
    }

    mod stag {
        use crate::simple::SimpleXmlParser;

        use super::*;

        #[test]
        fn single_element() {
            let mut parser = SimpleXmlParser::from_str("<elem></elem>");
            assert_evt!(Ok(Some(Event::Start("elem", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("elem"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn single_element_whitespace() {
            let mut parser = SimpleXmlParser::from_str("<elem  ></elem   >");
            assert_evt!(Ok(Some(Event::Start("elem", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("elem"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn empty_element() {
            let mut parser = SimpleXmlParser::from_str("<elem/>");
            assert_evt!(Ok(Some(Event::Start("elem", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("elem"))), parser);
            assert_evt!(Ok(None), parser);
        }
    }

    mod attributes {
        use super::*;

        #[test]
        fn attribute() {
            let mut parser = SimpleXmlParser::from_str("<elem attr=\"value\"/>");
            assert_evt!(
                Ok(Some(Event::Start(
                    "elem",
                    vec![("attr".into(), "value".into())]
                ))),
                parser
            );
            assert_evt!(Ok(Some(Event::End("elem"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn attribute_whitespace() {
            let mut parser = SimpleXmlParser::from_str("<elem \t \n \r attr  =  \"value\"  />");
            assert_evt!(
                Ok(Some(Event::Start(
                    "elem",
                    vec![("attr".into(), "value".into())]
                ))),
                parser
            );
            assert_evt!(Ok(Some(Event::End("elem"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn single_quote_attribute() {
            let mut parser = SimpleXmlParser::from_str("<elem attr='value'/>");
            assert_evt!(
                Ok(Some(Event::Start(
                    "elem",
                    vec![("attr".into(), "value".into())]
                ))),
                parser
            );
            assert_evt!(Ok(Some(Event::End("elem"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn single_quote_attribute_whitespace() {
            let mut parser = SimpleXmlParser::from_str("<elem attr  =  'value'  />");
            assert_evt!(
                Ok(Some(Event::Start(
                    "elem",
                    vec![("attr".into(), "value".into())]
                ))),
                parser
            );
            assert_evt!(Ok(Some(Event::End("elem"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn multiple_attributes() {
            let mut parser = SimpleXmlParser::from_str("<e a='v' b='w' />");
            assert_evt!(
                Ok(Some(Event::Start(
                    "e",
                    vec![("a".into(), "v".into()), ("b".into(), "w".into())]
                ))),
                parser
            );
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn attribute_duplicate() {
            let mut parser = SimpleXmlParser::from_str("<e a='' a='' />");
            assert_evt!(
                Err(XmlError::NonUniqueAttribute {
                    attribute: "a".to_string()
                }),
                parser
            );
        }

        #[test]
        fn attribute_missing_value() {
            let mut parser = SimpleXmlParser::from_str("<e a></e>");
            assert_evt!(Err(XmlError::ExpectToken("=")), parser);
        }

        #[test]
        fn attribute_wrong_quote() {
            let mut parser = SimpleXmlParser::from_str("<e a='v\"></e>");
            assert_evt!(
                Err(XmlError::IllegalAttributeValue(
                    "< not allowed in attribute value"
                )),
                parser
            );
        }

        #[test]
        fn attribute_missing_quote() {
            let mut parser = SimpleXmlParser::from_str("<e a=v></e>");
            assert_evt!(Err(XmlError::ExpectToken("quote or single quote")), parser);
        }
    }

    mod etag {
        use super::*;

        #[test]
        fn fail_on_missing_etag() {
            let mut parser = SimpleXmlParser::from_str("<e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::OpenElementAtEof), parser);
        }

        #[test]
        fn fail_on_open_etag() {
            let mut parser = SimpleXmlParser::from_str("<e></e></e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(
                Err(XmlError::Expected(Box::new([
                    XmlErrorAtom::Comment,
                    XmlErrorAtom::PI
                ]))),
                parser
            );
        }

        #[test]
        fn fail_on_wrong_etag() {
            let mut parser = SimpleXmlParser::from_str("<e></d>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::WrongETagName {
                    expected_name: "e".to_string(),
                }),
                parser
            );
        }

        #[test]
        fn fail_on_wrong_etag_in_depth_graph() {
            let mut parser = SimpleXmlParser::from_str("<a><e><e></e><e/></d></a>");
            assert_evt!(Ok(Some(Event::Start("a", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(
                Err(XmlError::WrongETagName {
                    expected_name: "e".to_string(),
                }),
                parser
            );
        }
    }

    mod top_level_content {
        use super::*;

        #[test]
        fn only_one_top_level_element_empty() {
            let mut parser = SimpleXmlParser::from_str("<e/><e/>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(
                Err(XmlError::Expected(Box::new([
                    XmlErrorAtom::Comment,
                    XmlErrorAtom::PI
                ]))),
                parser
            );
        }

        #[test]
        fn accept_whitespace_after_root() {
            let mut parser = SimpleXmlParser::from_str("<e/> \r\t\n");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn only_one_top_level_element() {
            let mut parser = SimpleXmlParser::from_str("<e></e><e/>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(
                Err(XmlError::Expected(Box::new([
                    XmlErrorAtom::Comment,
                    XmlErrorAtom::PI
                ]))),
                parser
            );
        }
    }

    mod decl {
        use super::*;

        #[test]
        fn parse_minimal_decl() {
            let mut parser = SimpleXmlParser::from_str("<?xml version='1.0' ?><e/>");
            assert_evt!(Ok(Some(Event::decl("1.0", None, None))), parser);
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn parse_full_decl() {
            let mut parser = SimpleXmlParser::from_str(
                "<?xml version='1.0' encoding='UTF-8' standalone='yes' ?><e/>",
            );
            assert_evt!(
                Ok(Some(Event::decl("1.0", Some("UTF-8"), Some(true)))),
                parser
            );
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn parse_decl_double_qoute() {
            let mut parser = SimpleXmlParser::from_str(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?><e/>",
            );
            assert_evt!(
                Ok(Some(Event::decl("1.0", Some("UTF-8"), Some(true)))),
                parser
            );
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn parse_decl_whitespace() {
            let mut parser = SimpleXmlParser::from_str(
                "<?xml version =\t'1.0' encoding\n = \r'UTF-8' standalone =  'yes'?><e/>",
            );
            assert_evt!(
                Ok(Some(Event::decl("1.0", Some("UTF-8"), Some(true)))),
                parser
            );
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }
    }

    mod characters {
        use super::*;

        #[test]
        fn parse_chars() {
            let mut parser = SimpleXmlParser::from_str("<e>abc</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("abc".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn fail_on_chars_in_prolog() {
            let mut parser = SimpleXmlParser::from_str("abc <e/>");
            assert_evt!(Err(XmlError::UnexpectedCharacter('a')), parser);
        }

        #[test]
        fn fail_on_chars_in_epilog() {
            let mut parser = SimpleXmlParser::from_str("<e/>abc");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Err(XmlError::UnexpectedCharacter('a')), parser);
        }

        #[test]
        fn fail_on_cdata_section_end() {
            let mut parser = SimpleXmlParser::from_str("<e>]]></e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::IllegalCDataSectionEnd), parser);
        }

        #[test]
        fn valid_content() {
            let mut parser = SimpleXmlParser::from_str(
                "<e>\u{9}\u{A}\u{20}\u{D7FF}\u{E000}\u{FFFD}\u{10000}\u{10FFFF}</e>",
            );
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Ok(Some(Event::Chars(
                    "\u{9}\u{A}\u{20}\u{D7FF}\u{E000}\u{FFFD}\u{10000}\u{10FFFF}".into()
                ))),
                parser
            );
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn valid_content_entities() {
            let mut parser = SimpleXmlParser::from_str(
                "<e>&#x9;&#xA;&#xD;&#x20;&#xD7FF;&#xE000;&#xFFFD;&#x10000;&#x10FFFF;</e>",
            );
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{9}".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{A}".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{D}".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{20}".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{D7FF}".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{E000}".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{FFFD}".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{10000}".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{10FFFF}".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn invalid_content1() {
            let mut parser = SimpleXmlParser::from_str("<e>\u{1}</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{1}')), parser);
        }

        #[test]
        fn invalid_content2() {
            let mut parser = SimpleXmlParser::from_str("<e>&#1;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("1".to_string())),
                parser
            );
        }

        #[test]
        fn invalid_content3() {
            let mut parser = SimpleXmlParser::from_str("<e>\u{8}</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{8}')), parser);
        }

        #[test]
        fn invalid_content4() {
            let mut parser = SimpleXmlParser::from_str("<e>\u{B}</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{B}')), parser);
        }

        #[test]
        fn invalid_content5() {
            let mut parser = SimpleXmlParser::from_str("<e>\u{1F}</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{1F}')), parser);
        }

        #[test]
        fn invalid_content6() {
            let mut parser = SimpleXmlParser::from_str("<e>&#1F;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("1F".to_string())),
                parser
            );
        }

        #[test]
        fn invalid_content8() {
            let mut parser = SimpleXmlParser::from_str("<e>&#D800;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("D800".to_string())),
                parser
            );
        }

        #[test]
        fn invalid_content10() {
            let mut parser = SimpleXmlParser::from_str("<e>&#DFFF;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("DFFF".to_string())),
                parser
            );
        }

        #[test]
        fn invalid_content11() {
            let mut parser = SimpleXmlParser::from_str("<e>\u{FFFE}</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{FFFE}')), parser);
        }

        #[test]
        fn invalid_content12() {
            let mut parser = SimpleXmlParser::from_str("<e>&#FFFE;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("FFFE".to_string())),
                parser
            );
        }

        #[test]
        fn invalid_content13() {
            let mut parser = SimpleXmlParser::from_str("<e>\u{FFFF}</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{FFFF}')), parser);
        }

        #[test]
        fn invalid_content14() {
            let mut parser = SimpleXmlParser::from_str("<e>&#xFFFF;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("FFFF".to_string())),
                parser
            );
        }

        #[test]
        fn invalid_content15() {
            let mut parser = SimpleXmlParser::from_str("<e>&#x110000;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("110000".to_string())),
                parser
            );
        }
    }

    mod comment {
        use super::*;

        #[test]
        fn parse_comment() {
            let mut parser = SimpleXmlParser::from_str("<!-- declarations for <head> & <body> -->");
            assert_evt!(
                Ok(Some(Event::Comment(" declarations for <head> & <body> "))),
                parser
            );
        }

        #[test]
        fn parse_empty_comment() {
            let mut parser = SimpleXmlParser::from_str("<!---->");
            assert_evt!(Ok(Some(Event::Comment(""))), parser);
        }

        #[test]
        fn parse_invalid_comment() {
            let mut parser = SimpleXmlParser::from_str("<!-- B+, B, or B--->");
            assert_evt!(Err(XmlError::CommentColonColon), parser);
        }
    }

    mod pi {
        use std::borrow::Cow;

        use super::*;

        #[test]
        fn parse_pi() {
            let mut parser = SimpleXmlParser::from_str("<?e?>");
            assert_evt!(Ok(Some(Event::PI("e", None))), parser);
        }

        #[test]
        fn parse_pi_data() {
            let mut parser = SimpleXmlParser::from_str("<?e abc=gdsfh ?>");
            assert_evt!(Ok(Some(Event::PI("e", Some("abc=gdsfh ")))), parser);
        }

        #[test]
        fn parse_pi_starting_with_xml_1() {
            let mut parser = SimpleXmlParser::from_str("<?xml-abc?>");
            assert_evt!(Ok(Some(Event::PI("xml-abc", None))), parser);
        }

        #[test]
        fn parse_pi_starting_with_xml_2() {
            let mut parser = SimpleXmlParser::from_str("<?xml version='1.0'?><?xml-abc?>");
            assert_evt_matches!(Ok(Some(Event::Decl(_))), parser);
            assert_evt!(Ok(Some(Event::PI("xml-abc", None))), parser);
        }

        #[test]
        fn invalid_1() {
            let mut parser = SimpleXmlParser::from_str("<?e/fsdg?>");
            assert_evt!(Err(XmlError::ExpectToken("?>")), parser);
        }

        #[test]
        fn invalid_target_name_1() {
            let mut parser = SimpleXmlParser::from_str("<?xml version='1.0'?><?xml?>");
            assert_evt_matches!(Ok(Some(Event::Decl(_))), parser);
            assert_evt!(Err(XmlError::InvalidPITarget), parser);
        }

        #[test]
        fn invalid_target_name_2() {
            let mut parser = SimpleXmlParser::from_str("<?xml version='1.0'?><?XmL?>");
            assert_evt_matches!(Ok(Some(Event::Decl(_))), parser);
            assert_evt!(Err(XmlError::InvalidPITarget), parser);
        }

        #[test]
        fn missing_end() {
            let mut parser = SimpleXmlParser::from_str("<?e abc=gdsfh");
            assert_evt!(Err(XmlError::ExpectToken("?>")), parser);
        }
    }

    mod cdata {
        use super::*;

        #[test]
        fn pass1() {
            let mut parser =
                SimpleXmlParser::from_str("<e><![CDATA[<greeting>Hello, world!</greeting>]]></e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Ok(Some(Event::Chars(
                    "<greeting>Hello, world!</greeting>".into()
                ))),
                parser
            );
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn pass2() {
            let mut parser = SimpleXmlParser::from_str("<e><![CDATA[]]]]></e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("]]".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn pass3() {
            let mut parser = SimpleXmlParser::from_str("<e><![CDATA[[]]]></e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("[]".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn pass4() {
            let mut parser = SimpleXmlParser::from_str("<e><![CDATA[]]></e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn fail1() {
            let mut parser = SimpleXmlParser::from_str("<e><![CDATA[]></e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::UnexpectedEof), parser);
        }
    }

    mod char_ref {
        use super::*;

        #[test]
        fn pass_ascii_char() {
            let mut parser = SimpleXmlParser::from_str("<e>&#x20;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{20}".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn pass_decimal() {
            let mut parser = SimpleXmlParser::from_str("<e>&#32;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{20}".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn pass_emoji() {
            let mut parser = SimpleXmlParser::from_str("<e>&#x1F600;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{1F600}".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn pass_ref_in_chars() {
            let mut parser = SimpleXmlParser::from_str("<e>test&#x20;seq</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("test".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\u{20}".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("seq".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn fail_invalid_char() {
            let mut parser = SimpleXmlParser::from_str("<e>&#x0;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("0".to_string())),
                parser
            );
        }

        #[test]
        fn fail_too_big() {
            let mut parser = SimpleXmlParser::from_str("<e>&#x10000000;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("10000000".to_string())),
                parser
            );
        }

        #[test]
        fn fail_too_big_decimal() {
            let mut parser = SimpleXmlParser::from_str("<e>&#10000000;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("10000000".to_string())),
                parser
            );
        }

        #[test]
        fn fail_non_digit() {
            let mut parser = SimpleXmlParser::from_str("<e>&#xFGH;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("FGH".to_string())),
                parser
            );
        }

        #[test]
        fn fail_non_digit_decimal() {
            let mut parser = SimpleXmlParser::from_str("<e>&#1F;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("1F".to_string())),
                parser
            );
        }
    }

    mod entity_replacement {
        use super::*;

        #[test]
        fn replace_lt() {
            let mut parser = SimpleXmlParser::from_str("<e>&lt;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("<".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn replace_gt() {
            let mut parser = SimpleXmlParser::from_str("<e>&gt;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars(">".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn replace_amp() {
            let mut parser = SimpleXmlParser::from_str("<e>&amp;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("&".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn replace_apos() {
            let mut parser = SimpleXmlParser::from_str("<e>&apos;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("'".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn replace_quot() {
            let mut parser = SimpleXmlParser::from_str("<e>&quot;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("\"".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn fail_on_open() {
            let mut parser = SimpleXmlParser::from_str("<e>&quot</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::ExpectToken(";")), parser);
        }

        #[test]
        fn fail_on_unknown_entity() {
            let mut parser = SimpleXmlParser::from_str("<e>&nent;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::UnknownEntity("nent".to_string())), parser);
        }

        #[test]
        fn fail_on_open2() {
            let mut parser = SimpleXmlParser::from_str("<e>&lt&gt;</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Err(XmlError::ExpectToken(";")), parser);
        }
    }

    /// 4.3.3 Character Encoding in Entities
    mod encoding {
        use super::*;

        #[test]
        fn utf8_lower() {
            let mut parser =
                SimpleXmlParser::from_str("<?xml version='1.0' encoding='utf-8'?><e/>");
            assert_evt!(
                Ok(Some(Event::decl("1.0", Option::Some("utf-8"), None))),
                parser
            );
        }

        #[test]
        fn utf8_upper() {
            let mut parser =
                SimpleXmlParser::from_str("<?xml version='1.0' encoding='UTF-8'?><e/>");
            assert_evt!(
                Ok(Some(Event::decl("1.0", Option::Some("UTF-8"), None))),
                parser
            );
        }

        #[test]
        fn unsupported() {
            let mut parser =
                SimpleXmlParser::from_str("<?xml version='1.0' encoding='UTF128'?><e/>");
            assert_evt!(
                Err(XmlError::UnsupportedEncoding("UTF128".to_string())),
                parser
            );
        }
    }

    /// 2.11 End-of-Line Handling
    mod end_of_line_handling {
        use super::*;

        #[test]
        fn passthrough_line_feed() {
            let mut parser = SimpleXmlParser::from_str("<e>a\nb</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("a\nb".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn convert_carriage_return() {
            let mut parser = SimpleXmlParser::from_str("<e>a\rb</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("a".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\n".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("b".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn ignore_carriage_return_before_line_feed() {
            let mut parser = SimpleXmlParser::from_str("<e>a\r\nb</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("a".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\nb".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn ignore_carriage_return_before_line_feed2() {
            let mut parser = SimpleXmlParser::from_str("<e>a\r\n</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("a".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\n".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn ignore_carriage_return_before_line_feed3() {
            let mut parser = SimpleXmlParser::from_str("<e>\r\n</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("\n".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }

        #[test]
        fn ignore_carriage_return_before_line_feed4() {
            let mut parser = SimpleXmlParser::from_str("<e>\r\n\r\n</e>");
            assert_evt!(Ok(Some(Event::Start("e", vec![]))), parser);
            assert_evt!(Ok(Some(Event::Chars("\n".into()))), parser);
            assert_evt!(Ok(Some(Event::Chars("\n".into()))), parser);
            assert_evt!(Ok(Some(Event::End("e"))), parser);
            assert_evt!(Ok(None), parser);
        }
    }
}
