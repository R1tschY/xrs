use std::arch::x86_64::{
    __m128i, _mm_and_si128, _mm_cmpeq_epi8, _mm_load_si128, _mm_loadu_si128, _mm_movemask_epi8,
    _mm_set1_epi8, _mm_setr_epi8, _mm_shuffle_epi8, _mm_srl_epi32, _mm_srli_epi32,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use xrs_chars::XmlAsciiChar;
use xrs_parser::parser::cursor::Cursor;
use xrs_parser::shufti::{FafExecutor, SimdExecutor, UExecutor, UfExecutor, UmExecutor};
use xrs_parser::shufti::{ShuftiSEE3, UauExecutor};
use xrs_parser::XmlError;

fn take_while(cursor: Cursor) -> Result<((), Cursor), XmlError> {
    let size = cursor
        .rest_bytes()
        .iter()
        .take_while(|c| c.is_xml_whitespace())
        .count();
    if size > 0 {
        Ok(((), cursor.advance(size)))
    } else {
        Err(XmlError::ExpectedWhitespace)
    }
}

fn take_while_by_hand1(cursor: Cursor) -> Result<((), Cursor), XmlError> {
    let mut size = 0;
    for c in cursor.rest_bytes() {
        if !c.is_xml_whitespace() {
            break;
        } else {
            size += 1;
        }
    }

    if size > 0 {
        Ok(((), cursor.advance(size)))
    } else {
        Err(XmlError::ExpectedWhitespace)
    }
}

fn take_while_by_hand2(cursor: Cursor) -> Result<((), Cursor), XmlError> {
    let bytes = cursor.rest_bytes();
    let mut size = 0;
    for i in 0..bytes.len() {
        if !bytes[i].is_xml_whitespace() {
            size = i;
            break;
        }
    }

    if size > 0 {
        Ok(((), cursor.advance(size)))
    } else {
        Err(XmlError::ExpectedWhitespace)
    }
}

fn simd_whitespace_skipper() -> ShuftiSEE3 {
    unsafe {
        ShuftiSEE3::new(
            _mm_setr_epi8(
                0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x1, 0x0, 0x0, 0x1, 0x0, 0x0,
            ),
            _mm_setr_epi8(
                0x1, 0x0, 0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            ),
        )
    }
}

fn simd_uau(cursor: Cursor) -> Result<((), Cursor), XmlError> {
    let executor = UauExecutor(simd_whitespace_skipper());
    let size = executor.skip(cursor.rest_bytes());
    if size > 0 {
        Ok(((), cursor.advance(size)))
    } else {
        Err(XmlError::ExpectedWhitespace)
    }
}

fn simd_u(cursor: Cursor) -> Result<((), Cursor), XmlError> {
    let executor = UExecutor(simd_whitespace_skipper());
    let size = executor.skip(cursor.rest_bytes());
    if size > 0 {
        Ok(((), cursor.advance(size)))
    } else {
        Err(XmlError::ExpectedWhitespace)
    }
}

fn simd_uf(cursor: Cursor) -> Result<((), Cursor), XmlError> {
    let executor = UfExecutor(simd_whitespace_skipper());
    let size = executor.skip(cursor.rest_bytes());
    if size > 0 {
        Ok(((), cursor.advance(size)))
    } else {
        Err(XmlError::ExpectedWhitespace)
    }
}

fn simd_um(cursor: Cursor) -> Result<((), Cursor), XmlError> {
    let executor = UmExecutor(simd_whitespace_skipper());
    let size = unsafe { executor.ssse3_skip(cursor.rest_bytes()) };
    if size > 0 {
        Ok(((), cursor.advance(size)))
    } else {
        Err(XmlError::ExpectedWhitespace)
    }
}

const ONE_SPACE: &'static str = " <xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/>";
const INDENT: &'static str = "\n                        <xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/>";
const BIG_INDENT: &'static str = "\n                                                                                                                     <xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/>";
const TAB_INDENT: &'static str = "\n\t\t\t<xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/>";

pub fn one_space(c: &mut Criterion) {
    let input = ONE_SPACE;
    let mut group = c.benchmark_group("one space");
    group.bench_with_input("take_while", input, |b, i| {
        b.iter(|| take_while(Cursor::new(black_box(i))))
    });
    group.bench_with_input("table by hand", input, |b, i| {
        b.iter(|| take_while_by_hand1(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD U", input, |b, i| {
        b.iter(|| simd_u(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD UF", input, |b, i| {
        b.iter(|| simd_uf(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD UM", input, |b, i| {
        b.iter(|| simd_um(Cursor::new(black_box(i))))
    });
}

pub fn indent(c: &mut Criterion) {
    let mut group = c.benchmark_group("indent");
    let input = INDENT;
    group.bench_with_input("take_while", input, |b, i| {
        b.iter(|| take_while(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD U", input, |b, i| {
        b.iter(|| simd_u(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD UF", input, |b, i| {
        b.iter(|| simd_uf(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD UM", input, |b, i| {
        b.iter(|| simd_um(Cursor::new(black_box(i))))
    });
}

pub fn big_indent(c: &mut Criterion) {
    let mut group = c.benchmark_group("big indent");
    let input = BIG_INDENT;
    group.bench_with_input("take_while", input, |b, i| {
        b.iter(|| take_while(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD U", input, |b, i| {
        b.iter(|| simd_u(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD UF", input, |b, i| {
        b.iter(|| simd_uf(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD UM", input, |b, i| {
        b.iter(|| simd_um(Cursor::new(black_box(i))))
    });
}

pub fn tab_indent(c: &mut Criterion) {
    let mut group = c.benchmark_group("tab indent");
    let input = TAB_INDENT;
    group.bench_with_input("take_while", input, |b, i| {
        b.iter(|| take_while(Cursor::new(black_box(i))))
    });
    group.bench_with_input("table by hand 1", input, |b, i| {
        b.iter(|| take_while_by_hand1(Cursor::new(black_box(i))))
    });
    group.bench_with_input("table by hand 2", input, |b, i| {
        b.iter(|| take_while_by_hand2(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD U", input, |b, i| {
        b.iter(|| simd_u(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD UF", input, |b, i| {
        b.iter(|| simd_uf(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD UM", input, |b, i| {
        b.iter(|| simd_um(Cursor::new(black_box(i))))
    });
}

criterion_group!(benches, /*one_space, indent,*/ big_indent, /*tab_indent*/);
criterion_main!(benches);
