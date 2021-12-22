use std::borrow::Cow;
use xml_dom::Element;

enum Object<'a> {
    Number(f64),
    NodeSet(Vec<&'a Element>),
    String(Cow<'a, str>),
    Boolean(bool),
}
