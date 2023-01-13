use std::borrow::Cow;
use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use quick_xml::events::Event as QXEvent;
use quick_xml::Reader as QXReader;
use xml::reader::{EventReader as XReader, XmlEvent as XEvent};

use xrs_parser::simple::{
    AttributeAccess, CowVisitor, SimpleXmlParser, SimpleXmlVisitor, StringVisitor,
};
use xrs_parser::{Reader, XmlDecl, XmlError, XmlEvent};

fn parse_using_quick_xml(input: &[u8]) -> (HashMap<Cow<str>, usize>, HashMap<Cow<str>, usize>) {
    let mut buf = Vec::new();
    let mut reader = QXReader::from_reader(input);
    reader.check_comments(true);
    reader.check_end_names(true);
    reader.expand_empty_elements(true);
    let mut elems = HashMap::new();
    let mut attrs = HashMap::new();

    loop {
        match reader.read_event(&mut buf).unwrap() {
            QXEvent::Start(stag) => {
                elems
                    .entry(std::str::from_utf8(stag.name()).unwrap().to_string().into())
                    .and_modify(|v| *v += 1)
                    .or_insert(0);

                for attr in stag.attributes().with_checks(true) {
                    attrs
                        .entry(
                            std::str::from_utf8(attr.unwrap().key)
                                .unwrap()
                                .to_string()
                                .into(),
                        )
                        .and_modify(|v| *v += 1)
                        .or_insert(0);
                }
            }
            QXEvent::Eof => break,
            _ => {}
        }
    }

    (elems, attrs)
}

fn parse_using_xml_rs(input: &[u8]) -> (HashMap<Cow<str>, usize>, HashMap<Cow<str>, usize>) {
    let reader = XReader::new(input);
    let mut elems = HashMap::new();
    let mut attrs = HashMap::new();

    for evt in reader {
        if let XEvent::StartElement {
            name, attributes, ..
        } = evt.unwrap()
        {
            elems
                .entry(name.to_string().into())
                .and_modify(|v| *v += 1)
                .or_insert(0);

            for attr in attributes {
                attrs
                    .entry(attr.name.to_string().into())
                    .and_modify(|v| *v += 1)
                    .or_insert(0);
            }
        }
    }

    (elems, attrs)
}

fn parse_using_xrs(input: &[u8]) -> (HashMap<Cow<str>, usize>, HashMap<Cow<str>, usize>) {
    let mut reader = Reader::new(std::str::from_utf8(input).unwrap());
    let mut elems = HashMap::new();
    let mut attrs = HashMap::new();

    while let Some(evt) = reader.next().unwrap() {
        if let XmlEvent::STag(stag) = evt {
            elems.entry(stag.name).and_modify(|v| *v += 1).or_insert(0);

            for attr in reader.drain_attributes() {
                attrs.entry(attr.name).and_modify(|v| *v += 1).or_insert(0);
            }
        }
    }

    (elems, attrs)
}

fn parse_using_simple_xrs(input: &[u8]) -> (HashMap<Cow<str>, usize>, HashMap<Cow<str>, usize>) {
    struct Visitor<'i> {
        elems: HashMap<Cow<'i, str>, usize>,
        attrs: HashMap<Cow<'i, str>, usize>,
    }

    impl<'a, 'i> SimpleXmlVisitor<'i> for &'a mut Visitor<'i> {
        type Value = ();

        fn visit_start_element<A: AttributeAccess<'i>>(
            self,
            tag: &'i str,
            mut attrs: A,
        ) -> Result<Self::Value, XmlError> {
            self.elems
                .entry(Cow::Borrowed(tag))
                .and_modify(|v| *v += 1)
                .or_insert(0);

            while let Some((name, value)) = attrs.next_entry(CowVisitor, CowVisitor)? {
                self.attrs.entry(name).and_modify(|v| *v += 1).or_insert(0);
            }

            Ok(())
        }

        fn visit_end_element(self, tag: &'i str) -> Result<Self::Value, XmlError> {
            Ok(())
        }

        fn visit_declaration(self, decl: XmlDecl) -> Result<Self::Value, XmlError> {
            Ok(())
        }

        fn visit_characters(self, characters: &'i str) -> Result<Self::Value, XmlError> {
            Ok(())
        }

        fn visit_borrowed_characters(self, characters: &str) -> Result<Self::Value, XmlError> {
            Ok(())
        }

        fn visit_pi(self, target: &'i str, data: Option<&'i str>) -> Result<Self::Value, XmlError> {
            Ok(())
        }

        fn visit_comment(self, comment: &'i str) -> Result<Self::Value, XmlError> {
            Ok(())
        }
    }

    let mut visitor = Visitor {
        elems: Default::default(),
        attrs: Default::default(),
    };
    let mut parser = SimpleXmlParser::from_str(std::str::from_utf8(input).unwrap());
    while parser.parse_next(&mut visitor).unwrap().is_some() {}

    (visitor.elems, visitor.attrs)
}

const MINIMAL: &'static [u8] = b"<e/>";
const GPX: &'static [u8] = include_bytes!("4218078.gpx");
const ATOM_FEED: &'static [u8] = include_bytes!("atom.xml");

pub fn minimal_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("minimal");
    group.bench_with_input("minimal xrs", MINIMAL, |b, i| {
        b.iter(|| parse_using_xrs(black_box(i)))
    });
    group.bench_with_input("minimal simple xrs", MINIMAL, |b, i| {
        b.iter(|| parse_using_simple_xrs(black_box(i)))
    });
    group.bench_with_input("minimal quick-xml", MINIMAL, |b, i| {
        b.iter(|| parse_using_quick_xml(black_box(i)))
    });
    group.bench_with_input("minimal xml-rs", MINIMAL, |b, i| {
        b.iter(|| parse_using_xml_rs(black_box(i)))
    });
    group.finish();
}

pub fn gpx_benchmark(c: &mut Criterion) {
    c.bench_function("gpx xrs", |b| b.iter(|| parse_using_xrs(black_box(GPX))));
    c.bench_function("gpx simple xrs", |b| {
        b.iter(|| parse_using_simple_xrs(black_box(GPX)))
    });
    c.bench_function("gpx quick-xml", |b| {
        b.iter(|| parse_using_quick_xml(black_box(GPX)))
    });
    c.bench_function("gpx xml-rs", |b| {
        b.iter(|| parse_using_xml_rs(black_box(GPX)))
    });
}

pub fn atom_benchmark(c: &mut Criterion) {
    c.bench_function("atom xrs", |b| {
        b.iter(|| parse_using_xrs(black_box(ATOM_FEED)))
    });
    c.bench_function("atom simple xrs", |b| {
        b.iter(|| parse_using_simple_xrs(black_box(ATOM_FEED)))
    });
    c.bench_function("atom quick-xml", |b| {
        b.iter(|| parse_using_quick_xml(black_box(ATOM_FEED)))
    });
    c.bench_function("atom xml-rs", |b| {
        b.iter(|| parse_using_xml_rs(black_box(ATOM_FEED)))
    });
}

criterion_group!(benches, minimal_benchmark, gpx_benchmark, atom_benchmark);
criterion_main!(benches);
