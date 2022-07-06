extern crate core;

use crate::State::{Epilog, Main};
use std::borrow::Cow;
use std::io;
use std::io::{ErrorKind, Write};

enum State {
    Prolog,
    Main,
    Epilog,
}

pub struct XmlWriter<'w, W: Write> {
    state: State,
    stack: Vec<Cow<'w, str>>,
    writer: W,
}

impl<'w, W: Write> XmlWriter<'w, W> {
    pub fn for_writer(writer: W) -> Self {
        Self {
            state: State::Prolog,
            stack: vec![],
            writer,
        }
    }

    pub fn element<'a>(
        &'a mut self,
        name: Cow<'w, str>,
    ) -> io::Result<XmlElementWriter<'a, 'w, W>> {
        write!(self.writer, "<{}", name)?;
        self.state = Main;
        Ok(XmlElementWriter { name, ser: self })
    }

    pub fn end_element(&mut self) -> io::Result<()> {
        if let Some(name) = self.stack.pop() {
            write!(self.writer, "</{}>", name)?;
            if self.stack.is_empty() {
                self.state = Epilog;
            }
            Ok(())
        } else {
            Err(io::Error::new(ErrorKind::Other, "TODO: stack underflow"))
        }
    }

    pub fn characters(&mut self, characters: &str) -> io::Result<()> {
        // TODO: check for only whitespace in prolog and epilog, check characters
        write!(self.writer, "{}", characters)
    }

    pub fn cdata(&mut self, characters: &str) -> io::Result<()> {
        // TODO: check characters
        write!(self.writer, "<![CDATA[{}]]>", characters)
    }

    pub fn comment(&mut self, comment: &str) -> io::Result<()> {
        // TODO: check for no --, check comment
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

pub struct XmlElementWriter<'ser, 'w, W: Write> {
    name: Cow<'w, str>,
    ser: &'ser mut XmlWriter<'w, W>,
}

impl<'ser, 'w, W: Write> XmlElementWriter<'ser, 'w, W> {
    pub fn attribute(self, key: &str, value: &str) -> io::Result<Self> {
        write!(self.ser.writer, " {}=\"{}\"", key, value)?; // TODO: escape value, check key
        Ok(self)
    }

    pub fn finish(self) -> io::Result<()> {
        self.ser.stack.push(self.name);
        write!(self.ser.writer, ">")
    }

    pub fn finish_empty(self) -> io::Result<()> {
        write!(self.ser.writer, "/>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal() {
        let mut writer: Vec<u8> = vec![];
        let mut xml_writer = XmlWriter::for_writer(&mut writer);
        xml_writer
            .element("xrs".into())
            .unwrap()
            .finish_empty()
            .unwrap();

        assert_eq!("<xrs/>", std::str::from_utf8(&writer).unwrap());
    }

    #[test]
    fn test_attributes() {
        let mut writer: Vec<u8> = vec![];
        let mut xml_writer = XmlWriter::for_writer(&mut writer);
        xml_writer
            .element("xrs".into())
            .unwrap()
            .attribute("a", "1")
            .unwrap()
            .attribute("b", "2")
            .unwrap()
            .finish_empty()
            .unwrap();

        assert_eq!(
            "<xrs a=\"1\" b=\"2\"/>",
            std::str::from_utf8(&writer).unwrap()
        );
    }

    #[test]
    fn test_non_empty() {
        let mut writer: Vec<u8> = vec![];
        let mut xml_writer = XmlWriter::for_writer(&mut writer);
        xml_writer.element("xrs".into()).unwrap().finish().unwrap();
        xml_writer.end_element().unwrap();

        assert_eq!("<xrs></xrs>", std::str::from_utf8(&writer).unwrap());
    }

    #[test]
    fn test_nested() {
        let mut writer: Vec<u8> = vec![];
        let mut xml_writer = XmlWriter::for_writer(&mut writer);
        xml_writer.element("x".into()).unwrap().finish().unwrap();
        xml_writer.element("y".into()).unwrap().finish().unwrap();
        xml_writer.end_element().unwrap();
        xml_writer.end_element().unwrap();

        assert_eq!("<x><y></y></x>", std::str::from_utf8(&writer).unwrap());
    }
}
