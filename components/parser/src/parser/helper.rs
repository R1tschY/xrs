use crate::parser::Parser;
use crate::Cursor;
use std::marker::PhantomData;

// struct MapAttr<T, U, F: Fn(T::Attribute) -> U>(T, F);
//
// impl<'a, T: Parser<'a>, U, F: Fn(T::Attribute) -> U> Parser<'a> for MapAttr<T, U, F> {
//     type Attribute = U;
//     type Error = T::Error;
//
//     fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), T::Error> {
//         self.0.parse(cur).map(&self.1)
//     }
// }

#[inline]
pub fn map_error<'a, T: Parser<'a>, E, F: Fn(T::Error) -> E>(
    parser: T,
    f: F,
) -> MapError<'a, T, E, F> {
    MapError(parser, f, PhantomData)
}

pub struct MapError<'a, T: Parser<'a>, E, F: Fn(T::Error) -> E>(T, F, PhantomData<&'a E>);

impl<'a, T: Parser<'a>, E, F: Fn(T::Error) -> E> Parser<'a> for MapError<'a, T, E, F> {
    type Attribute = T::Attribute;
    type Error = E;

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        self.0.parse(cur).map_err(&self.1)
    }
}

#[inline]
pub fn omit<'a, T: Parser<'a>>(parser: T) -> Omit<T> {
    Omit(parser)
}

pub struct Omit<T>(T);

impl<'a, T: Parser<'a>> Parser<'a> for Omit<T> {
    type Attribute = ();
    type Error = T::Error;

    fn parse(&self, mut cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        self.0.parse(cur).map(|(_, cur)| ((), cur))
    }
}

#[inline]
pub fn omit_error<'a, T: Parser<'a>>(parser: T) -> OmitError<T> {
    OmitError(parser)
}

pub struct OmitError<T>(T);

impl<'a, T: Parser<'a>> Parser<'a> for OmitError<T> {
    type Attribute = T::Attribute;
    type Error = ();

    fn parse(&self, mut cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        self.0.parse(cur).map_err(|err| ())
    }
}
