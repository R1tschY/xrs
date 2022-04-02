#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::HashMap;

use crate::functions::FunctionLibrary;
use crate::XPathError;

/// Expanded name
///
/// See https://www.w3.org/TR/REC-xml-names/#dt-expname
#[derive(Debug, PartialEq, Clone)]
pub struct ExpandedName<'a> {
    pub namespace: Option<Cow<'a, str>>,
    pub local: Cow<'a, str>,
}

/// Root Node
/// See https://www.w3.org/TR/1999/REC-xpath-19991116/#root-node
#[derive(Debug, PartialEq, Clone)]
pub struct RootNode<'a> {
    pub children: Vec<Child<'a>>,
}

/// Element Node
/// See https://www.w3.org/TR/1999/REC-xpath-19991116/#element-nodes
#[derive(Debug, PartialEq, Clone)]
pub struct ElementNode<'a> {
    pub qname: Cow<'a, str>,
    pub namespace_name: Option<Cow<'a, str>>,
    pub prefix: Option<Cow<'a, str>>,
    pub local_name: Cow<'a, str>,
    pub attributes: Vec<AttributeNode<'a>>,
    pub namespaces: Vec<NamespaceNode<'a>>,
    pub children: Vec<Child<'a>>,
}

/// Attribute Node
/// See https://www.w3.org/TR/1999/REC-xpath-19991116/#attribute-nodes
#[derive(Debug, PartialEq, Clone)]
pub struct AttributeNode<'a> {
    pub qname: Cow<'a, str>,
    pub namespace_name: Option<Cow<'a, str>>,
    pub prefix: Option<Cow<'a, str>>,
    pub local_name: Cow<'a, str>,
    pub value: Cow<'a, str>,
}

/// Attribute Node
/// See https://www.w3.org/TR/1999/REC-xpath-19991116/#attribute-nodes
#[derive(Debug, PartialEq, Clone)]
pub struct NamespaceNode<'a> {
    pub prefix: Option<Cow<'a, str>>,
    pub uri: Cow<'a, str>,
}

/// Processing Instruction Node
/// See https://www.w3.org/TR/1999/REC-xpath-19991116/#section-Processing-Instruction-Nodes
#[derive(Debug, PartialEq, Clone)]
pub struct PINode<'a> {
    pub target: Cow<'a, str>,
    pub data: Cow<'a, str>,
}

/// Element Child
#[derive(Debug, PartialEq, Clone)]
pub enum Child<'a> {
    Element(ElementNode<'a>),
    Text(Cow<'a, str>),
    ProcessingInstruction(PINode<'a>),
    Comment(Cow<'a, str>),
}

impl<'a> Child<'a> {
    /// Concatenation of the string-values of all text node descendants in document order
    pub fn text_value(&self) -> String {
        let mut result = String::new();
        self.text_value_(&mut result);
        result
    }

    fn text_value_(&self, result: &mut String) {
        match self {
            Child::Element(elem) => {
                for child in &elem.children {
                    child.text_value_(result);
                }
            }
            Child::Text(text) => result.push_str(text),
            _ => {}
        }
    }
}

/// Node
///
/// See https://www.w3.org/TR/1999/REC-xpath-19991116/#data-model
#[derive(Debug, PartialEq, Clone)]
pub enum Node<'a> {
    Root(RootNode<'a>),
    Element(ElementNode<'a>),
    Attribute(AttributeNode<'a>),
    Namespace(NamespaceNode<'a>),
    /// Text node
    Text(Cow<'a, str>),
    ProcessingInstruction(PINode<'a>),
    Comment(Cow<'a, str>),
}

impl<'a> Node<'a> {
    fn text_value(children: &Vec<Child<'a>>) -> Cow<'a, str> {
        let mut result = String::new();
        for child in children {
            child.text_value_(&mut result);
        }
        result.into()
    }

    /// https://www.w3.org/TR/1999/REC-xpath-19991116/#dt-string-value
    pub fn string_value(&self) -> Cow<'a, str> {
        match self {
            Node::Root(root) => Self::text_value(&root.children),
            Node::Element(elem) => Self::text_value(&elem.children),
            Node::Attribute(attr) => attr.value.clone(), // TODO: normalize
            Node::Namespace(ns) => ns.uri.clone(),
            Node::ProcessingInstruction(pi) => pi.data.clone(),
            Node::Comment(comment) => comment.clone(),
            Node::Text(text) => text.clone(),
        }
    }

    /// https://www.w3.org/TR/1999/REC-xpath-19991116/#dt-expanded-name
    pub fn expanded_name(&self) -> Option<ExpandedName<'a>> {
        match self {
            Node::Element(elem) => Some(ExpandedName {
                namespace: elem.namespace_name.clone(),
                local: elem.local_name.clone(),
            }),
            Node::Attribute(attr) => Some(ExpandedName {
                namespace: attr.namespace_name.clone(),
                local: attr.local_name.clone(),
            }),
            Node::Namespace(ns) => Some(ExpandedName {
                namespace: None,
                local: ns.prefix.clone().unwrap_or_else(|| "".into()),
            }),
            Node::ProcessingInstruction(pi) => Some(ExpandedName {
                namespace: None,
                local: pi.target.clone(),
            }),
            _ => None,
        }
    }
}

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

pub trait Function {
    fn name(&self) -> &str;
    fn signature(&self) -> &str;
    fn call<'i, 't>(&self, args: Vec<Object<'i, 't>>) -> Result<Object<'i, 't>, XPathError>;
}

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
