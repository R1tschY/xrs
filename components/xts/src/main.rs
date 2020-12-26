use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::ops::Add;
use std::panic;
use std::path::PathBuf;

use serde::Deserialize;
use serde_explicit_xml::from_reader;

use xml_dom::DomReader;

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

#[derive(Deserialize, Debug)]
enum YesNo {
    #[serde(rename = "yes")]
    Yes,
    #[serde(rename = "no")]
    No,
}

fn yes() -> YesNo {
    YesNo::Yes
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

fn print_test_result(indent: &str, test: &Test, symbol: &str, message: &str) {
    println!(
        "{}  {} {} ({}) {}",
        indent,
        symbol,
        test.id,
        message,
        test.description[0].replace("\n", " ")
    );
}

fn process_test(test: &Test, indent: &str, base: &str) {
    // ARRANGE
    let path = PathBuf::from(base.to_string() + &test.uri);
    let content = fs::read(path).unwrap();
    let reader = xml_dom::QuickXmlDomReader::new(&content);

    // ACT
    let result = panic::catch_unwind(|| reader.parse());

    // ASSERT
    let result = match result {
        Err(panic) => {
            print_test_result(indent, test, "🔥", &format!("{:?}", panic));
            return;
        }
        Ok(ok) => ok,
    };

    match test.ty {
        Type::Valid => print_test_result(indent, test, "🚫", &"valid test not implement"),
        Type::Invalid => print_test_result(indent, test, "🚫", &"invalid test not implement"),
        Type::Error => print_test_result(indent, test, "🚫", &"error test not implement"),
        Type::NotWf => {
            let success = result.is_err();
            print_test_result(
                indent,
                test,
                if success { "✔️" } else { "❌" },
                &result
                    .err()
                    .map(|err| format!("{:?}", err))
                    .unwrap_or("unexpected OK".to_string()),
            );
        }
    };
}

fn process_test_cases(tcs: &[TestCases], indent: &str, base: &str) {
    let next_indent = indent.to_string() + "  ";

    for tc in tcs {
        let next_base = base.to_string() + &tc.base;
        println!("{}Test case: {} {}", indent, tc.base, tc.profile);

        process_test_cases(&tc.test_cases, &next_indent, &next_base);

        for test in &tc.tests {
            process_test(test, &next_indent, &next_base);
        }
    }
}

fn main() {
    let file = File::open(PathBuf::from("xmlts20130923/xmlconf/xmlconf.complete.xml")).unwrap();
    let test_suite: TestSuite = from_reader(BufReader::new(file)).unwrap();

    println!("Test suite: {}", &test_suite.profile);
    process_test_cases(&test_suite.test_cases, "  ", "xmlts20130923/xmlconf/");
}
