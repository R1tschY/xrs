use crate::{Attribute, Cursor, ETag, XmlDecl, XmlError, XmlEvent};
use std::marker::PhantomData;
use xml_chars::{XmlAsciiChar, XmlChar};

pub trait Parser<'a> {
    type Attribute;
    type Error;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error>;
}

// Core

#[inline]
pub fn lexeme<'a, T: 'a + Parser<'a>>(parser: T) -> Lexeme<'a, T> {
    Lexeme(parser, PhantomData)
}

pub struct Lexeme<'a, T: Parser<'a>>(T, PhantomData<&'a T>);

impl<'a, T: Parser<'a>> Parser<'a> for Lexeme<'a, T> {
    type Attribute = &'a str;
    type Error = T::Error;

    fn parse(&self, start: Cursor<'a>) -> Result<(&'a str, Cursor<'a>), T::Error> {
        let (_, end) = self.0.parse(start)?;
        Ok(start.advance2(end.offset() - start.offset()))
    }
}

struct Optional<T>(T);

impl<'a, T: Parser<'a>> Parser<'a> for Optional<T> {
    type Attribute = Option<T::Attribute>;
    type Error = T::Error;

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), T::Error> {
        match self.0.parse(cur) {
            Ok((attr, cur)) => Ok((Some(attr), cur)),
            Err(err) => Ok((None, cur)),
        }
    }
}

struct Kleene<T>(T);

impl<'a, T: Parser<'a>> Parser<'a> for Kleene<T> {
    type Attribute = Vec<T::Attribute>;
    type Error = T::Error;

    fn parse(&self, mut cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), T::Error> {
        let mut res = vec![];
        while let Ok((attr, cursor)) = self.0.parse(cur) {
            cur = cursor;
            res.push(attr);
        }
        Ok((res, cur))
    }
}

struct Plus<T>(T);

impl<'a, T: Parser<'a>> Parser<'a> for Plus<T> {
    type Attribute = Vec<T::Attribute>;
    type Error = T::Error;

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), T::Error> {
        let mut res = vec![];
        let (first, mut cur) = self.0.parse(cur)?;
        res.push(first);

        while let Ok((attr, cursor)) = self.0.parse(cur) {
            cur = cursor;
            res.push(attr);
        }

        Ok((res, cur))
    }
}

#[inline]
pub fn seq2<'a, T1: Parser<'a, Error = E>, T2: Parser<'a, Error = E>, E>(
    parser1: T1,
    parser2: T2,
) -> Sequence2<T1, T2> {
    Sequence2(parser1, parser2)
}

pub struct Sequence2<T1, T2>(T1, T2);

impl<'a, T1: Parser<'a, Error = E>, T2: Parser<'a, Error = E>, E> Parser<'a> for Sequence2<T1, T2> {
    type Attribute = (T1::Attribute, T2::Attribute);
    type Error = E;

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (v1, cur) = self.0.parse(cur)?;
        let (v2, cur) = self.1.parse(cur)?;
        Ok(((v1, v2), cur))
    }
}

// Helper

// struct MapAttr<T, U, F: Fn(T::Attribute) -> U>(T, F);
//
// impl<'a, T: Parser<'a>, U, F: Fn(T::Attribute) -> U> Parser<'a> for MapAttr<T, U, F> {
//     type Attribute = U;
//     type Error = T::Error;
//
//     fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), T::Error> {
//         self.0.parse(cur).map(&self.1)
//     }
// }

#[inline]
pub fn map_error<'a, T: Parser<'a>, E, F: Fn(T::Error) -> E>(
    parser: T,
    f: F,
) -> MapError<'a, T, E, F> {
    MapError(parser, f, PhantomData)
}

pub struct MapError<'a, T: Parser<'a>, E, F: Fn(T::Error) -> E>(T, F, PhantomData<&'a E>);

impl<'a, T: Parser<'a>, E, F: Fn(T::Error) -> E> Parser<'a> for MapError<'a, T, E, F> {
    type Attribute = T::Attribute;
    type Error = E;

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        self.0.parse(cur).map_err(&self.1)
    }
}

#[inline]
pub fn omit<'a, T: Parser<'a>>(parser: T) -> Omit<T> {
    Omit(parser)
}

pub struct Omit<T>(T);

impl<'a, T: Parser<'a>> Parser<'a> for Omit<T> {
    type Attribute = ();
    type Error = T::Error;

    fn parse(&self, mut cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        self.0.parse(cur).map(|(_, cur)| ((), cur))
    }
}

#[inline]
pub fn omit_error<'a, T: Parser<'a>>(parser: T) -> OmitError<T> {
    OmitError(parser)
}

pub struct OmitError<T>(T);

impl<'a, T: Parser<'a>> Parser<'a> for OmitError<T> {
    type Attribute = T::Attribute;
    type Error = ();

    fn parse(&self, mut cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        self.0.parse(cur).map_err(|err| ())
    }
}

// Strings

pub fn lit(lit: &'static str) -> Lit {
    Lit { lit }
}

pub struct Lit {
    lit: &'static str,
}

impl<'a> Parser<'a> for Lit {
    type Attribute = ();
    type Error = ();

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if !cur.has_next_str(self.lit) {
            return Err(());
        } else {
            Ok(((), cur.advance(self.lit.len())))
        }
    }
}

pub fn char_<P: Fn(char) -> bool>(predicate: P) -> Char<P> {
    Char { predicate }
}

pub struct Char<P: Fn(char) -> bool> {
    predicate: P,
}

impl<'a, P: Fn(char) -> bool> Parser<'a> for Char<P> {
    type Attribute = ();
    type Error = ();

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Some(c) = cur.next_char() {
            if (self.predicate)(c) {
                return Ok(((), cur.advance(1)));
            }
        }
        Err(())
    }
}

// XML

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
        let (_, cursor) =
            map_error(lit("<?xml"), |_| XmlError::ExpectToken("<?xml")).parse(cursor)?;
        let (version, cursor) = VersionInfoToken.parse(cursor)?;
        let (encoding, cursor) = Optional(EncodingDeclToken).parse(cursor)?;
        let (standalone, cursor) = Optional(SDDeclToken).parse(cursor)?;
        let (_, cursor) = Optional(SToken).parse(cursor)?;
        let (_, cursor) = map_error(lit("?>"), |_| XmlError::ExpectToken("?>")).parse(cursor)?;

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
        let (_, cursor) = map_error(lit("="), |_| XmlError::ExpectToken("=")).parse(cursor)?;
        SToken.parse(cursor)
    }
}

struct VersionNumToken;

impl<'a> Parser<'a> for VersionNumToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        map_error(
            lexeme(seq2(lit("1."), Plus(char_(|c: char| c.is_ascii_digit())))),
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
            let (yes_no, cursor) = map_error(lexeme(Plus(char_(|c: char| c != '\''))), |_| {
                XmlError::ExpectToken("'yes' | 'no'")
            })
            .parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\'")?;
            (yes_no, cursor)
        } else if cursor.next_byte(0) == Some(b'\"') {
            let cursor = cursor.advance(1);
            let (yes_no, cursor) = map_error(lexeme(Plus(char_(|c: char| c != '\"'))), |_| {
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
            lexeme(seq2(
                char_(|c: char| c.is_ascii_alphabetic()),
                Kleene(char_(|c: char| {
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
                _ if c.is_xml_whitespace() => self.cursor = self.cursor.advance(1),
                _ => {
                    println!("{}", c);
                    todo!()
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
}
