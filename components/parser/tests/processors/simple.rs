use crate::processors::{escape, Processor};
use std::fmt::Write;
use xrs_parser::simple::{
    AttributeAccess, SimpleXmlParser, SimpleXmlVisitor, StrVisitor, StringVisitor,
};
use xrs_parser::{XmlDecl, XmlError};

struct NoOpVisitor;

impl<'i> SimpleXmlVisitor<'i> for NoOpVisitor {
    type Value = ();

    fn visit_start_element<A: AttributeAccess<'i>>(
        self,
        _tag: &'i str,
        mut attrs: A,
    ) -> Result<Self::Value, XmlError> {
        while let Some(_) = attrs.next_entry(NoOpVisitor, NoOpVisitor)? {}
        Ok(())
    }

    fn visit_end_element(self, _tag: &'i str) -> Result<Self::Value, XmlError> {
        Ok(())
    }

    fn visit_declaration(self, _decl: XmlDecl) -> Result<Self::Value, XmlError> {
        Ok(())
    }

    fn visit_characters(self, _characters: &'i str) -> Result<Self::Value, XmlError> {
        Ok(())
    }

    fn visit_borrowed_characters(self, _characters: &str) -> Result<Self::Value, XmlError> {
        Ok(())
    }

    fn visit_pi(self, _target: &'i str, _data: Option<&'i str>) -> Result<Self::Value, XmlError> {
        Ok(())
    }

    fn visit_comment(self, _comment: &'i str) -> Result<Self::Value, XmlError> {
        Ok(())
    }
}

impl<'i> StrVisitor<'i> for NoOpVisitor {
    type Value = ();

    fn visit_str(self, value: &str) -> Result<Self::Value, XmlError> {
        Ok(())
    }

    fn visit_string(self, value: String) -> Result<Self::Value, XmlError> {
        Ok(())
    }
}

struct NormXmlVisitor(String);

impl<'i> SimpleXmlVisitor<'i> for &mut NormXmlVisitor {
    type Value = ();

    fn visit_start_element<A: AttributeAccess<'i>>(
        self,
        tag: &'i str,
        mut attrs: A,
    ) -> Result<Self::Value, XmlError> {
        let _ = write!(&mut self.0, "<{tag}");

        while let Some((key, value)) = attrs.next_entry(StringVisitor, StringVisitor)? {
            let _ = write!(&mut self.0, " {}=\"{}\"", escape(&key), escape(&value));
        }

        let _ = write!(&mut self.0, ">");

        Ok(())
    }

    fn visit_end_element(self, tag: &'i str) -> Result<Self::Value, XmlError> {
        let _ = write!(&mut self.0, "</{tag}>");
        Ok(())
    }

    fn visit_declaration(self, decl: XmlDecl) -> Result<Self::Value, XmlError> {
        let _ = write!(&mut self.0, "<?xml version=\"{}\"", decl.version());
        if let Some(enc) = decl.encoding() {
            let _ = write!(&mut self.0, " encoding=\"{enc}\"");
        }
        if let Some(standalone) = decl.standalone() {
            let _ = write!(
                &mut self.0,
                " standalone=\"{}\"",
                if standalone { "yes" } else { "no" }
            );
        }
        let _ = write!(&mut self.0, "?>");
        Ok(())
    }

    fn visit_characters(self, characters: &'i str) -> Result<Self::Value, XmlError> {
        let _ = write!(&mut self.0, "{}", escape(characters));
        Ok(())
    }

    fn visit_borrowed_characters(self, characters: &str) -> Result<Self::Value, XmlError> {
        let _ = write!(&mut self.0, "{}", escape(characters));
        Ok(())
    }

    fn visit_pi(self, target: &'i str, data: Option<&'i str>) -> Result<Self::Value, XmlError> {
        if let Some(data) = data {
            let _ = write!(&mut self.0, "<?{target} {data}?>");
        } else {
            let _ = write!(&mut self.0, "<?{target}?>");
        }
        Ok(())
    }

    fn visit_comment(self, comment: &'i str) -> Result<Self::Value, XmlError> {
        let _ = write!(&mut self.0, "<!--{}-->", comment);
        Ok(())
    }
}

pub struct SimpleProcessor;

impl Processor for SimpleProcessor {
    fn check_wf(&self, xml: &str) -> Result<(), String> {
        let mut parser = SimpleXmlParser::from_str(xml);

        loop {
            match parser.parse_next(NoOpVisitor) {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(err) => return Err(err.to_string()),
            }
        }

        Ok(())
    }

    fn norm(&self, xml: &str) -> Result<String, String> {
        let mut parser = SimpleXmlParser::from_str(xml);
        let mut writer = NormXmlVisitor(String::new());

        loop {
            match parser.parse_next(&mut writer) {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(err) => return Err(err.to_string()),
            }
        }

        Ok(writer.0)
    }
}
