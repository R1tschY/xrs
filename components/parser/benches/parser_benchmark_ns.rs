use criterion::{black_box, criterion_group, criterion_main, Criterion};
use quick_xml::events::Event as QXEvent;
use quick_xml::Reader as QXReader;
use std::collections::HashMap;
use xml::reader::{EventReader as XReader, XmlEvent as XEvent};
use xml_parser::{NsReader, XmlNsEvent};

fn from_utf8(i: &[u8]) -> String {
    std::str::from_utf8(i).unwrap().to_string()
}

fn parse_using_quick_xml(
    input: &[u8],
) -> (
    HashMap<(Option<String>, String), usize>,
    HashMap<(Option<String>, String), usize>,
) {
    let mut buf = Vec::new();
    let mut nsbuf = Vec::new();
    let mut reader = QXReader::from_reader(input);
    reader.check_comments(true);
    reader.check_end_names(true);
    reader.expand_empty_elements(true);
    let mut elems = HashMap::new();
    let mut attrs = HashMap::new();

    loop {
        match reader.read_namespaced_event(&mut buf, &mut nsbuf).unwrap() {
            (ns, QXEvent::Start(stag)) => {
                elems
                    .entry((ns.map(|ns| from_utf8(ns)), from_utf8(stag.local_name())))
                    .and_modify(|v| *v += 1)
                    .or_insert(0);

                for attr in stag.attributes().with_checks(true) {
                    let attr = attr.unwrap();
                    let (uri, name) = reader.attribute_namespace(attr.key, &mut nsbuf);
                    attrs
                        .entry((uri.map(|uri| from_utf8(uri)), from_utf8(name)))
                        .and_modify(|v| *v += 1)
                        .or_insert(0);
                }
            }
            (_, QXEvent::Eof) => break,
            _ => {}
        }
    }

    (elems, attrs)
}

fn parse_using_xml_rs(
    input: &[u8],
) -> (
    HashMap<(Option<String>, String), usize>,
    HashMap<(Option<String>, String), usize>,
) {
    let reader = XReader::new(input);
    let mut elems = HashMap::new();
    let mut attrs = HashMap::new();

    for evt in reader {
        if let XEvent::StartElement {
            name, attributes, ..
        } = evt.unwrap()
        {
            elems
                .entry((name.namespace, name.local_name))
                .and_modify(|v| *v += 1)
                .or_insert(0);

            for attr in attributes {
                attrs
                    .entry((attr.name.namespace, attr.name.local_name))
                    .and_modify(|v| *v += 1)
                    .or_insert(0);
            }
        }
    }

    (elems, attrs)
}

fn parse_using_xrs(
    input: &[u8],
) -> (
    HashMap<(Option<String>, String), usize>,
    HashMap<(Option<String>, String), usize>,
) {
    let mut reader = NsReader::new(std::str::from_utf8(input).unwrap());
    let mut elems = HashMap::new();
    let mut attrs = HashMap::new();

    while let Some(evt) = reader.next().unwrap() {
        if let XmlNsEvent::STag(stag) = evt {
            elems
                .entry((
                    reader
                        .resolve_element_namespace(&stag.qname)
                        .unwrap()
                        .map(|s| s.to_string()),
                    stag.qname.local_part.into_owned(),
                ))
                .and_modify(|v| *v += 1)
                .or_insert(0);

            for attr in reader.attributes() {
                attrs
                    .entry((
                        reader
                            .resolve_attribute_namespace(&attr.qname)
                            .unwrap()
                            .map(|s| s.to_string()),
                        attr.qname.local_part.as_ref().to_string(),
                    ))
                    .and_modify(|v| *v += 1)
                    .or_insert(0);
            }
        }
    }

    (elems, attrs)
}

const GPX: &'static [u8] = include_bytes!("4218078.gpx");
const ATOM_FEED: &'static [u8] = include_bytes!("atom.xml");
const XHTML: &'static [u8] = include_bytes!("Sample XHTML 1.0 document.xml");

pub fn gpx_benchmark(c: &mut Criterion) {
    c.bench_function("gpx xrs", |b| b.iter(|| parse_using_xrs(black_box(GPX))));
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
    c.bench_function("atom quick-xml", |b| {
        b.iter(|| parse_using_quick_xml(black_box(ATOM_FEED)))
    });
    c.bench_function("atom xml-rs", |b| {
        b.iter(|| parse_using_xml_rs(black_box(ATOM_FEED)))
    });
}

pub fn xhtml_benchmark(c: &mut Criterion) {
    c.bench_function("XHTML xrs", |b| {
        b.iter(|| parse_using_xrs(black_box(XHTML)))
    });
    c.bench_function("XHTML quick-xml", |b| {
        b.iter(|| parse_using_quick_xml(black_box(XHTML)))
    });
    c.bench_function("XHTML xml-rs", |b| {
        b.iter(|| parse_using_xml_rs(black_box(XHTML)))
    });
}

criterion_group!(benches, gpx_benchmark, xhtml_benchmark, atom_benchmark);
criterion_main!(benches);
