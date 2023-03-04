use std::borrow::Cow;

pub(crate) trait CowStrHelpers<'a> {
    fn push_borrowed_str(&mut self, string: &'a str);
    fn push_str(&mut self, string: &str);
    fn push_cow(&mut self, string: Cow<'a, str>);
}

impl<'a> CowStrHelpers<'a> for Cow<'a, str> {
    fn push_borrowed_str(&mut self, string: &'a str) {
        if string.is_empty() {
            return;
        }

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

    fn push_str(&mut self, string: &str) {
        if string.is_empty() {
            return;
        }

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

    fn push_cow(&mut self, string: Cow<'a, str>) {
        if string.is_empty() {
            return;
        }

        if self.is_empty() {
            *self = string;
            return;
        }

        match self {
            Cow::Borrowed(borrowed) => {
                let mut res = String::with_capacity(borrowed.len() + string.len());
                res.push_str(borrowed);
                res.push_str(string.as_ref());
                *self = Cow::Owned(res);
            }
            Cow::Owned(owned) => owned.push_str(&string),
        }
    }
}
