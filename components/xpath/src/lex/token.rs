#![allow(dead_code)]

use std::fmt;
use std::fmt::Debug;

use crate::lex::span::Span;
use crate::lex::TokenCursor;

#[derive(Clone)]
pub struct Ident {
    sym: String,
    span: Span,
}

impl Ident {
    pub fn new(sym: String, span: Span) -> Self {
        Self { sym, span }
    }

    pub fn as_str(&self) -> &str {
        &self.sym
    }
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

#[derive(Clone)]
pub struct Literal {
    pub value: String,
    pub span: Span,
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

#[derive(Clone)]
pub struct Number {
    pub value: f64,
    pub span: Span,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Spacing {
    Joined,
    Alone,
}

#[derive(Clone)]
pub struct Punct {
    pub ch: char,
    pub spacing: Spacing,
    pub span: Span,
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

    pub fn as_char(&self) -> char {
        self.ch
    }

    pub fn spacing(&self) -> Spacing {
        self.spacing
    }
}

impl fmt::Display for Punct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.ch, f)
    }
}

#[derive(Clone)]
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
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens }
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn as_slice(&self) -> &[Token] {
        self.tokens.as_slice()
    }
}

impl fmt::Debug for Tokens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Tokens").field(&self.tokens).finish()
    }
}

pub trait ParserToken {
    fn peek(cursor: &TokenCursor) -> bool;
    fn len() -> usize;
    fn display() -> &'static str;
}

macro_rules! def_punct_struct {
    ( $token:tt pub struct $name:ident[$len:tt] #[$doc:meta] ) => {
        #[$doc]
        pub struct $name {
            spans: [$crate::lex::Span; $len],
        }
    };
}

macro_rules! def_single_punct {
    ( $( $str:tt/$char:tt pub struct $name:ident #[$doc:meta] )* ) => {
        $(
            def_punct_struct!($str pub struct $name[1] #[$doc]);

            impl ParserToken for $name {
                fn peek(cursor: &TokenCursor) -> bool {
                    matches!(cursor.punct(), Some(punct) if punct.as_char() == $char)
                }

                fn len() -> usize {
                    1
                }

                fn display() -> &'static str {
                    $str
                }
            }
        )*
    };
}

def_single_punct! {
    "+"/'+' pub struct PlusToken /// `+`
    "-"/'-' pub struct MinusToken /// `-`
    "*"/'*' pub struct StarToken /// `*`
    "/"/'/' pub struct SlashToken /// `/`
    "|"/'|' pub struct PipeToken /// `|`
    "="/'=' pub struct EqualToken /// `=`
    "."/'.' pub struct DotToken /// `.`
    ","/',' pub struct CommaToken /// `,`
    "@"/'@' pub struct AtToken /// `@`
    "<"/'<' pub struct LessToken /// `<`
    ">"/'>' pub struct GreaterToken /// `>`
    "$"/'$' pub struct DollarToken /// `$`

    "("/'(' pub struct OpenParenthesisToken /// `(`
    ")"/')' pub struct CloseParenthesisToken /// `)`
    "["/'[' pub struct OpenBracketToken /// `[`
    "]"/']' pub struct CloseBracketToken /// `]`
}

macro_rules! def_double_punct {
    ( $( $str:tt/$char1:tt/$char2:tt pub struct $name:ident #[$doc:meta] )* ) => {
        $(
            def_punct_struct!($str pub struct $name[2] #[$doc]);

            impl ParserToken for $name {
                fn peek(cursor: &TokenCursor) -> bool {
                    if matches!(cursor.punct(), Some(punct) if punct.as_char() == $char1 && punct.spacing() == $crate::lex::Spacing::Joined) {
                        let cursor = cursor.consume_first();
                        if matches!(cursor.punct(), Some(punct) if punct.as_char() == $char2) {
                            return true;
                        }
                    }
                    false
                }

                fn len() -> usize {
                    2
                }

                fn display() -> &'static str {
                    $str
                }
            }
        )*
    };
}

def_double_punct! {
    "!="/'!'/'=' pub struct NotEqualToken /// `!=`
    "<="/'<'/'=' pub struct LessEqualToken /// `<=`
    ">="/'>'/'=' pub struct GreaterEqualToken /// `>=`
    "//"/'/'/'/' pub struct SlashSlashToken /// `//`
    "::"/':'/':' pub struct ColonColonPipeToken /// `::`
}

macro_rules! def_keywords {
    ( $( $str:tt pub struct $name:ident #[$doc:meta] )* ) => {
        $(
            def_punct_struct!($str pub struct $name[2] #[$doc]);

            impl ParserToken for $name {
                fn peek(cursor: &TokenCursor) -> bool {
                    matches!(cursor.ident(), Some(ident) if ident.as_str() == $str)
                }

                fn len() -> usize {
                    2
                }

                fn display() -> &'static str {
                    $str
                }
            }
        )*
    };
}

def_keywords! {
    "and" pub struct AndToken /// `and`
    "or" pub struct OrToken /// `or`
    "mod" pub struct ModToken /// `mod`
    "div" pub struct DivToken /// `div`
}
