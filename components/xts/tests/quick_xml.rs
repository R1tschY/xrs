use quick_xml::events::Event;
use quick_xml::{Error, Reader};
use serde::de::Unexpected::Str;
use std::fmt::{Debug, Write};
use xml_xts::TestableParser;
use xml_xts::XmlTester;

struct QuickXmlIT;

impl QuickXmlIT {
    fn canonxml_internal(&self, input: &[u8]) -> Result<String, Error> {
        let mut reader = Reader::from_reader(input);

        let mut writer = String::new();
        let mut buf = Vec::new();
        loop {
            match reader.read_event(&mut buf)? {
                Event::Eof => return Ok(writer),
                Event::Start(start) => {
                    write!(writer, "<{}", reader.decode(start.name())?);
                    for attr_res in start.attributes().with_checks(true) {
                        match attr_res {
                            Ok(attr) => {
                                write!(
                                    writer,
                                    " {}=\"{}\"",
                                    reader.decode(attr.key)?,
                                    reader.decode(attr.unescaped_value()?.as_ref())?
                                );
                            }
                            Err(err) => return Err(err.into()),
                        }
                    }
                    write!(writer, ">");
                }
                Event::End(end) => {
                    write!(writer, "</{}>", reader.decode(end.name())?);
                }
                Event::Empty(start) => {
                    write!(writer, "<{}/>", reader.decode(start.name())?);
                }
                Event::Text(text) => {
                    writer.push_str(reader.decode(text.unescaped()?.as_ref())?);
                }
                Event::Comment(_) => {}
                Event::CData(cdata) => {
                    writer.push_str(reader.decode(cdata.unescaped()?.as_ref())?);
                }
                Event::Decl(decl) => {
                    write!(writer, "<?xml");
                    if let Some(encoding) = decl.encoding() {
                        write!(
                            writer,
                            "encoding=\"{}\"",
                            reader.decode(encoding?.as_ref())?
                        );
                    }
                    if let Some(standalone) = decl.standalone() {
                        write!(
                            writer,
                            "standalone=\"{}\"",
                            reader.decode(standalone?.as_ref())?
                        );
                    }
                    write!(
                        writer,
                        "version=\"{}\"?>",
                        reader.decode(decl.version()?.as_ref())?
                    );
                }
                Event::PI(text) => {
                    write!(writer, "<?{}?>", reader.decode(text.escaped())?);
                }
                Event::DocType(text) => {}
            }
        }
    }
}

impl TestableParser for QuickXmlIT {
    fn is_wf(&self, input: &[u8]) -> bool {
        let mut reader = Reader::from_reader(input);
        reader.trim_text(false);
        reader.check_comments(true);
        reader.check_end_names(true);

        let mut buf = Vec::new();
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Eof) => return true,
                Ok(Event::Start(start)) => {
                    if start
                        .attributes()
                        .with_checks(true)
                        .any(|attr| attr.is_err())
                    {
                        return false;
                    }
                }
                Ok(_) => buf.clear(),
                Err(_) => return false,
            }
        }
    }

    fn canonxml(&self, input: &[u8]) -> Result<String, Box<dyn Debug>> {
        self.canonxml_internal(input)
            .map_err(|err| Box::new(err) as Box<dyn Debug>)
    }
}

#[test]
fn main() {
    let report = XmlTester::new().test(&QuickXmlIT);
    report.print();
    report.assert();
}
