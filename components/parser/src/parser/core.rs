use std::marker::PhantomData;

use crate::parser::Parser;
use crate::Cursor;

#[inline]
pub fn raw<'a, T: 'a + Parser<'a>>(parser: T) -> Raw<'a, T> {
    Raw(parser, PhantomData)
}

pub struct Raw<'a, T: Parser<'a>>(T, PhantomData<&'a T>);

impl<'a, T: Parser<'a>> Parser<'a> for Raw<'a, T> {
    type Attribute = &'a str;
    type Error = T::Error;

    fn parse(&self, start: Cursor<'a>) -> Result<(&'a str, Cursor<'a>), T::Error> {
        let (_, end) = self.0.parse(start)?;
        Ok(start.advance2(end.offset() - start.offset()))
    }
}

pub fn optional<'a, T: Parser<'a>>(parser: T) -> Optional<T> {
    Optional(parser)
}

pub struct Optional<T>(T);

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

pub fn kleene<'a, T: Parser<'a>>(parser: T) -> Kleene<T> {
    Kleene(parser)
}

pub struct Kleene<T>(T);

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

pub fn plus<'a, T: Parser<'a>>(parser: T) -> Plus<T> {
    Plus(parser)
}

pub struct Plus<T>(T);

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

macro_rules! def_seq {
    ($($i:tt: $t:ident),+ $(,)?) => {
        impl<
                'a,
                $($t: Parser<'a, Error = E>),*,
                E,
            > Parser<'a> for ($($t),*,)
        {
            type Attribute = ($($t::Attribute),*,);
            type Error = E;

            #[allow(non_snake_case)]
            fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
                $(let ($t, cur) = self.$i.parse(cur)?;)*
                Ok((($($t),*,), cur))
            }
        }
    };
}

def_seq!(0: T1);
def_seq!(0: T1, 1: T2);
def_seq!(0: T1, 1: T2, 2: T3);
def_seq!(0: T1, 1: T2, 2: T3, 3: T4);
def_seq!(0: T1, 1: T2, 2: T3, 3: T4, 4: T5);
