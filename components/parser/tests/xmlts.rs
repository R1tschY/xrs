use std::fmt::{Debug, Write};
use xml_parser::{ETag, Reader, STag, XmlDecl, XmlError, XmlEvent, PI};
use xml_xts::TestableParser;
use xml_xts::XmlTester;

struct ReaderIT;

impl ReaderIT {
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
                    result.push_str(chars.as_ref());
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
            write!(writer, " {}=\"{}\"", attr.name(), attr.raw_value());
        }
        write!(writer, ">");
        Ok(())
    }

    fn write_etag<'a>(&self, writer: &mut String, etag: ETag<'a>) -> Result<(), XmlError> {
        write!(writer, "</{}>", etag.name());
        Ok(())
    }

    fn write_decl<'a>(&self, writer: &mut String, decl: XmlDecl<'a>) -> Result<(), XmlError> {
        write!(writer, "<?xml version=\"{}\"", decl.version());
        if let Some(encoding) = decl.encoding() {
            write!(writer, " encoding=\"{}\"", encoding);
        }
        if let Some(standalone) = decl.standalone() {
            write!(
                writer,
                " standalone=\"{}\"",
                if standalone { "yes" } else { "no" }
            );
        }
        write!(writer, "/>");
        Ok(())
    }

    fn write_pi<'a>(&self, writer: &mut String, pi: PI<'a>) -> Result<(), XmlError> {
        write!(writer, "<?{}{}?>", pi.target(), pi.data());
        Ok(())
    }
}

impl TestableParser for ReaderIT {
    fn is_wf(&self, input: &[u8], namespace: bool) -> bool {
        let input = if let Ok(input) = std::str::from_utf8(input) {
            input
        } else {
            return false;
        };

        let mut reader = Reader::new(input);
        loop {
            match reader.next() {
                Ok(Some(_)) => {}
                Ok(None) => {
                    return true;
                }
                Err(err) => {
                    println!("      ERROR: {:?}", err);
                    return false;
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

#[test]
fn main() {
    let report = XmlTester::new().test(&ReaderIT);
    report.print_statistic();
    report.assert();
}
