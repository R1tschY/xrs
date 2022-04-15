use crate::lex::{CloseParenthesisToken, OpenParenthesisToken, Token, TokenCursor};
use crate::parser::expr::{BinOp, Expr, ExprBinary, ExprUnary, FunctionCall, UnaryOp};
use crate::{Span, XPathError};

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
    pub fn of(op: &Operator) -> Self {
        match op {
            Operator::Binary(op) => Precedence::of_bin_op(op),
            Operator::Unary(_) => Precedence::Unary,
            _ => unreachable!(),
        }
    }

    pub fn of_bin_op(op: &BinOp) -> Self {
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
#[derive(Clone, Debug)]
enum Operator {
    Binary(BinOp),
    Unary(UnaryOp),
    Function(String),
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

    fn parse(mut self) -> Result<Expr, XPathError> {
        self.operators.push(Operator::Sentinel);
        self.e()?;
        if let Some(token) = self.cur.next() {
            Err(XPathError::syntax(
                format!("not parsed till end: {:?}", &self.cur),
                token.span(),
            ))
        } else {
            Ok(self.operands.into_iter().last().unwrap())
        }
    }

    fn e(&mut self) -> Result<(), XPathError> {
        self.p()?;

        while let Some((bin_op, cursor)) = try_consume_binary_op(&self.cur) {
            self.push_operator(Operator::Binary(bin_op))?;
            self.cur = cursor;
            self.p()?;
        }

        while !matches!(self.operators.last(), Some(Operator::Sentinel)) {
            self.pop_operator()?;
        }

        Ok(())
    }

    fn p(&mut self) -> Result<(), XPathError> {
        if let Some(next) = self.cur.next() {
            match next {
                Token::Ident(ident) => {
                    let cur = self.cur.consume_first();
                    if let Some(_) = cur.try_consume::<OpenParenthesisToken>() {
                        // function call
                        self.cur = cur.consume_first();
                        self.operators.push(Operator::Sentinel);

                        let args = if self.cur.try_consume::<CloseParenthesisToken>().is_none() {
                            let mut number_of_args = 0usize;

                            self.e()?;
                            number_of_args += 1;

                            while let Some(_) = self.cur.try_consume::<token!(,)>() {
                                self.cur = self.cur.consume_first();
                                self.e()?;
                                number_of_args += 1;
                            }
                            self.operands
                                .split_off(self.operands.len() - number_of_args)
                        } else {
                            vec![]
                        };

                        self.cur = self.cur.expect::<CloseParenthesisToken>();
                        self.operands.push(Expr::FunctionCall(FunctionCall {
                            ident: ident.clone(),
                            args,
                        }))
                    } else {
                        self.cur = cur;
                        self.operands.push(Expr::Ident(ident.clone()));
                    }
                    Ok(())
                }
                Token::Literal(literal) => {
                    self.operands.push(Expr::Literal(literal.clone()));
                    self.cur = self.cur.consume_first();
                    Ok(())
                }
                Token::Number(number) => {
                    self.operands.push(Expr::Number(number.clone()));
                    self.cur = self.cur.consume_first();
                    Ok(())
                }
                Token::Punct(punct) if punct.as_char() == '(' => {
                    self.cur = self.cur.consume_first();
                    self.operators.push(Operator::Sentinel);
                    self.e()?;
                    self.cur = self.cur.expect::<CloseParenthesisToken>();
                    self.operators.pop();
                    Ok(())
                }
                _ => {
                    if let Some((op, cursor)) = try_consume_unary_op(&self.cur) {
                        self.push_operator(Operator::Unary(op))?;
                        self.cur = cursor;
                        self.p()?;
                        Ok(())
                    } else {
                        Err(XPathError::syntax("Parser error", next.span()))
                    }
                }
            }
        } else {
            Ok(())
        }
    }

    fn pop_operator(&mut self) -> Result<(), XPathError> {
        let op = self
            .operators
            .pop()
            .ok_or_else(|| XPathError::syntax("stack underflow", Span::new(0, 0)))?;
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
        Ok(())
    }

    fn push_operator(&mut self, operator: Operator) -> Result<(), XPathError> {
        while compare_ops(self.operators.last().unwrap(), &operator) {
            self.pop_operator()?;
        }
        self.operators.push(operator);
        Ok(())
    }
}

fn compare_ops(left: &Operator, right: &Operator) -> bool {
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
    } else if let Some(cursor) = cursor.try_consume::<token!(-)>() {
        Some((BinOp::Sub, cursor))
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

    fn parse(input: &str) -> Result<String, XPathError> {
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
    fn unary_5() {
        assert_eq!(Ok("(1 - -1)".to_string()), parse("1 - -1"),);
    }

    #[test]
    fn unary_6() {
        assert_eq!(Ok("-1".to_string()), parse("- 1"),);
    }

    #[test]
    fn unary_7() {
        assert_eq!(Ok("(1 - -1)".to_string()), parse("1 - - 1"),);
    }

    #[test]
    fn number() {
        assert_eq!(Ok("1".to_string()), parse("1"),);
    }

    mod functions {
        use super::*;

        #[test]
        fn args() {
            assert_eq!(Ok("max(1, 2, 3)".to_string()), parse("max(1, 2, 3)"),);
        }

        #[test]
        fn arg() {
            assert_eq!(Ok("max(1)".to_string()), parse("max(1)"),);
        }

        #[test]
        fn no_arg() {
            assert_eq!(Ok("max()".to_string()), parse("max()"),);
        }
    }
}
