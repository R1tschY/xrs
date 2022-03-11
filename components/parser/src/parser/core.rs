use crate::parser::Parser;
use crate::Cursor;
use std::marker::PhantomData;

#[inline]
pub fn lexeme<'a, T: 'a + Parser<'a>>(parser: T) -> Lexeme<'a, T> {
    Lexeme(parser, PhantomData)
}

pub struct Lexeme<'a, T: Parser<'a>>(T, PhantomData<&'a T>);

impl<'a, T: Parser<'a>> Parser<'a> for Lexeme<'a, T> {
    type Attribute = &'a str;
    type Error = T::Error;

    fn parse(&self, start: Cursor<'a>) -> Result<(&'a str, Cursor<'a>), T::Error> {
        let (_, end) = self.0.parse(start)?;
        Ok(start.advance2(end.offset() - start.offset()))
    }
}

struct Optional<T>(T);

impl<'a, T: Parser<'a>> Parser<'a> for Optional<T> {
    type Attribute = Option<T::Attribute>;
    type Error = T::Error;

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), T::Error> {
        match self.0.parse(cur) {
            Ok((attr, cur)) => Ok((Some(attr), cur)),
            Err(err) => Ok((None, cur)),
        }
    }
}

struct Kleene<T>(T);

impl<'a, T: Parser<'a>> Parser<'a> for Kleene<T> {
    type Attribute = Vec<T::Attribute>;
    type Error = T::Error;

    fn parse(&self, mut cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), T::Error> {
        let mut res = vec![];
        while let Ok((attr, cursor)) = self.0.parse(cur) {
            cur = cursor;
            res.push(attr);
        }
        Ok((res, cur))
    }
}

struct Plus<T>(T);

impl<'a, T: Parser<'a>> Parser<'a> for Plus<T> {
    type Attribute = Vec<T::Attribute>;
    type Error = T::Error;

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), T::Error> {
        let mut res = vec![];
        let (first, mut cur) = self.0.parse(cur)?;
        res.push(first);

        while let Ok((attr, cursor)) = self.0.parse(cur) {
            cur = cursor;
            res.push(attr);
        }

        Ok((res, cur))
    }
}

#[inline]
pub fn seq2<'a, T1: Parser<'a, Error = E>, T2: Parser<'a, Error = E>, E>(
    parser1: T1,
    parser2: T2,
) -> Sequence2<T1, T2> {
    Sequence2(parser1, parser2)
}

pub struct Sequence2<T1, T2>(T1, T2);

impl<'a, T1: Parser<'a, Error = E>, T2: Parser<'a, Error = E>, E> Parser<'a> for Sequence2<T1, T2> {
    type Attribute = (T1::Attribute, T2::Attribute);
    type Error = E;

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (v1, cur) = self.0.parse(cur)?;
        let (v2, cur) = self.1.parse(cur)?;
        Ok(((v1, v2), cur))
    }
}
