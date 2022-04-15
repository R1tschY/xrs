use std::fmt::{Display, Formatter};

use crate::lex::{Ident, Literal, Number};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BinOp {
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

impl Display for BinOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            BinOp::Or => "or",
            BinOp::And => "and",
            BinOp::Equal => "=",
            BinOp::NotEqual => "!=",
            BinOp::Less => "<",
            BinOp::Greater => ">",
            BinOp::LessEqual => "<=",
            BinOp::GreaterEqual => ">=",
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Multiply => "*",
            BinOp::Divide => "div",
            BinOp::Modulo => "mod",
            BinOp::Path => "/",
            BinOp::RecursivePath => "//",
        })
    }
}

#[derive(Debug)]
pub struct ExprBinary {
    pub left: Box<Expr>,
    pub op: BinOp,
    pub right: Box<Expr>,
}

impl Display for ExprBinary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("({} {} {})", self.left, self.op, self.right))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum UnaryOp {
    Negative,
}

impl Display for UnaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            UnaryOp::Negative => "-",
        })
    }
}

#[derive(Debug)]
pub struct ExprUnary {
    pub op: UnaryOp,
    pub right: Box<Expr>,
}

impl Display for ExprUnary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}{}", self.op, self.right))
    }
}

#[derive(Debug)]
pub struct FunctionCall {
    pub ident: Ident,
    pub args: Vec<Expr>,
}

impl Display for FunctionCall {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}({})",
            self.ident,
            self.args
                .iter()
                .map(|arg| arg.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        ))
    }
}

#[derive(Debug)]
pub enum Expr {
    Binary(ExprBinary),
    Unary(ExprUnary),
    Ident(Ident),
    Literal(Literal),
    Number(Number),
    FunctionCall(FunctionCall),
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Binary(expr) => Display::fmt(expr, f),
            Expr::Unary(expr) => Display::fmt(expr, f),
            Expr::Ident(ident) => Display::fmt(ident, f),
            Expr::Literal(lit) => Display::fmt(lit, f),
            Expr::Number(num) => Display::fmt(num, f),
            Expr::FunctionCall(func_call) => Display::fmt(func_call, f),
        }
    }
}

/// only internal
#[derive(Copy, Clone, Debug, PartialEq)]
enum Operator {
    Binary(BinOp),
    Unary(UnaryOp),
    Sentinel,
}

enum Axis {
    Ancestor,
    AncestorOrSelf,
    Attribute,
    Child,
    Descendant,
    DescendantOrSelf,
    Following,
    FollowingSibling,
    Namespace,
    Parent,
    Preceding,
    PrecedingSibling,
    Self_,
}

enum NodeTest {
    Name(String),
    AnyName,
    AnyNameInNamespace(String),
    Comment,
    Text,
    ProcessingInstruction(Option<String>),
    Node,
}

struct Predicate {}

struct LocationStep {
    axis: Axis,
    node_test: NodeTest,
    predicates: Vec<Predicate>,
}
