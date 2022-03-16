use std::cmp::Ordering::{Equal, Greater, Less};

fn binary_search_table(c: char, table: &[(char, char)]) -> bool {
    table
        .binary_search_by(|&(low, high)| {
            if c < low {
                Greater
            } else if c > high {
                Less
            } else {
                Equal
            }
        })
        .is_ok()
}

const XML_START_CHAR_TABLE: &[(char, char)] = &[
    (':', ':'),
    ('A', 'Z'),
    ('_', '_'),
    ('a', 'z'),
    ('\u{c0}', '\u{d6}'),
    ('\u{d8}', '\u{f6}'),
    ('\u{f8}', '\u{2ff}'),
    ('\u{370}', '\u{37d}'),
    ('\u{37f}', '\u{1fff}'),
    ('\u{200c}', '\u{200d}'),
    ('\u{2070}', '\u{218f}'),
    ('\u{2C00}', '\u{2FEF}'),
    ('\u{3001}', '\u{D7FF}'),
    ('\u{F900}', '\u{FDCF}'),
    ('\u{FDF0}', '\u{FFFD}'),
    ('\u{10000}', '\u{EFFFF}'),
];

const XML_CONTINUE_CHAR_TABLE: &[(char, char)] = &[
    ('-', '.'),
    ('0', '9'),
    (':', ':'),
    ('A', 'Z'),
    ('_', '_'),
    ('a', 'z'),
    ('\u{b7}', '\u{b7}'),
    ('\u{c0}', '\u{d6}'),
    ('\u{d8}', '\u{f6}'),
    ('\u{f8}', '\u{37d}'),
    ('\u{37f}', '\u{1fff}'),
    ('\u{200c}', '\u{200d}'),
    ('\u{203f}', '\u{2040}'),
    ('\u{2070}', '\u{218f}'),
    ('\u{2C00}', '\u{2FEF}'),
    ('\u{3001}', '\u{D7FF}'),
    ('\u{F900}', '\u{FDCF}'),
    ('\u{FDF0}', '\u{FFFD}'),
    ('\u{10000}', '\u{EFFFF}'),
];

const XML_CHAR: &[(char, char)] = &[
    ('\u{9}', '\u{9}'),
    ('\u{a}', '\u{a}'),
    ('\u{d}', '\u{d}'),
    ('\u{20}', '\u{D7FF}'),
    ('\u{E000}', '\u{FFFD}'),
    ('\u{10000}', '\u{10FFFF}'),
];

pub trait XmlAsciiChar {
    /// https://www.w3.org/TR/REC-xml/#NT-S
    fn is_xml_whitespace(&self) -> bool;

    fn is_xml_punct(&self) -> bool;
}

pub trait XmlChar: XmlAsciiChar {
    /// https://www.w3.org/TR/REC-xml/#NT-NameStartChar
    fn is_xml_name_start_char(&self) -> bool;

    /// https://www.w3.org/TR/REC-xml/#NT-NameChar
    fn is_xml_name_char(&self) -> bool;

    /// https://www.w3.org/TR/REC-xml/#NT-Char
    fn is_xml_char(&self) -> bool;
}

impl XmlAsciiChar for u8 {
    #[inline]
    fn is_xml_whitespace(&self) -> bool {
        *self == b'\x20' || *self == b'\x09' || *self == b'\x0D' || *self == b'\x0A'
    }

    #[inline]
    fn is_xml_punct(&self) -> bool {
        b"/()[].@,:*+-=!<>$".contains(self)
    }
}

impl XmlAsciiChar for char {
    #[inline]
    fn is_xml_whitespace(&self) -> bool {
        *self == '\x20' || *self == '\x09' || *self == '\x0D' || *self == '\x0A'
    }

    #[inline]
    fn is_xml_punct(&self) -> bool {
        "/()[].@,:*+-=!<>$".contains(*self)
    }
}

impl XmlChar for char {
    #[inline]
    fn is_xml_name_start_char(&self) -> bool {
        binary_search_table(*self, XML_START_CHAR_TABLE)
    }

    #[inline]
    fn is_xml_name_char(&self) -> bool {
        binary_search_table(*self, XML_CONTINUE_CHAR_TABLE)
    }

    #[inline]
    fn is_xml_char(&self) -> bool {
        binary_search_table(*self, XML_CHAR)
    }
}
