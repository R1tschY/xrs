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
