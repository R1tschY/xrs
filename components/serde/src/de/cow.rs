use std::borrow::Cow;
use xrs_chars::XmlAsciiChar;

pub(crate) trait CowStrExt<'a> {
    fn push_str(&mut self, string: &str);
    fn trim_matches<P>(&mut self, p: P)
    where
        P: Fn(char) -> bool;
    fn tail(&mut self, i: usize);
}

fn string_trim_matches(s: &mut String, p: impl Fn(char) -> bool) {
    let trimmed = s.trim_matches(p);
    let prefix = (s.as_ptr() as usize) - (trimmed.as_ptr() as usize);
    let len = trimmed.len();
    s.truncate(prefix + len);
    s.drain(..prefix);
}

impl<'a> CowStrExt<'a> for Cow<'a, str> {
    fn push_str(&mut self, string: &str) {
        match self {
            Cow::Borrowed(borrowed) => {
                let mut res = String::with_capacity(borrowed.len() + string.len());
                res.push_str(borrowed);
                res.push_str(&string);
                *self = Cow::Owned(res);
            }
            Cow::Owned(owned) => owned.push_str(&string),
        }
    }

    fn trim_matches<P>(&mut self, p: P)
    where
        P: Fn(char) -> bool,
    {
        match self {
            Cow::Borrowed(borrowed) => {
                *self = Cow::Borrowed(borrowed.trim_matches(p));
            }
            Cow::Owned(ref mut owned) => string_trim_matches(owned, p),
        }
    }

    fn tail(&mut self, i: usize) {
        match self {
            Cow::Borrowed(borrowed) => {
                *self = Cow::Borrowed(&borrowed[i..]);
            }
            Cow::Owned(owned) => {
                owned.drain(..i);
            }
        }
    }
}

pub(crate) trait StrExt {
    fn is_xml_whitespace(&self) -> bool;
}

impl StrExt for &str {
    fn is_xml_whitespace(&self) -> bool {
        self.as_bytes()
            .iter()
            .copied()
            .all(|c: u8| c.is_xml_whitespace())
    }
}
