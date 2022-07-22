#![allow(unused, non_snake_case)]

use std::path::Path;

use xrs_xts::Test;
use xrs_xts::XmlTester;

use crate::tester::ReaderIT;

mod tester;

include!(concat!(env!("OUT_DIR"), "/xts.rs"));

fn main() {}
