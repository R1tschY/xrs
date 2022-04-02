#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Span {
    lo: usize,
    hi: usize,
}

impl Span {
    pub(crate) fn new(lo: usize, hi: usize) -> Self {
        Self { lo, hi }
    }
}
