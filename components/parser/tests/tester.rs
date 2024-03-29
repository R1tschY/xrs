use std::fmt::Debug;
use std::fmt::Write;

use xrs_parser::{ETag, Reader, STag, XmlDecl, XmlError, XmlEvent, PI};
use xrs_xts::TestableParser;

pub struct ReaderIT;

impl ReaderIT {
    fn process_cdata(cdata: &str) -> String {
        let mut result = String::with_capacity(cdata.len());
        for c in cdata.chars() {
            match c {
                '\n' => result.push_str("&#10;"),
                '\t' => result.push_str("&#9;"),
                '\r' => result.push_str("&#13;"),
                '&' => result.push_str("&amp;"),
                '<' => result.push_str("&lt;"),
                '>' => result.push_str("&gt;"),
                '"' => result.push_str("&quot;"),
                _ => result.push(c),
            }
        }
        result
    }

    fn canonxml_internal(&self, input: &str) -> Result<String, XmlError> {
        let mut reader = Reader::new(input);
        let mut result = String::new();

        while let Some(evt) = reader.next()? {
            match evt {
                XmlEvent::XmlDecl(decl) => {
                    self.write_decl(&mut result, decl)?;
                }
                XmlEvent::STag(stag) => {
                    self.write_stag(&reader, &mut result, stag)?;
                }
                XmlEvent::ETag(etag) => {
                    self.write_etag(&mut result, etag)?;
                }
                XmlEvent::Characters(chars) => {
                    result.push_str(&Self::process_cdata(chars.as_ref()));
                }
                XmlEvent::PI(pi) => {
                    self.write_pi(&mut result, pi)?;
                }
                XmlEvent::Dtd(_) => {}
                XmlEvent::Comment(_) => {}
            }
        }

        Ok(result)
    }

    fn write_stag<'a>(
        &self,
        reader: &Reader<'a>,
        writer: &mut String,
        stag: STag<'a>,
    ) -> Result<(), XmlError> {
        write!(writer, "<{}", stag.name());
        for attr in reader.attributes() {
            write!(writer, " {}=\"{}\"", attr.name(), attr.value());
        }
        write!(writer, ">");
        Ok(())
    }

    fn write_etag<'a>(&self, writer: &mut String, etag: ETag<'a>) -> Result<(), XmlError> {
        write!(writer, "</{}>", etag.name());
        Ok(())
    }

    fn write_decl<'a>(&self, writer: &mut String, decl: XmlDecl) -> Result<(), XmlError> {
        if decl.version() != "1.0" {
            write!(writer, "<?xml version=\"{}\"?>", decl.version());
        }
        Ok(())
    }

    fn write_pi<'a>(&self, writer: &mut String, pi: PI<'a>) -> Result<(), XmlError> {
        if let Some(data) = pi.data() {
            write!(writer, "<?{}{}?>", pi.target(), data);
        } else {
            write!(writer, "<?{}?>", pi.target());
        }
        Ok(())
    }
}

impl TestableParser for ReaderIT {
    fn check_well_formed(&self, input: &[u8], namespace: bool) -> Result<(), (String, usize)> {
        let input = match std::str::from_utf8(input) {
            Ok(input) => input,
            Err(err) => return Err((format!("{}", err), err.valid_up_to())),
        };

        let mut reader = Reader::new(input);
        loop {
            match reader.next() {
                Ok(Some(_)) => {}
                Ok(None) => {
                    return Ok(());
                }
                Err(err) => {
                    return Err((format!("{:?}", err), reader.cursor_offset()));
                }
            }
        }
    }

    fn canonxml(&self, input: &[u8], namespace: bool) -> Result<String, Box<dyn Debug>> {
        let input = std::str::from_utf8(input).map_err(|err| Box::new(err) as Box<dyn Debug>)?;

        self.canonxml_internal(input)
            .map_err(|err| Box::new(err) as Box<dyn Debug>)
    }
}
