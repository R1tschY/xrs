use std::borrow::Cow;

use xrs_chars::XmlAsciiChar;

#[derive(Default)]
pub(crate) struct CowStrBuilder<'a>(Cow<'a, str>);

impl<'a> CowStrBuilder<'a> {
    pub fn build(self) -> Cow<'a, str> {
        self.0
    }

    pub fn push_borrow_str(&mut self, string: &'a str) {
        if string.is_empty() {
            return;
        }

        if self.0.is_empty() {
            self.0 = Cow::Borrowed(string);
            return;
        }

        self.push_str_internal(string);
    }

    pub fn push_str(&mut self, string: &str) {
        if string.is_empty() {
            return;
        }

        self.push_str_internal(&string);
    }

    pub fn push_cow(&mut self, string: Cow<'a, str>) {
        if string.is_empty() {
            return;
        }

        if self.0.is_empty() {
            self.0 = string;
            return;
        }

        self.push_str_internal(&string);
    }

    fn push_str_internal(&mut self, string: &str) {
        match &mut self.0 {
            Cow::Borrowed(borrowed) => {
                let mut res = String::with_capacity(borrowed.len() + string.len());
                res.push_str(borrowed);
                res.push_str(&string);
                self.0 = Cow::Owned(res);
            }
            Cow::Owned(owned) => owned.push_str(&string),
        }
    }
}
