use std::fs;
use xml_dom::reader::quick_xml::QuickXmlDomReader;
use xml_dom::reader::DomReader;
use xml_dom::validate::NonValidator;

fn main() {
    // ARRANGE
    let content = fs::read("/home/richard/dev/xml-support/components/xts/xmlts20130923/xmlconf/ibm/not-wf/P72/ibm72n08.xml").unwrap();
    let reader = QuickXmlDomReader::new(&content, NonValidator);
    // ACT
    let result = reader.parse();
    // ASSERT
    assert!(result.is_err());
    println!("Error: {:?}", result.err());
}
