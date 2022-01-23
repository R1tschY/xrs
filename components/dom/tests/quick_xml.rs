use xml_xts::TestableParser;
use xml_xts::XmlTester;

struct QuickXmlIT;

impl TestableParser for QuickXmlIT {
    fn check_wf(&self, input: &[u8]) -> bool {}

    fn canonxml(&self, input: &[u8]) -> String {}
}

fn main() {}
