use crate::processors::full::FullProcessor;
use crate::processors::quick_xml::QuickXmlProcessor;
use crate::processors::simple::SimpleProcessor;
use crate::processors::Processor;
use serde::Deserialize;
use std::any::Any;
use std::fs::File;
use std::io::{BufReader, Read};
use std::panic;
use xrs_parser::simple::SimpleXmlVisitor;

mod processors;

#[derive(Deserialize, Debug)]
enum TestLevel {
    #[serde(rename = "nwf")]
    NotWellFormed,
    #[serde(rename = "wf")]
    WellFormed,
    #[serde(rename = "invalid")]
    Invalid,
    #[serde(rename = "valid")]
    Valid,
}

#[derive(Deserialize, Debug)]
struct Test {
    description: String,
    level: TestLevel,
    sections: Vec<String>,
    input: String,
    norm: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TestSuite {
    #[serde(rename = "test")]
    tests: Vec<Test>,
}

struct Tester {
    succeeded: usize,
    failed: usize,
}

impl Tester {
    pub fn new() -> Self {
        Self {
            succeeded: 0,
            failed: 0,
        }
    }

    pub fn check(
        &mut self,
        test: &Test,
        f: impl Fn() -> Result<(), String> + panic::RefUnwindSafe,
    ) {
        // print!("? = {}", test.description);

        match panic::catch_unwind(|| f()) {
            Err(crash) => {
                println!("\r💥 = {}: {}", test.description, crash_message(&crash));
                self.failed += 1;
            }
            Ok(Err(err)) => {
                println!("\r❌ = {}: {}", test.description, err);
                self.failed += 1;
            }
            Ok(Ok(())) => {
                println!("\r✅ = {}", test.description);
                self.succeeded += 1;
            }
        }
    }

    pub fn assert_success(&self) {
        assert_eq!(self.failed, 0);
    }
}

fn read_test_suite<R: Read>(read: R) -> TestSuite {
    let mut buf = BufReader::new(read);
    let mut content = Vec::with_capacity(4098);
    buf.read_to_end(&mut content)
        .expect("should read XML test suite");
    let string = std::str::from_utf8(&content).expect("should be valid UTF-8");

    quick_xml::de::from_str(string).expect("should be valid XML")
}

fn test_processor(proc: &(impl Processor + panic::RefUnwindSafe)) {
    let f = File::open("tests/suite/test-suite.xml").expect("test suite should exist");
    let test_suite = read_test_suite(f);
    let mut tester = Tester::new();

    for test in test_suite.tests.iter() {
        tester.check(test, || match test.level {
            TestLevel::NotWellFormed => {
                if let Err(_err) = proc.check_wf(&test.input) {
                    Ok(())
                } else {
                    Err("Should not be well-formed".to_string())
                }
            }
            TestLevel::WellFormed => {
                if let Some(expected) = &test.norm {
                    let actual = proc.norm(&test.input)?;
                    text_diff(actual.trim(), expected.trim())
                } else {
                    proc.check_wf(&test.input)
                }
            }
            TestLevel::Invalid => Ok(()),
            TestLevel::Valid => Ok(()),
        });
    }

    tester.assert_success();
}

fn text_diff(actual: &str, expected: &str) -> Result<(), String> {
    for (i, (ca, cb)) in actual.chars().zip(expected.chars()).enumerate() {
        if ca != cb {
            return Err(format!(
                "Expected '{expected}' != actual '{actual}' - Diff starting at offset {i}"
            ));
        }
    }
    if actual.len() != expected.len() {
        return Err(format!("Expected '{expected}' != actual '{actual}'"));
    }
    Ok(())
}

fn crash_message(crash: &Box<dyn Any + Send>) -> String {
    if let Some(s) = crash.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = crash.downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic payload".to_owned()
    }
}

#[test]
fn simple_parser() {
    test_processor(&SimpleProcessor);
}

#[test]
fn quick_xml() {
    test_processor(&QuickXmlProcessor);
}

#[test]
fn full() {
    test_processor(&FullProcessor);
}
