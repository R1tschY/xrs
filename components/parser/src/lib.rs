#![allow(unused)]

use std::fmt::Formatter;
use std::fs::read_to_string;
use std::str::from_utf8;
use std::{fmt, io};
use xml_chars::XmlAsciiChar;
use xml_chars::XmlChar;

use crate::XmlError::{ExpectedElementEnd, ExpectedName};

mod dtd;
mod namespace;
mod shufti;

#[derive(Clone, Debug, PartialEq)]
pub struct STagStart<'a> {
    name: &'a str,
}

impl<'a> STagStart<'a> {
    pub fn name(&self) -> &'a str {
        self.name
    }
}

#[derive(Clone, PartialEq)]
pub struct Attribute<'a> {
    name: &'a str,
    raw_value: &'a str,
}

impl<'a> Attribute<'a> {
    pub fn raw_value(&self) -> &'a str {
        self.raw_value
    }

    pub fn name(&self) -> &'a str {
        self.name
    }
}

impl<'a> fmt::Debug for Attribute<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Attribute")
            .field("name", &self.name)
            .field("value", &self.raw_value)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct STagEnd<'a> {
    name: &'a str,
}

impl<'a> STagEnd<'a> {
    pub fn name(&self) -> &'a str {
        self.name
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ETag<'a> {
    name: &'a str,
}

impl<'a> ETag<'a> {
    pub fn name(&self) -> &'a str {
        self.name
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum XmlEvent<'a> {
    STagStart(STagStart<'a>),
    Attribute(Attribute<'a>),
    STagEnd,
    ETag(ETag<'a>),
    STagEndEmpty,
    Characters(&'a str),
}

impl<'a> XmlEvent<'a> {
    pub fn stag_start(name: &'a str) -> Self {
        XmlEvent::STagStart(STagStart { name })
    }

    pub fn stag_end() -> Self {
        XmlEvent::STagEnd
    }

    pub fn stag_end_empty() -> Self {
        XmlEvent::STagEndEmpty
    }

    pub fn attr(name: &'a str, raw_value: &'a str) -> Self {
        XmlEvent::Attribute(Attribute { name, raw_value })
    }

    pub fn etag(name: &'a str) -> Self {
        XmlEvent::ETag(ETag { name })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum XmlError {
    ExpectedName,
    ExpectedElementEnd,
    ExpectedAttrName,
    ExpectedAttrValue,
    ExpectedEquals,
    UnexpectedEof,
    IllegalName { name: String },
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Cursor<'a> {
    rest: &'a str,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn from_str(input: &'a str) -> Self {
        Self {
            rest: input,
            offset: 0,
        }
    }

    fn next_char(&self) -> Option<char> {
        self.rest.chars().next()
    }

    fn next_byte(&self, i: usize) -> Option<u8> {
        self.rest.as_bytes().get(i).copied()
    }

    #[inline]
    fn has_next_char(&self, pat: char) -> bool {
        self.rest.starts_with(pat)
    }

    #[inline]
    fn has_next_str(&self, pat: impl AsRef<str>) -> bool {
        self.rest.starts_with(pat.as_ref())
    }

    fn offset(&self) -> usize {
        self.offset
    }

    fn rest(&self) -> &'a str {
        self.rest
    }

    fn rest_bytes(&self) -> &'a [u8] {
        self.rest.as_bytes()
    }

    fn advance(&self, bytes: usize) -> Self {
        let (_, rest) = self.rest.split_at(bytes);
        Self {
            rest,
            offset: bytes,
        }
    }

    fn advance2(&self, bytes: usize) -> (&'a str, Self) {
        let (diff, rest) = self.rest.split_at(bytes);
        (
            diff,
            Self {
                rest,
                offset: bytes,
            },
        )
    }
}

fn skip_whitespace(cursor: Cursor) -> Cursor {
    if let Some(whitespace) = cursor.rest_bytes().split(|c| c.is_xml_whitespace()).next() {
        cursor.advance(whitespace.len())
    } else {
        cursor
    }
}

fn scan_name<'a>(cursor: Cursor<'a>) -> Result<(&'a str, Cursor<'a>), XmlError> {
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

fn scan_attr_value(cursor: Cursor) -> Result<(&str, Cursor), XmlError> {
    if let Some(c) = cursor.next_byte(0) {
        if c == b'"' {
            let start = cursor.advance(1);
            if let Some((i, c)) = start
                .rest_bytes()
                .iter()
                .enumerate()
                .find(|(_, &c)| c != b'"')
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
                .find(|(_, &c)| c != b'\'')
            {
                return Ok((start.rest().split_at(i).0, start.advance(i + 1)));
            }
            return Err(XmlError::ExpectedAttrValue);
        }
    }

    Err(XmlError::ExpectedAttrValue)
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
    in_element: bool,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            cursor: Cursor::from_str(input),
            in_element: false,
        }
    }

    pub fn next(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        if self.in_element {
            self.cursor = skip_whitespace(self.cursor);

            if let Some(c) = self.cursor.next_byte(0) {
                if c == b'/' {
                    return if Some(b'>') == self.cursor.next_byte(1) {
                        self.in_element = false;
                        self.cursor = self.cursor.advance(2);
                        Ok(Some(XmlEvent::STagEndEmpty))
                    } else {
                        Err(XmlError::ExpectedElementEnd)
                    };
                }
                if c == b'>' {
                    self.in_element = false;
                    self.cursor = self.cursor.advance(1);
                    return Ok(Some(XmlEvent::STagEnd));
                }

                let (attr_name, cursor) = scan_name(self.cursor)?;
                let cursor = skip_whitespace(cursor);
                let cursor = expect_byte(self.cursor, b'=', || XmlError::ExpectedEquals)?;
                let (raw_value, cursor) = scan_attr_value(cursor)?;
                self.cursor = cursor;
                return Ok(Some(XmlEvent::Attribute(Attribute {
                    name: attr_name,
                    raw_value,
                })));
            }
        }

        while let Some(c) = self.cursor.next_byte(0) {
            let evt = match c {
                b'<' => {
                    if let Some(c) = self.cursor.next_byte(1) {
                        match c {
                            b'/' => {
                                let cursor = self.cursor.advance(2);
                                let (name, cursor) = scan_name(cursor)?;
                                let cursor = skip_whitespace(cursor);
                                let cursor =
                                    expect_byte(cursor, b'>', || XmlError::ExpectedElementEnd)?;
                                self.cursor = cursor;
                                return Ok(Some(XmlEvent::ETag(ETag { name })));
                            }
                            _ => {
                                let cursor = self.cursor.advance(1);
                                let (name, cursor) = scan_name(self.cursor)?;
                                self.in_element = true;
                                return Ok(Some(XmlEvent::STagStart(STagStart { name })));
                            }
                        }
                    } else {
                    }
                }
                _ if c.is_xml_whitespace() => self.cursor = self.cursor.advance(1),
                _ => {
                    println!("{}", c);
                    todo!()
                }
            };
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_evt {
        ($exp:expr, $reader:expr) => {
            assert_eq!($exp, $reader.next(), "error at {}", $reader.cursor.offset)
        };
    }

    #[test]
    fn single_element() {
        let mut reader = Reader::new("<elem></elem>");
        assert_evt!(Ok(Some(XmlEvent::stag_start("elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end())), reader);
        assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn single_element_whitespace() {
        let mut reader = Reader::new("<elem  ></elem   >");
        assert_evt!(Ok(Some(XmlEvent::stag_start("elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end())), reader);
        assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn empty_element() {
        let mut reader = Reader::new("<elem/>");
        assert_evt!(Ok(Some(XmlEvent::stag_start("elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end_empty())), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn attribute() {
        let mut reader = Reader::new("<elem attr=\"value\"/>");
        assert_evt!(Ok(Some(XmlEvent::stag_start("elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::attr("attr", "value"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end_empty())), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn attribute_whitespace() {
        let mut reader = Reader::new("<elem   attr  =  \"value\"  />");
        assert_evt!(Ok(Some(XmlEvent::stag_start("elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::attr("attr", "value"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end_empty())), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn single_quote_attribute() {
        let mut reader = Reader::new("<elem attr='value'/>");
        assert_evt!(Ok(Some(XmlEvent::stag_start("elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::attr("attr", "value"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end_empty())), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn single_quote_attribute_whitespace() {
        let mut reader = Reader::new("<elem   attr  =  'value'  />");
        assert_evt!(Ok(Some(XmlEvent::stag_start("elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::attr("attr", "value"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end_empty())), reader);
        assert_evt!(Ok(None), reader);
    }
}
