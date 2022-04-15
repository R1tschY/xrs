#![allow(dead_code)]

use crate::lex::Span;
use crate::parser::Expr;

macro_rules! token {
    (*) => {
        $crate::lex::StarToken
    };
    (+) => {
        $crate::lex::PlusToken
    };
    (-) => {
        $crate::lex::MinusToken
    };
    (.) => {
        $crate::lex::DotToken
    };
    (..) => {
        $crate::lex::DotDotToken
    };
    (@) => {
        $crate::lex::AtToken
    };
    (,) => {
        $crate::lex::CommaToken
    };
    (:) => {
        $crate::lex::ColonToken
    };
    (::) => {
        $crate::lex::ColonColonToken
    };
    (/) => {
        $crate::lex::SlashToken
    };
    (|) => {
        $crate::lex::PipeToken
    };
    ($) => {
        $crate::lex::DollarToken
    };
    (=) => {
        $crate::lex::EqualToken
    };
    (!=) => {
        $crate::lex::NotEqualToken
    };
    (<) => {
        $crate::lex::LessToken
    };
    (<=) => {
        $crate::lex::LessEqualToken
    };
    (>) => {
        $crate::lex::GreaterToken
    };
    (>=) => {
        $crate::lex::GreaterEqualToken
    };
    (and) => {
        $crate::lex::AndToken
    };
    (or) => {
        $crate::lex::OrToken
    };
    (mod) => {
        $crate::lex::ModToken
    };
    (div) => {
        $crate::lex::DivToken
    };
}

mod context;
mod datamodel;
mod functions;
mod lex;
mod object;
mod parser;
mod select;

mod utils;

#[derive(Debug, PartialEq, Clone)]
pub struct SyntaxError {
    message: String,
    span: Span,
}

#[derive(Debug, PartialEq, Clone)]
pub enum XPathError {
    WrongFunctionArgument(String),
    CallToUndefinedFunction(String),
    SyntaxError(SyntaxError),
}

impl XPathError {
    pub fn syntax(message: impl Into<String>, span: Span) -> XPathError {
        XPathError::SyntaxError(SyntaxError {
            message: message.into(),
            span,
        })
    }
}

struct XPath {
    expr: Expr,
}
