use crate::chars::XmlStrExt;
use crate::dom::Element;
use crate::error::{Error, Reason};
use crate::Span;
use std::io::Read;
use std::str::from_utf8;

pub trait XmlValidatorBuilder<'a> {
    type Item: XmlValidator<'a>;

    fn build(self, doc: &'a [u8]) -> Self::Item;
}

pub trait XmlValidator<'a> {
    fn validate_start(&self, pos: usize, tag: &[u8], attributes: &[u8]) -> Result<(), Error>;
    fn validate_end(&self, pos: usize, tag: &[u8], element: &Element) -> Result<(), Error>;
    fn validate_comment(&self, pos: usize, comment: &[u8]) -> Result<(), Error>;
    fn validate_text(&self, pos: usize, text: &[u8]) -> Result<(), Error>;
    fn validate_cdata(&self, pos: usize, cdata: &[u8]) -> Result<(), Error>;
    fn validate_pi(&self, pos: usize, instr: &[u8]) -> Result<(), Error>;
}

pub struct NonValidator;

impl<'a> XmlValidatorBuilder<'a> for NonValidator {
    type Item = NonValidator;

    fn build(self, _doc: &'a [u8]) -> Self {
        Self
    }
}

impl<'a> XmlValidator<'a> for NonValidator {
    fn validate_start(&self, _pos: usize, _tag: &[u8], _attributes: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_end(&self, _pos: usize, _tag: &[u8], _element: &Element) -> Result<(), Error> {
        Ok(())
    }

    fn validate_comment(&self, _pos: usize, _comment: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_text(&self, _pos: usize, _text: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_cdata(&self, _pos: usize, _cdata: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_pi(&self, _pos: usize, _name: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

pub struct StructureValidatorBuilder;

pub struct StructureValidator<'a> {
    doc: &'a [u8],
}

impl<'a> XmlValidatorBuilder<'a> for StructureValidatorBuilder {
    type Item = StructureValidator<'a>;

    fn build(self, doc: &'a [u8]) -> StructureValidator<'a> {
        StructureValidator { doc }
    }
}

impl<'a> XmlValidator<'a> for StructureValidator<'a> {
    fn validate_start(&self, _pos: usize, _tag: &[u8], _attributes: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_end(&self, pos: usize, tag: &[u8], element: &Element) -> Result<(), Error> {
        let start_tag = element.tag_span().to_slice(self.doc);
        if tag != start_tag {
            Err(Error::new(
                Span::new(pos + 2, tag.len()),
                Reason::EndEventMismatch {
                    expected: String::from_utf8_lossy(start_tag).into_owned(),
                    found: String::from_utf8_lossy(tag).into_owned(),
                },
            ))
        } else {
            Ok(())
        }
    }

    fn validate_comment(&self, _pos: usize, _comment: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_text(&self, _pos: usize, _text: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_cdata(&self, _pos: usize, _cdata: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_pi(&self, _pos: usize, _name: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

pub struct WellFormedValidatorBuilder;

pub struct WellFormedValidator<'a> {
    next: StructureValidator<'a>,
}

impl<'a> XmlValidatorBuilder<'a> for WellFormedValidatorBuilder {
    type Item = WellFormedValidator<'a>;

    fn build(self, doc: &'a [u8]) -> WellFormedValidator<'a> {
        WellFormedValidator {
            next: StructureValidatorBuilder.build(doc),
        }
    }
}

impl<'a> XmlValidator<'a> for WellFormedValidator<'a> {
    fn validate_start(&self, pos: usize, tag: &[u8], attributes: &[u8]) -> Result<(), Error> {
        self.next.validate_start(pos, tag, attributes)?;

        let tag_str = from_utf8(tag).map_err(|err| {
            Error::new(Span::new(pos + err.valid_up_to() + 1, 0), Reason::Utf8(err))
        })?;
        from_utf8(attributes).map_err(|err| {
            Error::new(
                Span::new(pos + tag.len() + err.valid_up_to() + 1, 0),
                Reason::Utf8(err),
            )
        })?;

        if !tag_str.is_xml_name() {
            return Err(Error::new(Span::new(pos, tag.len()), Reason::InvalidName));
        }

        Ok(())
    }

    fn validate_end(&self, pos: usize, tag: &[u8], element: &Element) -> Result<(), Error> {
        self.next.validate_end(pos, tag, element)?;
        Ok(())
    }

    fn validate_comment(&self, pos: usize, comment: &[u8]) -> Result<(), Error> {
        self.next.validate_comment(pos, comment)?;

        let comment_str = from_utf8(comment).map_err(|err| {
            Error::new(
                Span::new(pos + 4 + err.valid_up_to() + 1, 0),
                Reason::Utf8(err),
            )
        })?;

        if let Some((err_pos, c)) = comment_str.find_invalid_xml_char() {
            return Err(Error::new(
                Span::new(pos + err_pos, c.len_utf8()),
                Reason::IllegalChar(c),
            ));
        }

        for i in memchr::memchr_iter(b'-', comment) {
            if i + 1 != comment.len() && comment[i + 1] == b'-' {
                let span = Span::new(pos + 4 + i, 2);
                return Err(Error::new(span, Reason::IllegalPatternInComment));
            }
        }

        Ok(())
    }

    fn validate_text(&self, pos: usize, text: &[u8]) -> Result<(), Error> {
        self.next.validate_text(pos, text)?;

        let text_str = from_utf8(text).map_err(|err| {
            Error::new(Span::new(pos + err.valid_up_to() + 1, 0), Reason::Utf8(err))
        })?;

        if let Some((err_pos, c)) = text_str.find_invalid_xml_char() {
            return Err(Error::new(
                Span::new(pos + err_pos, c.len_utf8()),
                Reason::IllegalChar(c),
            ));
        }

        Ok(())
    }

    fn validate_cdata(&self, pos: usize, cdata: &[u8]) -> Result<(), Error> {
        self.next.validate_cdata(pos, cdata)?;

        let cdata_str = from_utf8(cdata).map_err(|err| {
            Error::new(Span::new(pos + err.valid_up_to() + 1, 0), Reason::Utf8(err))
        })?;

        if let Some((err_pos, c)) = cdata_str.find_invalid_xml_char() {
            return Err(Error::new(
                Span::new(pos + err_pos, c.len_utf8()),
                Reason::IllegalChar(c),
            ));
        }

        Ok(())
    }

    fn validate_pi(&self, pos: usize, instr: &[u8]) -> Result<(), Error> {
        self.next.validate_pi(pos, instr)?;

        let instr_str = from_utf8(instr).map_err(|err| {
            Error::new(Span::new(pos + err.valid_up_to() + 1, 0), Reason::Utf8(err))
        })?;

        if let Some((err_pos, c)) = instr_str.find_invalid_xml_char() {
            return Err(Error::new(
                Span::new(pos + err_pos, c.len_utf8()),
                Reason::IllegalChar(c),
            ));
        }

        Ok(())
    }
}
