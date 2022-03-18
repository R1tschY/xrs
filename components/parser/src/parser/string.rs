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

pub fn char_<P: Fn(char) -> bool>(predicate: P) -> Char<P> {
    Char { predicate }
}

pub struct Char<P: Fn(char) -> bool> {
    predicate: P,
}

impl<'a, P: Fn(char) -> bool> Parser<'a> for Char<P> {
    type Attribute = ();
    type Error = ();

    fn parse(&self, cur: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Some(c) = cur.next_char() {
            if (self.predicate)(c) {
                return Ok(((), cur.advance(1)));
            }
        }
        Err(())
    }
}
