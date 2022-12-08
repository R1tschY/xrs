extern crate core;

use std::borrow::Cow;
use std::{fmt, io};

use crate::escape::{DefaultEscaper, Escape};
use crate::write::UnicodeWrite;

pub mod escape;
pub mod write;

pub trait XmlStagWrite {
    type Error;

    fn write_attribute(&mut self, key: &str, value: &str) -> Result<(), Self::Error>;
    fn finish(&mut self) -> Result<(), Self::Error>;
    fn finish_empty(&mut self) -> Result<(), Self::Error>;
}

pub trait XmlWrite {
    type Error;
    type StagWrite<'w>: XmlStagWrite<Error = Self::Error>
    where
        Self: 'w;

    fn write_stag(&mut self, name: &str) -> Result<Self::StagWrite<'_>, Self::Error>;
    fn write_comment(&mut self, comment: &str) -> Result<(), Self::Error>;
    fn write_characters(&mut self, characters: &str) -> Result<(), Self::Error>;
    fn write_cdata(&mut self, cdata: &str) -> Result<(), Self::Error>;
    fn write_pi(&mut self, target: &str, data: Option<&str>) -> Result<(), Self::Error>;
    fn write_xmldecl(
        &mut self,
        version: Option<&str>,
        standalone: Option<bool>,
        write_encoding: bool,
    ) -> Result<(), Self::Error>;
    fn write_etag(&mut self, name: &str) -> Result<(), Self::Error>;
}

pub struct CompactXmlWrite<W: UnicodeWrite> {
    write: W,
}

impl<W: UnicodeWrite> CompactXmlWrite<W> {
    pub fn new(write: W) -> Self {
        Self { write }
    }
}

impl<W: UnicodeWrite> XmlWrite for CompactXmlWrite<W> {
    type Error = io::Error;
    type StagWrite<'w> = CompactXmlStagWrite<'w, W> where Self: 'w;

    fn write_stag<'w>(&'w mut self, name: &str) -> Result<CompactXmlStagWrite<'w, W>, Self::Error> {
        self.write
            .write_fmt(format_args!("<{}", name))
            .map(|_| CompactXmlStagWrite {
                write: &mut self.write,
            })
    }

    fn write_comment(&mut self, comment: &str) -> Result<(), Self::Error> {
        write!(self.write, "<!--{}-->", comment)
    }

    fn write_characters(&mut self, characters: &str) -> Result<(), Self::Error> {
        DefaultEscaper.escape_content(characters, &mut self.write)
    }

    fn write_cdata(&mut self, cdata: &str) -> Result<(), Self::Error> {
        write!(self.write, "<![CDATA[{}]]>", cdata)
    }

    fn write_pi(&mut self, target: &str, data: Option<&str>) -> Result<(), Self::Error> {
        if let Some(data) = data {
            write!(self.write, "<?{} {}?>", target, data)
        } else {
            write!(self.write, "<?{}?>", target)
        }
    }

    fn write_xmldecl(
        &mut self,
        version: Option<&str>,
        standalone: Option<bool>,
        write_encoding: bool,
    ) -> Result<(), Self::Error> {
        todo!()
    }

    fn write_etag(&mut self, name: &str) -> Result<(), Self::Error> {
        self.write.write_fmt(format_args!("</{}>", name))
    }
}

pub struct CompactXmlStagWrite<'w, W: UnicodeWrite> {
    write: &'w mut W,
}

impl<'w, W: UnicodeWrite> XmlStagWrite for CompactXmlStagWrite<'w, W> {
    type Error = io::Error;

    fn write_attribute(&mut self, key: &str, value: &str) -> Result<(), Self::Error> {
        self.write.write_all(" ")?;
        self.write.write_all(key)?;
        self.write.write_all("=\"")?;
        DefaultEscaper.escape_attr_value_quot(value, &mut self.write)?;
        self.write.write_all("\"")
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        self.write.write_all(">")
    }

    fn finish_empty(&mut self) -> Result<(), Self::Error> {
        self.write.write_all("/>")
    }
}

enum State {
    Prolog,
    Main,
    Epilog,
}

pub struct XmlWriter<'o, W: XmlWrite> {
    state: State,
    stack: Vec<Cow<'o, str>>,
    write: W,
}

impl<'o, W: XmlWrite> XmlWriter<'o, W> {
    pub fn with_decl(
        mut write: W,
        version: Option<&str>,
        standalone: Option<bool>,
        write_encoding: bool,
    ) -> Result<XmlWriter<'o, W>, W::Error> {
        write.write_xmldecl(version, standalone, write_encoding)?;
        Ok(XmlWriter::without_decl(write))
    }

    pub fn without_decl(mut write: W) -> XmlWriter<'o, W> {
        XmlWriter {
            state: State::Prolog,
            stack: vec![],
            write,
        }
    }

    pub fn element<'w>(&'w mut self, name: &'o str) -> Result<XmlElementWriter<'w, W>, W::Error> {
        // TODO: check name
        self.stack.push(name.into());
        Ok(XmlElementWriter {
            stag_write: self.write.write_stag(name)?,
        })
    }

    pub fn end_element(&mut self) -> Result<(), W::Error> {
        if let Some(name) = self.stack.pop() {
            self.write.write_etag(&name)?;
            if self.stack.is_empty() {
                self.state = State::Epilog;
            }
            Ok(())
        } else {
            // TODO: Err(W::Error::stack_underflow())
            panic!()
        }
    }

    pub fn characters(&mut self, characters: &str) -> Result<(), W::Error> {
        // TODO: check for only whitespace in prolog and epilog, check characters
        self.write.write_characters(characters)
    }

    pub fn cdata(&mut self, characters: &str) -> Result<(), W::Error> {
        // TODO: check characters and for ]]>
        self.write.write_cdata(characters)
    }

    pub fn comment(&mut self, comment: &str) -> Result<(), W::Error> {
        // TODO: check for no -- and - at end, check comment
        self.write.write_comment(comment)
    }

    pub fn pi(&mut self, name: &str, data: Option<&str>) -> Result<(), W::Error> {
        // TODO: check name and data
        self.write.write_pi(name, data)
    }

    pub fn finish(self) {
        if !self.stack.is_empty() {
            panic!("missing end_element call(s): {}", self.stack.join(", "));
        }
    }
}

pub struct XmlElementWriter<'w, W: XmlWrite + 'w> {
    stag_write: W::StagWrite<'w>,
}

impl<'w, W: XmlWrite> XmlElementWriter<'w, W> {
    pub fn attribute(mut self, key: &str, value: &str) -> Result<Self, W::Error> {
        // TODO: check key

        self.stag_write.write_attribute(key, value)?;
        Ok(self)
    }

    pub fn finish(mut self) -> Result<(), W::Error> {
        self.stag_write.finish()
    }

    pub fn finish_empty(mut self) -> Result<(), W::Error> {
        self.stag_write.finish_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::escape::DefaultEscaper;

    use super::*;

    #[test]
    fn test_empty() -> io::Result<()> {
        let mut buf = String::new();
        let mut xml_writer = XmlWriter::without_decl(CompactXmlWrite::new(&mut buf));
        xml_writer.element("xrs".into())?.finish_empty()?;

        assert_eq!("<xrs/>", &buf);

        Ok(())
    }

    #[test]
    fn test_attributes() -> io::Result<()> {
        let mut buf = String::new();
        let mut xml_writer = XmlWriter::without_decl(CompactXmlWrite::new(&mut buf));
        xml_writer
            .element("xrs".into())?
            .attribute("a", "1")?
            .attribute("b", "<")?
            .finish_empty()?;

        assert_eq!(r#"<xrs a="1" b="&lt;"/>"#, &buf);

        Ok(())
    }

    #[test]
    fn test_non_empty() -> io::Result<()> {
        let mut buf = String::new();
        let mut xml_writer = XmlWriter::without_decl(CompactXmlWrite::new(&mut buf));
        xml_writer.element("xrs".into())?.finish()?;
        xml_writer.end_element()?;

        assert_eq!("<xrs></xrs>", &buf);

        Ok(())
    }

    #[test]
    fn test_nested() -> io::Result<()> {
        let mut buf = String::new();
        let mut xml_writer = XmlWriter::without_decl(CompactXmlWrite::new(&mut buf));
        xml_writer.element("x".into())?.finish()?;
        xml_writer.element("y".into())?.finish()?;
        xml_writer.end_element()?;
        xml_writer.end_element()?;

        assert_eq!("<x><y></y></x>", &buf);

        Ok(())
    }

    #[test]
    fn test_content() -> io::Result<()> {
        let mut buf = String::new();
        let mut xml_writer = XmlWriter::without_decl(CompactXmlWrite::new(&mut buf));
        xml_writer.element("xrs".into())?.finish()?;
        xml_writer.characters("abc <")?;
        xml_writer.end_element()?;

        assert_eq!("<xrs>abc &lt;</xrs>", &buf);

        Ok(())
    }

    #[test]
    fn test_escape() -> io::Result<()> {
        let mut buf = String::new();
        let mut xml_writer = XmlWriter::without_decl(CompactXmlWrite::new(&mut buf));
        xml_writer
            .element("xrs".into())?
            .attribute("attr", r#"<&'""#)?
            .finish()?;
        xml_writer.characters("<&]]>")?;
        xml_writer.end_element()?;

        assert_eq!(
            r#"<xrs attr="&lt;&amp;&apos;&quot;">&lt;&amp;]]&gt;</xrs>"#,
            &buf
        );

        Ok(())
    }
}
