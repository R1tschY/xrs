use crate::processors::{escape, Processor};
use std::fmt::Write;
use xrs_parser::{Reader, XmlEvent};

pub struct FullProcessor;

impl Processor for FullProcessor {
    fn check_wf(&self, xml: &str) -> Result<(), String> {
        let mut parser = Reader::new(xml);

        loop {
            match parser.next() {
                Ok(None) => return Ok(()),
                Ok(_) => {}
                Err(err) => return Err(err.to_string()),
            }
        }
    }

    fn norm(&self, xml: &str) -> Result<String, String> {
        let mut parser = Reader::new(xml);
        let mut writer = String::new();

        while let Some(evt) = parser.next().map_err(|err| err.to_string())? {
            match evt {
                XmlEvent::STag(tag) => {
                    let _ = write!(&mut writer, "<{}", tag.name());

                    for attr in tag.attributes() {
                        let _ = write!(&mut writer, " {}=\"{}\"", attr.name, &attr.value);
                    }

                    let _ = write!(&mut writer, ">");
                }
                XmlEvent::ETag(tag) => {
                    let _ = write!(&mut writer, "</{}>", tag.name());
                }
                XmlEvent::Characters(txt) => {
                    writer.push_str(&escape(&txt));
                }
                XmlEvent::Comment(comment) => {
                    let _ = write!(&mut writer, "<!--{}-->", comment);
                }
                XmlEvent::XmlDecl(decl) => {
                    let _ = write!(&mut writer, "<?xml version=\"{}\"", decl.version());
                    if let Some(enc) = decl.encoding() {
                        let _ = write!(&mut writer, " encoding=\"{}\"", enc);
                    }
                    if let Some(standalone) = decl.standalone() {
                        let _ = write!(&mut writer, " standalone=\"{}\"", standalone);
                    }
                    let _ = write!(&mut writer, "?>");
                }
                XmlEvent::PI(pi) => {
                    if let Some(data) = pi.data() {
                        let _ = write!(&mut writer, "<?{} {}?>", pi.target(), data);
                    } else {
                        let _ = write!(&mut writer, "<?{}?>", pi.target());
                    }
                }
                XmlEvent::Dtd(_) => {
                    todo!()
                }
            }
        }

        Ok(writer)
    }
}
