#![allow(dead_code)]

use std::borrow::Cow;

use crate::datamodel::Node;

pub trait AdditionalObject: std::fmt::Debug {
    fn type_name(&self) -> &'static str;

    fn clone(&self) -> Box<dyn AdditionalObject>;
    fn eq(&self, other: &dyn AdditionalObject) -> bool;

    fn boolean_value(&self) -> bool;
}

impl Clone for Box<dyn AdditionalObject> {
    fn clone(&self) -> Self {
        AdditionalObject::clone(self.as_ref())
    }
}

impl PartialEq for &dyn AdditionalObject {
    fn eq(&self, other: &Self) -> bool {
        AdditionalObject::eq(*self, *other)
    }
}

impl PartialEq for Box<dyn AdditionalObject> {
    fn eq(&self, other: &Self) -> bool {
        AdditionalObject::eq(self.as_ref(), other.as_ref())
    }
}

#[derive(Debug, Clone)]
pub enum Object<'i, 't> {
    Number(f64),
    NodeSet(Vec<&'t Node<'i>>),
    String(Cow<'i, str>),
    Boolean(bool),
    Additional(Box<dyn AdditionalObject>),
}

impl<'i, 't> PartialEq for Object<'i, 't> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Object::Number(a), Object::Number(b)) => a == b,
            (Object::NodeSet(a), Object::NodeSet(b)) => a == b,
            (Object::String(a), Object::String(b)) => a == b,
            (Object::Boolean(a), Object::Boolean(b)) => a == b,
            (Object::Additional(a), Object::Additional(b)) => a == b,
            _ => false,
        }
    }
}

impl<'i, 't> Object<'i, 't> {
    pub fn type_name(&self) -> &'static str {
        match self {
            Object::Number(_) => "number",
            Object::NodeSet(_) => "node-set",
            Object::String(_) => "string",
            Object::Boolean(_) => "boolean",
            Object::Additional(additional) => additional.type_name(),
        }
    }

    pub fn new_string(s: impl Into<Cow<'i, str>>) -> Self {
        Object::String(s.into())
    }
}

impl<'i, 't> From<Object<'i, 't>> for bool {
    fn from(obj: Object<'i, 't>) -> Self {
        match obj {
            Object::Number(number) => number != 0.0 && !number.is_nan(),
            Object::NodeSet(node_set) => !node_set.is_empty(),
            Object::String(string) => !string.is_empty(),
            Object::Boolean(boolean) => boolean,
            Object::Additional(additional) => additional.boolean_value(),
        }
    }
}

impl<'i, 't> From<bool> for Object<'i, 't> {
    fn from(value: bool) -> Self {
        Object::Boolean(value)
    }
}
