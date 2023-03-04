use std::fmt;
use std::fmt::Write;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

const SMALL_HIT: &'static str = "no";
const LARGE_NO_HIT: &'static str = include_str!("atom.xml");
const EVIL_NO_HIT: &'static str = "[[dfhdjf]] [[dfdgfj]]";

fn escape_gt1<W: fmt::Write>(input: &str, write: &mut W) -> fmt::Result {
    let mut p = 0;
    for (i, r) in input.match_indices(|c: char| c == '>' || c == '<' || c == '\'' || c == '\"') {
        write.write_str(&input[p..i])?;
        if r == ">" {
            write.write_str("&gt;")?;
        } else if r == "<" {
            write.write_str("&lt;")?;
        } else if r == "\'" {
            write.write_str("&apos;")?;
        } else {
            write.write_str("&quot;")?;
        }
        p = i + 1;
    }
    write.write_str(if p == 0 { input } else { &input[p..] })
}

fn escape_gt2<W: fmt::Write>(input: &str, write: &mut W) -> fmt::Result {
    let mut p = 0;
    for (i, r) in input.match_indices(['>', '<', '\'', '\"']) {
        write.write_str(&input[p..i])?;
        if r == ">" {
            write.write_str("&gt;")?;
        } else if r == "<" {
            write.write_str("&lt;")?;
        } else if r == "\'" {
            write.write_str("&apos;")?;
        } else {
            write.write_str("&quot;")?;
        }
        p = i + 1;
    }
    write.write_str(if p == 0 { input } else { &input[p..] })
}

fn escape_gt3<W: fmt::Write>(mut input: &str, write: &mut W) -> fmt::Result {
    unsafe {
        let mut p = 0;
        for (i, r) in input.match_indices(|c: char| c == '>' || c == '<' || c == '\'' || c == '\"')
        {
            write.write_str(input.get_unchecked(p..i))?;
            if r == ">" {
                write.write_str("&gt;")?;
            } else if r == "<" {
                write.write_str("&lt;")?;
            } else if r == "\'" {
                write.write_str("&apos;")?;
            } else {
                write.write_str("&quot;")?;
            }
            p = i + 1;
        }
        write.write_str(if p == 0 {
            input
        } else {
            input.get_unchecked(p..)
        })
    }
}

fn escape_gt4<W: fmt::Write>(mut input: &str, write: &mut W) -> fmt::Result {
    let mut p = 0;
    let mut rest = input;
    for (i, _) in input.match_indices(|c: char| c == '>' || c == '<' || c == '\'' || c == '\"') {
        let (x, y) = rest.split_at(i - p);
        write.write_str(x)?;

        if y.starts_with(">") {
            write.write_str("&gt;")?;
        } else if y.starts_with("<") {
            write.write_str("&lt;")?;
        } else if y.starts_with("\'") {
            write.write_str("&apos;")?;
        } else {
            write.write_str("&quot;")?;
        }

        p = i + 1;
        rest = &y[1..];
    }
    write.write_str(rest)
}

//fn escape_gt4<W: fmt::Write>(input: &str, write: &mut W) -> fmt::Result {
//    write.write_str(input)
//}

pub fn small_hit(c: &mut Criterion) {
    c.bench_function("small_hit1", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(SMALL_HIT.len());
            escape_gt1(black_box(SMALL_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
    c.bench_function("small_hit2", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(SMALL_HIT.len());
            escape_gt2(black_box(SMALL_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
    c.bench_function("small_hit3", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(SMALL_HIT.len());
            escape_gt3(black_box(SMALL_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
    c.bench_function("small_hit4", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(SMALL_HIT.len());
            escape_gt4(black_box(SMALL_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
}

pub fn large(c: &mut Criterion) {
    c.bench_function("large1", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(LARGE_NO_HIT.len());
            escape_gt1(black_box(LARGE_NO_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
    c.bench_function("large2", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(LARGE_NO_HIT.len());
            escape_gt2(black_box(LARGE_NO_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
    c.bench_function("large3", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(LARGE_NO_HIT.len());
            escape_gt3(black_box(LARGE_NO_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
    c.bench_function("large4", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(LARGE_NO_HIT.len());
            escape_gt4(black_box(LARGE_NO_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
}

pub fn test(c: &mut Criterion) {
    c.bench_function("test1", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(EVIL_NO_HIT.len());
            escape_gt1(black_box(EVIL_NO_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
    c.bench_function("test2", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(EVIL_NO_HIT.len());
            escape_gt2(black_box(EVIL_NO_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
    c.bench_function("test3", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(EVIL_NO_HIT.len());
            escape_gt3(black_box(EVIL_NO_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
    c.bench_function("test4", |b| {
        b.iter(|| {
            let mut buf = String::new();
            buf.reserve(EVIL_NO_HIT.len());
            escape_gt4(black_box(EVIL_NO_HIT), &mut buf).unwrap();
            black_box(buf)
        })
    });
}

criterion_group!(benches, small_hit, large, test);
criterion_main!(benches);
