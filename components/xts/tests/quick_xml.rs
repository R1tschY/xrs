use serde_explicit_xml::quick_xml::events::Event;
use serde_explicit_xml::quick_xml::Reader;
use xml_xts::TestableParser;
use xml_xts::XmlTester;

struct QuickXmlIT;

impl TestableParser for QuickXmlIT {
    fn is_wf(&self, input: &[u8]) -> bool {
        let mut reader = Reader::from_reader(input);
        reader.trim_text(true);
        reader.check_comments(true);
        reader.check_end_names(true);

        let mut buf = Vec::new();
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::Eof) => return true,
                Ok(_) => buf.clear(),
                Err(e) => {
                    println!("Error at position {}: {:?}", reader.buffer_position(), e);
                    return false;
                }
            }
        }
    }

    fn canonxml(&self, input: &[u8]) -> String {
        "".to_string()
    }
}

#[test]
fn main() {
    let report = XmlTester::new().test(&QuickXmlIT);
    report.print();
    report.assert();
}
