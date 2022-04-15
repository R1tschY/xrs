use std::collections::HashMap;

use crate::datamodel::Node;
use crate::functions::FunctionLibrary;
use crate::object::Object;
use crate::XPathError;

pub struct Context<'i, 't> {
    /// context node
    node: &'t Node<'i>,
    /// context position
    position: usize,
    /// context size
    size: usize,
    /// set of variable bindings
    variable_bindings: HashMap<String, Object<'i, 't>>,
    /// function library
    function_library: FunctionLibrary,
    /// set of namespace declarations in scope for the expression
    namespaces: HashMap<String, String>,
}

impl<'i, 't> Context<'i, 't> {
    pub fn call_function(
        &self,
        name: &str,
        args: Vec<Object<'i, 't>>,
    ) -> Result<Object<'i, 't>, XPathError> {
        self.function_library.call(name, args)
    }
}
