use std::fmt::Write;
pub mod full;
pub mod quick_xml;
pub mod simple;

pub trait Processor {
    fn check_wf(&self, xml: &str) -> Result<(), String>;
    fn norm(&self, xml: &str) -> Result<String, String>;
}

pub fn escape(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    for ch in xml.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\'' => out.push_str("&apos;"),
            '"' => out.push_str("&quot;"),
            '\x0D' => out.push_str("&#xD;"),
            c if !c.is_ascii() => {
                let _ = write!(&mut out, "&#x{:X};", c as u32);
            }
            _ => out.push(ch),
        }
    }
    out
}
