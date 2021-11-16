use crate::{characters, Span};
use std::cmp::Ordering::{Equal, Greater, Less};
use std::fmt;
use std::fmt::Debug;
use std::str::{Bytes, Chars, FromStr};

#[derive(Debug)]
pub struct LexError {
    span: Span,
}

struct Reject;

#[derive(Copy, Clone, Debug)]
struct LexCursor<'a> {
    rest: &'a str,
    offset: usize,
}

impl<'a> LexCursor<'a> {
    fn for_str(input: &'a str) -> Self {
        Self {
            rest: input,
            offset: 0,
        }
    }

    fn advance(&self, bytes: usize) -> Self {
        let (_, rest) = self.rest.split_at(bytes);
        Self {
            rest,
            offset: self.offset + bytes,
        }
    }

    fn advance_to(&self, bytes: usize) -> (&'a str, Self) {
        let (skip, rest) = self.rest.split_at(bytes);
        (
            skip,
            Self {
                rest,
                offset: self.offset + bytes,
            },
        )
    }

    fn starts_with(&self, prefix: &str) -> bool {
        self.rest.starts_with(prefix)
    }

    fn next_byte(&self) -> Option<u8> {
        self.rest.bytes().next()
    }

    fn next_char(&self) -> Option<char> {
        self.rest.chars().next()
    }

    fn eof(&self) -> bool {
        self.rest.is_empty()
    }

    fn len(&self) -> usize {
        self.rest.len()
    }

    fn bytes(&self) -> Bytes<'a> {
        self.rest.bytes()
    }

    fn as_bytes(&self) -> &'a [u8] {
        self.rest.as_bytes()
    }

    fn chars(&self) -> Chars<'a> {
        self.rest.chars()
    }
}

pub struct Ident {
    sym: String,
    span: Span,
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.sym)
    }
}

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Ident").field(&self.sym).finish()
    }
}

pub struct Literal {
    value: String,
    span: Span,
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.value, f)
    }
}

impl fmt::Debug for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Literal").field(&self.value).finish()
    }
}

pub struct Number {
    value: f64,
    span: Span,
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.value, f)
    }
}

impl fmt::Debug for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Number").field(&self.value).finish()
    }
}

#[derive(Debug)]
pub enum Spacing {
    Joined,
    Alone,
}

pub struct Punct {
    ch: char,
    spacing: Spacing,
    span: Span,
}

impl fmt::Debug for Punct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Punct")
            .field(&self.ch)
            .field(&self.spacing)
            .finish()
    }
}

impl Punct {
    pub fn new(c: char, spacing: Spacing, span: Span) -> Self {
        Self {
            ch: c,
            spacing,
            span,
        }
    }
}

impl fmt::Display for Punct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.ch, f)
    }
}

#[derive(Debug)]
enum Delimiter {
    /// `( ... )`
    Parenthesis,
    /// `[ ... ]`
    Bracket,
}

pub enum Token {
    Ident(Ident),
    Literal(Literal),
    Number(Number),
    Punct(Punct),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(ident) => fmt::Display::fmt(ident, f),
            Token::Literal(literal) => fmt::Display::fmt(literal, f),
            Token::Number(number) => fmt::Display::fmt(number, f),
            Token::Punct(punct) => fmt::Display::fmt(punct, f),
        }
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(ident) => fmt::Debug::fmt(ident, f),
            Token::Literal(literal) => fmt::Debug::fmt(literal, f),
            Token::Number(number) => fmt::Debug::fmt(number, f),
            Token::Punct(punct) => fmt::Debug::fmt(punct, f),
        }
    }
}

pub struct Tokens {
    tokens: Vec<Token>,
}

impl IntoIterator for Tokens {
    type Item = Token;
    type IntoIter = std::vec::IntoIter<Token>;

    fn into_iter(self) -> Self::IntoIter {
        self.tokens.into_iter()
    }
}

impl Tokens {
    pub fn len(&self) -> usize {
        self.tokens.len()
    }
}

impl fmt::Debug for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Tokens").field(&self.tokens).finish()
    }
}

impl FromStr for Tokens {
    type Err = LexError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        lex_expr(LexCursor::for_str(input))
    }
}

fn lex_expr(mut input: LexCursor) -> Result<Tokens, LexError> {
    let mut tokens: Vec<Token> = vec![];

    while !input.eof() {
        input = skip_whitespace(&input);

        if let Ok((cursor, punct)) = lex_number(&input) {
            tokens.push(Token::Number(punct));
            input = cursor;
        } else if let Ok((cursor, punct)) = lex_punct(&input) {
            tokens.push(Token::Punct(punct));
            input = cursor;
        } else if let Ok((cursor, ident)) = lex_ncname(&input) {
            tokens.push(Token::Ident(ident));
            input = cursor;
        } else if let Ok((cursor, literal)) = lex_literal(&input) {
            tokens.push(Token::Literal(literal));
            input = cursor;
        } else if input.eof() {
            break;
        } else {
            panic!("error: {:?}", input);
        }
    }

    Ok(Tokens { tokens })
}

fn skip_whitespace<'a>(input: &LexCursor<'a>) -> LexCursor<'a> {
    let len = input
        .bytes()
        .take_while(|&ch| characters::is_whitespace_byte(ch))
        .count();
    if len > 0 {
        input.advance(len)
    } else {
        input.clone()
    }
}

fn lex_punct<'a>(input: &LexCursor<'a>) -> Result<(LexCursor<'a>, Punct), Reject> {
    if let Some(ch) = input.next_byte() {
        if characters::is_punct_char(ch) {
            let cursor = input.advance(1);
            let spacing = if cursor
                .next_byte()
                .filter(|&ch| characters::is_punct_char(ch))
                .is_some()
            {
                Spacing::Joined
            } else {
                Spacing::Alone
            };
            return Ok((
                cursor,
                Punct::new(
                    ch as char,
                    spacing,
                    Span::new(input.offset, input.offset + 1),
                ),
            ));
        }
    }

    Err(Reject)
}

fn lex_ncname<'a>(input: &LexCursor<'a>) -> Result<(LexCursor<'a>, Ident), Reject> {
    let mut buffer = String::new();
    let mut chars = input.chars();

    match chars.next() {
        Some(ch) if ch != ':' && characters::is_name_start_char(ch) => {
            buffer.push(ch);
        }
        _ => return Err(Reject),
    }

    for ch in chars {
        if ch == ':' || !characters::is_name_continue_char(ch) {
            break;
        } else {
            buffer.push(ch);
        }
    }

    let len = buffer.as_bytes().len();
    Ok((
        input.advance(len),
        Ident {
            sym: buffer,
            span: Span::new(input.offset, input.offset + len),
        },
    ))
}

fn lex_literal<'a>(input: &LexCursor<'a>) -> Result<(LexCursor<'a>, Literal), Reject> {
    let mut chars = input.chars();

    let literal: String = match chars.next() {
        Some('\'') => chars.take_while(|&ch| ch != '\'').collect(),
        Some('\"') => chars.take_while(|&ch| ch != '\"').collect(),
        _ => return Err(Reject),
    };

    let len = literal.as_bytes().len() + 2;
    Ok((
        input.advance(len),
        Literal {
            value: literal,
            span: Span::new(input.offset, input.offset + len),
        },
    ))
}

fn lex_number<'a>(input: &LexCursor<'a>) -> Result<(LexCursor<'a>, Number), Reject> {
    let mut chars = input.bytes();

    let mut pre: usize = 0;
    let mut mid: Option<u8> = None;
    for ch in &mut chars {
        if ch.is_ascii_digit() {
            pre += 1;
        } else {
            mid = Some(ch);
            break;
        }
    }

    let mut dot: usize = 0;
    let mut suf: usize = 0;
    if let Some(b'.') = mid {
        dot += 1;
        for ch in &mut chars {
            if ch.is_ascii_digit() {
                suf += 1;
            } else {
                break;
            }
        }
    }

    if pre == 0 && suf == 0 {
        return Err(Reject);
    }

    let len = pre + dot + suf;
    let (span, cursor) = input.advance_to(len);

    let value: f64 = if pre == 0 {
        // .000 syntax
        format!("0{}", span).parse().unwrap()
    } else {
        span.parse().unwrap()
    };

    Ok((
        cursor,
        Number {
            value,
            span: Span::new(input.offset, input.offset + len),
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::token::{lex_expr, LexCursor, Tokens};

    #[test]
    fn it_works() {
        let test = "array:append([],\"3\",.0,1.0)";
        let tokens = lex_expr(LexCursor {
            rest: test,
            offset: 0,
        })
        .unwrap();
        println!("{:?}", tokens);
    }

    fn assert_tokens(actual: Tokens, expected: &[&'static str]) {
        let mut iter = actual.into_iter();
        for expected_token in expected {
            let token = iter.next().map(|x| x.to_string());
            assert_eq!(Some(*expected_token), token.as_ref().map(|x| x as &str));
        }
        let token = iter.next().map(|x| x.to_string());
        assert_eq!(None, token);
    }

    #[test]
    fn tokenize_ident_1() {
        assert_tokens("azAZ190_".parse().unwrap(), &["azAZ190_"]);
    }

    #[test]
    fn tokenize_ident_underline() {
        assert_tokens("_1".parse().unwrap(), &["_1"]);
    }

    #[test]
    fn tokenize_number_full() {
        assert_tokens("1.2".parse().unwrap(), &["1.2"]);
    }

    #[test]
    fn tokenize_integer() {
        assert_tokens("123456789".parse().unwrap(), &["123456789"]);
    }

    #[test]
    fn tokenize_number_short() {
        assert_tokens(".123456789".parse().unwrap(), &["0.123456789"]);
    }

    #[test]
    fn tokenize_literal_single_quote() {
        assert_tokens("'abc'".parse().unwrap(), &["\"abc\""]);
    }

    #[test]
    fn tokenize_literal_double_quote() {
        assert_tokens("\"abc\"".parse().unwrap(), &["\"abc\""]);
    }

    #[test]
    fn tokenize_punct() {
        assert_tokens(
            ".,()[]@:".parse().unwrap(),
            &[".", ",", "(", ")", "[", "]", "@", ":"],
        );
    }

    #[test]
    fn tokenize_function() {
        assert_tokens(
            "array:append([],\"3\")".parse().unwrap(),
            &["array", ":", "append", "(", "[", "]", ",", "\"3\"", ")"],
        );
    }

    #[test]
    fn tokenize_skip_whitespace() {
        assert_tokens(
            " \t1\n\r + 2\t\t\t-6 \r\n  ".parse().unwrap(),
            &["1", "+", "2", "-", "6"],
        );
    }
}
