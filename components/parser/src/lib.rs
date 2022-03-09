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
pub struct STag<'a> {
    name: &'a str,
    empty: bool,
}

impl<'a> STag<'a> {
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
    pub fn new(name: &'a str, raw_value: &'a str) -> Self {
        Self { name, raw_value }
    }

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
    STag(STag<'a>),
    ETag(ETag<'a>),
    Characters(&'a str),
}

impl<'a> XmlEvent<'a> {
    pub fn stag(name: &'a str, empty: bool) -> Self {
        XmlEvent::STag(STag { name, empty })
    }

    pub fn etag(name: &'a str) -> Self {
        XmlEvent::ETag(ETag { name })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum XmlError {
    ExpectedName,
    ExpectedElementStart,
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
        let (_ignore, rest) = self.rest.split_at(bytes);
        println!("ADVANCE {}: {}", bytes, _ignore);
        Self {
            rest,
            offset: bytes,
        }
    }

    fn advance2(&self, bytes: usize) -> (&'a str, Self) {
        let (diff, rest) = self.rest.split_at(bytes);
        println!("ADVANCE {}: {}", bytes, diff);
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

fn scan_name(cursor: Cursor) -> Result<(&str, Cursor), XmlError> {
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
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            cursor: Cursor::from_str(input),
            attributes: Vec::with_capacity(4),
            xml_lang: None,
            depth: 0,
        }
    }

    pub fn attributes(&self) -> &[Attribute<'a>] {
        &self.attributes
    }

    pub fn next(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        self.attributes.clear();

        while let Some(c) = self.cursor.next_byte(0) {
            let evt = match c {
                b'<' => {
                    return if let Some(c) = self.cursor.next_byte(1) {
                        if c == b'/' {
                            let cursor = self.cursor.advance(2);
                            let (name, cursor) = scan_name(cursor)?;
                            let cursor = skip_whitespace(cursor);
                            let cursor =
                                expect_byte(cursor, b'>', || XmlError::ExpectedElementEnd)?;
                            self.cursor = cursor;
                            Ok(Some(XmlEvent::ETag(ETag { name })))
                        } else {
                            let cursor = self.cursor.advance(1);
                            let (name, cursor) = scan_name(cursor)?;

                            self.cursor = skip_whitespace(cursor);

                            while let Some(c) = self.cursor.next_byte(0) {
                                if c == b'/' {
                                    return if Some(b'>') == self.cursor.next_byte(1) {
                                        self.cursor = self.cursor.advance(2);
                                        Ok(Some(XmlEvent::stag(name, true)))
                                    } else {
                                        Err(XmlError::ExpectedElementEnd)
                                    };
                                }
                                if c == b'>' {
                                    self.cursor = self.cursor.advance(1);
                                    return Ok(Some(XmlEvent::stag(name, false)));
                                }
                                if c.is_xml_whitespace() {
                                    self.cursor = self.cursor.advance(1);
                                    continue;
                                }

                                let (attr_name, cursor) = scan_name(self.cursor)?;
                                let cursor = skip_whitespace(cursor);
                                let cursor =
                                    expect_byte(cursor, b'=', || XmlError::ExpectedEquals)?;
                                let cursor = skip_whitespace(cursor);
                                let (raw_value, cursor) = scan_attr_value(cursor)?;
                                self.cursor = cursor;

                                self.attributes.push(Attribute {
                                    name: attr_name,
                                    raw_value,
                                });
                            }

                            Err(XmlError::ExpectedElementEnd)
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
        let mut reader = Reader::new("<elem   attr  =  \"value\"  />");
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
        let mut reader = Reader::new("<elem   attr  =  'value'  />");
        assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
        assert_eq!(&[Attribute::new("attr", "value")], reader.attributes());
        assert_evt!(Ok(None), reader);
    }
}
