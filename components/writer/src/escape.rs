use std::borrow::Cow;
use std::{fmt, io};

use crate::UnicodeWrite;

/// Strategy to escape character data
///
/// see https://www.w3.org/TR/REC-xml/#dt-chardata
pub trait Escape {
    fn escape_content<W: UnicodeWrite>(&self, input: &str, write: &mut W) -> io::Result<()>;
    fn escape_attr_value_apos<W: UnicodeWrite>(&self, input: &str, write: &mut W)
        -> io::Result<()>;
    fn escape_attr_value_quot<W: UnicodeWrite>(&self, input: &str, write: &mut W)
        -> io::Result<()>;
}

pub struct MinimalEscaper;

impl Escape for MinimalEscaper {
    fn escape_content<W: UnicodeWrite>(&self, mut input: &str, write: &mut W) -> io::Result<()> {
        let mut p = 0;
        for (i, r) in input.match_indices(|c: char| c == '&' || c == '<' || c == '>') {
            write.write_all(&input[p..i])?;
            if r == "&" {
                write.write_all("&amp;")?;
            } else if r == "<" {
                write.write_all("&lt;")?;
            } else if input[..i].ends_with("]]") {
                write.write_all("&gt;")?;
            } else {
                write.write_all(r)?;
            }
            p = i + 1;
        }
        write.write_all(if p == 0 { input } else { &input[p..] })
    }

    fn escape_attr_value_apos<W: UnicodeWrite>(
        &self,
        mut input: &str,
        write: &mut W,
    ) -> io::Result<()> {
        let mut p = 0;
        for (i, r) in input.match_indices(|c: char| c == '&' || c == '<' || c == '\'') {
            write.write_all(&input[p..i])?;
            if r == "&" {
                write.write_all("&amp;")?;
            } else if r == "<" {
                write.write_all("&lt;")?;
            } else {
                write.write_all("&apos;")?;
            }
            p = i + 1;
        }
        write.write_all(if p == 0 { input } else { &input[p..] })
    }

    fn escape_attr_value_quot<W: UnicodeWrite>(
        &self,
        mut input: &str,
        write: &mut W,
    ) -> io::Result<()> {
        let mut p = 0;
        for (i, r) in input.match_indices(|c: char| c == '&' || c == '<' || c == '\"') {
            write.write_all(&input[p..i])?;
            if r == "&" {
                write.write_all("&amp;")?;
            } else if r == "<" {
                write.write_all("&lt;")?;
            } else {
                write.write_all("&#34;")?;
            }
            p = i + 1;
        }
        write.write_all(if p == 0 { input } else { &input[p..] })
    }
}

pub struct DefaultEscaper;

impl DefaultEscaper {
    fn escape<W: UnicodeWrite>(input: &str, write: &mut W) -> io::Result<()> {
        let mut p = 0;
        for (i, r) in input
            .match_indices(|c: char| c == '>' || c == '<' || c == '&' || c == '\'' || c == '\"')
        {
            write.write_all(unsafe { input.get_unchecked(p..i) })?;
            if r == ">" {
                write.write_all("&gt;")?;
            } else if r == "<" {
                write.write_all("&lt;")?;
            } else if r == "&" {
                write.write_all("&amp;")?;
            } else if r == "\'" {
                write.write_all("&apos;")?;
            } else {
                write.write_all("&quot;")?;
            }
            p = i + 1;
        }
        write.write_all(if p == 0 {
            input
        } else {
            unsafe { input.get_unchecked(p..) }
        })
    }
}

impl Escape for DefaultEscaper {
    fn escape_content<W: UnicodeWrite>(&self, input: &str, write: &mut W) -> io::Result<()> {
        Self::escape(input, write)
    }

    fn escape_attr_value_apos<W: UnicodeWrite>(
        &self,
        mut input: &str,
        write: &mut W,
    ) -> io::Result<()> {
        Self::escape(input, write)
    }

    fn escape_attr_value_quot<W: UnicodeWrite>(
        &self,
        mut input: &str,
        write: &mut W,
    ) -> io::Result<()> {
        Self::escape(input, write)
    }
}

pub struct AsciiEscaper;

impl AsciiEscaper {
    fn escape<W: UnicodeWrite>(&self, input: &str, write: &mut W) -> io::Result<()> {
        let mut p = 0;
        for (i, r) in input.match_indices(|c: char| {
            c == '>' || c == '<' || c == '&' || c == '\'' || c == '\"' || !c.is_ascii()
        }) {
            write.write_all(
                &input[p..i], //unsafe { input.get_unchecked(p..i) }
            )?;
            if r == ">" {
                write.write_all("&gt;")?;
            } else if r == "<" {
                write.write_all("&lt;")?;
            } else if r == "&" {
                write.write_all("&amp;")?;
            } else if r == "\'" {
                write.write_all("&apos;")?;
            } else if r == "\"" {
                write.write_all("&quot;")?;
            } else {
                write.write_fmt(format_args!("&#{};", r.chars().next().unwrap() as u32))?;
            }
            p = i + r.len();
        }
        write.write_all(if p == 0 {
            input
        } else {
            &input[p..]
            //unsafe { input.get_unchecked(p..) }
        })
    }
}

impl Escape for AsciiEscaper {
    fn escape_content<W: UnicodeWrite>(&self, input: &str, write: &mut W) -> io::Result<()> {
        self.escape(input, write)
    }

    fn escape_attr_value_apos<W: UnicodeWrite>(
        &self,
        mut input: &str,
        write: &mut W,
    ) -> io::Result<()> {
        self.escape(input, write)
    }

    fn escape_attr_value_quot<W: UnicodeWrite>(
        &self,
        mut input: &str,
        write: &mut W,
    ) -> io::Result<()> {
        self.escape(input, write)
    }
}

pub fn escape(input: &str) -> String {
    let mut output = String::new();
    output.reserve(output.len());
    DefaultEscaper::escape(input, &mut &mut output).unwrap();
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn escape_content(esc: impl Escape, input: &str) -> String {
        let mut output = String::new();
        output.reserve(output.len());
        esc.escape_content(input, &mut &mut output).unwrap();
        output
    }

    pub fn escape_attr_value(esc: impl Escape, input: &str) -> String {
        let mut output = String::new();
        output.reserve(output.len());
        esc.escape_attr_value_quot(input, &mut &mut output).unwrap();
        output
    }

    mod minimal {
        use super::*;

        #[test]
        fn content_lt() {
            assert_eq!("&lt;", escape_content(MinimalEscaper, "<"));
        }

        #[test]
        fn content_gt() {
            assert_eq!(">", escape_content(MinimalEscaper, ">"));
        }

        #[test]
        fn content_amp() {
            assert_eq!("&amp;", escape_content(MinimalEscaper, "&"));
        }

        #[test]
        fn content_apos() {
            assert_eq!("'", escape_content(MinimalEscaper, "'"));
        }

        #[test]
        fn content_quot() {
            assert_eq!("\"", escape_content(MinimalEscaper, "\""));
        }

        #[test]
        fn content_cdata_end() {
            assert_eq!("]]&gt;", escape_content(MinimalEscaper, "]]>"));
        }

        #[test]
        fn attr_value_lt() {
            assert_eq!("&lt;", escape_attr_value(MinimalEscaper, "<"));
        }

        #[test]
        fn attr_value_gt() {
            assert_eq!(">", escape_attr_value(MinimalEscaper, ">"));
        }

        #[test]
        fn attr_value_amp() {
            assert_eq!("&amp;", escape_attr_value(MinimalEscaper, "&"));
        }

        #[test]
        fn attr_value_apos() {
            assert_eq!("'", escape_attr_value(MinimalEscaper, "'"));
        }

        #[test]
        fn attr_value_quot() {
            assert_eq!("&#34;", escape_attr_value(MinimalEscaper, "\""));
        }

        #[test]
        fn attr_value_cdata_end() {
            assert_eq!("]]>", escape_attr_value(MinimalEscaper, "]]>"));
        }
    }

    mod default {
        use super::*;

        #[test]
        fn lt() {
            assert_eq!("&lt;", escape_content(DefaultEscaper, "<"));
        }

        #[test]
        fn gt() {
            assert_eq!("&gt;", escape_content(DefaultEscaper, ">"));
        }

        #[test]
        fn amp() {
            assert_eq!("&amp;", escape_content(DefaultEscaper, "&"));
        }

        #[test]
        fn apos() {
            assert_eq!("&apos;", escape_content(DefaultEscaper, "'"));
        }

        #[test]
        fn quot() {
            assert_eq!("&quot;", escape_content(DefaultEscaper, "\""));
        }

        #[test]
        fn cdata_end() {
            assert_eq!("]]&gt;", escape_content(DefaultEscaper, "]]>"));
        }
    }

    mod ascii {
        use super::*;

        #[test]
        fn lt() {
            assert_eq!("&lt;", escape_content(AsciiEscaper, "<"));
        }

        #[test]
        fn gt() {
            assert_eq!("&gt;", escape_content(AsciiEscaper, ">"));
        }

        #[test]
        fn amp() {
            assert_eq!("&amp;", escape_content(AsciiEscaper, "&"));
        }

        #[test]
        fn apos() {
            assert_eq!("&apos;", escape_content(AsciiEscaper, "'"));
        }

        #[test]
        fn quot() {
            assert_eq!("&quot;", escape_content(AsciiEscaper, "\""));
        }

        #[test]
        fn esc() {
            assert_eq!("\u{0F}", escape_content(AsciiEscaper, "\u{0F}"));
        }

        #[test]
        fn small_unicode_char() {
            assert_eq!("&#128;", escape_content(AsciiEscaper, "\u{80}"));
        }

        #[test]
        fn large_unicode_char() {
            assert_eq!("&#1114111;", escape_content(AsciiEscaper, "\u{10FFFF}"));
        }
    }
}
