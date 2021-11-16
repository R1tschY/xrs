mod ast;
mod characters;
mod token;

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
