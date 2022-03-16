use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
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
                    self.write_stag(&reader, &mut writer, start)?;
                }
                Event::End(end) => {
                    self.write_etag(&reader, &mut writer, end)?;
                }
                Event::Empty(start) => {
                    self.write_stag(&reader, &mut writer, start.clone())?;
                    self.write_etag(&reader, &mut writer, BytesEnd::borrowed(start.name()))?;
                }
                Event::Text(text) => {
                    writer.push_str(reader.decode(text.unescaped()?.as_ref())?);
                }
                Event::Comment(_) => {}
                Event::CData(cdata) => {
                    writer.push_str(reader.decode(cdata.unescaped()?.as_ref())?);
                }
                Event::Decl(decl) => {
                    self.write_decl(&reader, &mut writer, decl);
                }
                Event::PI(text) => {
                    write!(writer, "<?{}?>", reader.decode(text.escaped())?);
                }
                Event::DocType(_) => {}
            }
        }
    }

    fn canonxml_internal_namespace(&self, input: &[u8]) -> Result<String, Error> {
        let mut reader = Reader::from_reader(input);

        let mut writer = String::new();
        let mut buf = Vec::new();
        let mut ns_buf = Vec::new();
        loop {
            match reader.read_namespaced_event(&mut buf, &mut ns_buf)? {
                (ns, Event::Eof) => return Ok(writer),
                (ns, Event::Start(start)) => {
                    self.write_stag(&reader, &mut writer, start)?;
                }
                (ns, Event::End(end)) => {
                    self.write_etag(&reader, &mut writer, end)?;
                }
                (ns, Event::Empty(start)) => {
                    self.write_stag(&reader, &mut writer, start.clone())?;
                    self.write_etag(&reader, &mut writer, BytesEnd::borrowed(start.name()))?;
                }
                (ns, Event::Text(text)) => {
                    writer.push_str(reader.decode(text.unescaped()?.as_ref())?);
                }
                (ns, Event::Comment(_)) => {}
                (ns, Event::CData(cdata)) => {
                    writer.push_str(reader.decode(cdata.unescaped()?.as_ref())?);
                }
                (ns, Event::Decl(decl)) => {
                    self.write_decl(&reader, &mut writer, decl);
                }
                (ns, Event::PI(text)) => {
                    write!(writer, "<?{}?>", reader.decode(text.escaped())?);
                }
                (ns, Event::DocType(_)) => {}
            }
        }
    }

    fn write_stag<'a>(
        &self,
        reader: &Reader<&[u8]>,
        writer: &mut String,
        stag: BytesStart<'a>,
    ) -> Result<(), Error> {
        write!(writer, "<{}", reader.decode(stag.name())?);
        for attr_res in stag.attributes().with_checks(true) {
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
        Ok(())
    }

    fn write_etag<'a>(
        &self,
        reader: &Reader<&[u8]>,
        writer: &mut String,
        etag: BytesEnd<'a>,
    ) -> Result<(), Error> {
        write!(writer, "</{}>", reader.decode(etag.name())?);
        Ok(())
    }

    fn write_decl<'a>(
        &self,
        reader: &Reader<&[u8]>,
        writer: &mut String,
        decl: BytesDecl<'a>,
    ) -> Result<(), Error> {
        if decl.version()?.as_ref() != b"1.0" {
            write!(
                writer,
                "<?xml version=\"{}\"/>",
                reader.decode(decl.version()?.as_ref())?
            );
        }
        Ok(())
    }
}

impl TestableParser for QuickXmlIT {
    fn is_wf(&self, input: &[u8], namespace: bool) -> bool {
        let mut reader = Reader::from_reader(input);
        reader.trim_text(false);
        reader.check_comments(true);
        reader.check_end_names(true);

        let mut buf = Vec::new();
        if namespace {
            let mut ns_buf = Vec::new();
            loop {
                match reader.read_namespaced_event(&mut buf, &mut ns_buf) {
                    Ok((_, Event::Eof)) => return true,
                    Ok((ns, Event::Start(start))) => {
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
        } else {
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
    }

    fn canonxml(&self, input: &[u8], namespace: bool) -> Result<String, Box<dyn Debug>> {
        if namespace {
            self.canonxml_internal_namespace(input)
                .map_err(|err| Box::new(err) as Box<dyn Debug>)
        } else {
            self.canonxml_internal(input)
                .map_err(|err| Box::new(err) as Box<dyn Debug>)
        }
    }
}

#[test]
fn main() {
    let report = XmlTester::new().test(&QuickXmlIT);
    report.print_statistic();
    report.assert();
}
