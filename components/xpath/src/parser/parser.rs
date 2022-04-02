use crate::lex::{CloseParenthesisToken, Token, TokenCursor};
use crate::parser::expr::{BinOp, Expr, ExprBinary, ExprUnary, UnaryOp};
use crate::parser::ParseError;

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

/// only internal
#[derive(Copy, Clone, Debug, PartialEq)]
enum Operator {
    Binary(BinOp),
    Unary(UnaryOp),
    Sentinel,
}

/// a recursive shunting yard parser
///
/// see https://www.engr.mun.ca/~theo/Misc/exp_parsing.htm
pub struct RecursiveShuntingYardParser<'a> {
    cur: TokenCursor<'a>,
    operands: Vec<Expr>,
    operators: Vec<Operator>,
}

impl<'a> RecursiveShuntingYardParser<'a> {
    fn new(cur: TokenCursor<'a>) -> Self {
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
        if let Some(next) = self.cur.next() {
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
                    self.cur = self.cur.expect::<CloseParenthesisToken>();
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
        while compare_ops(*self.operators.last().unwrap(), operator) {
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

#[allow(clippy::manual_map)]
fn try_consume_binary_op<'a>(cursor: &TokenCursor<'a>) -> Option<(BinOp, TokenCursor<'a>)> {
    if let Some(cursor) = cursor.try_consume::<token!(+)>() {
        Some((BinOp::Add, cursor))
    } else if let Some(cursor) = cursor.try_consume::<token!(*)>() {
        Some((BinOp::Multiply, cursor))
    } else {
        None
    }
}

#[allow(clippy::manual_map)]
fn try_consume_unary_op<'a>(cursor: &TokenCursor<'a>) -> Option<(UnaryOp, TokenCursor<'a>)> {
    if let Some(cursor) = cursor.try_consume::<token!(-)>() {
        Some((UnaryOp::Negative, cursor))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::lex::Tokens;

    use super::*;

    fn parse(input: &str) -> Result<String, ParseError> {
        let tokens: Tokens = input.parse().unwrap();
        let parser = RecursiveShuntingYardParser::new(TokenCursor::new(&tokens));
        parser.parse().map(|ast| ast.to_string())
    }

    #[test]
    fn op_precedence_1() {
        assert_eq!(Ok("((x * y) + z)".to_string()), parse("x*y+z"),);
    }

    #[test]
    fn op_precedence_2() {
        assert_eq!(
            Ok("((a + ((b * c) * d)) + e)".to_string()),
            parse("a + b * c * d + e"),
        );
    }

    #[test]
    fn parenthesis_1() {
        assert_eq!(Ok("(x * (y + z))".to_string()), parse("x*(y+z)"),);
    }

    #[test]
    fn parenthesis_2() {
        assert_eq!(Ok("1".to_string()), parse("((((1))))"),);
    }

    #[test]
    fn unary_1() {
        assert_eq!(Ok("-1".to_string()), parse("-1"),);
    }

    #[test]
    fn unary_2() {
        assert_eq!(Ok("(-1 + 1)".to_string()), parse("-1 + 1"),);
    }

    #[test]
    fn unary_3() {
        assert_eq!(Ok("-(1 + 1)".to_string()), parse("-(1 + 1)"),);
    }

    #[test]
    fn unary_4() {
        assert_eq!(Ok("---1".to_string()), parse("-(-(-1))"),);
    }

    #[test]
    fn number() {
        assert_eq!(Ok("1".to_string()), parse("1"),);
    }
}
