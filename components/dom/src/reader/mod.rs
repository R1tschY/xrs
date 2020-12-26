use crate::dom::Document;

pub mod quick_xml;

pub trait DomReader<'r> {
    type Error;

    fn parse(self) -> Result<Document<'r>, Self::Error>;
}
