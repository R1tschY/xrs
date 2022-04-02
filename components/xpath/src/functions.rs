use std::collections::HashMap;

use crate::datamodel::{Function, Object};
use crate::XPathError;

fn function_argument_error<'i, 't>(
    fun: &dyn Function,
    args: &Vec<Object<'i, 't>>,
    message: &str,
) -> XPathError {
    let wrong_sig = args
        .iter()
        .map(|arg| arg.type_name())
        .collect::<Vec<&'static str>>()
        .join(", ");

    XPathError::WrongFunctionArgument(format!(
        "{}: got '({})' for function '{}'",
        message,
        wrong_sig,
        fun.signature(),
    ))
}

fn check_fixed_argument_size<'i, 't>(
    fun: &dyn Function,
    args: &Vec<Object<'i, 't>>,
    n: u16,
) -> Result<(), XPathError> {
    if args.len() == n as usize {
        Err(function_argument_error(
            fun,
            args,
            "wrong number of arguments",
        ))
    } else {
        Ok(())
    }
}

fn check_argument_size<'i, 't>(
    fun: &dyn Function,
    args: &Vec<Object<'i, 't>>,
    min: u16,
    max: u16,
) -> Result<(), XPathError> {
    if args.len() < min as usize && args.len() > max as usize {
        Err(function_argument_error(
            fun,
            args,
            "wrong number of arguments",
        ))
    } else {
        Ok(())
    }
}

fn check_variable_argument_size<'i, 't>(
    fun: &dyn Function,
    args: &Vec<Object<'i, 't>>,
    min: u16,
) -> Result<(), XPathError> {
    if args.len() < min as usize {
        Err(function_argument_error(
            fun,
            args,
            "wrong number of arguments",
        ))
    } else {
        Ok(())
    }
}

fn expect_string_argument<'a, 'i, 't>(
    fun: &dyn Function,
    args: &'a Vec<Object<'i, 't>>,
    i: usize,
) -> Result<&'a str, XPathError> {
    match &args[i] {
        Object::String(s) => Ok(s),
        _ => Err(function_argument_error(
            fun,
            args,
            &format!("expected string for argument {}", i + 1),
        )),
    }
}

fn expect_boolean_argument<'a, 'i, 't>(
    fun: &dyn Function,
    args: &'a Vec<Object<'i, 't>>,
    i: usize,
) -> Result<bool, XPathError> {
    match &args[i] {
        Object::Boolean(b) => Ok(*b),
        _ => Err(function_argument_error(
            fun,
            args,
            &format!("expected boolean for argument {}", i + 1),
        )),
    }
}

// 4.2 String Functions

struct ConcatFunction;

impl Function for ConcatFunction {
    fn name(&self) -> &str {
        "concat"
    }

    fn signature(&self) -> &str {
        "string concat(string, string, string*)"
    }

    fn call<'i, 't>(&self, args: Vec<Object<'i, 't>>) -> Result<Object<'i, 't>, XPathError> {
        check_variable_argument_size(self, &args, 2)?;

        let mut size = 0usize;
        for i in 0..args.len() {
            size += expect_string_argument(self, &args, i)?.len();
        }

        let mut result = String::with_capacity(size);
        for i in 0..args.len() {
            result.push_str(expect_string_argument(self, &args, i)?);
        }

        Ok(Object::String(result.into()))
    }
}

// 4.3 Boolean Functions

pub fn boolean(obj: &Object) -> bool {
    match obj {
        Object::Number(number) => *number != 0.0 && !number.is_nan(),
        Object::NodeSet(node_set) => !node_set.is_empty(),
        Object::String(string) => !string.is_empty(),
        Object::Boolean(boolean) => *boolean,
        Object::Additional(additional) => additional.boolean_value(),
    }
}

struct BooleanFunction;

impl Function for BooleanFunction {
    fn name(&self) -> &str {
        "boolean"
    }

    fn signature(&self) -> &str {
        "boolean boolean(object)"
    }

    fn call<'i, 't>(&self, args: Vec<Object<'i, 't>>) -> Result<Object<'i, 't>, XPathError> {
        check_fixed_argument_size(self, &args, 1)?;

        Ok(Object::Boolean(boolean(&args[0])))
    }
}

struct NotFunction;

impl Function for NotFunction {
    fn name(&self) -> &str {
        "not"
    }

    fn signature(&self) -> &str {
        "boolean not(boolean)"
    }

    fn call<'i, 't>(&self, args: Vec<Object<'i, 't>>) -> Result<Object<'i, 't>, XPathError> {
        check_fixed_argument_size(self, &args, 1)?;
        Ok(Object::Boolean(!expect_boolean_argument(self, &args, 0)?))
    }
}

struct TrueFunction;

impl Function for TrueFunction {
    fn name(&self) -> &str {
        "true"
    }

    fn signature(&self) -> &str {
        "boolean true()"
    }

    fn call<'i, 't>(&self, args: Vec<Object<'i, 't>>) -> Result<Object<'i, 't>, XPathError> {
        check_fixed_argument_size(self, &args, 0)?;
        Ok(Object::Boolean(true))
    }
}

struct FalseFunction;

impl Function for FalseFunction {
    fn name(&self) -> &str {
        "false"
    }

    fn signature(&self) -> &str {
        "boolean false()"
    }

    fn call<'i, 't>(&self, args: Vec<Object<'i, 't>>) -> Result<Object<'i, 't>, XPathError> {
        check_fixed_argument_size(self, &args, 0)?;
        Ok(Object::Boolean(false))
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concat_pass1() {
        let ret = ConcatFunction.call(vec![Object::new_string("ab"), Object::new_string("c")]);

        assert_eq!(Ok(Object::new_string("abc")), ret);
    }

    #[test]
    fn concat_pass2() {
        let ret = ConcatFunction.call(vec![
            Object::new_string("a"),
            Object::new_string("b"),
            Object::new_string(""),
            Object::new_string("c"),
        ]);

        assert_eq!(Ok(Object::new_string("abc")), ret);
    }

    #[test]
    fn concat_fail1() {
        let ret = ConcatFunction.call(vec![Object::new_string("ab")]);

        assert_eq!(
            Err(XPathError::WrongFunctionArgument("wrong number of arguments: got '(string)' for function 'string concat(string, string, string*)'".to_string())),
            ret
        );
    }

    #[test]
    fn concat_fail2() {
        let ret = ConcatFunction.call(vec![Object::new_string("ab"), Object::Number(1.0)]);

        assert_eq!(
            Err(XPathError::WrongFunctionArgument("expected string for argument 2: got '(string, number)' for function 'string concat(string, string, string*)'".to_string())),
            ret
        );
    }
}
