#![allow(unused, non_snake_case)]

use std::path::Path;

use xml_xts::Test;
use xml_xts::XmlTester;

use crate::tester::ReaderIT;

mod tester;

include!(concat!(env!("OUT_DIR"), "/xts.rs"));

fn main() {}
