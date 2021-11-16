use core::fmt;
use std::cmp::Ordering::{Equal, Greater, Less};
use std::fmt::{Formatter, Write};
use std::str::{Bytes, Chars, FromStr};
use std::vec;

mod characters;
mod tokenize;

trait Cursor: Clone {
    fn next(&self) -> Option<char>;
    fn consume(&mut self);
    fn error(&mut self);
}

trait Parse: Sized {
    fn parse<T: Cursor>(input: &mut T) -> Option<Self>;
}

#[derive(Clone)]
struct IteratorCursor<T: Iterator<Item = char> + Clone + 'static> {
    iterator: T,
    next: Option<char>,
}

impl<T: Iterator<Item = char> + Clone + 'static> Cursor for IteratorCursor<T> {
    fn next(&self) -> Option<char> {
        self.next
    }

    fn consume(&mut self) {
        self.next = self.iterator.next();
    }

    fn error(&mut self) {
        panic!("parser error");
    }
}

#[derive(Debug)]
pub struct Span {
    lo: usize,
    hi: usize,
}

impl Span {
    pub(crate) fn new(lo: usize, hi: usize) -> Self {
        Self { lo, hi }
    }
}

#[derive(Debug)]
pub struct LexError {
    span: Span,
}

enum XPathToken {}

struct XPathLexer {}

struct Ast {
    expr: Expr,
}

#[derive(Eq, PartialEq, Ord, PartialOrd, Debug)]
enum Precedence {
    Or,
    And,
    Equality,
    Relational,
    Additive,
    Multiplicative,
    Unary,
    Union,
    Path,
}

enum BinOp {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Add,
    Sub,
    Multiply,
    Divide,
    Modulo,
    Path,
    RecursivePath,
}

struct ExprBinary {
    left: Box<Expr>,
    op: BinOp,
    right: Box<Expr>,
}

enum UnaryOp {
    Negative,
}

struct ExprUnary {
    op: UnaryOp,
    right: Box<Expr>,
}

enum Expr {
    Number(Number),
    Binary(ExprBinary),
}

// impl Parse for Number {
//     fn parse<T: Cursor>(input: &mut T) -> Option<Self> {
//         loop {
//             let mut res: i32 = 0;
//             let save = input.clone();
//             if let Some(c) = input.next() {
//                 if c.is_ascii_digit() {
//                     res = res * 10 + i32::from_str(&c.to_string()).unwrap();
//                 } else {
//                     *input = save;
//                     return Some(Number { value: res });
//                 }
//             } else {
//                 return Some(Number { value: res });
//                 s
//             }
//         }
//     }
// }

struct Reject;

#[derive(Copy, Clone, Debug)]
pub struct LexCursor<'a> {
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
    type IntoIter = vec::IntoIter<Token>;

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
        tokenize::lex_expr(LexCursor::for_str(input))
    }
}

#[cfg(test)]
mod tests {
    use crate::tokenize::lex_expr;
    use crate::{LexCursor, Tokens};

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
        assert_tokens("id".parse().unwrap(), &["id"]);
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
            ".,()[]@".parse().unwrap(),
            &[".", ",", "(", ")", "[", "]", "@"],
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
            " \t1 + 2\t\t\t-6   ".parse().unwrap(),
            &["1", "+", "2", "-", "6"],
        );
    }
}
