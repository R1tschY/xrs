use std::borrow::Cow;

pub(crate) trait CowStrHelpers<'a> {
    fn push_str(&mut self, string: &'a str);
}

impl<'a> CowStrHelpers<'a> for Cow<'a, str> {
    fn push_str(&mut self, string: &'a str) {
        match self {
            Cow::Borrowed(borrowed) => {
                let mut res = String::with_capacity(borrowed.len() + string.len());
                res.push_str(borrowed);
                res.push_str(&string);
                *self = Cow::Owned(res);
            }
            Cow::Owned(owned) if owned.is_empty() => *self = Cow::Borrowed(string),
            Cow::Owned(owned) => owned.push_str(&string),
        }
    }
}
