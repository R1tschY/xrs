#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Cursor<'a> {
    rest: &'a str,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            rest: input,
            offset: 0,
        }
    }

    pub fn next_char(&self) -> Option<char> {
        self.rest.chars().next()
    }

    pub fn next_byte(&self, i: usize) -> Option<u8> {
        self.rest.as_bytes().get(i).copied()
    }

    #[inline]
    pub fn has_next_char(&self, pat: char) -> bool {
        self.rest.starts_with(pat)
    }

    #[inline]
    pub fn has_next_byte(&self, pat: u8) -> bool {
        self.rest.as_bytes().get(0) == Some(&pat)
    }

    #[inline]
    pub fn has_next_str(&self, pat: impl AsRef<str>) -> bool {
        self.rest.starts_with(pat.as_ref())
    }

    #[inline]
    pub fn has_next_bytes(&self, pat: impl AsRef<[u8]>) -> bool {
        self.rest.as_bytes().starts_with(pat.as_ref())
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn rest(&self) -> &'a str {
        self.rest
    }

    pub fn rest_bytes(&self) -> &'a [u8] {
        self.rest.as_bytes()
    }

    pub fn is_at_end(&self) -> bool {
        self.rest.is_empty()
    }

    pub fn advance(&self, bytes: usize) -> Self {
        let (_ignore, rest) = self.rest.split_at(bytes);
        println!("ADVANCE {}: {:?}", bytes, _ignore);
        Self {
            rest,
            offset: self.offset + bytes,
        }
    }

    pub fn advance2(&self, bytes: usize) -> (&'a str, Self) {
        let (diff, rest) = self.rest.split_at(bytes);
        println!("ADVANCE {}: {:?}", bytes, diff);
        (
            diff,
            Self {
                rest,
                offset: self.offset + bytes,
            },
        )
    }
}
