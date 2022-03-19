use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use xml_xts::{Test, TestCases, TestSuite, Type, XmlTester};

fn process_test<W: Write>(
    writer: &mut W,
    test: &Test,
    indent: &str,
    base: &Path,
) -> Result<(), Box<dyn Error>> {
    // ARRANGE
    writeln!(writer, "{}#[test]", indent)?;
    if matches!(test.ty, Type::Error) {
        writeln!(
            writer,
            "{}#[ignore = \"error tests not supported yet\"]",
            indent
        )?;
    }
    writeln!(
        writer,
        "{}fn {}() {{",
        indent,
        test.id.replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    )?;
    writeln!(
        writer,
        "{}  let test: Test = serde_json::from_str({:?}).unwrap();",
        indent,
        serde_json::to_string(test).unwrap(),
    )?;
    writeln!(
        writer,
        "{}  XmlTester::execute_test(&ReaderIT, &test, &Path::new({:?}));",
        indent, base
    )?;
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
    let tester = XmlTester::new();
    let test_suite: TestSuite = tester.parse_test_suite();

    let out_dir: PathBuf = std::env::var_os("OUT_DIR").unwrap().into();
    let mut writer = File::create(out_dir.join("xts.rs"))?;

    writeln!(&mut writer, "/// {}", &test_suite.profile)?;
    writeln!(&mut writer)?;
    process_test_cases(&mut writer, &test_suite.test_cases, "", tester.xmlts_root())?;
    Ok(())
}
