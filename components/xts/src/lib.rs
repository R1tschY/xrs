use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use serde::export::fmt::Debug;
use serde::Deserialize;
use serde_explicit_xml::from_reader;
use std::error::Error;
use std::fs;

#[derive(Deserialize)]
#[serde(rename = "TESTSUITE")]
struct TestSuite {
    #[serde(rename = "@PROFILE", default)]
    profile: String,

    #[serde(rename = "TESTCASES")]
    test_cases: Vec<TestCases>,
}

#[derive(Deserialize)]
#[serde(rename = "TESTCASES")]
struct TestCases {
    #[serde(rename = "TEST", default)]
    tests: Vec<Test>,

    #[serde(rename = "TESTCASES", default)]
    test_cases: Vec<TestCases>,

    #[serde(rename = "@xml:base", default)]
    base: String,

    #[serde(rename = "@PROFILE", default)]
    profile: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "TEST")]
struct Test {
    #[serde(rename = "@ENTITIES", default)]
    entities: Entities,

    #[serde(rename = "@ID")]
    id: String,

    #[serde(rename = "@OUTPUT", default)]
    output: String,

    #[serde(rename = "@OUTPUT3", default)]
    output3: String,

    #[serde(rename = "@SECTIONS")]
    sections: String,

    #[serde(rename = "@RECOMMENDATION", default)]
    recommendation: Recommendation,

    #[serde(rename = "@TYPE")]
    ty: Type,

    #[serde(rename = "@VERSION", default)]
    version: String,

    #[serde(rename = "@EDITION", default)]
    edition: String,

    #[serde(rename = "@URI")]
    uri: String,

    #[serde(rename = "@NAMESPACE", default = "yes")]
    namespace: YesNo,

    #[serde(rename = "$value")]
    description: Vec<String>,
}

#[derive(Deserialize, Debug, PartialEq)]
enum YesNo {
    #[serde(rename = "yes")]
    Yes,
    #[serde(rename = "no")]
    No,
}

fn yes() -> YesNo {
    YesNo::Yes
}

impl From<bool> for YesNo {
    fn from(value: bool) -> Self {
        if value {
            YesNo::Yes
        } else {
            YesNo::No
        }
    }
}

impl From<YesNo> for bool {
    fn from(value: YesNo) -> Self {
        match value {
            YesNo::Yes => true,
            YesNo::No => false,
        }
    }
}

#[derive(Deserialize, Debug)]
enum Type {
    #[serde(rename = "valid")]
    Valid,
    #[serde(rename = "invalid")]
    Invalid,
    #[serde(rename = "not-wf")]
    NotWf,
    #[serde(rename = "error")]
    Error,
}

#[derive(Deserialize, Debug)]
enum Entities {
    #[serde(rename = "both")]
    Both,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "parameter")]
    Parameter,
    #[serde(rename = "general")]
    General,
}

impl Default for Entities {
    fn default() -> Self {
        Entities::None
    }
}

#[derive(Deserialize, Debug)]
#[allow(non_camel_case_types)]
enum Recommendation {
    #[serde(rename = "XML1.0")]
    Xml_1_0,
    #[serde(rename = "XML1.1")]
    Xml_1_1,
    #[serde(rename = "NS1.0")]
    Ns_1_0,
    #[serde(rename = "NS1.1")]
    Ns_1_1,
    #[serde(rename = "XML1.0-errata2e")]
    Xml_1_0_errata2e,
    #[serde(rename = "XML1.0-errata3e")]
    Xml_1_1_errata2e,
    #[serde(rename = "XML1.0-errata4e")]
    Ns_1_0_errata2e,
    #[serde(rename = "NS1.0-errata1e")]
    Ns_1_1_errata2e,
}

impl Default for Recommendation {
    fn default() -> Self {
        Recommendation::Xml_1_0
    }
}

pub trait TestableParser {
    fn check_wf(&self, input: &[u8]) -> bool;
    fn canonxml(&self, input: &[u8]) -> String;
}

pub struct XmlTester {
    xmlts_root: PathBuf,
}

impl XmlTester {
    pub fn new() -> Self {
        let xmlts_root = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join("../../../../components/xts/xmlts20130923/xmlconf")
            .canonicalize()
            .unwrap();
        Self { xmlts_root }
    }

    pub fn test(&self, parser: &dyn TestableParser) {
        let file = File::open(self.xmlts_root.join("xmlconf.complete.xml")).unwrap();
        let test_suite: TestSuite = from_reader(BufReader::new(file)).unwrap();

        println!("PROFILE: {}", &test_suite.profile);
        self.process_test_cases(parser, &test_suite.test_cases, &self.xmlts_root);
    }

    fn process_test_cases(&self, parser: &dyn TestableParser, tcs: &[TestCases], base: &Path) {
        for tc in tcs {
            let next_base = base.join(&tc.base);
            println!("// Test case: {} {}", tc.base, tc.profile);

            let mut id = tc.base.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
            if id.is_empty() {
                id = tc
                    .profile
                    .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
                    .to_string();
            }

            println!("PROFILE: {}", tc.profile);
            println!("TEST: {}", id);

            self.process_test_cases(parser, &tc.test_cases, &next_base);

            for test in &tc.tests {
                self.process_test(parser, test, &next_base);
            }
        }
    }

    fn process_test(&self, parser: &dyn TestableParser, test: &Test, base: &Path) {
        let path = base.join(&test.uri);

        println!(
            "TEST: {}: {}",
            test.id,
            test.description[0].replace('\n', " ")
        );
        let content = fs::read(path).unwrap();
        match test.ty {
            Type::Valid => (),
            Type::Invalid => (),
            Type::Error => (),
            Type::NotWf => assert!(parser.check_wf(&content), "FAILED: {}", test.id),
        };
    }
}
