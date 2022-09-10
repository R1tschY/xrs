extern crate core;

use std::borrow::Cow;
use std::{fmt, io};

use crate::escape::Escape;
use crate::write::UnicodeWrite;
use crate::State::{Epilog, Main};

pub mod escape;
pub mod write;

enum State {
    Prolog,
    Main,
    Epilog,
}

pub struct XmlWriter<'w, W: UnicodeWrite, E: Escape> {
    state: State,
    stack: Vec<Cow<'w, str>>,
    writer: W,
    escaper: E,
}

impl<'w, W: UnicodeWrite, E: Escape> XmlWriter<'w, W, E> {
    pub fn for_writer(writer: W, escaper: E) -> Self {
        Self {
            state: State::Prolog,
            stack: vec![],
            writer,
            escaper,
        }
    }

    pub fn element<'a>(
        &'a mut self,
        name: Cow<'w, str>,
    ) -> io::Result<XmlElementWriter<'a, 'w, W, E>> {
        // TODO: check name
        self.writer.write_fmt(format_args!("<{}", name))?;
        self.state = Main;
        Ok(XmlElementWriter { name, ser: self })
    }

    pub fn end_element(&mut self) -> io::Result<()> {
        if let Some(name) = self.stack.pop() {
            self.writer.write_fmt(format_args!("</{}>", name))?;
            if self.stack.is_empty() {
                self.state = Epilog;
            }
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "TODO: stack underflow",
            ))
        }
    }

    pub fn characters(&mut self, characters: &str) -> io::Result<()> {
        // TODO: check for only whitespace in prolog and epilog, check characters
        self.escaper.escape_content(characters, &mut self.writer)
    }

    pub fn cdata(&mut self, characters: &str) -> io::Result<()> {
        // TODO: check characters and for ]]>
        write!(self.writer, "<![CDATA[{}]]>", characters)
    }

    pub fn comment(&mut self, comment: &str) -> io::Result<()> {
        // TODO: check for no -- and - at end, check comment
        write!(self.writer, "<!--{}-->", comment)
    }

    pub fn pi(&mut self, name: &str, data: Option<&str>) -> io::Result<()> {
        // TODO: check name and data
        if let Some(data) = data {
            write!(self.writer, "<?{} {}?>", name, data)
        } else {
            write!(self.writer, "<?{}?>", name)
        }
    }

    pub fn finish(self) {
        if !self.stack.is_empty() {
            panic!("missing end_element call(s): {}", self.stack.join(", "));
        }
    }
}

pub struct XmlElementWriter<'ser, 'w, W: UnicodeWrite, E: Escape> {
    name: Cow<'w, str>,
    ser: &'ser mut XmlWriter<'w, W, E>,
}

impl<'ser, 'w, W: UnicodeWrite, E: Escape> XmlElementWriter<'ser, 'w, W, E> {
    pub fn attribute(self, key: &str, value: &str) -> io::Result<Self> {
        // TODO: check key
        self.ser.writer.write_all(" ")?;
        self.ser.writer.write_all(key)?;
        self.ser.writer.write_all("=\"")?;
        self.ser
            .escaper
            .escape_attr_value_quot(value, &mut self.ser.writer)?;
        self.ser.writer.write_all("\"")?;
        Ok(self)
    }

    pub fn finish(self) -> io::Result<()> {
        self.ser.stack.push(self.name);
        self.ser.writer.write_all(">")
    }

    pub fn finish_empty(self) -> io::Result<()> {
        self.ser.writer.write_all("/>")
    }
}

#[cfg(test)]
mod tests {
    use crate::escape::DefaultEscaper;

    use super::*;

    #[test]
    fn test_empty() -> io::Result<()> {
        let mut writer = String::new();
        let mut xml_writer = XmlWriter::for_writer(&mut writer, DefaultEscaper);
        xml_writer.element("xrs".into())?.finish_empty()?;

        assert_eq!("<xrs/>", &writer);

        Ok(())
    }

    #[test]
    fn test_attributes() -> io::Result<()> {
        let mut writer = String::new();
        let mut xml_writer = XmlWriter::for_writer(&mut writer, DefaultEscaper);
        xml_writer
            .element("xrs".into())?
            .attribute("a", "1")?
            .attribute("b", "<")?
            .finish_empty()?;

        assert_eq!(r#"<xrs a="1" b="&lt;"/>"#, &writer);

        Ok(())
    }

    #[test]
    fn test_non_empty() -> io::Result<()> {
        let mut writer = String::new();
        let mut xml_writer = XmlWriter::for_writer(&mut writer, DefaultEscaper);
        xml_writer.element("xrs".into())?.finish()?;
        xml_writer.end_element()?;

        assert_eq!("<xrs></xrs>", &writer);

        Ok(())
    }

    #[test]
    fn test_nested() -> io::Result<()> {
        let mut writer = String::new();
        let mut xml_writer = XmlWriter::for_writer(&mut writer, DefaultEscaper);
        xml_writer.element("x".into())?.finish()?;
        xml_writer.element("y".into())?.finish()?;
        xml_writer.end_element()?;
        xml_writer.end_element()?;

        assert_eq!("<x><y></y></x>", &writer);

        Ok(())
    }

    #[test]
    fn test_content() -> io::Result<()> {
        let mut writer = String::new();
        let mut xml_writer = XmlWriter::for_writer(&mut writer, DefaultEscaper);
        xml_writer.element("xrs".into())?.finish()?;
        xml_writer.characters("abc <")?;
        xml_writer.end_element()?;

        assert_eq!("<xrs>abc &lt;</xrs>", &writer);

        Ok(())
    }

    #[test]
    fn test_escape() -> io::Result<()> {
        let mut writer = String::new();
        let mut xml_writer = XmlWriter::for_writer(&mut writer, DefaultEscaper);
        xml_writer
            .element("xrs".into())?
            .attribute("attr", r#"<&'""#)?
            .finish()?;
        xml_writer.characters("<&]]>")?;
        xml_writer.end_element()?;

        assert_eq!(
            r#"<xrs attr="&lt;&amp;&apos;&quot;">&lt;&amp;]]&gt;</xrs>"#,
            &writer
        );

        Ok(())
    }
}
