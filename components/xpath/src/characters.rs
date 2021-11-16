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

pub fn is_name_start_char(c: char) -> bool {
    binary_search_table(c, XML_START_CHAR_TABLE)
}

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

pub fn is_name_continue_char(c: char) -> bool {
    binary_search_table(c, XML_CONTINUE_CHAR_TABLE)
}

pub fn is_whitespace_byte(ch: u8) -> bool {
    ch == b'\x20' || ch == b'\x09' || ch == b'\x0D' || ch == b'\x0A'
}

pub fn is_punct_char(ch: u8) -> bool {
    b"/()[].@,:*+-=!<>$".contains(&ch)
}
