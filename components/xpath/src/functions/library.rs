use std::collections::HashMap;

use crate::functions::boolean::{BooleanFunction, FalseFunction, NotFunction, TrueFunction};
use crate::functions::string::ConcatFunction;
use crate::functions::Function;
use crate::object::Object;
use crate::XPathError;

pub struct FunctionLibrary {
    functions: HashMap<&'static str, &'static dyn Function>,
}

impl Default for FunctionLibrary {
    fn default() -> Self {
        let funs: &[&'static dyn Function] = &[
            &ConcatFunction,
            &BooleanFunction,
            &NotFunction,
            &TrueFunction,
            &FalseFunction,
        ];

        let mut res = Self {
            functions: HashMap::with_capacity(funs.len()),
        };
        for fun in funs {
            res.register(*fun);
        }
        res
    }
}

impl FunctionLibrary {
    pub fn register(&mut self, fun: &'static dyn Function) {
        self.functions.insert(fun.name(), fun);
    }

    pub fn call<'i, 't>(
        &self,
        name: &str,
        args: Vec<Object<'i, 't>>,
    ) -> Result<Object<'i, 't>, XPathError> {
        match self.functions.get(name) {
            Some(fun) => fun.call(args),
            None => Err(XPathError::CallToUndefinedFunction(name.to_string())),
        }
    }
}
