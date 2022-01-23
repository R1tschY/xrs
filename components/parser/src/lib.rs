#![allow(unused)]

use std::fmt::Formatter;
use std::fs::read_to_string;
use std::str::from_utf8;
use std::{fmt, io};

use crate::XmlError::{ExpectedElementEnd, ExpectedName};

mod dtd;
mod shufti;

#[derive(Clone, Debug, PartialEq)]
pub struct STagStart<'a> {
    name: &'a [u8],
}

#[derive(Clone, PartialEq)]
pub struct Attribute<'a> {
    name: &'a [u8],
    value: &'a [u8],
}

impl<'a> Attribute<'a> {
    pub fn value(&self) -> &str {
        from_utf8(self.value).unwrap()
    }

    pub fn name(&self) -> &str {
        from_utf8(self.name).unwrap()
    }
}

impl<'a> fmt::Debug for Attribute<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = from_utf8(self.name).unwrap();
        let value = from_utf8(self.value).unwrap();

        f.debug_struct("Attribute")
            .field("name", &name)
            .field("value", &value)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct STagEnd<'a> {
    name: &'a [u8],
}

impl<'a> STagEnd<'a> {
    pub fn name(&self) -> &str {
        from_utf8(self.name).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ETag<'a> {
    name: &'a [u8],
}

impl<'a> ETag<'a> {
    pub fn name(&self) -> &str {
        from_utf8(self.name).unwrap()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum XmlEvent<'a> {
    STagStart(STagStart<'a>),
    Attribute(Attribute<'a>),
    STagEnd,
    ETag(ETag<'a>),
    STagEndEmpty,
    Characters(&'a [u8]),
}

impl<'a> XmlEvent<'a> {
    pub fn stag_start(name: &'a [u8]) -> Self {
        XmlEvent::STagStart(STagStart { name })
    }

    pub fn stag_end() -> Self {
        XmlEvent::STagEnd
    }

    pub fn stag_end_empty() -> Self {
        XmlEvent::STagEndEmpty
    }

    pub fn attr(name: &'a [u8], value: &'a [u8]) -> Self {
        XmlEvent::Attribute(Attribute { name, value })
    }

    pub fn etag(name: &'a [u8]) -> Self {
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
}

pub struct Reader<'a> {
    input: &'a [u8],
    pos: usize,
    in_element: bool,
}

fn is_whitespace(c: u8) -> bool {
    c == b'\x20' || c == b'\x09' || c == b'\x0D' || c == b'\x0A'
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            in_element: false,
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.input.get(self.pos) {
            if is_whitespace(*c) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn scan_start_name(&mut self) -> Result<usize, XmlError> {
        while let Some(c) = self.input.get(self.pos) {
            if *c == b'>' || *c == b'/' {
                return Ok(self.pos);
            }
            if is_whitespace(*c) {
                let res = self.pos;
                self.skip_whitespace();
                return Ok(res);
            }
            self.pos += 1;
        }

        println!("X3");
        Err(XmlError::ExpectedName)
    }

    fn scan_end_name(&mut self) -> Result<usize, XmlError> {
        while let Some(c) = self.input.get(self.pos) {
            if *c == b'>' {
                self.pos += 1;
                return Ok(self.pos - 1);
            }
            if is_whitespace(*c) {
                let end = self.pos;
                while let Some(c) = self.input.get(self.pos) {
                    if *c == b'>' {
                        self.pos += 1;
                        return Ok(end);
                    }
                    if !is_whitespace(*c) {
                        return Err(XmlError::ExpectedElementEnd);
                    }
                    self.pos += 1;
                }
                return Err(XmlError::ExpectedName);
            }
            self.pos += 1;
        }

        Err(XmlError::ExpectedName)
    }

    fn scan_attr_value(&mut self) -> Result<&'a [u8], XmlError> {
        while let Some(c) = self.input.get(self.pos) {
            if *c == b'"' {
                self.pos += 1;
                let start = self.pos;
                while let Some(c) = self.input.get(self.pos) {
                    if *c == b'"' {
                        self.pos += 1;
                        return Ok(&self.input[start..self.pos - 1]);
                    }
                    self.pos += 1;
                }
                return Err(XmlError::ExpectedAttrValue);
            }
            if *c == b'\'' {
                self.pos += 1;
                let start = self.pos;
                while let Some(c) = self.input.get(self.pos) {
                    if *c == b'\'' {
                        self.pos += 1;
                        return Ok(&self.input[start..self.pos - 1]);
                    }
                    self.pos += 1;
                }
                return Err(XmlError::ExpectedAttrValue);
            }
            if is_whitespace(*c) {
                self.pos += 1;
                continue;
            }
            return Err(XmlError::ExpectedAttrValue);
        }

        Err(XmlError::ExpectedAttrValue)
    }

    fn scan_attr_name(&mut self) -> Result<usize, XmlError> {
        while let Some(c) = self.input.get(self.pos) {
            if *c == b'=' {
                let end = self.pos;
                self.pos += 1;
                return Ok(end);
            }
            if is_whitespace(*c) {
                let end = self.pos;
                while let Some(c) = self.input.get(self.pos) {
                    if *c == b'=' {
                        self.pos += 1;
                        return Ok(end);
                    }
                    if !is_whitespace(*c) {
                        return Err(XmlError::ExpectedEquals);
                    }
                    self.pos += 1;
                }
                return Err(XmlError::ExpectedName);
            }
            self.pos += 1;
        }

        Err(XmlError::ExpectedName)
    }

    pub fn next(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        if self.in_element {
            self.skip_whitespace();

            if let Some(c) = self.input.get(self.pos) {
                if *c == b'/' {
                    self.pos += 1;
                    return if Some(b'>') == self.input.get(self.pos).copied() {
                        self.pos += 1;
                        self.in_element = false;
                        Ok(Some(XmlEvent::STagEndEmpty))
                    } else {
                        Err(XmlError::ExpectedElementEnd)
                    };
                }
                if *c == b'>' {
                    self.pos += 1;
                    self.in_element = false;
                    return Ok(Some(XmlEvent::STagEnd));
                }

                let name_start = self.pos;
                let name_end = self.scan_attr_name()?;
                let value = self.scan_attr_value()?;
                return Ok(Some(XmlEvent::Attribute(Attribute {
                    name: &self.input[name_start..name_end],
                    value,
                })));
            }
        }

        while let Some(c) = self.input.get(self.pos) {
            let evt = match c {
                b'<' => {
                    self.pos += 1;
                    if let Some(c) = self.input.get(self.pos) {
                        match c {
                            b'/' => {
                                self.pos += 1;
                                let start = self.pos;
                                let end = self.scan_end_name()?;
                                return Ok(Some(XmlEvent::ETag(ETag {
                                    name: &self.input[start..end],
                                })));
                            }
                            _ => {
                                let start = self.pos;
                                let end = self.scan_start_name()?;
                                self.in_element = true;
                                return Ok(Some(XmlEvent::STagStart(STagStart {
                                    name: &self.input[start..end],
                                })));
                            }
                        }
                    } else {
                    }
                }
                _ if is_whitespace(*c) => continue,
                _ => {
                    println!("{}", self.pos);
                    todo!()
                }
            };

            self.pos += 1;
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_evt {
        ($exp:expr, $reader:expr) => {
            assert_eq!($exp, $reader.next(), "error at {}", $reader.pos)
        };
    }

    #[test]
    fn single_element() {
        let mut reader = Reader::new(b"<elem></elem>");
        assert_evt!(Ok(Some(XmlEvent::stag_start(b"elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end())), reader);
        assert_evt!(Ok(Some(XmlEvent::etag(b"elem"))), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn single_element_whitespace() {
        let mut reader = Reader::new(b"<elem  ></elem   >");
        assert_evt!(Ok(Some(XmlEvent::stag_start(b"elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end())), reader);
        assert_evt!(Ok(Some(XmlEvent::etag(b"elem"))), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn empty_element() {
        let mut reader = Reader::new(b"<elem/>");
        assert_evt!(Ok(Some(XmlEvent::stag_start(b"elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end_empty())), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn attribute() {
        let mut reader = Reader::new(b"<elem attr=\"value\"/>");
        assert_evt!(Ok(Some(XmlEvent::stag_start(b"elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::attr(b"attr", b"value"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end_empty())), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn attribute_whitespace() {
        let mut reader = Reader::new(b"<elem   attr  =  \"value\"  />");
        assert_evt!(Ok(Some(XmlEvent::stag_start(b"elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::attr(b"attr", b"value"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end_empty())), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn single_quote_attribute() {
        let mut reader = Reader::new(b"<elem attr='value'/>");
        assert_evt!(Ok(Some(XmlEvent::stag_start(b"elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::attr(b"attr", b"value"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end_empty())), reader);
        assert_evt!(Ok(None), reader);
    }

    #[test]
    fn single_quote_attribute_whitespace() {
        let mut reader = Reader::new(b"<elem   attr  =  'value'  />");
        assert_evt!(Ok(Some(XmlEvent::stag_start(b"elem"))), reader);
        assert_evt!(Ok(Some(XmlEvent::attr(b"attr", b"value"))), reader);
        assert_evt!(Ok(Some(XmlEvent::stag_end_empty())), reader);
        assert_evt!(Ok(None), reader);
    }
}
