use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufReader;
use std::panic::RefUnwindSafe;
use std::path::{Path, PathBuf};
use std::{fs, panic};

use serde::{Deserialize, Serialize};
use xserde::from_reader;

#[derive(Deserialize, Serialize)]
#[serde(rename = "TESTSUITE")]
pub struct TestSuite {
    #[serde(rename = "@PROFILE", default)]
    pub profile: String,

    #[serde(rename = "TESTCASES")]
    pub test_cases: Vec<TestCases>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename = "TESTCASES")]
pub struct TestCases {
    #[serde(rename = "TEST", default)]
    pub tests: Vec<Test>,

    #[serde(rename = "TESTCASES", default)]
    pub test_cases: Vec<TestCases>,

    #[serde(rename = "@xml:base", default)]
    pub base: String,

    #[serde(rename = "@PROFILE", default)]
    pub profile: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename = "TEST")]
pub struct Test {
    #[serde(rename = "@ENTITIES", default)]
    pub entities: Entities,

    #[serde(rename = "@ID")]
    pub id: String,

    #[serde(rename = "@OUTPUT", default)]
    pub output: Option<String>,

    #[serde(rename = "@OUTPUT3", default)]
    pub output3: Option<String>,

    #[serde(rename = "@SECTIONS")]
    pub sections: String,

    #[serde(rename = "@RECOMMENDATION", default)]
    pub recommendation: Recommendation,

    #[serde(rename = "@TYPE")]
    pub ty: Type,

    #[serde(rename = "@VERSION", default)]
    pub version: Option<String>,

    #[serde(rename = "@EDITION", default)]
    pub edition: String,

    #[serde(rename = "@URI")]
    pub uri: String,

    #[serde(rename = "@NAMESPACE", default = "yes")]
    pub namespace: YesNo,

    #[serde(rename = "$value")]
    pub description: Vec<String>, // TODO: that should not be a Vec
}

#[derive(Deserialize, Serialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum YesNo {
    #[serde(rename = "yes")]
    Yes,
    #[serde(rename = "no")]
    No,
}

pub fn yes() -> YesNo {
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

#[derive(Deserialize, Serialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Hash, Eq, PartialEq, Copy, Clone)]
pub enum Entities {
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

#[derive(Deserialize, Serialize, Debug, Hash, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Recommendation {
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
    Xml_1_0_errata3e,
    #[serde(rename = "XML1.0-errata4e")]
    Xml_1_0_errata4e,
    #[serde(rename = "NS1.0-errata1e")]
    Ns_1_0_errata1e,
}

impl Default for Recommendation {
    fn default() -> Self {
        Recommendation::Xml_1_0
    }
}

pub trait TestableParser {
    fn check_well_formed(&self, input: &[u8], namespace: bool) -> Result<(), (String, usize)>;
    fn canonxml(&self, input: &[u8], namespace: bool) -> Result<String, Box<dyn Debug>>;
}

pub struct XmlTester {
    xmlts_root: PathBuf,
}

impl Default for XmlTester {
    fn default() -> Self {
        XmlTester::new()
    }
}

fn offset_to_line_and_column(text: &[u8], offset: usize) -> Option<(usize, usize)> {
    let mut cr = false;
    let mut line = 1;
    let mut line_start = 0;

    for (i, c) in text[..offset].iter().enumerate() {
        match c {
            b'\r' if cr => {
                line_start = i + 1;
                line += 1;
            }
            b'\r' => {
                cr = true;
            }
            b'\n' => {
                line_start = i + 1;
                line += 1;
                cr = false;
            }
            _ if cr => {
                line_start = i;
                line += 1;
                cr = false;
            }
            _ => cr = false,
        }
    }

    Some((line, offset - line_start + 1))
}

impl XmlTester {
    pub fn new() -> Self {
        Self {
            xmlts_root: Path::new(env!("CARGO_MANIFEST_DIR")).join("xmlts20130923/xmlconf"),
        }
    }

    pub fn parse_test_suite(&self) -> TestSuite {
        let file = File::open(self.xmlts_root.join("xmlconf.complete.xml")).unwrap();
        from_reader(BufReader::new(file)).unwrap()
    }

    pub fn test(&self, parser: &(dyn TestableParser + RefUnwindSafe)) -> XmlConfirmReport {
        let test_suite: TestSuite = self.parse_test_suite();
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
        parser: &(dyn TestableParser + RefUnwindSafe),
        tcs: &[TestCases],
        base: &Path,
    ) {
        for tc in tcs {
            let next_base = base.join(&tc.base);
            println!("# Test case: {} {}", tc.base, tc.profile);

            let mut subreport = XmlConfirmReport::new(&tc.profile);
            self.process_test_cases(&mut subreport, parser, &tc.test_cases, &next_base);

            for test in &tc.tests {
                self.process_test(&mut subreport, parser, test, &next_base);
            }

            report.statistic.merge_with(&subreport.statistic);
            for (ty, ty_stat) in &subreport.type_statistic {
                report
                    .type_statistic
                    .entry(*ty)
                    .and_modify(|s| s.merge_with(ty_stat))
                    .or_insert_with(|| ty_stat.clone());
            }
            report.subtests.push(subreport);
        }
    }

    fn process_test(
        &self,
        report: &mut XmlConfirmReport,
        parser: &(dyn TestableParser + RefUnwindSafe),
        test: &Test,
        base: &Path,
    ) {
        println!("## {}", test.uri);

        let success = panic::catch_unwind(|| Self::execute_test(parser, test, base)).is_ok();

        report.results.push(XmlTestResult {
            name: test.uri.to_string(),
            description: test.description[0].replace('\n', " "),
            ty: test.ty,
            namespace: test.namespace.into(),
            success,
        });

        report.statistic.inc_result(success);
        report
            .type_statistic
            .entry(test.ty)
            .and_modify(|s| s.inc_result(success))
            .or_default();
    }

    pub fn execute_test(parser: &dyn TestableParser, test: &Test, base: &Path) {
        let path = base.join(&test.uri);
        let content = fs::read(&path).unwrap();

        let well_formed = parser.check_well_formed(&content, test.namespace.into());

        match well_formed.clone() {
            Err((message, _)) if message == "<IGNORE>" => return,
            _ => (),
        }

        match test.ty {
            Type::Valid | Type::Invalid => match well_formed {
                Ok(()) => (),
                Err((message, offset)) => {
                    let (line, column) = offset_to_line_and_column(&content, offset).unwrap();
                    assert!(
                        false,
                        "{}:{}:{}: should be well-formed ({}) [{}]: {}",
                        path.display(),
                        line,
                        column,
                        &test.description[0],
                        &test.sections,
                        message
                    );
                }
            },
            Type::Error => return,
            Type::NotWf => assert!(
                well_formed.is_err(),
                "{}:0:0: should not be well-formed ({}) [{}]",
                path.display(),
                &test.description[0],
                &test.sections
            ),
        };

        if let Some(output) = &test.output {
            match parser.canonxml(&content, test.namespace.into()) {
                Ok(out) => {
                    let out_path = base.join(&output);
                    let out_content = fs::read(out_path).unwrap();
                    assert_eq!(
                        out,
                        std::str::from_utf8(&out_content).unwrap(),
                        "{}:0:0",
                        path.display()
                    );
                }
                Err(err) => {
                    panic!("{:?}", err)
                }
            }
        }
    }

    pub fn xmlts_root(&self) -> &Path {
        &self.xmlts_root
    }
}

pub struct XmlTestResult {
    pub name: String,
    pub description: String,
    pub ty: Type,
    pub namespace: bool,
    pub success: bool,
}

#[derive(Clone, Default)]
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

pub struct XmlConfirmReport {
    pub name: String,
    pub results: Vec<XmlTestResult>,

    pub subtests: Vec<XmlConfirmReport>,

    pub statistic: TestStatistic,
    pub type_statistic: HashMap<Type, TestStatistic>,
}

impl XmlConfirmReport {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            results: Vec::new(),
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

    fn print_single_statistic(name: &'static str, stat: &TestStatistic) {
        let success = stat.count - stat.failed;
        println!(
            "{:20}: {:5} / {:5} ({:3.2}%)",
            name,
            success,
            stat.count,
            if stat.count > 0 {
                (success as f32) / (stat.count as f32) * 100.0
            } else {
                0.0
            }
        );
    }

    pub fn print_statistic(&self) {
        println!(
            "{} ({}/{})",
            self.name,
            self.statistic.count - self.statistic.failed,
            self.statistic.count
        );
        println!();

        let mut failures_by_type = HashMap::new();
        self.compute_failures_by_type(&mut failures_by_type);
        println!("FAILURES BY TYPE");
        println!("----------------\n");
        Self::print_single_statistic(
            "NOT-WF",
            failures_by_type
                .get(&Type::NotWf)
                .unwrap_or(&TestStatistic::default()),
        );
        Self::print_single_statistic(
            "Valid",
            failures_by_type
                .get(&Type::Valid)
                .unwrap_or(&TestStatistic::default()),
        );
        Self::print_single_statistic(
            "Invalid",
            failures_by_type
                .get(&Type::Invalid)
                .unwrap_or(&TestStatistic::default()),
        );
        Self::print_single_statistic(
            "Error",
            failures_by_type
                .get(&Type::Error)
                .unwrap_or(&TestStatistic::default()),
        );
        println!();

        let mut failures_by_namespace = HashMap::new();
        self.compute_failures_by_namespace(&mut failures_by_namespace);
        println!("FAILURES BY NAMESPACE");
        println!("---------------------\n");
        Self::print_single_statistic(
            "NAMESPACE",
            failures_by_namespace
                .get(&true)
                .unwrap_or(&TestStatistic::default()),
        );
        Self::print_single_statistic(
            "NO NAMESPACE",
            failures_by_namespace
                .get(&false)
                .unwrap_or(&TestStatistic::default()),
        );
    }

    pub fn print(&self) {
        let mut res = String::new();
        self.print_internal(&mut res, 0);
        print!("{}", res);
    }

    fn print_internal(&self, writer: &mut String, indention: usize) {
        use std::fmt::Write;

        writeln!(
            writer,
            "{}- {} ({}/{})",
            " ".repeat(indention),
            self.name,
            self.statistic.count - self.statistic.failed,
            self.statistic.count
        )
        .unwrap();

        for report in &self.subtests {
            report.print_internal(writer, indention + 2);
        }

        for result in &self.results {
            if !result.success {
                writeln!(
                    writer,
                    "{}- FAILED: {}",
                    " ".repeat(indention + 2),
                    result.name,
                )
                .unwrap();
            }
        }
    }

    fn compute_failures_by_type(&self, failures: &mut HashMap<Type, TestStatistic>) {
        for result in &self.results {
            failures
                .entry(result.ty)
                .or_insert_with(TestStatistic::default)
                .inc_result(result.success);
        }

        for report in &self.subtests {
            report.compute_failures_by_type(failures)
        }
    }

    fn compute_failures_by_namespace(&self, failures: &mut HashMap<bool, TestStatistic>) {
        for result in &self.results {
            failures
                .entry(result.namespace)
                .or_insert_with(TestStatistic::default)
                .inc_result(result.success);
        }

        for report in &self.subtests {
            report.compute_failures_by_namespace(failures)
        }
    }
}
