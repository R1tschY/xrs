use xml_parser::{NsReader, XmlError, XmlNsEvent};

use crate::datamodel::{AttributeNode, Child, ElementNode, PINode, RootNode};
use crate::utils::CowStrHelpers;

pub struct DomBuilder {}

impl DomBuilder {
    pub fn build<'a>(input: &'a str) -> Result<RootNode<'a>, XmlError> {
        let mut reader = NsReader::new(input);

        let mut root: Vec<Child<'a>> = vec![];
        let mut stack: Vec<ElementNode<'a>> = vec![];

        while let Some(evt) = reader.next()? {
            match evt {
                XmlNsEvent::STag(stag) => stack.push(ElementNode {
                    namespace_name: reader
                        .resolve_element_namespace(&stag.qname)?
                        .map(|x| x.to_string().into()),
                    qname: stag.qname.to_string().into(),
                    prefix: stag.qname.prefix.clone(),
                    local_name: stag.qname.local_part.clone(),
                    attributes: {
                        let mut attrs = Vec::with_capacity(reader.attributes().len());
                        for attr in reader.attributes() {
                            attrs.push(AttributeNode {
                                qname: attr.qname.to_string().into(),
                                namespace_name: reader
                                    .resolve_attribute_namespace(&attr.qname)?
                                    .map(|x| x.to_string().into()),
                                prefix: attr.qname.prefix.clone(),
                                local_name: attr.qname.local_part.clone(),
                                value: attr.value.clone(),
                            });
                        }
                        attrs
                    },
                    namespaces: vec![], // TODO
                    children: vec![],
                }),
                XmlNsEvent::ETag(_) => {
                    if let Some(elem) = stack.pop() {
                        stack
                            .last_mut()
                            .map(|top| &mut top.children)
                            .unwrap_or(&mut root)
                            .push(Child::Element(elem));
                    } else {
                        unreachable!();
                    }
                }
                XmlNsEvent::Characters(cdata) => {
                    if let Some(top) = stack.last_mut() {
                        if let Some(Child::Text(ref mut text)) = top.children.last_mut() {
                            text.push_str(&cdata);
                        } else {
                            top.children.push(Child::Text(cdata));
                        }
                    }
                }
                XmlNsEvent::PI(pi) => stack
                    .last_mut()
                    .map(|elem| &mut elem.children)
                    .unwrap_or(&mut root)
                    .push(Child::ProcessingInstruction(PINode {
                        target: pi.target,
                        data: pi.data,
                    })),
                XmlNsEvent::Comment(comment) => stack
                    .last_mut()
                    .map(|elem| &mut elem.children)
                    .unwrap_or(&mut root)
                    .push(Child::Comment(comment)),
                _ => {}
            }
        }

        Ok(RootNode { children: root })
    }
}

#[cfg(test)]
mod tests {
    use crate::datamodel::parser::DomBuilder;

    #[test]
    fn test() {
        println!("{:#?}", DomBuilder::build("<a attr='value'>x&lt;y<b/></a>"));
    }
}
