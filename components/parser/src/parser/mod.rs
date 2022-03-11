use crate::Cursor;

pub mod core;
pub mod cursor;
pub mod helper;
pub mod string;

pub trait Parser<'a> {
    type Attribute;
    type Error;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error>;
}
