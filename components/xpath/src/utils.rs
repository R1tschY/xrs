use std::borrow::Cow;

pub(crate) trait CowStrHelpers {
    fn push_str(&mut self, string: &str);
}

impl<'a> CowStrHelpers for Cow<'a, str> {
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
}
