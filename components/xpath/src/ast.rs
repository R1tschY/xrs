use crate::token::{Ident, Literal, Number, Punct, Token, Tokens};
use syn::token::parsing::punct;

macro_rules! token {
    (*) => {
        $crate::ast::StarToken
    };
    (+) => {
        $crate::ast::PlusToken
    };
    (-) => {
        $crate::ast::MinusToken
    };
    (.) => {
        $crate::ast::DotToken
    };
    (..) => {
        $crate::ast::DotDotToken
    };
    (@) => {
        $crate::ast::AtToken
    };
    (,) => {
        $crate::ast::CommaToken
    };
    (:) => {
        $crate::ast::ColonToken
    };
    (::) => {
        $crate::ast::ColonColonToken
    };
    (/) => {
        $crate::ast::SlashToken
    };
    (|) => {
        $crate::ast::PipeToken
    };
    ($) => {
        $crate::ast::DollarToken
    };
    (=) => {
        $crate::ast::EqualToken
    };
    (!=) => {
        $crate::ast::NotEqualToken
    };
    (<) => {
        $crate::ast::LessToken
    };
    (<=) => {
        $crate::ast::LessEqualToken
    };
    (>) => {
        $crate::ast::GreaterToken
    };
    (>=) => {
        $crate::ast::GreaterEqualToken
    };
    (and) => {
        $crate::ast::AndToken
    };
    (or) => {
        $crate::ast::OrToken
    };
    (mod) => {
        $crate::ast::ModToken
    };
    (div) => {
        $crate::ast::DivToken
    };
}

#[derive(Clone, Copy, Debug)]
pub struct Cursor<'a> {
    rest: &'a [Token],
}

impl<'a> Cursor<'a> {
    pub fn new(tokens: &'a Tokens) -> Self {
        Self {
            rest: tokens.as_slice(),
        }
    }

    fn next(&self) -> Option<&Token> {
        self.rest.get(0)
    }

    fn consume_first(&self) -> Self {
        if let Some((_, rest)) = self.rest.split_first() {
            Self { rest }
        } else {
            self.clone()
        }
    }

    fn consume(&self, n: usize) -> Self {
        let (_, rest) = self.rest.split_at(n);
        Self { rest }
    }

    fn error(&mut self) {
        panic!("parser error")
    }

    fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }

    fn punct(&'a self) -> Option<&'a Punct> {
        match self.next() {
            Some(Token::Punct(punct)) => Some(punct),
            _ => None,
        }
    }

    fn ident(&'a self) -> Option<&'a Ident> {
        match self.next() {
            Some(Token::Ident(ident)) => Some(ident),
            _ => None,
        }
    }

    fn try_consume<T: ParserToken>(&self) -> Option<Self> {
        if T::peek(self) {
            Some(self.consume(T::len()))
        } else {
            None
        }
    }

    fn expect<T: ParserToken>(&self) -> Self {
        if T::peek(self) {
            self.consume(T::len())
        } else {
            panic!("parser error: expected {}", T::display())
        }
    }
}

pub struct ParseBuffer<'a> {
    cur: Cursor<'a>,
}

#[derive(Debug)]
pub struct ParseError;

pub trait Parse: Sized {
    fn parse(cur: &mut ParseBuffer) -> Result<Self, ParseError>;
}

pub struct Ast {
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

impl Precedence {
    pub fn of(op: Operator) -> Self {
        match op {
            Operator::Binary(op) => Precedence::of_bin_op(op),
            Operator::Unary(_) => Precedence::Unary,
            Operator::Sentinel => unreachable!(),
        }
    }

    pub fn of_bin_op(op: BinOp) -> Self {
        match op {
            BinOp::Or => Precedence::Or,
            BinOp::And => Precedence::And,
            BinOp::Equal => Precedence::Equality,
            BinOp::NotEqual => Precedence::Equality,
            BinOp::Less => Precedence::Relational,
            BinOp::Greater => Precedence::Relational,
            BinOp::LessEqual => Precedence::Relational,
            BinOp::GreaterEqual => Precedence::Relational,
            BinOp::Add => Precedence::Additive,
            BinOp::Sub => Precedence::Additive,
            BinOp::Multiply => Precedence::Multiplicative,
            BinOp::Divide => Precedence::Multiplicative,
            BinOp::Modulo => Precedence::Multiplicative,
            BinOp::Path => Precedence::Path,
            BinOp::RecursivePath => Precedence::Path,
        }
    }
}

#[derive(Copy, Clone, Debug)]
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

#[derive(Debug)]
pub struct ExprBinary {
    left: Box<Expr>,
    op: BinOp,
    right: Box<Expr>,
}

#[derive(Copy, Clone, Debug)]
pub enum UnaryOp {
    Negative,
}

#[derive(Debug)]
pub struct ExprUnary {
    op: UnaryOp,
    right: Box<Expr>,
}

#[derive(Debug)]
pub enum Expr {
    Binary(ExprBinary),
    Unary(ExprUnary),
    Ident(Ident),
    Literal(Literal),
    Number(Number),
}

/// only internal
#[derive(Copy, Clone, Debug)]
enum Operator {
    Binary(BinOp),
    Unary(UnaryOp),
    Sentinel,
}

/// a shunting yard parser
///
/// see https://www.engr.mun.ca/~theo/Misc/exp_parsing.htm
struct ShuntingYardParser<'a> {
    cur: Cursor<'a>,
    operands: Vec<Expr>,
    operators: Vec<Operator>,
}

impl<'a> ShuntingYardParser<'a> {
    fn new(cur: Cursor<'a>) -> Self {
        Self {
            cur,
            operands: vec![],
            operators: vec![],
        }
    }

    fn parse(mut self) -> Result<Expr, ParseError> {
        self.operators.push(Operator::Sentinel);
        self.e();
        if !self.cur.is_empty() {
            panic!("not parsed till end: {:?}", &self.cur);
        }
        Ok(self.operands.into_iter().last().unwrap())
    }

    fn e(&mut self) {
        self.p();

        while let Some((bin_op, cursor)) = try_consume_binary_op(&self.cur) {
            self.push_operator(Operator::Binary(bin_op));
            self.cur = cursor;
            self.p();
        }

        while !matches!(self.operators.last(), Some(Operator::Sentinel)) {
            self.pop_operator();
        }
    }

    fn p(&mut self) {
        let next = self.cur.next();
        if let Some(next) = next {
            match next {
                Token::Ident(ident) => {
                    self.operands.push(Expr::Ident(ident.clone()));
                    self.cur = self.cur.consume_first();
                }
                Token::Literal(literal) => {
                    self.operands.push(Expr::Literal(literal.clone()));
                    self.cur = self.cur.consume_first();
                }
                Token::Number(number) => {
                    self.operands.push(Expr::Number(number.clone()));
                    self.cur = self.cur.consume_first();
                }
                Token::Punct(punct) if punct.as_char() == '(' => {
                    self.cur = self.cur.consume_first();
                    self.operators.push(Operator::Sentinel);
                    self.e();
                    self.cur.expect::<CloseParenthesisToken>();
                    self.operators.pop();
                }
                _ => {
                    if let Some((op, cursor)) = try_consume_unary_op(&self.cur) {
                        self.push_operator(Operator::Unary(op));
                        self.cur = cursor;
                        self.p();
                    } else {
                        panic!("parser error");
                    }
                }
            }
        }
    }

    fn pop_operator(&mut self) {
        let op = self.operators.pop().expect("stack underflow");
        if let Operator::Binary(bin_op) = op {
            let t1 = self.operands.pop().unwrap();
            let t0 = self.operands.pop().unwrap();
            let expr = Expr::Binary(ExprBinary {
                left: Box::new(t0),
                op: bin_op,
                right: Box::new(t1),
            });
            self.operands.push(expr);
        } else if let Operator::Unary(unary_op) = op {
            let expr = Expr::Unary(ExprUnary {
                op: unary_op,
                right: Box::new(self.operands.pop().unwrap()),
            });
            self.operands.push(expr);
        } else {
            unreachable!();
        }
    }

    fn push_operator(&mut self, operator: Operator) {
        while compare_ops(*self.operators.last().unwrap(), operator.clone()) {
            self.pop_operator();
        }
        self.operators.push(operator);
    }
}

fn compare_ops(left: Operator, right: Operator) -> bool {
    match (left, right) {
        (Operator::Binary(x), Operator::Binary(y)) => {
            Precedence::of_bin_op(x) >= Precedence::of_bin_op(y)
        }
        (Operator::Unary(_), Operator::Binary(y)) => Precedence::Unary >= Precedence::of_bin_op(y),
        (_, Operator::Unary(_)) => false,
        (Operator::Sentinel, _) => false,
        _ => unreachable!(),
    }
}

fn try_consume_binary_op<'a>(cursor: &Cursor<'a>) -> Option<(BinOp, Cursor<'a>)> {
    if let Some(cursor) = cursor.try_consume::<token!(+)>() {
        Some((BinOp::Add, cursor))
    } else if let Some(cursor) = cursor.try_consume::<token!(*)>() {
        Some((BinOp::Multiply, cursor))
    } else {
        None
    }
}

fn try_consume_unary_op<'a>(cursor: &Cursor<'a>) -> Option<(UnaryOp, Cursor<'a>)> {
    if let Some(cursor) = cursor.try_consume::<token!(-)>() {
        Some((UnaryOp::Negative, cursor))
    } else {
        None
    }
}

trait ParserToken {
    fn peek(cursor: &Cursor) -> bool;
    fn len() -> usize;
    fn display() -> &'static str;
}

macro_rules! def_punct_struct {
    ( $token:tt pub struct $name:ident[$len:tt] #[$doc:meta] ) => {
        #[$doc]
        pub struct $name {
            spans: [$crate::Span; $len],
        }
    };
}

macro_rules! def_single_punct {
    ( $( $str:tt/$char:tt pub struct $name:ident #[$doc:meta] )* ) => {
        $(
            def_punct_struct!($str pub struct $name[1] #[$doc]);

            impl ParserToken for $name {
                fn peek(cursor: &Cursor) -> bool {
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
                fn peek(cursor: &Cursor) -> bool {
                    if matches!(cursor.punct(), Some(punct) if punct.as_char() == $char1 && punct.spacing() == $crate::token::Spacing::Joined) {
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
                fn peek(cursor: &Cursor) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let tokens: Tokens = "x*y+z".parse().unwrap();
        let parser = ShuntingYardParser::new(Cursor::new(&tokens));
        println!("{:#?}", parser.parse().unwrap());
    }
}
