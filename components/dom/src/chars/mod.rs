use std::convert::TryFrom;

mod char_maps;

fn lookup_char_u16(c: char, map: &[(u16, u16)]) -> bool {
    if let Ok(code) = u16::try_from(c as u32) {
        for (from, to) in map {
            if code < *to {
                return code >= *from;
            }
        }
    }

    false
}

fn lookup_char_u32(c: char, map: &[(u32, u32)]) -> bool {
    let code = c as u32;
    for (from, to) in map {
        if code < *to {
            return code >= *from;
        }
    }

    false
}

pub trait XmlByteExt {
    fn is_xml_whitespace(&self) -> bool;
}

impl XmlByteExt for u8 {
    fn is_xml_whitespace(&self) -> bool {
        matches!(*self, b'\t' | b'\n' | b'\r' | b' ')
    }
}

pub trait XmlBytesExt {
    fn only_xml_whitespace(&self) -> bool;
}

impl XmlBytesExt for &[u8] {
    fn only_xml_whitespace(&self) -> bool {
        self.iter().all(XmlByteExt::is_xml_whitespace)
    }
}

pub trait XmlStrExt {
    fn is_xml_name(&self) -> bool;
    fn find_invalid_xml_char(&self) -> Option<(usize, char)>;
}

impl XmlStrExt for &str {
    fn is_xml_name(&self) -> bool {
        let mut chars = self.chars();

        match chars.next() {
            Some(c) if lookup_char_u16(c, &char_maps::START_NAME_CHAR) => (),
            _ => return false,
        }

        chars.all(|c| lookup_char_u16(c, &char_maps::NAME_CHAR))
    }

    fn find_invalid_xml_char(&self) -> Option<(usize, char)> {
        self.char_indices()
            .find(|(i, c)| !lookup_char_u32(*c, &char_maps::CHAR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod is_xml_name {
        use super::*;

        #[test]
        fn empty() {
            assert_eq!(false, "".is_xml_name())
        }

        #[test]
        fn digit_start() {
            assert_eq!(false, "1sd".is_xml_name());
        }

        #[test]
        fn dot() {
            assert_eq!(false, ".".is_xml_name());
        }

        #[test]
        fn at() {
            assert_eq!(false, "A@".is_xml_name());
        }

        #[test]
        fn hashtag() {
            assert_eq!(false, "A#".is_xml_name());
        }

        #[test]
        fn without_namespace() {
            assert_eq!(true, "type67".is_xml_name());
        }

        #[test]
        fn with_namespace() {
            assert_eq!(true, "xsi:type".is_xml_name());
        }

        #[test]
        fn nonalpha() {
            assert_eq!(true, "_:_.-_".is_xml_name());
        }
    }
}
