use std::borrow::Cow;

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
    pub data: Option<Cow<'a, str>>,
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
            Node::ProcessingInstruction(pi) => pi.data.clone().unwrap_or_default(),
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
                local: ns.prefix.clone().unwrap_or_else(|| Cow::Borrowed("")),
            }),
            Node::ProcessingInstruction(pi) => Some(ExpandedName {
                namespace: None,
                local: pi.target.clone(),
            }),
            _ => None,
        }
    }
}
