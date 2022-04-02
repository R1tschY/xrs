use crate::context::Context;
use crate::object::Object;
use crate::parser::{BinOp, Expr, ExprBinary};
use crate::{XPath, XPathError};

pub trait Selector {
    fn select<'i, 't>(&self, ctx: &Context<'i, 't>) -> Result<Object<'i, 't>, XPathError>;
}

impl Selector for XPath {
    fn select<'i, 't>(&self, ctx: &Context<'i, 't>) -> Result<Object<'i, 't>, XPathError> {
        self.expr.select(ctx)
    }
}

impl Selector for Expr {
    fn select<'i, 't>(&self, _ctx: &Context<'i, 't>) -> Result<Object<'i, 't>, XPathError> {
        match self {
            Expr::Binary(_) => todo!(),
            Expr::Unary(_) => todo!(),
            Expr::Ident(_) => todo!(),
            Expr::Literal(string) => Ok(Object::String(string.value.clone().into())),
            Expr::Number(number) => Ok(Object::Number(number.value)),
        }
    }
}

impl Selector for ExprBinary {
    fn select<'i, 't>(&self, ctx: &Context<'i, 't>) -> Result<Object<'i, 't>, XPathError> {
        match self.op {
            BinOp::Or => Ok(Object::Boolean(
                bool::from(self.left.select(ctx)?) || bool::from(self.right.select(ctx)?),
            )),
            BinOp::And => Ok(Object::Boolean(
                bool::from(self.left.select(ctx)?) && bool::from(self.right.select(ctx)?),
            )),
            BinOp::Equal => todo!(),
            BinOp::NotEqual => todo!(),
            BinOp::Less => todo!(),
            BinOp::Greater => todo!(),
            BinOp::LessEqual => todo!(),
            BinOp::GreaterEqual => todo!(),
            BinOp::Add => todo!(),
            BinOp::Sub => todo!(),
            BinOp::Multiply => todo!(),
            BinOp::Divide => todo!(),
            BinOp::Modulo => todo!(),
            BinOp::Path => todo!(),
            BinOp::RecursivePath => todo!(),
        }
    }
}
