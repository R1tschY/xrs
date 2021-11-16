use crate::token::{Token, Tokens};
use std::panic::resume_unwind;

#[derive(Clone, Copy)]
pub struct Cursor<'a> {
    rest: &'a [Token],
}

impl<'a> Cursor<'a> {
    fn next(&self) -> Option<&Token> {
        self.rest.split_first().map(|x| x.0)
    }

    fn consume(&self) -> Self {
        if let Some((_, rest)) = self.rest.split_first() {
            Self { rest }
        } else {
            self.clone()
        }
    }

    fn error(&mut self) {}
}

pub struct ParseBuffer<'a> {
    cur: Cursor<'a>,
}

pub struct ParseError;

pub trait Parser: Sized {
    fn parse(cur: &mut ParseBuffer) -> Result<Self, ParseError>;
}

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
    Binary(ExprBinary),
}
