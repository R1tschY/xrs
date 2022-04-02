pub use library::FunctionLibrary;

use crate::object::Object;
use crate::XPathError;

mod boolean;
mod library;
mod string;

pub trait Function {
    fn name(&self) -> &str;
    fn signature(&self) -> &str;
    fn call<'i, 't>(&self, args: Vec<Object<'i, 't>>) -> Result<Object<'i, 't>, XPathError>;
}

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
