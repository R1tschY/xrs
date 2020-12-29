use crate::dom::Element;
use std::borrow::Cow;

enum Object<'a> {
    Number(f64),
    NodeSet(Vec<&'a Element>),
    String(Cow<'a, str>),
    Boolean(bool),
}
