use std::borrow::Cow;
use std::cmp::min;
use std::io::{BufRead, ErrorKind, Read};
use std::marker::PhantomData;
use std::mem::transmute;
use std::{io, slice};

use encoding_rs::{Encoding, UTF_8};

use xrs_chars::XmlAsciiChar;

use crate::parser::Parser;
use crate::{XmlDecl, XmlError};

#[derive(Copy, Clone)]
struct BytesStream<R: Read> {
    offset: usize,
    reader: R,
    peeked: Option<u8>,
}

impl<R: Read> BytesStream<R> {
    pub fn new(reader: R, offset: usize) -> Self {
        Self {
            reader,
            offset,
            peeked: None,
        }
    }

    pub fn peek(&mut self) -> Result<u8, XmlError> {
        if let Some(c) = &self.peeked {
            Ok(*c)
        } else {
            let c = self.advance()?;
            self.peeked = Some(c);
            Ok(c)
        }
    }

    pub fn take(&mut self) -> Result<u8, XmlError> {
        if let Some(c) = self.peeked.take() {
            Ok(c)
        } else {
            self.advance()
        }
    }

    pub fn discard(&mut self) {
        self.peeked.take();
    }

    pub fn discard_while(&mut self, f: impl Fn(u8) -> bool) -> Result<usize, XmlError> {
        let mut n = 0;

        if let Some(c) = &self.peeked {
            if !(f)(*c) {
                return Ok(n);
            }
        }

        loop {
            n += 1;
            let c = self.advance()?;
            self.peeked = Some(c);
            if !(f)(c) {
                return Ok(n);
            }
        }
    }

    fn advance(&mut self) -> Result<u8, XmlError> {
        let mut byte = 0;
        loop {
            return match self.reader.read(slice::from_mut(&mut byte)) {
                Ok(0) => Err(XmlError::UnexpectedEof),
                Ok(..) => Ok(byte),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => Err(e.into()),
            };
        }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

trait BytesParse<R: Read> {
    type Attribute;

    fn parse(&self, cursor: &mut BytesStream<R>) -> Result<Self::Attribute, XmlError>;
}

struct Literal(&'static str);

impl<R: Read> BytesParse<R> for Literal {
    type Attribute = ();

    fn parse(&self, cursor: &mut BytesStream<R>) -> Result<Self::Attribute, XmlError> {
        let mut lit = self.0.as_bytes().iter();

        while let Some(l) = lit.next() {
            let c = cursor.peek()?;
            if c != *l {
                return Err(XmlError::ExpectToken(self.0));
            }
            cursor.discard();
        }

        Ok(())
    }
}

struct OptionalWhitespace;

impl<R: Read> BytesParse<R> for OptionalWhitespace {
    type Attribute = ();

    fn parse(&self, cursor: &mut BytesStream<R>) -> Result<Self::Attribute, XmlError> {
        cursor.discard_while(|c| c.is_xml_whitespace())?;
        Ok(())
    }
}

struct Whitespace;

impl<R: Read> BytesParse<R> for Whitespace {
    type Attribute = ();

    fn parse(&self, cursor: &mut BytesStream<R>) -> Result<Self::Attribute, XmlError> {
        let n = cursor.discard_while(|c| c.is_xml_whitespace())?;
        if n == 0 {
            return Err(XmlError::ExpectedWhitespace);
        }
        Ok(())
    }
}

struct XmlDeclParser;

fn parse_encoding<R: Read>(cursor: &mut BytesStream<R>) -> Result<String, XmlError> {
    Attribute("encoding").parse(cursor)
}

fn parse_standalone<R: Read>(cursor: &mut BytesStream<R>) -> Result<bool, XmlError> {
    Ok(match &Attribute("standalone").parse(cursor)? as &str {
        "yes" => true,
        "no" => false,
        _ => return Err(XmlError::ExpectToken("yes or no")),
    })
}

impl<R: Read> BytesParse<R> for XmlDeclParser {
    type Attribute = XmlDecl;

    fn parse(&self, cursor: &mut BytesStream<R>) -> Result<Self::Attribute, XmlError> {
        Literal("<?xml").parse(cursor)?;

        Whitespace.parse(cursor)?;
        let version = Attribute("version").parse(cursor)?;
        if version != "1.0" {
            return Err(XmlError::UnsupportedVersion(version));
        }

        let mut encoding = None;
        let mut standalone = None;

        if cursor.peek()?.is_xml_whitespace() {
            Whitespace.parse(cursor)?;

            let c = cursor.peek()?;
            if c == b'e' {
                encoding = Some(parse_encoding(cursor)?);

                if cursor.peek()?.is_xml_whitespace() {
                    Whitespace.parse(cursor)?;
                    if cursor.peek()? == b's' {
                        standalone = Some(parse_standalone(cursor)?);
                    }
                }
            } else if c == b's' {
                standalone = Some(parse_standalone(cursor)?);
            };
        }

        OptionalWhitespace.parse(cursor)?;
        Literal("?>").parse(cursor)?;

        Ok(XmlDecl {
            version,
            encoding,
            standalone,
        })
    }
}

struct Attribute(&'static str);

impl<R: Read> BytesParse<R> for Attribute {
    type Attribute = String;

    fn parse(&self, cursor: &mut BytesStream<R>) -> Result<Self::Attribute, XmlError> {
        Literal(self.0).parse(cursor)?;
        Eq.parse(cursor)?;

        let mut result = String::new();
        let c = cursor.take()?;
        Ok(if c == b'\'' {
            loop {
                let c = cursor.take()?;
                if c == b'\'' {
                    break;
                }
                cursor.discard();
                result.push(c as char);
            }
            result
        } else if c == b'\"' {
            loop {
                let c = cursor.take()?;
                if c == b'\"' {
                    break;
                }
                cursor.discard();
                result.push(c as char);
            }
            result
        } else {
            return Err(XmlError::ExpectToken("' or \""));
        })
    }
}

struct Eq;

impl<R: Read> BytesParse<R> for Eq {
    type Attribute = ();

    fn parse(&self, cursor: &mut BytesStream<R>) -> Result<Self::Attribute, XmlError> {
        OptionalWhitespace.parse(cursor)?;
        Literal("=").parse(cursor)?;
        OptionalWhitespace.parse(cursor)?;
        Ok(())
    }
}

pub fn guess_encoding<'a>(input: &'a [u8]) -> Result<&'static Encoding, XmlError> {
    if let Some((enc, _)) = Encoding::for_bom(input) {
        return Ok(enc);
    }

    let mut cursor = BytesStream::new(input, 0);
    let decl = match XmlDeclParser.parse(&mut cursor) {
        Err(XmlError::ExpectToken("<?xml")) => return Ok(UTF_8),
        Err(err) => return Err(err),
        Ok(decl) => decl,
    };

    match decl.encoding {
        Some(enc) => {
            Encoding::for_label(enc.as_bytes()).ok_or_else(|| XmlError::UnsupportedEncoding(enc))
        }
        None => Ok(UTF_8),
    }
}

pub fn decode<'a>(
    input: &'a [u8],
    known_encoding: Option<&str>,
) -> Result<(Cow<'a, str>, &'static str, bool), XmlError> {
    let encoding = match known_encoding {
        Some(enc) => Encoding::for_label(enc.as_bytes())
            .ok_or_else(|| XmlError::UnsupportedEncoding(enc.to_string()))?,
        None => guess_encoding(input)?,
    };

    let (res, enc, errors) = encoding.decode(input);
    Ok((res, enc.name(), errors))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod decl_parse {
        use super::*;

        fn parse(input: &[u8]) -> XmlDecl {
            let mut cursor = BytesStream::new(input, 0);
            XmlDeclParser.parse(&mut cursor).unwrap()
        }

        #[test]
        fn check_minimal() {
            assert_eq!(
                parse(b"<?xml version=\"1.0\"?>"),
                XmlDecl {
                    version: "1.0".to_string(),
                    standalone: None,
                    encoding: None
                }
            );
        }
    }

    mod decode {
        use encoding_rs::WINDOWS_1252;

        use super::*;

        fn expect_decode<'a>(
            input: &'a [u8],
            known_encoding: Option<&str>,
        ) -> (Cow<'a, str>, &'static str) {
            let (res, enc, errors) = decode(input, known_encoding).unwrap();
            assert!(!errors);
            (res, enc)
        }

        #[test]
        fn check_no_decl() {
            assert_eq!(
                expect_decode(b"<a/>", None),
                (Cow::Borrowed("<a/>"), UTF_8.name())
            );
        }

        #[test]
        fn check_no_decl_encoding() {
            assert_eq!(
                expect_decode(b"<?xml version=\"1.0\"?><a/>", None),
                (Cow::Borrowed("<?xml version=\"1.0\"?><a/>"), UTF_8.name())
            );
        }

        #[test]
        fn check_external() {
            assert_eq!(
                expect_decode(b"<?xml version=\"1.0\"?><a>\xA4</a>", Some("Windows-1252")),
                (
                    Cow::from("<?xml version=\"1.0\"?><a>¤</a>"),
                    WINDOWS_1252.name()
                )
            );
        }

        #[test]
        fn check_read_decl() {
            assert_eq!(
                expect_decode(
                    b"<?xml version=\"1.0\" encoding=\"Windows-1252\"?><a>\xA4</a>",
                    None,
                ),
                (
                    Cow::from("<?xml version=\"1.0\" encoding=\"Windows-1252\"?><a>¤</a>"),
                    WINDOWS_1252.name(),
                )
            );
        }

        #[test]
        fn check_external_iso_8859_1() {
            assert_eq!(
                expect_decode(b"<?xml version=\"1.0\"?><a>\xA4</a>", Some("ISO-8859-1")),
                (
                    Cow::from("<?xml version=\"1.0\"?><a>¤</a>"),
                    WINDOWS_1252.name()
                )
            );
        }
    }
}
