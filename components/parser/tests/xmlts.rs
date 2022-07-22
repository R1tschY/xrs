use std::fmt::{Debug, Write};

use xrs_parser::{ETag, Reader, STag, XmlDecl, XmlError, XmlEvent, PI};
use xrs_xts::TestableParser;
use xrs_xts::XmlTester;

mod tester;

/*#[test]
fn main() {
    let report = XmlTester::new().test(&tester::ReaderIT);
    report.print_statistic();
    report.assert();
}*/
