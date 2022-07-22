use crate::value::Value;
use std::borrow::Cow;

pub mod de;
pub mod ser;
pub mod value;

pub trait DeserializeParams {
    fn deserialize() -> Self;
}

pub trait SerializeParams {
    fn serialize() -> Self;
}

pub struct MethodCall<'a, T> {
    method_name: Cow<'a, str>,
    params: T,
}

impl<'a, T> MethodCall<'a, T> {
    pub fn new(method_name: Cow<'a, str>, params: T) -> Self {
        Self {
            method_name,
            params,
        }
    }

    pub fn into_owned(self) -> MethodCall<'static, T> {
        MethodCall {
            method_name: self.method_name.into_owned().into(),
            params: self.params,
        }
    }

    pub fn method_name(&self) -> &str {
        &self.method_name
    }

    pub fn params(&self) -> &T {
        &self.params
    }
}

impl<'a, T> Clone for MethodCall<'a, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            method_name: self.method_name.clone(),
            params: self.params.clone(),
        }
    }
}

impl<'a, T> PartialEq for MethodCall<'a, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.method_name == other.method_name && self.params == other.params
    }
}

#[derive(Clone, PartialEq)]
pub struct Fault<'a> {
    fault_code: i32,
    fault_string: Cow<'a, str>,
}

pub enum MethodResponse<'a, T> {
    Success(T),
    Fault(Fault<'a>),
}

impl<'a, T> Clone for MethodResponse<'a, T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            MethodResponse::Success(success) => MethodResponse::Success(success.clone()),
            MethodResponse::Fault(fault) => MethodResponse::Fault(fault.clone()),
        }
    }
}

impl<'a, T> PartialEq for MethodResponse<'a, T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match self {
            MethodResponse::Success(s1) => {
                matches!(other, MethodResponse::Success(s2) if s1 == s2)
            }
            MethodResponse::Fault(f1) => {
                matches!(other, MethodResponse::Fault(f2) if f1 == f2)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Node {
        abc: i32,
    }

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
