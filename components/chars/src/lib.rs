use std::cmp::Ordering::{Equal, Greater, Less};

use crate::Category::{Char, Name, NameStart, PubId, Punct, Whitespace};

fn search_table(c: char, table: &[(char, char)]) -> bool {
    table.iter().any(|rng| c >= rng.0 && c <= rng.1)
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

#[repr(u8)]
enum Category {
    Whitespace = 0,
    Char = 1,
    NameStart = 2,
    Name = 3,
    Punct = 4,
    PubId = 5,
}

#[inline]
fn check_ascii(c: u8, cat: Category) -> bool {
    match XML_CHAR_MAP.get(c as usize) {
        Some(cats) if cats & mask(cat) != 0 => true,
        _ => false,
    }
}

#[inline]
const fn mask(cat: Category) -> u8 {
    1 << (cat as u8)
}

const fn mask_if(cat: Category, pred: bool) -> u8 {
    if pred {
        mask(cat)
    } else {
        0
    }
}

const fn ascii_char_mask(c: u8) -> u8 {
    mask_if(
        Whitespace,
        c == b'\x20' || c == b'\x09' || c == b'\x0D' || c == b'\x0A',
    ) | mask_if(
        Punct,
        c == b'/'
            || c == b'('
            || c == b')'
            || c == b'['
            || c == b']'
            || c == b'.'
            || c == b'@'
            || c == b','
            || c == b':'
            || c == b'*'
            || c == b'+'
            || c == b'-'
            || c == b'='
            || c == b'!'
            || c == b'<'
            || c == b'>'
            || c == b'$',
    ) | mask_if(
        NameStart,
        c == b':' || c == b'_' || (c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z'),
    ) | mask_if(
        Name,
        c == b'-'
            || c == b'.'
            || c == b':'
            || c == b'_'
            || (c >= b'0' && c <= b'9')
            || (c >= b'A' && c <= b'Z')
            || (c >= b'a' && c <= b'z'),
    ) | mask_if(
        Char,
        c >= b'\x20' || c == b'\x09' || c == b'\x0D' || c == b'\x0A',
    ) | mask_if(
        PubId,
        matches!(
            c,
            b'\x0a'
            | b'\x0d'
            | b'\x20'..=b'\x21'
            | b'\x23'..=b'\x25'
            | b'\x27'..=b'\x3B'
            | b'\x3D'
            | b'\x3F'..=b'\x5A'
            | b'\x5F'..=b'\x5F'
            | b'\x61'..=b'\x7A'),
    )
}

macro_rules! ascii_char_mask {
    ($( $c:expr ),*) => {
        [ $( ascii_char_mask($c) ),* ]
    };
}

const XML_CHAR_MAP: [u8; 127] = ascii_char_mask![
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126
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

    /// `PubidChar ::= #x20 | #xD | #xA | [a-zA-Z0-9] | [-'()+,./:=?;!*#@$_%]`
    fn is_xml_pubid_char(&self) -> bool;
}

impl XmlAsciiChar for u8 {
    #[inline]
    fn is_xml_whitespace(&self) -> bool {
        *self == b'\x20' || *self == b'\x0A' || *self == b'\x09' || *self == b'\x0D'
    }

    #[inline]
    fn is_xml_punct(&self) -> bool {
        check_ascii(*self as u8, Punct)
    }
}

impl XmlAsciiChar for char {
    #[inline]
    fn is_xml_whitespace(&self) -> bool {
        self.is_ascii() && check_ascii(*self as u8, Whitespace)
    }

    #[inline]
    fn is_xml_punct(&self) -> bool {
        self.is_ascii() && check_ascii(*self as u8, Punct)
    }
}

impl XmlChar for char {
    #[inline]
    fn is_xml_name_start_char(&self) -> bool {
        if self.is_ascii() {
            check_ascii(*self as u8, NameStart)
        } else {
            search_table(*self, XML_START_CHAR_TABLE)
        }
    }

    #[inline]
    fn is_xml_name_char(&self) -> bool {
        if self.is_ascii() {
            check_ascii(*self as u8, Name)
        } else {
            search_table(*self, XML_CONTINUE_CHAR_TABLE)
        }
    }

    #[inline]
    fn is_xml_char(&self) -> bool {
        if self.is_ascii() {
            *self >= '\u{20}' || *self == '\x09' || *self == '\x0D' || *self == '\x0A'
        } else {
            search_table(*self, XML_CHAR)
        }
    }

    fn is_xml_pubid_char(&self) -> bool {
        self.is_ascii() && check_ascii(*self as u8, PubId)
    }
}
