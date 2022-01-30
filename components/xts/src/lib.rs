use std::collections::HashMap;
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
    output: Option<String>,

    #[serde(rename = "@OUTPUT3", default)]
    output3: Option<String>,

    #[serde(rename = "@SECTIONS")]
    sections: String,

    #[serde(rename = "@RECOMMENDATION", default)]
    recommendation: Recommendation,

    #[serde(rename = "@TYPE")]
    ty: Type,

    #[serde(rename = "@VERSION", default)]
    version: Option<String>,

    #[serde(rename = "@EDITION", default)]
    edition: String,

    #[serde(rename = "@URI")]
    uri: String,

    #[serde(rename = "@NAMESPACE", default = "yes")]
    namespace: YesNo,

    #[serde(rename = "$value")]
    description: Vec<String>,
}

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
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

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum Type {
    #[serde(rename = "valid")]
    Valid,
    #[serde(rename = "invalid")]
    Invalid,
    #[serde(rename = "not-wf")]
    NotWf,
    #[serde(rename = "error")]
    Error,
}

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
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

#[derive(Deserialize, Debug, Hash, Eq, PartialEq)]
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
    fn is_wf(&self, input: &[u8]) -> bool;
    fn canonxml(&self, input: &[u8]) -> Result<String, Box<dyn Debug>>;
}

pub struct XmlTester {
    xmlts_root: PathBuf,
}

impl XmlTester {
    pub fn new() -> Self {
        Self {
            xmlts_root: Path::new(env!("CARGO_MANIFEST_DIR")).join("xmlts20130923/xmlconf"),
        }
    }

    pub fn test(&self, parser: &dyn TestableParser) -> XmlConfirmReport {
        let file = File::open(self.xmlts_root.join("xmlconf.complete.xml")).unwrap();
        let test_suite: TestSuite = from_reader(BufReader::new(file)).unwrap();
        let mut report = XmlConfirmReport::new(&test_suite.profile);

        println!("PROFILE: {}", &test_suite.profile);
        self.process_test_cases(
            &mut report,
            parser,
            &test_suite.test_cases,
            &self.xmlts_root,
        );
        report
    }

    fn process_test_cases(
        &self,
        report: &mut XmlConfirmReport,
        parser: &dyn TestableParser,
        tcs: &[TestCases],
        base: &Path,
    ) {
        for tc in tcs {
            let next_base = base.join(&tc.base);
            println!("// Test case: {} {}", tc.base, tc.profile);

            let mut subreport = XmlConfirmReport::new(&tc.profile);
            self.process_test_cases(&mut subreport, parser, &tc.test_cases, &next_base);

            for test in &tc.tests {
                self.process_test(&mut subreport, parser, test, &next_base);
            }

            report.statistic.merge_with(&subreport.statistic);
            for (ty, ty_stat) in &subreport.type_statistic {
                report
                    .type_statistic
                    .entry(ty.clone())
                    .and_modify(|s| s.merge_with(ty_stat))
                    .or_insert(ty_stat.clone());
            }
            report.subtests.push(subreport);
        }
    }

    fn process_test(
        &self,
        report: &mut XmlConfirmReport,
        parser: &dyn TestableParser,
        test: &Test,
        base: &Path,
    ) {
        println!("##   Test case: {} {}", test.id, test.uri);
        let path = base.join(&test.uri);
        let content = fs::read(path).unwrap();
        let mut success = match test.ty {
            Type::Valid => parser.is_wf(&content),
            Type::Invalid => parser.is_wf(&content),
            Type::Error => return,
            Type::NotWf => !parser.is_wf(&content),
        };
        if let Some(output) = &test.output {
            match parser.canonxml(&content) {
                Ok(out) => {
                    let out_path = base.join(&output);
                    let out_content = fs::read(out_path).unwrap();
                    if out.as_bytes() != out_content {
                        success = false;
                    }
                }
                Err(_err) => {
                    success = false;
                }
            }
        }

        report.test_count += 1;
        if !success {
            report.failed.push(XmlTestFailure {
                name: test.uri.to_string(),
                description: test.description[0].replace('\n', " "),
            });
        }

        report.statistic.inc_result(success);
        report
            .type_statistic
            .entry(test.ty)
            .and_modify(|s| s.inc_result(success))
            .or_default();
    }
}

pub struct XmlTestFailure {
    pub name: String,
    pub description: String,
}

#[derive(Clone)]
pub struct TestStatistic {
    failed: usize,
    count: usize,
}

impl TestStatistic {
    pub fn inc_result(&mut self, success: bool) {
        self.count += 1;
        if !success {
            self.failed += 1;
        }
    }

    pub fn merge_with(&mut self, other: &TestStatistic) {
        self.failed += other.failed;
        self.count += other.count;
    }
}

impl Default for TestStatistic {
    fn default() -> Self {
        Self {
            failed: 0,
            count: 0,
        }
    }
}

pub struct XmlConfirmReport {
    pub name: String,
    pub failed: Vec<XmlTestFailure>,
    pub test_count: usize,

    pub subtests: Vec<XmlConfirmReport>,

    pub statistic: TestStatistic,
    pub type_statistic: HashMap<Type, TestStatistic>,
}

impl XmlConfirmReport {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            failed: Vec::new(),
            test_count: 0,
            subtests: Vec::new(),
            statistic: TestStatistic::default(),
            type_statistic: HashMap::default(),
        }
    }

    pub fn assert(&self) {
        assert_eq!(
            0, self.statistic.failed,
            "There are {} failed tests from {}",
            self.statistic.failed, self.statistic.count,
        );
    }

    pub fn print(&self) {
        let mut res = String::new();
        self.print_internal(&mut res, 0);
        print!("{}", res);
    }

    fn print_internal(&self, writer: &mut String, indention: usize) {
        use std::fmt::Write;

        write!(
            writer,
            "{}|- {} ({}/{})\n",
            " ".repeat(indention),
            self.name,
            self.statistic.count - self.statistic.failed,
            self.statistic.count
        );

        for report in &self.subtests {
            report.print_internal(writer, indention + 2);
        }

        for failed in &self.failed {
            write!(
                writer,
                "{}|- FAILED: {}\n",
                " ".repeat(indention + 2),
                failed.name,
            );
        }
    }
}
