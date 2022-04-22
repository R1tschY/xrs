use xml_chars::XmlChar;

#[repr(u8)]
enum Category {
    ContentChar = 0,
}

#[inline]
fn check_ascii(c: u8, cat: Category) -> bool {
    match PARSER_CHAR_MAP.get(c as usize) {
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
        Category::ContentChar,
        // (#x9 | #xA | #xD | [#x20-#x7F]) - (#xD | '&' | '<')
        c == b'\x09' || c == b'\x0A' || (c >= b'\x20' && c <= b'\x7F' && c != b'&' && c != b'<'),
    )
}

macro_rules! ascii_char_mask {
    ($( $c:expr ),*) => {
        [ $( ascii_char_mask($c) ),* ]
    };
}

const PARSER_CHAR_MAP: [u8; 127] = ascii_char_mask![
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97,
    98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    117, 118, 119, 120, 121, 122, 123, 124, 125, 126
];

#[inline]
pub fn is_ascii_content_char(c: char) -> bool {
    if c.is_ascii() {
        check_ascii(c as u8, Category::ContentChar)
    } else {
        c.is_xml_char()
    }
}
