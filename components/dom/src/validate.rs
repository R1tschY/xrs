use quick_xml::Error;

pub trait XmlValidator {
    fn validate_tag(&self, tag: &[u8]) -> Result<(), Error>;
    fn validate_comment(&self, comment: &[u8]) -> Result<(), Error>;
    fn validate_text(&self, text: &[u8]) -> Result<(), Error>;
}

pub struct NonValidator;

impl XmlValidator for NonValidator {
    fn validate_tag(&self, _tag: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_comment(&self, _comment: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_text(&self, _text: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

pub struct WellFormedValidator;

impl XmlValidator for WellFormedValidator {
    fn validate_tag(&self, tag: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_comment(&self, _comment: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn validate_text(&self, _text: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}
