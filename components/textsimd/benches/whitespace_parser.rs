use std::arch::x86_64::_mm_setr_epi8;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use textsimd::shufti::{FafExecutor, SimdExecutor, UExecutor, UfExecutor, UmExecutor};
use textsimd::shufti::{ShuftiSSSE3, UauExecutor};
use xrs_chars::XmlAsciiChar;

fn take_while(cursor: &str) -> Result<((), &str), ()> {
    let size = cursor
        .as_bytes()
        .iter()
        .take_while(|c| c.is_xml_whitespace())
        .count();
    if size > 0 {
        Ok(((), cursor.split_at(size).1))
    } else {
        Err(())
    }
}

fn take_while_by_hand1(cursor: &str) -> Result<((), &str), ()> {
    let mut size = 0;
    for c in cursor.as_bytes() {
        if !c.is_xml_whitespace() {
            break;
        } else {
            size += 1;
        }
    }

    if size > 0 {
        Ok(((), cursor.split_at(size).1))
    } else {
        Err(())
    }
}

fn take_while_by_hand2(cursor: &str) -> Result<((), &str), ()> {
    let bytes = cursor.as_bytes();
    let mut size = 0;
    for i in 0..bytes.len() {
        if !bytes[i].is_xml_whitespace() {
            size = i;
            break;
        }
    }

    if size > 0 {
        Ok(((), cursor.split_at(size).1))
    } else {
        Err(())
    }
}

fn simd_whitespace_skipper() -> ShuftiSSSE3 {
    unsafe {
        ShuftiSSSE3::new(
            _mm_setr_epi8(
                0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x1, 0x0, 0x0, 0x1, 0x0, 0x0,
            ),
            _mm_setr_epi8(
                0x1, 0x0, 0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            ),
        )
    }
}

fn simd_uau(cursor: &str) -> Result<((), &str), ()> {
    let executor = UauExecutor(simd_whitespace_skipper());
    let size = executor.find_index(cursor.as_bytes());
    if size > 0 {
        Ok(((), cursor.split_at(size).1))
    } else {
        Err(())
    }
}

fn simd_u(cursor: &str) -> Result<((), &str), ()> {
    let executor = UExecutor(simd_whitespace_skipper());
    let size = executor.find_index(cursor.as_bytes());
    if size > 0 {
        Ok(((), cursor.split_at(size).1))
    } else {
        Err(())
    }
}

fn simd_uf(cursor: &str) -> Result<((), &str), ()> {
    let executor = UfExecutor(simd_whitespace_skipper());
    let size = executor.find_index(cursor.as_bytes());
    if size > 0 {
        Ok(((), cursor.split_at(size).1))
    } else {
        Err(())
    }
}

fn simd_um(cursor: &str) -> Result<((), &str), ()> {
    let executor = UmExecutor(simd_whitespace_skipper());
    let size = unsafe { executor.find_index(cursor.as_bytes()) };
    if size > 0 {
        Ok(((), cursor.split_at(size).1))
    } else {
        Err(())
    }
}

fn simd_faf(cursor: &str) -> Result<((), &str), ()> {
    let executor = FafExecutor(simd_whitespace_skipper());
    let size = executor.find_index(cursor.as_bytes());
    if size > 0 {
        Ok(((), cursor.split_at(size).1))
    } else {
        Err(())
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
        b.iter(|| take_while(black_box(i)))
    });
    group.bench_with_input("table by hand", input, |b, i| {
        b.iter(|| take_while_by_hand1(black_box(i)))
    });
    group.bench_with_input("SIMD U", input, |b, i| b.iter(|| simd_u(black_box(i))));
    group.bench_with_input("SIMD UF", input, |b, i| b.iter(|| simd_uf(black_box(i))));
    group.bench_with_input("SIMD UM", input, |b, i| b.iter(|| simd_um(black_box(i))));
}

pub fn indent(c: &mut Criterion) {
    let mut group = c.benchmark_group("indent");
    let input = INDENT;
    group.bench_with_input("take_while", input, |b, i| {
        b.iter(|| take_while(black_box(i)))
    });
    group.bench_with_input("SIMD UAU", input, |b, i| b.iter(|| simd_uau(black_box(i))));
    group.bench_with_input("SIMD U", input, |b, i| b.iter(|| simd_u(black_box(i))));
    group.bench_with_input("SIMD UF", input, |b, i| b.iter(|| simd_uf(black_box(i))));
    group.bench_with_input("SIMD UM", input, |b, i| b.iter(|| simd_um(black_box(i))));
    group.bench_with_input("SIMD FAF", input, |b, i| b.iter(|| simd_faf(black_box(i))));
}

pub fn big_indent(c: &mut Criterion) {
    let mut group = c.benchmark_group("big indent");
    let input = BIG_INDENT;
    group.bench_with_input("take_while", input, |b, i| {
        b.iter(|| take_while(black_box(i)))
    });
    group.bench_with_input("SIMD U", input, |b, i| b.iter(|| simd_u(black_box(i))));
    group.bench_with_input("SIMD UF", input, |b, i| b.iter(|| simd_uf(black_box(i))));
    group.bench_with_input("SIMD UM", input, |b, i| b.iter(|| simd_um(black_box(i))));
}

pub fn tab_indent(c: &mut Criterion) {
    let mut group = c.benchmark_group("tab indent");
    let input = TAB_INDENT;
    group.bench_with_input("take_while", input, |b, i| {
        b.iter(|| take_while(black_box(i)))
    });
    group.bench_with_input("table by hand 1", input, |b, i| {
        b.iter(|| take_while_by_hand1(black_box(i)))
    });
    group.bench_with_input("table by hand 2", input, |b, i| {
        b.iter(|| take_while_by_hand2(black_box(i)))
    });
    group.bench_with_input("SIMD U", input, |b, i| b.iter(|| simd_u(black_box(i))));
    group.bench_with_input("SIMD UF", input, |b, i| b.iter(|| simd_uf(black_box(i))));
    group.bench_with_input("SIMD UM", input, |b, i| b.iter(|| simd_um(black_box(i))));
}

pub fn short(c: &mut Criterion) {
    for n in 0..3 {
        let mut group = c.benchmark_group(format!("short{}", n));
        let input = "\x0D".repeat(n);
        group.bench_with_input("UF", &input, |b, i| b.iter(|| simd_uf(black_box(i))));
        group.bench_with_input("UM", &input, |b, i| b.iter(|| simd_um(black_box(i))));
    }
}

criterion_group!(benches, short); //one_space, indent, big_indent, tab_indent);
criterion_main!(benches);
