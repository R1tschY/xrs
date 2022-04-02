use std::str::{Bytes, Chars};

use crate::lex::span::Span;

#[derive(Copy, Clone, Debug)]
pub(crate) struct LexCursor<'a> {
    rest: &'a str,
    offset: usize,
}

impl<'a> LexCursor<'a> {
    pub(crate) fn for_str(input: &'a str) -> Self {
        Self {
            rest: input,
            offset: 0,
        }
    }

    pub(crate) fn advance(&self, bytes: usize) -> Self {
        let (_, rest) = self.rest.split_at(bytes);
        Self {
            rest,
            offset: self.offset + bytes,
        }
    }

    pub(crate) fn advance_to(&self, bytes: usize) -> (&'a str, Self) {
        let (skip, rest) = self.rest.split_at(bytes);
        (
            skip,
            Self {
                rest,
                offset: self.offset + bytes,
            },
        )
    }

    pub(crate) fn starts_with(&self, prefix: &str) -> bool {
        self.rest.starts_with(prefix)
    }

    pub(crate) fn next_byte(&self) -> Option<u8> {
        self.rest.bytes().next()
    }

    pub(crate) fn next_char(&self) -> Option<char> {
        self.rest.chars().next()
    }

    pub(crate) fn eof(&self) -> bool {
        self.rest.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.rest.len()
    }

    pub(crate) fn bytes(&self) -> Bytes<'a> {
        self.rest.bytes()
    }

    pub(crate) fn as_bytes(&self) -> &'a [u8] {
        self.rest.as_bytes()
    }

    pub(crate) fn chars(&self) -> Chars<'a> {
        self.rest.chars()
    }

    pub(crate) fn span(&self, n: usize) -> Span {
        Span::new(self.offset, self.offset + n)
    }
}
