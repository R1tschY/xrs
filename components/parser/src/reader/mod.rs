//! XML Pull Reader

use std::marker::PhantomData;

use xml_chars::{XmlAsciiChar, XmlChar};

use crate::parser::core::{kleene, lexeme, optional, plus, Plus};
use crate::parser::helper::map_error;
use crate::parser::string::{char_, lit};
use crate::parser::Parser;
use crate::XmlError::{UnexpectedCharacter, UnexpectedEof};
use crate::XmlEvent::Characters;
use crate::{Attribute, Cursor, ETag, XmlDecl, XmlError, XmlEvent};

pub mod dtd;

// Common

#[inline]
pub fn xml_lit<'a>(literal: &'static str) -> impl Parser<'a, Attribute = (), Error = XmlError> {
    map_error(lit(literal), move |_| XmlError::ExpectToken(literal))
}

// XML

// 2.3 Common Syntactic Constructs

/// White Space
/// S ::= (#x20 | #x9 | #xD | #xA)+
struct SToken;

impl<'a> Parser<'a> for SToken {
    type Attribute = ();
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        Ok(((), skip_whitespace(cursor)))
    }
}

struct NameToken;

impl<'a> Parser<'a> for NameToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let mut chars = cursor.rest().char_indices();

        if !matches!(chars.next(), Some((_, c)) if c.is_xml_name_start_char()) {
            return Err(XmlError::ExpectedName);
        }

        if let Some((i, _)) = chars.find(|(_, c)| !c.is_xml_name_char()) {
            Ok(cursor.advance2(i))
        } else {
            Err(XmlError::ExpectedElementEnd)
        }
    }
}

struct AttValueToken;

impl<'a> Parser<'a> for AttValueToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Some(c) = cursor.next_byte(0) {
            if c == b'"' {
                let start = cursor.advance(1);
                if let Some((i, c)) = start
                    .rest_bytes()
                    .iter()
                    .enumerate()
                    .find(|(_, &c)| c == b'"')
                {
                    return Ok((start.rest().split_at(i).0, start.advance(i + 1)));
                }
                return Err(XmlError::ExpectedAttrValue);
            }
            if c == b'\'' {
                let start = cursor.advance(1);
                if let Some((i, c)) = start
                    .rest_bytes()
                    .iter()
                    .enumerate()
                    .find(|(_, &c)| c == b'\'')
                {
                    return Ok((start.rest().split_at(i).0, start.advance(i + 1)));
                }
                return Err(XmlError::ExpectedAttrValue);
            }
        }

        Err(XmlError::ExpectedAttrValue)
    }
}

struct EqLiteralToken;

impl<'a> Parser<'a> for EqLiteralToken {
    type Attribute = ();
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if cursor.next_byte(0) == Some(b'=') {
            Ok(((), cursor.advance(1)))
        } else {
            Err(XmlError::ExpectedEquals)
        }
    }
}

// 2.8 Prolog and Document Type Declaration

struct XmlDeclToken;

impl<'a> Parser<'a> for XmlDeclToken {
    type Attribute = XmlDecl<'a>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = xml_lit("<?xml").parse(cursor)?;
        let (version, cursor) = VersionInfoToken.parse(cursor)?;
        let (encoding, cursor) = optional(EncodingDeclToken).parse(cursor)?;
        let (standalone, cursor) = optional(SDDeclToken).parse(cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (_, cursor) = xml_lit("?>").parse(cursor)?;

        Ok((
            XmlDecl {
                version,
                encoding,
                standalone,
            },
            cursor,
        ))
    }
}

struct VersionInfoToken;

impl<'a> Parser<'a> for VersionInfoToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = SToken.parse(cursor)?;
        let (_, cursor) = expect_token(cursor, "version")?;
        let (_, cursor) = EqToken.parse(cursor)?;

        let c = cursor.next_byte(0);
        Ok(if c == Some(b'\'') {
            let cursor = cursor.advance(1);
            let (version, cursor) = VersionNumToken.parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\'")?;
            (version, cursor)
        } else if c == Some(b'\"') {
            let cursor = cursor.advance(1);
            let (version, cursor) = VersionNumToken.parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\"")?;
            (version, cursor)
        } else {
            return Err(XmlError::ExpectToken("' or \""));
        })
    }
}

struct EqToken;

impl<'a> Parser<'a> for EqToken {
    type Attribute = ();
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = SToken.parse(cursor)?;
        let (_, cursor) = xml_lit("=").parse(cursor)?;
        SToken.parse(cursor)
    }
}

struct VersionNumToken;

impl<'a> Parser<'a> for VersionNumToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        map_error(
            lexeme((lit("1."), plus(char_(|c: char| c.is_ascii_digit())))),
            |_| XmlError::ExpectToken("1.[0-9]+"),
        )
        .parse(cursor)
    }
}

// 2.9 Standalone Document Declaration

struct SDDeclToken;

impl<'a> Parser<'a> for SDDeclToken {
    type Attribute = bool;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = SToken.parse(cursor)?;
        let (_, cursor) = expect_token(cursor, "standalone")?;
        let (_, cursor) = EqToken.parse(cursor)?;

        let (yes_no, cursor) = if cursor.next_byte(0) == Some(b'\'') {
            let cursor = cursor.advance(1);
            let (yes_no, cursor) = map_error(lexeme(plus(char_(|c: char| c != '\''))), |_| {
                XmlError::ExpectToken("'yes' | 'no'")
            })
            .parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\'")?;
            (yes_no, cursor)
        } else if cursor.next_byte(0) == Some(b'\"') {
            let cursor = cursor.advance(1);
            let (yes_no, cursor) = map_error(lexeme(plus(char_(|c: char| c != '\"'))), |_| {
                XmlError::ExpectToken("'yes' | 'no'")
            })
            .parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\"")?;
            (yes_no, cursor)
        } else {
            return Err(XmlError::ExpectToken("' or \""));
        };

        if yes_no == "yes" {
            Ok((true, cursor))
        } else if yes_no == "no" {
            Ok((false, cursor))
        } else {
            return Err(XmlError::IllegalAttributeValue("Expected yes or no"));
        }
    }
}

// 4.3.3 Character Encoding in Entities

struct EncodingDeclToken;

impl<'a> Parser<'a> for EncodingDeclToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = SToken.parse(cursor)?;
        let (_, cursor) = expect_token(cursor, "encoding")?;
        let (_, cursor) = EqToken.parse(cursor)?;

        Ok(if cursor.next_byte(0) == Some(b'\'') {
            let cursor = cursor.advance(1);
            let (encoding, cursor) = EncNameToken.parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\'")?;
            (encoding, cursor)
        } else if cursor.next_byte(0) == Some(b'\"') {
            let cursor = cursor.advance(1);
            let (encoding, cursor) = EncNameToken.parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\"")?;
            (encoding, cursor)
        } else {
            return Err(XmlError::ExpectToken("' or \""));
        })
    }
}

struct EncNameToken;

impl<'a> Parser<'a> for EncNameToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        map_error(
            lexeme((
                char_(|c: char| c.is_ascii_alphabetic()),
                kleene(char_(|c: char| {
                    c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-'
                })),
            )),
            |_| XmlError::ExpectToken("Encoding name: [a-zA-Z][a-zA-Z0-9._-]+"),
        )
        .parse(cursor)
    }
}

fn expect_token<'a>(cursor: Cursor<'a>, token: &'static str) -> Result<((), Cursor<'a>), XmlError> {
    if !cursor.has_next_str(token) {
        return Err(XmlError::ExpectToken(token));
    } else {
        Ok(((), cursor.advance(token.len())))
    }
}

fn skip_whitespace(cursor: Cursor) -> Cursor {
    let size = cursor
        .rest_bytes()
        .iter()
        .take_while(|c| c.is_xml_whitespace())
        .count();
    if size > 0 {
        cursor.advance(size)
    } else {
        cursor
    }
}

fn expect_byte(cursor: Cursor, c: u8, err: fn() -> XmlError) -> Result<Cursor, XmlError> {
    if cursor.next_byte(0) == Some(c) {
        Ok(cursor.advance(1))
    } else {
        Err(err())
    }
}

pub struct Reader<'a> {
    cursor: Cursor<'a>,
    attributes: Vec<Attribute<'a>>,
    xml_lang: Option<&'a str>,
    depth: usize,
    empty: bool,
    seen_root: bool,
    version: Option<&'a str>,
    standalone: Option<bool>,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            cursor: Cursor::from_str(input),
            attributes: Vec::with_capacity(4),
            xml_lang: None,
            depth: 0,
            empty: false,
            seen_root: false,
            version: None,
            standalone: None,
        }
    }

    #[inline]
    pub fn is_prolog(&self) -> bool {
        !self.seen_root
    }

    #[inline]
    pub fn attributes(&self) -> &[Attribute<'a>] {
        &self.attributes
    }

    pub fn next(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        self.attributes.clear();
        if self.empty {
            self.depth -= 1;
            self.empty = false;
        }

        while let Some(c) = self.cursor.next_byte(0) {
            let evt = match c {
                b'<' => {
                    return if let Some(c) = self.cursor.next_byte(1) {
                        if c == b'/' {
                            self.cursor = self.cursor.advance(2);
                            self.parse_etag()
                        } else if c == b'?' {
                            if self.is_prolog() && self.version.is_none() {
                                self.parse_decl()
                            } else {
                                self.cursor = self.cursor.advance(2);
                                todo!()
                            }
                        } else {
                            self.cursor = self.cursor.advance(1);
                            self.parse_stag()
                        }
                    } else {
                        Err(XmlError::ExpectedElementStart)
                    }
                }
                _ => {
                    return if self.depth == 0 {
                        // only white space allowed
                        if c.is_xml_whitespace() {
                            let (_, cur) = SToken.parse(self.cursor)?;
                            self.cursor = cur;
                            continue;
                        } else {
                            Err(UnexpectedCharacter(self.cursor.next_char().unwrap()))
                        }
                    } else {
                        if let Some((i, _)) = self
                            .cursor
                            .rest_bytes()
                            .iter()
                            .enumerate()
                            .find(|(i, &c)| c == b'<')
                        {
                            let (chars, cur) = self.cursor.advance2(i);
                            self.cursor = cur;
                            Ok(Some(Characters(chars.into())))
                        } else {
                            Err(UnexpectedEof)
                        }
                    };
                }
            };
        }

        if self.depth != 0 {
            Err(XmlError::OpenElementAtEof)
        } else {
            Ok(None)
        }
    }

    fn is_after_root(&self) -> bool {
        self.depth == 0 && self.seen_root
    }

    fn parse_stag(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        if self.is_after_root() {
            return Err(XmlError::ExpectedDocumentEnd);
        }

        let (name, cursor) = NameToken.parse(self.cursor)?;

        self.cursor = skip_whitespace(cursor);

        while let Some(c) = self.cursor.next_byte(0) {
            // /> empty end
            if c == b'/' {
                return if Some(b'>') == self.cursor.next_byte(1) {
                    self.cursor = self.cursor.advance(2);
                    self.empty = true;
                    self.seen_root = true;
                    self.depth += 1;
                    Ok(Some(XmlEvent::stag(name, true)))
                } else {
                    Err(XmlError::ExpectedElementEnd)
                };
            }

            // normal end
            if c == b'>' {
                self.cursor = self.cursor.advance(1);
                self.empty = false;
                self.seen_root = true;
                self.depth += 1;
                return Ok(Some(XmlEvent::stag(name, false)));
            }

            // whitespace
            if c.is_xml_whitespace() {
                self.cursor = self.cursor.advance(1);
                continue;
            }

            // attribute
            let (attr_name, cursor) = NameToken.parse(self.cursor)?;
            let (_, cursor) = EqToken.parse(cursor)?;
            let (raw_value, cursor) = AttValueToken.parse(cursor)?;
            self.cursor = cursor;

            if self
                .attributes
                .iter()
                .find(|attr| attr.name == attr_name)
                .is_some()
            {
                return Err(XmlError::NonUniqueAttribute {
                    attribute: attr_name.to_string(),
                });
            }

            self.attributes.push(Attribute {
                name: attr_name,
                raw_value,
            });
        }

        Err(XmlError::ExpectedElementEnd)
    }

    fn parse_etag(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        if self.is_after_root() {
            return Err(XmlError::ExpectedDocumentEnd);
        }

        let (name, cursor) = NameToken.parse(self.cursor)?;
        let cursor = skip_whitespace(cursor);
        let cursor = expect_byte(cursor, b'>', || XmlError::ExpectedElementEnd)?;
        self.cursor = cursor;
        self.depth -= 1;
        Ok(Some(XmlEvent::ETag(ETag { name })))
    }

    fn parse_decl(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (decl, cursor) = XmlDeclToken.parse(self.cursor)?;

        self.version = Some(decl.version);
        self.standalone = decl.standalone;

        if let Some(encoding) = decl.encoding {
            if encoding != "UTF-8" {
                return Err(XmlError::UnsupportedEncoding(encoding.to_string()));
            }
        }

        self.cursor = cursor;
        Ok(Some(XmlEvent::XmlDecl(decl)))
    }
}

#[cfg(test)]
mod tests {
    use crate::reader::Reader;
    use crate::XmlEvent;
    use crate::{Attribute, XmlError};

    macro_rules! assert_evt {
        ($exp:expr, $reader:expr) => {
            assert_eq!($exp, $reader.next(), "error at {}", $reader.cursor.offset())
        };
    }

    fn empty_array<T>() -> &'static [T] {
        &[]
    }

    #[test]
    fn single_element() {
        let mut reader = Reader::new("<elem></elem>");
        assert_evt!(Ok(Some(XmlEvent::stag("elem", false))), reader);
        assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn single_element_whitespace() {
        let mut reader = Reader::new("<elem  ></elem   >");
        assert_evt!(Ok(Some(XmlEvent::stag("elem", false))), reader);
        assert_eq!(empty_array::<Attribute>(), reader.attributes());
        assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn empty_element() {
        let mut reader = Reader::new("<elem/>");
        assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
        assert_eq!(empty_array::<Attribute>(), reader.attributes());
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn attribute() {
        let mut reader = Reader::new("<elem attr=\"value\"/>");
        assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
        assert_eq!(&[Attribute::new("attr", "value")], reader.attributes());
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn attribute_whitespace() {
        let mut reader = Reader::new("<elem \t \n \r attr  =  \"value\"  />");
        assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
        assert_eq!(&[Attribute::new("attr", "value")], reader.attributes());
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn single_quote_attribute() {
        let mut reader = Reader::new("<elem attr='value'/>");
        assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
        assert_eq!(&[Attribute::new("attr", "value")], reader.attributes());
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn single_quote_attribute_whitespace() {
        let mut reader = Reader::new("<elem attr  =  'value'  />");
        assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
        assert_eq!(&[Attribute::new("attr", "value")], reader.attributes());
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn multiple_attributes() {
        let mut reader = Reader::new("<e a='v' b='w' />");
        assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
        assert_eq!(
            &[Attribute::new("a", "v"), Attribute::new("b", "w")],
            reader.attributes()
        );
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn attribute_duplicate() {
        let mut reader = Reader::new("<e a='' a='' />");
        assert_evt!(
            Err(XmlError::NonUniqueAttribute {
                attribute: "a".to_string()
            }),
            reader
        );
    }

    #[test]
    fn only_one_top_level_element() {
        let mut reader = Reader::new("<e></e><e/>");
        assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
        assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
        assert_evt!(Err(XmlError::ExpectedDocumentEnd), reader);
    }

    #[test]
    fn fail_on_open_etag() {
        let mut reader = Reader::new("<e></e></e>");
        assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
        assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
        assert_evt!(Err(XmlError::ExpectedDocumentEnd), reader);
    }

    #[test]
    fn only_one_top_level_element_empty() {
        let mut reader = Reader::new("<e/><e/>");
        assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
        assert_evt!(Err(XmlError::ExpectedDocumentEnd), reader);
    }

    #[test]
    fn accept_whitespace_after_root() {
        let mut reader = Reader::new("<e/> \r\t\n");
        assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn fail_on_open_stag() {
        let mut reader = Reader::new("<e>");
        assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
        assert_evt!(Err(XmlError::OpenElementAtEof), reader);
    }

    #[test]
    fn parse_minimal_decl() {
        let mut reader = Reader::new("<?xml version='1.0' ?><e/>");
        assert_evt!(Ok(Some(XmlEvent::decl("1.0", None, None))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn parse_full_decl() {
        let mut reader =
            Reader::new("<?xml version='1.0' encoding='UTF-8' standalone='yes' ?><e/>");
        assert_evt!(
            Ok(Some(XmlEvent::decl("1.0", Some("UTF-8"), Some(true)))),
            reader
        );
        assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn parse_decl_double_qoute() {
        let mut reader =
            Reader::new("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?><e/>");
        assert_evt!(
            Ok(Some(XmlEvent::decl("1.0", Some("UTF-8"), Some(true)))),
            reader
        );
        assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn parse_decl_whitespace() {
        let mut reader =
            Reader::new("<?xml version =\t'1.0' encoding\n = \r'UTF-8' standalone =  'yes'?><e/>");
        assert_evt!(
            Ok(Some(XmlEvent::decl("1.0", Some("UTF-8"), Some(true)))),
            reader
        );
        assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
        assert_evt!(Ok(None), reader);
    }

    mod characters {
        use crate::reader::Reader;
        use crate::{XmlError, XmlEvent};

        #[test]
        fn parse_chars() {
            let mut reader = Reader::new("<e>abc</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("abc"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn fail_on_chars_in_prolog() {
            let mut reader = Reader::new("abc <e/>");
            assert_evt!(Err(XmlError::UnexpectedCharacter('a')), reader);
        }

        #[test]
        fn fail_on_chars_in_epilog() {
            let mut reader = Reader::new("<e/>abc");
            assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
            assert_evt!(Err(XmlError::UnexpectedCharacter('a')), reader);
        }

        #[test]
        fn parse_chars() {
            let mut reader = Reader::new("<e>abc</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("abc"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }
    }
}