use crate::simple::{AttributeAccess, CowVisitor, SimpleXmlParser, SimpleXmlVisitor, StrVisitor};
use crate::{XmlDecl, XmlError};
use std::borrow::Cow;
use std::fmt;
use std::fmt::{Formatter, Write};
use std::rc::Rc;
use std::sync::Arc;

pub const XML_URI: &str = "http://www.w3.org/XML/1998/namespace";
pub const XMLNS_URI: &str = "http://www.w3.org/2000/xmlns/";

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct QName<'i> {
    prefix: Option<&'i str>,
    local: &'i str,
}

impl<'i> QName<'i> {
    pub fn from_str(input: &'i str) -> Result<Self, XmlError> {
        if let Some((prefix, local)) = input.split_once(|c| c == ':') {
            if local.as_bytes().contains(&b':') {
                return Err(XmlError::IllegalName {
                    name: input.to_string(),
                });
            }

            Ok(QName {
                prefix: Some(prefix),
                local,
            })
        } else {
            Ok(QName {
                prefix: None,
                local: input,
            })
        }
    }

    pub fn new(prefix: Option<&'i str>, local: &'i str) -> Self {
        QName { prefix, local }
    }

    pub fn prefix(&self) -> Option<&'i str> {
        self.prefix
    }

    pub fn local(&self) -> &'i str {
        self.local
    }
}

impl<'a> fmt::Display for QName<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(prefix) = &self.prefix {
            f.write_fmt(format_args!("{}:{}", prefix, &self.local))
        } else {
            f.write_str(&self.local)
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Namespace {
    uri: Rc<str>,
}

impl AsRef<str> for Namespace {
    fn as_ref(&self) -> &str {
        &self.uri
    }
}

impl fmt::Debug for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Namespace").field(&self.uri).finish()
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.uri)
    }
}

impl Namespace {
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into().into_boxed_str().into(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.uri
    }
}

/// Visitor for SimpleNXmlParser
pub trait SimpleNsXmlVisitor<'i>: Sized {
    type Value;

    fn visit_start_element<A: NsAttributeAccess<'i>>(
        self,
        tag: QName<'i>,
        namespace: Option<Namespace>,
        attrs: A,
    ) -> Result<Self::Value, XmlError>;
    fn visit_end_element(
        self,
        tag: QName<'i>,
        namespace: Option<Namespace>,
    ) -> Result<Self::Value, XmlError>;
    fn visit_declaration(self, decl: XmlDecl) -> Result<Self::Value, XmlError>;
    fn visit_characters(self, characters: &'i str) -> Result<Self::Value, XmlError>;
    fn visit_borrowed_characters(self, characters: &str) -> Result<Self::Value, XmlError>;
    fn visit_pi(self, target: &'i str, data: Option<&'i str>) -> Result<Self::Value, XmlError>;
    fn visit_comment(self, comment: &'i str) -> Result<Self::Value, XmlError>;
}

/// Namespaced attribute access in visitor
pub trait NsAttributeAccess<'i>: Sized {
    fn next_entries(&self) -> usize;
    fn next_entry<K: QNameVisitor<'i>, V: StrVisitor<'i>>(
        &mut self,
        key_visitor: K,
        value_visitor: V,
    ) -> Result<Option<(K::Value, V::Value)>, XmlError>;
}

pub trait QNameVisitor<'i>: Sized {
    type Value;

    fn visit_qname(
        self,
        key: QName<'i>,
        namespace: Option<Namespace>,
    ) -> Result<Self::Value, XmlError>;
}

/// Simple XML parser
///
/// Does not support DTDs and only UTF-8 strings.
///
/// Should be sufficient for most modern XML.
pub struct SimpleNsXmlParser<'i> {
    parser: SimpleXmlParser<'i>,
    state: InnerState<'i>,
}

struct InnerState<'i> {
    scopes: NamespaceStack<'i>,
    stack: Vec<(QName<'i>, Option<Namespace>)>,

    /// buffer for resolved attributes (most of the time empty)
    attrs: Vec<(Option<Namespace>, QName<'i>, Cow<'i, str>)>,
}

impl<'i> SimpleNsXmlParser<'i> {
    pub fn from_str(input: &'i str) -> Self {
        Self {
            parser: SimpleXmlParser::from_str(input),
            state: InnerState {
                scopes: NamespaceStack::default(),
                stack: vec![],
                attrs: vec![],
            },
        }
    }

    pub fn cursor_offset(&self) -> usize {
        self.parser.cursor_offset()
    }

    pub fn unparsed(&self) -> &'i str {
        self.parser.unparsed()
    }

    pub fn parse_next<V: SimpleNsXmlVisitor<'i>>(
        &mut self,
        visitor: V,
    ) -> Result<Option<V::Value>, XmlError> {
        self.parser.parse_next(VisitorAdapter {
            state: &mut self.state,
            visitor,
        })
    }
}

struct VisitorAdapter<'i, 'a, V> {
    state: &'a mut InnerState<'i>,
    visitor: V,
}

impl<'i, 'a, V> VisitorAdapter<'i, 'a, V> {
    fn build_scope<A: AttributeAccess<'i>>(&mut self, mut attrs_access: A) -> Result<(), XmlError> {
        self.state.attrs.clear();

        let mut scope = self.state.scopes.build_scope();
        while let Some((qname, value)) = attrs_access.next_entry(QNameStrVisitor, CowVisitor)? {
            if let Some(prefix) = &qname.prefix() {
                if *prefix == "xmlns" {
                    // namespace definition
                    if value.is_empty()
                        || ((qname.local() == "xml") != (value == XML_URI))
                        || ((qname.local() == "xmlns") != (value == XMLNS_URI))
                    {
                        return Err(XmlError::IllegalNamespaceUri(value.to_string()));
                    }
                    scope.add(qname.local(), value.to_string())
                }
            } else if qname.local() == "xmlns" {
                // new default namespace
                if value == XML_URI || value == XMLNS_URI {
                    return Err(XmlError::IllegalNamespaceUri(value.to_string()));
                }
                if value.is_empty() {
                    scope.reset_default()
                } else {
                    scope.set_default(value.to_string())
                }
            }

            self.state.attrs.push((None, qname, value));
        }
        scope.finish();

        Ok(())
    }

    fn check_unique_attributes(&mut self) -> Result<(), XmlError> {
        for (i, (namespace, qname, _)) in self.state.attrs.iter().enumerate() {
            if let Some(namespace) = namespace {
                if self
                    .state
                    .attrs
                    .iter()
                    .take(i)
                    .any(|(other_namespace, other_qname, _)| {
                        Some(namespace) == other_namespace.as_ref()
                            && qname.local() == other_qname.local()
                    })
                {
                    return Err(XmlError::NonUniqueAttribute {
                        attribute: format!("{{{}}}{}", namespace, qname.local),
                    });
                }
            }
        }

        Ok(())
    }
}

struct AttrAccessAdapter<'i, 'a> {
    scopes: &'a mut NamespaceStack<'i>,
    rest_attrs: std::vec::Drain<'a, (Option<Namespace>, QName<'i>, Cow<'i, str>)>,
}

impl<'i, 'a> NsAttributeAccess<'i> for AttrAccessAdapter<'i, 'a> {
    fn next_entries(&self) -> usize {
        self.rest_attrs.size_hint().0
    }

    fn next_entry<K: QNameVisitor<'i>, V: StrVisitor<'i>>(
        &mut self,
        key_visitor: K,
        value_visitor: V,
    ) -> Result<Option<(K::Value, V::Value)>, XmlError> {
        if let Some((namespace, qname, attr_value)) = self.rest_attrs.next() {
            let key = key_visitor.visit_qname(qname, namespace)?;
            let value = match attr_value {
                Cow::Borrowed(borrowed) => value_visitor.visit_borrowed(borrowed)?,
                Cow::Owned(owned) => value_visitor.visit_string(owned)?,
            };
            Ok(Some((key, value)))
        } else {
            Ok(None)
        }
    }
}

struct QNameStrVisitor;

impl<'i> StrVisitor<'i> for QNameStrVisitor {
    type Value = QName<'i>;

    fn visit_borrowed(self, value: &'i str) -> Result<Self::Value, XmlError> {
        QName::from_str(value)
    }

    fn visit_string(self, value: String) -> Result<Self::Value, XmlError> {
        unreachable!()
    }
}

impl<'i, 'a, V: SimpleNsXmlVisitor<'i>> SimpleXmlVisitor<'i> for VisitorAdapter<'i, 'a, V> {
    type Value = V::Value;

    fn visit_start_element<A: AttributeAccess<'i>>(
        mut self,
        tag: &'i str,
        mut attrs_access: A,
    ) -> Result<Self::Value, XmlError> {
        self.build_scope(attrs_access)?;

        // resolve attribute namespaces
        for (namespace, qname, value) in self.state.attrs.iter_mut() {
            *namespace = self.state.scopes.resolve_attribute_namespace(&qname)?;
        }

        self.check_unique_attributes()?;

        // resolve element namespace
        let qname = QName::from_str(tag)?;
        if qname.local() == "xmlns" {
            return Err(XmlError::IllegalName {
                name: qname.to_string(),
            });
        }
        let namespace = self.state.scopes.resolve_element_namespace(&qname)?;
        self.state.stack.push((qname.clone(), namespace.clone()));

        self.visitor.visit_start_element(
            qname,
            namespace,
            AttrAccessAdapter {
                scopes: &mut self.state.scopes,
                rest_attrs: self.state.attrs.drain(..),
            },
        )
    }

    fn visit_end_element(self, tag: &'i str) -> Result<Self::Value, XmlError> {
        if let Some((qname, namespace)) = self.state.stack.pop() {
            self.visitor.visit_end_element(qname, namespace)
        } else {
            unreachable!()
        }
    }

    fn visit_declaration(self, decl: XmlDecl) -> Result<Self::Value, XmlError> {
        self.visitor.visit_declaration(decl)
    }

    fn visit_characters(self, characters: &'i str) -> Result<Self::Value, XmlError> {
        self.visitor.visit_characters(characters)
    }

    fn visit_borrowed_characters(self, characters: &str) -> Result<Self::Value, XmlError> {
        self.visitor.visit_borrowed_characters(characters)
    }

    fn visit_pi(self, target: &'i str, data: Option<&'i str>) -> Result<Self::Value, XmlError> {
        self.visitor.visit_pi(target, data)
    }

    fn visit_comment(self, comment: &'i str) -> Result<Self::Value, XmlError> {
        self.visitor.visit_comment(comment)
    }
}

struct NamespaceStack<'i> {
    namespaces: Vec<(Option<&'i str>, Option<Namespace>)>,
    sub_sizes: Vec<usize>,
}

impl<'i> NamespaceStack<'i> {
    pub fn new() -> Self {
        Self {
            namespaces: vec![
                (Some("xml"), Some(Namespace::new(XML_URI))),
                (Some("xmlns"), Some(Namespace::new(XMLNS_URI))),
            ],
            sub_sizes: vec![],
        }
    }

    pub fn build_scope<'a>(&'a mut self) -> NamespaceStackScopeBuilder<'i, 'a> {
        NamespaceStackScopeBuilder {
            stack: self,
            size: 0,
        }
    }

    pub fn pop_scope(&mut self) {
        let scope_namespaces = self.sub_sizes.pop().expect("stack underflow");
        self.namespaces
            .truncate(self.namespaces.len() - scope_namespaces);
    }

    pub fn resolve(&self, prefix: &str) -> Option<Namespace> {
        self.namespaces
            .iter()
            .rev()
            .find(|(pre, _)| pre == &Some(prefix))
            .and_then(|(_, ns)| ns.clone())
    }

    pub fn resolve_default(&self) -> Option<Namespace> {
        self.namespaces
            .iter()
            .rev()
            .find(|(pre, _)| pre.is_none())
            .and_then(|(_, ns)| ns.clone())
    }

    #[inline]
    pub fn resolve_namespace(&self, prefix: Option<&str>) -> Option<Namespace> {
        if let Some(prefix) = prefix {
            self.resolve(prefix)
        } else {
            self.resolve_default()
        }
    }

    #[inline]
    pub fn resolve_element_namespace(
        &self,
        qname: &QName<'i>,
    ) -> Result<Option<Namespace>, XmlError> {
        if let Some(prefix) = &qname.prefix {
            match self.resolve(prefix) {
                Some(ns) => Ok(Some(ns)),
                None => Err(XmlError::UnknownNamespacePrefix(prefix.to_string())),
            }
        } else {
            Ok(self.resolve_default())
        }
    }

    #[inline]
    pub fn resolve_attribute_namespace(
        &self,
        qname: &QName<'i>,
    ) -> Result<Option<Namespace>, XmlError> {
        if let Some(prefix) = &qname.prefix {
            match self.resolve(prefix) {
                Some(ns) => Ok(Some(ns)),
                None => Err(XmlError::UnknownNamespacePrefix(prefix.to_string())),
            }
        } else {
            Ok(None)
        }
    }
}

impl<'i> Default for NamespaceStack<'i> {
    fn default() -> Self {
        Self::new()
    }
}

struct NamespaceStackScopeBuilder<'i, 'a> {
    stack: &'a mut NamespaceStack<'i>,
    size: usize,
}

impl<'i, 'a> NamespaceStackScopeBuilder<'i, 'a> {
    pub fn add(&mut self, prefix: &'i str, uri: String) {
        self.stack
            .namespaces
            .push((Some(prefix), Some(Namespace::new(uri))));
        self.size += 1;
    }

    pub fn set_default(&mut self, uri: String) {
        self.stack
            .namespaces
            .push((None, Some(Namespace::new(uri))));
        self.size += 1;
    }

    pub fn reset_default(&mut self) {
        self.stack.namespaces.push((None, None));
        self.size += 1;
    }

    pub fn finish(self) -> &'a mut NamespaceStack<'i> {
        self.stack.sub_sizes.push(self.size);
        self.stack
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simple::StringVisitor;

    #[derive(Debug)]
    enum XmlEvent<'i> {
        StartElem {
            tag: QName<'i>,
            namespace: Option<Namespace>,
            attrs: Vec<(QName<'i>, Option<Namespace>, String)>,
        },
        EndElem {
            tag: QName<'i>,
            namespace: Option<Namespace>,
        },
        Characters(String),
        PI {
            target: &'i str,
            data: Option<&'i str>,
        },
        Comment(&'i str),
    }

    struct XmlEventVisitor;

    impl<'i> SimpleNsXmlVisitor<'i> for XmlEventVisitor {
        type Value = XmlEvent<'i>;

        fn visit_start_element<A: NsAttributeAccess<'i>>(
            self,
            tag: QName<'i>,
            namespace: Option<Namespace>,
            mut attrs_access: A,
        ) -> Result<Self::Value, XmlError> {
            let mut attrs = Vec::with_capacity(attrs_access.next_entries());
            while let Some(((qname, namespace), value)) =
                attrs_access.next_entry(NamespacedNameStrVisitor, StringVisitor)?
            {
                attrs.push((qname, namespace, value));
            }

            Ok(XmlEvent::StartElem {
                tag,
                namespace,
                attrs,
            })
        }
        fn visit_end_element(
            self,
            tag: QName<'i>,
            namespace: Option<Namespace>,
        ) -> Result<Self::Value, XmlError> {
            Ok(XmlEvent::EndElem { tag, namespace })
        }
        fn visit_declaration(self, decl: XmlDecl) -> Result<Self::Value, XmlError> {
            unimplemented!()
        }
        fn visit_characters(self, characters: &'i str) -> Result<Self::Value, XmlError> {
            Ok(XmlEvent::Characters(characters.to_string()))
        }
        fn visit_borrowed_characters(self, characters: &str) -> Result<Self::Value, XmlError> {
            Ok(XmlEvent::Characters(characters.to_string()))
        }
        fn visit_pi(self, target: &'i str, data: Option<&'i str>) -> Result<Self::Value, XmlError> {
            Ok(XmlEvent::PI { target, data })
        }
        fn visit_comment(self, comment: &'i str) -> Result<Self::Value, XmlError> {
            Ok(XmlEvent::Comment(comment))
        }
    }

    struct NamespacedNameStrVisitor;

    impl<'i> QNameVisitor<'i> for NamespacedNameStrVisitor {
        type Value = (QName<'i>, Option<Namespace>);

        fn visit_qname(
            self,
            key: QName<'i>,
            namespace: Option<Namespace>,
        ) -> Result<Self::Value, XmlError> {
            Ok((key, namespace))
        }
    }

    fn expect_event<'i>(parser: &mut SimpleNsXmlParser<'i>) -> XmlEvent<'i> {
        parser.parse_next(XmlEventVisitor).unwrap().unwrap()
    }

    fn expect_error(parser: &mut SimpleNsXmlParser) -> XmlError {
        parser.parse_next(XmlEventVisitor).unwrap_err()
    }

    #[test]
    fn test_unnamespaced() {
        let mut parser = SimpleNsXmlParser::from_str("<e/>");
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace: None,
                attrs
            } if tag == QName::new(None, "e") && attrs.is_empty()
        ));
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::EndElem {
                tag,
                namespace: None,
            } if tag == QName::new(None, "e")
        ));
    }

    #[test]
    fn test_unnamespaced_attr() {
        let mut parser = SimpleNsXmlParser::from_str("<e a='value'/>");
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                attrs, ..
            } if attrs[0] == (QName::new(None, "a"), None, "value".to_string())
        ));
    }

    #[test]
    fn test_namespaced() {
        let mut parser = SimpleNsXmlParser::from_str("<n1:e xmlns:n1='https://example.org' />");
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if tag == QName::new(Some("n1"), "e")
                && namespace == Some(Namespace::new("https://example.org"))
        ));
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::EndElem {
                tag,
                namespace,
            } if tag == QName::new(Some("n1"), "e")
                && namespace == Some(Namespace::new("https://example.org"))
        ));
    }

    #[test]
    fn test_empty_namespace_name() {
        let mut parser = SimpleNsXmlParser::from_str("<n1:e xmlns:n1='' />");
        assert!(matches!(
            expect_error(&mut parser),
            XmlError::IllegalNamespaceUri(ref ns) if ns == ""
        ));
    }

    #[test]
    fn test_illegal_xml_namespace_redefinition() {
        let mut parser = SimpleNsXmlParser::from_str("<e xmlns:xml='https://example.org' />");
        let x = expect_error(&mut parser);
        assert!(
            matches!(
                x,
                XmlError::IllegalNamespaceUri(ref ns) if ns == "https://example.org"
            ),
            "{:?}",
            x
        );
    }

    #[test]
    fn test_illegal_xml_namespace_name() {
        let mut parser =
            SimpleNsXmlParser::from_str("<n1:e xmlns:n1='http://www.w3.org/XML/1998/namespace' />");
        assert!(matches!(
            expect_error(&mut parser),
            XmlError::IllegalNamespaceUri(ref ns) if ns == "http://www.w3.org/XML/1998/namespace"
        ));
    }

    #[test]
    fn test_illegal_xml_default_namespace_name() {
        let mut parser =
            SimpleNsXmlParser::from_str("<n1:e xmlns='http://www.w3.org/XML/1998/namespace' />");
        assert!(matches!(
            expect_error(&mut parser),
            XmlError::IllegalNamespaceUri(ref ns) if ns == "http://www.w3.org/XML/1998/namespace"
        ));
    }

    #[test]
    fn test_legal_xml_namespace_name() {
        let mut parser = SimpleNsXmlParser::from_str(
            "<e xml:base='' xmlns:xml='http://www.w3.org/XML/1998/namespace' />",
        );
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                attrs,
                ..
            } if attrs[0].1.as_ref().unwrap().as_str() == "http://www.w3.org/XML/1998/namespace"
        ));
    }

    #[test]
    fn test_illegal_xmlns_namespace_redefinition() {
        let mut parser = SimpleNsXmlParser::from_str("<e xmlns:xmlns='https://example.org' />");
        assert!(matches!(
            expect_error(&mut parser),
            XmlError::IllegalNamespaceUri(ref ns) if ns == "https://example.org"
        ));
    }

    #[test]
    fn test_illegal_xmlns_namespace_name() {
        let mut parser =
            SimpleNsXmlParser::from_str("<n1:e xmlns:n1='http://www.w3.org/2000/xmlns/' />");
        assert!(matches!(
            expect_error(&mut parser),
            XmlError::IllegalNamespaceUri(ref ns) if ns == "http://www.w3.org/2000/xmlns/"
        ));
    }

    #[test]
    fn test_illegal_xmlns_default_namespace_name() {
        let mut parser = SimpleNsXmlParser::from_str("<e xmlns='http://www.w3.org/2000/xmlns/' />");
        assert!(matches!(
            expect_error(&mut parser),
            XmlError::IllegalNamespaceUri(ref ns) if ns == "http://www.w3.org/2000/xmlns/"
        ));
    }

    #[test]
    fn test_legal_xmlns_namespace_name() {
        let mut parser =
            SimpleNsXmlParser::from_str("<e xmlns:xmlns='http://www.w3.org/2000/xmlns/' />");
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                attrs,
                ..
            } if attrs[0].1.as_ref().unwrap().as_str() == "http://www.w3.org/2000/xmlns/"
        ));
    }

    #[test]
    fn test_scoping() {
        let mut parser = SimpleNsXmlParser::from_str(
            "<x xmlns:n1='https://example.org'><n1:e a='' xmlns:n1='https://example.com'/></x>",
        );
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if tag == QName::new(None, "x")
        ));
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if namespace.as_ref().unwrap().as_str() == "https://example.com"
        ));
    }

    #[test]
    fn test_attr_scoping() {
        let mut parser = SimpleNsXmlParser::from_str(
            "<x xmlns:n1='https://example.org'><e n1:a='' xmlns:n1='https://example.com'/></x>",
        );
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if tag == QName::new(None, "x")
        ));
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if tag == QName::new(None, "e")
                && attrs[0].1.as_ref().unwrap().as_str() == "https://example.com"
        ));
    }

    #[test]
    fn test_elem_default_namespace() {
        let mut parser = SimpleNsXmlParser::from_str(
            "<x xmlns='https://example.org'><x xmlns='https://example.org'><x/></x></x>",
        );
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if namespace.as_ref().unwrap().as_ref() == "https://example.org"
        ));
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if namespace.as_ref().unwrap().as_ref() == "https://example.org"
        ));
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if namespace.as_ref().unwrap().as_ref() == "https://example.org"
        ));
    }

    #[test]
    fn test_elem_no_default_namespace() {
        let mut parser =
            SimpleNsXmlParser::from_str("<x xmlns='https://example.org'><x xmlns=''><x/></x></x>");
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if namespace.as_ref().unwrap().as_ref() == "https://example.org"
        ));
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if namespace == None
        ));
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if namespace == None
        ));
    }

    #[test]
    fn test_legal_unique_attributes() {
        let mut parser = SimpleNsXmlParser::from_str(
            "<x xmlns='https://example.org' xmlns:n1='https://example.org'><x a='' n1:a=''/></x>",
        );
        let ns = Namespace::new("https://example.org");
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem { .. }
        ));
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem {
                tag,
                namespace,
                attrs
            } if attrs[0].1 == None && attrs[1].1 == Some(ns)
        ));
    }

    #[test]
    fn test_unique_attributes() {
        let mut parser = SimpleNsXmlParser::from_str(
            "<x xmlns:n1='https://example.org' xmlns:n2='https://example.org'><x n1:a='' n2:a=''/></x>",
        );
        let ns = Namespace::new("https://example.org");
        assert!(matches!(
            expect_event(&mut parser),
            XmlEvent::StartElem { .. }
        ));
        assert!(matches!(
            expect_error(&mut parser),
            XmlError::NonUniqueAttribute{ attribute } if attribute == "{https://example.org}a"
        ),);
    }
}
