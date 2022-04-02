use crate::lex::span::Span;

#[derive(Debug)]
pub struct LexError {
    span: Span,
}
