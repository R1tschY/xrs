use crate::functions::{check_variable_argument_size, expect_string_argument, Function};
use crate::object::Object;
use crate::XPathError;

pub(crate) struct ConcatFunction;

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
