use std::fmt::{Debug, Write};
use xml_parser::{ETag, Reader, STag, XmlDecl, XmlError, XmlEvent, PI};
use xml_xts::TestableParser;
use xml_xts::XmlTester;

mod tester;

/*#[test]
fn main() {
    let report = XmlTester::new().test(&tester::ReaderIT);
    report.print_statistic();
    report.assert();
}*/
