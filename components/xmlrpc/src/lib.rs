extern crate core;

use std::borrow::Cow;
use std::fmt;
use std::fmt::Formatter;

use serde::{Deserialize, Serialize};

pub mod de;
mod error;
pub mod ser;
pub mod value;

pub use crate::error::XmlRpcError;

pub struct MethodCall<'a, T> {
    method_name: Cow<'a, str>,
    params: T,
}

impl<'a, T> MethodCall<'a, T> {
    pub fn new(method_name: impl Into<Cow<'a, str>>, params: T) -> Self {
        Self {
            method_name: method_name.into(),
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

impl<'a, T> fmt::Debug for MethodCall<'a, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("MethodCall")
            .field("method_name", &self.method_name)
            .field("params", &self.params)
            .finish()
    }
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Fault<'a> {
    #[serde(rename = "faultCode")]
    fault_code: i32,
    #[serde(rename = "faultString")]
    fault_string: Cow<'a, str>,
}

impl<'a> Fault<'a> {
    pub fn into_owned(self) -> Fault<'static> {
        Fault {
            fault_code: self.fault_code,
            fault_string: self.fault_string.into_owned().into(),
        }
    }
}

#[derive(Debug)]
pub enum MethodResponses<'a> {
    Success(String),
    Fault(Fault<'a>),
}

pub enum MethodResponse<'a, T> {
    Success(T),
    Fault(Fault<'a>),
}

impl<'a, T> MethodResponse<'a, T> {
    pub fn into_owned(self) -> MethodResponse<'static, T> {
        match self {
            MethodResponse::Success(success) => MethodResponse::Success(success),
            MethodResponse::Fault(fault) => MethodResponse::Fault(fault.into_owned()),
        }
    }
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

impl<'a, T> fmt::Debug for MethodResponse<'a, T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MethodResponse::Success(success) => f.debug_tuple("Success").field(success).finish(),
            MethodResponse::Fault(fault) => f.debug_tuple("Fault").field(fault).finish(),
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
