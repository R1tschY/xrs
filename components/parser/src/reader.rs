use crate::{Attribute, Cursor, ETag, XmlError, XmlEvent};
use xml_chars::{XmlAsciiChar, XmlChar};

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
                            self.cursor = self.cursor.advance(2);
                            self.parse_etag()
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

        Ok(None)
    }

    fn parse_stag(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (name, cursor) = scan_name(self.cursor)?;

        self.cursor = skip_whitespace(cursor);

        while let Some(c) = self.cursor.next_byte(0) {
            // /> empty end
            if c == b'/' {
                return if Some(b'>') == self.cursor.next_byte(1) {
                    self.cursor = self.cursor.advance(2);
                    Ok(Some(XmlEvent::stag(name, true)))
                } else {
                    Err(XmlError::ExpectedElementEnd)
                };
            }

            // normal end
            if c == b'>' {
                self.cursor = self.cursor.advance(1);
                return Ok(Some(XmlEvent::stag(name, false)));
            }

            // whitespace
            if c.is_xml_whitespace() {
                self.cursor = self.cursor.advance(1);
                continue;
            }

            // attribute
            let (attr_name, cursor) = scan_name(self.cursor)?;
            let cursor = skip_whitespace(cursor);
            let cursor = expect_byte(cursor, b'=', || XmlError::ExpectedEquals)?;
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

    fn parse_etag(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (name, cursor) = scan_name(self.cursor)?;
        let cursor = skip_whitespace(cursor);
        let cursor = expect_byte(cursor, b'>', || XmlError::ExpectedElementEnd)?;
        self.cursor = cursor;
        Ok(Some(XmlEvent::ETag(ETag { name })))
    }
}

#[cfg(test)]
mod tests {
    use crate::reader::Reader;
    use crate::Attribute;
    use crate::XmlEvent;

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
