use crate::functions::{check_fixed_argument_size, expect_boolean_argument, Function};
use crate::object::Object;
use crate::XPathError;

pub(crate) struct BooleanFunction;

impl Function for BooleanFunction {
    fn name(&self) -> &str {
        "boolean"
    }

    fn signature(&self) -> &str {
        "boolean boolean(object)"
    }

    fn call<'i, 't>(&self, mut args: Vec<Object<'i, 't>>) -> Result<Object<'i, 't>, XPathError> {
        check_fixed_argument_size(self, &args, 1)?;

        Ok(Object::Boolean(bool::from(args.pop().unwrap())))
    }
}

pub(crate) struct NotFunction;

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

pub(crate) struct TrueFunction;

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

pub(crate) struct FalseFunction;

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
