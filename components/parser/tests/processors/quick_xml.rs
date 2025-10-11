use crate::processors::{escape, Processor};
use quick_xml::events::attributes::AttrError;
use quick_xml::events::Event;
use quick_xml::name::QName;
use std::borrow::Cow;
use std::fmt::Write;
use std::str::FromStr;

pub struct QuickXmlProcessor;

fn create_reader(xml: &str) -> quick_xml::Reader<&[u8]> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let config = reader.config_mut();
    config.allow_unmatched_ends = false;
    config.check_comments = true;
    config.check_end_names = true;
    config.trim_markup_names_in_closing_tags = true;
    reader
}

impl Processor for QuickXmlProcessor {
    fn check_wf(&self, xml: &str) -> Result<(), String> {
        let mut parser = create_reader(xml);

        loop {
            match parser.read_event() {
                Ok(Event::Eof) => return Ok(()),
                Ok(Event::Start(tag)) => {
                    for attr in tag.attributes() {
                        if let Err(err) = attr {
                            return Err(err.to_string());
                        }
                    }
                }
                Ok(Event::Decl(decl)) => {
                    if let Err(err) = decl.version() {
                        return Err(err.to_string());
                    }
                    if let Some(Err(err)) = decl.encoding() {
                        return Err(err.to_string());
                    }
                    if let Some(Err(err)) = decl.standalone() {
                        return Err(err.to_string());
                    }
                }
                Ok(_) => {}
                Err(err) => return Err(err.to_string()),
            }
        }
    }

    fn norm(&self, xml: &str) -> Result<String, String> {
        let mut parser = create_reader(xml);
        let mut writer = String::new();

        loop {
            match parser.read_event() {
                Ok(evt) => match evt {
                    Event::Start(tag) => {
                        let _ = write!(&mut writer, "<{}", qname_str(tag.name()));

                        for attr in tag.attributes() {
                            let attr = attr.map_err(|err| format!("{err}"))?;
                            let _ = write!(
                                &mut writer,
                                " {}=\"{}\"",
                                qname_str(attr.key),
                                escape(&attr.unescape_value().map_err(|err| err.to_string())?)
                            );
                        }

                        let _ = write!(&mut writer, ">");
                    }
                    Event::End(tag) => {
                        let _ = write!(&mut writer, "</{}>", qname_str(tag.name()));
                    }
                    Event::Empty(tag) => {
                        let _ = write!(&mut writer, "<{}", qname_str(tag.name()));

                        for attr in tag.attributes() {
                            let attr = attr.map_err(|err| format!("{err}"))?;
                            let _ = write!(
                                &mut writer,
                                " {}=\"{}\"",
                                qname_str(attr.key),
                                escape(&attr.unescape_value().map_err(|err| err.to_string())?)
                            );
                        }

                        let _ = write!(&mut writer, "></{}>", qname_str(tag.name()));
                    }
                    Event::Text(txt) => {
                        writer.push_str(&escape(&txt.unescape().map_err(|err| err.to_string())?));
                    }
                    Event::CData(data) => {
                        writer.push_str(&text_str(&data.into_inner()));
                    }
                    Event::Comment(comment) => {
                        let _ = write!(&mut writer, "<!--{}-->", text_str(&comment.into_inner()));
                    }
                    Event::Decl(decl) => {
                        let _ = write!(
                            &mut writer,
                            "<?xml version=\"{}\"",
                            text_result_str(decl.version())?
                        );
                        if let Some(enc) = decl.encoding() {
                            let _ = write!(&mut writer, " encoding=\"{}\"", attr_result(enc)?);
                        }
                        if let Some(standalone) = decl.standalone() {
                            let _ =
                                write!(&mut writer, " standalone=\"{}\"", attr_result(standalone)?);
                        }
                        let _ = write!(&mut writer, "?>");
                    }
                    Event::PI(pi) => {
                        let _ = write!(&mut writer, "<?{}?>", text_str(&pi.into_inner()));
                    }
                    Event::DocType(ty) => {
                        let _ = write!(&mut writer, "<!DOCTYPE {}>", text_str(&ty.into_inner()));
                    }
                    Event::Eof => return Ok(writer),
                },
                Err(err) => return Err(err.to_string()),
            }
        }
    }
}

fn qname_str(qname: QName) -> String {
    String::from_utf8_lossy(qname.into_inner()).to_string()
}

fn text_str(text: &[u8]) -> String {
    escape(&String::from_utf8_lossy(text))
}

fn text_result_str(text: Result<Cow<[u8]>, quick_xml::Error>) -> Result<String, String> {
    Ok(escape(&String::from_utf8_lossy(
        &text.map_err(|err| err.to_string())?,
    )))
}

fn attr_result(text: Result<Cow<[u8]>, AttrError>) -> Result<String, String> {
    Ok(escape(&String::from_utf8_lossy(
        &text.map_err(|err| err.to_string())?,
    )))
}
