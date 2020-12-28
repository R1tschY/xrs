use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use serde::export::fmt::Debug;
use serde::Deserialize;
use serde_explicit_xml::from_reader;
use std::error::Error;

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

fn process_test<W: Write>(
    writer: &mut W,
    test: &Test,
    indent: &str,
    base: &Path,
) -> Result<(), Box<dyn Error>> {
    // ARRANGE
    let path = base.join(&test.uri);

    writeln!(
        writer,
        "{}/// {}",
        indent,
        test.description[0].replace('\n', " ")
    )?;
    writeln!(writer, "{}#[test]", indent)?;
    if !matches!(test.ty, Type::NotWf) {
        writeln!(
            writer,
            "{}#[ignore = \"only not-wf tests supported\"]",
            indent
        )?;
    }
    writeln!(
        writer,
        "{}fn {}() {{",
        indent,
        test.id.replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    )?;
    writeln!(writer, "{}  // ARRANGE", indent)?;
    writeln!(
        writer,
        "{}  let content = fs::read({:?}).unwrap();",
        indent, path
    )?;
    writeln!(
        writer,
        "{}  let reader = QuickXmlDomReader::new(&content, WellFormedValidatorBuilder);",
        indent
    )?;
    writeln!(writer, "{}  // ACT", indent)?;
    writeln!(writer, "{}  let result = reader.parse();", indent)?;
    writeln!(writer, "{}  // ASSERT", indent)?;
    writeln!(writer, "{}  let err = result.err();", indent)?;
    match test.ty {
        Type::Valid => (),
        Type::Invalid => (),
        Type::Error => (),
        Type::NotWf => {
            writeln!(
                writer,
                "{}  assert_eq!(err.as_ref().map(|err| err.is_not_wf()), Some(true));",
                indent
            )?;
            writeln!(writer, "{}  println!(\"Error: {{:?}}\", err);", indent)?;
        }
    };
    writeln!(writer, "{}}}\n", indent)?;
    Ok(())
}

fn process_test_cases<W: Write>(
    writer: &mut W,
    tcs: &[TestCases],
    indent: &str,
    base: &Path,
) -> Result<(), Box<dyn Error>> {
    let next_indent = indent.to_string() + "  ";
    for tc in tcs {
        let next_base = base.join(&tc.base);
        writeln!(writer, "{}// Test case: {} {}", indent, tc.base, tc.profile)?;

        let mut id = tc.base.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
        if id.is_empty() {
            id = tc
                .profile
                .replace(|c: char| !c.is_ascii_alphanumeric(), "_")
                .to_string();
        }

        writeln!(writer, "{}/// {}", indent, tc.profile)?;
        writeln!(writer, "{}mod {} {{", indent, id)?;
        writeln!(writer, "{}  use super::*;", indent)?;

        process_test_cases(writer, &tc.test_cases, &next_indent, &next_base)?;

        for test in &tc.tests {
            process_test(writer, test, &next_indent, &next_base)?;
        }

        writeln!(writer, "{}}}\n", indent)?;
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let base_dir = std::env::current_exe()?
        .parent()
        .unwrap()
        .join("../../../../components/xts/xmlts20130923/xmlconf")
        .canonicalize()?;

    let file = File::open(base_dir.join("xmlconf.complete.xml"))?;
    let test_suite: TestSuite = from_reader(BufReader::new(file))?;

    let out_dir: PathBuf = std::env::var_os("OUT_DIR").unwrap().into();
    let mut writer = File::create(out_dir.join("xts_dom.rs"))?;

    writeln!(&mut writer, "/// {}", &test_suite.profile)?;
    writeln!(
        &mut writer,
        "use xml_dom::reader::quick_xml::QuickXmlDomReader;"
    )?;
    writeln!(&mut writer, "use xml_dom::reader::DomReader;")?;
    writeln!(
        &mut writer,
        "use xml_dom::validate::WellFormedValidatorBuilder;"
    )?;
    writeln!(&mut writer, "use std::fs;")?;
    writeln!(&mut writer, "")?;
    process_test_cases(&mut writer, &test_suite.test_cases, "", &base_dir)?;
    Ok(())
}
