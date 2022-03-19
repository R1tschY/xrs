use crate::parser::Parser;
use crate::Cursor;

pub fn lit(lit: &'static str) -> Lit {
    Lit { lit }
}

pub struct Lit {
    lit: &'static str,
}

impl<'a> Parser<'a> for Lit {
    type Attribute = ();
    type Error = ();

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if !cur.has_next_str(self.lit) {
            Err(())
        } else {
            Ok(((), cur.advance(self.lit.len())))
        }
    }
}

pub fn chars<P: Fn(char) -> bool>(predicate: P) -> Chars<P> {
    Chars { predicate }
}

pub struct Chars<P: Fn(char) -> bool> {
    predicate: P,
}

impl<'a, P: Fn(char) -> bool> Parser<'a> for Chars<P> {
    type Attribute = &'a str;
    type Error = ();

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Some((i, c)) = cur
            .rest()
            .char_indices()
            .find(|(_, c)| !(self.predicate)(*c))
        {
            Ok(cur.advance2(i))
        } else {
            Err(())
        }
    }
}

pub fn bytes<P: Fn(u8) -> bool>(predicate: P) -> Bytes<P> {
    Bytes { predicate }
}

pub struct Bytes<P: Fn(u8) -> bool> {
    predicate: P,
}

impl<'a, P: Fn(u8) -> bool> Parser<'a> for Bytes<P> {
    type Attribute = &'a str;
    type Error = ();

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Some((i, c)) = cur
            .rest_bytes()
            .iter()
            .enumerate()
            .find(|(_, &c)| !(self.predicate)(c))
        {
            Ok(cur.advance2(i))
        } else {
            Err(())
        }
    }
}
