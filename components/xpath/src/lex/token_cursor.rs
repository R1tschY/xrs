use crate::lex::{Ident, ParserToken, Punct, Token, Tokens};

#[derive(Clone, Copy, Debug)]
pub struct TokenCursor<'a> {
    rest: &'a [Token],
}

impl<'a> TokenCursor<'a> {
    pub fn new(tokens: &'a Tokens) -> Self {
        Self {
            rest: tokens.as_slice(),
        }
    }

    pub fn next(&self) -> Option<&'a Token> {
        self.rest.get(0)
    }

    pub fn consume_first(&self) -> Self {
        if let Some((_, rest)) = self.rest.split_first() {
            Self { rest }
        } else {
            *self
        }
    }

    pub fn consume(&self, n: usize) -> Self {
        let (_, rest) = self.rest.split_at(n);
        Self { rest }
    }

    pub fn error(&mut self) {
        panic!("parser error")
    }

    pub fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }

    pub fn punct(&'a self) -> Option<&'a Punct> {
        match self.next() {
            Some(Token::Punct(punct)) => Some(punct),
            _ => None,
        }
    }

    pub fn ident(&'a self) -> Option<&'a Ident> {
        match self.next() {
            Some(Token::Ident(ident)) => Some(ident),
            _ => None,
        }
    }

    pub fn try_consume<T: ParserToken>(&self) -> Option<Self> {
        if T::peek(self) {
            Some(self.consume(T::len()))
        } else {
            None
        }
    }

    pub fn expect<T: ParserToken>(&self) -> Self {
        if T::peek(self) {
            self.consume(T::len())
        } else {
            panic!("parser error: expected {}", T::display())
        }
    }
}
