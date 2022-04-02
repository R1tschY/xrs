use std::collections::HashMap;

use crate::datamodel::Node;
use crate::functions::FunctionLibrary;
use crate::object::Object;

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
