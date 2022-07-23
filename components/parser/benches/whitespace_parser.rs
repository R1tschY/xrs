use std::arch::x86_64::{
    __m128i, _mm_and_si128, _mm_cmpeq_epi8, _mm_load_si128, _mm_loadu_si128, _mm_movemask_epi8,
    _mm_set1_epi8, _mm_setr_epi8, _mm_shuffle_epi8, _mm_srl_epi32, _mm_srli_epi32,
};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use xrs_chars::XmlAsciiChar;
use xrs_parser::parser::cursor::Cursor;
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

union U8x16Converter {
    out: __m128i,
    in_: [u8; 16],
}

const fn u8x16(in_: [u8; 16]) -> __m128i {
    unsafe { U8x16Converter { in_ }.out }
}

const LOW_NIBBLE_MASK: __m128i = u8x16([
    0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x1, 0x0, 0x0, 0x1, 0x0,
    0x0,
    //0x80, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x80, 0x80, 0x0, 0x0, 0x80, 0x0, 0x0,
]);
const HIGH_NIBBLE_MASK: __m128i = u8x16([
    0x1, 0x0, 0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
    0x0,
    //0x80, 0x0, 0x80, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
]);

#[target_feature(enable = "ssse3")]
unsafe fn simd_ssse3(v0: __m128i) -> Option<usize> {
    let zero = _mm_set1_epi8(0);

    let v_v0: __m128i = _mm_and_si128(
        _mm_shuffle_epi8(LOW_NIBBLE_MASK, v0),
        _mm_shuffle_epi8(
            HIGH_NIBBLE_MASK,
            _mm_and_si128(_mm_srli_epi32::<4>(v0), _mm_set1_epi8(0x0f)),
        ),
    );
    // _mm_cmpeq_epi8_mask can make sense but requires AVX512BW + AVX512VL
    let tmp_ws_v0: __m128i = _mm_cmpeq_epi8(v_v0, zero);
    let mask = _mm_movemask_epi8(tmp_ws_v0) as u16;
    if mask == 0xFFFF {
        None
    } else {
        Some(mask.trailing_zeros() as usize)
    }
}

fn simd(cursor: Cursor) -> Result<((), Cursor), XmlError> {
    let mut ptr = cursor.rest_bytes();
    let mut size = 0;

    if ptr.len() < 16 {
        size = cursor
            .rest_bytes()
            .iter()
            .take_while(|c| c.is_xml_whitespace())
            .count();
        return if size > 0 {
            Ok(((), cursor.advance(size)))
        } else {
            Err(XmlError::ExpectedWhitespace)
        };
    }

    let (prefix, middle, suffix) = unsafe { ptr.align_to::<__m128i>() };

    // iter prefix
    if let Some(l) = unsafe { simd_ssse3(_mm_loadu_si128(ptr.as_ptr() as *const __m128i)) } {
        return if l > 0 {
            Ok(((), cursor.advance(l)))
        } else {
            Err(XmlError::ExpectedWhitespace)
        };
    } else {
        size += prefix.len();
    }

    // iter aligned
    while (prefix.len() + middle.len()) < size {
        if let Some(l) = unsafe {
            simd_ssse3(_mm_load_si128(
                ptr.as_ptr().offset(size as isize) as *const __m128i
            ))
        } {
            size += l;
            return if size > 0 {
                Ok(((), cursor.advance(size)))
            } else {
                Err(XmlError::ExpectedWhitespace)
            };
        } else {
            size += 16;
        }
    }

    // iter suffix
    if let Some(l) = unsafe {
        simd_ssse3(_mm_loadu_si128(
            ptr.as_ptr().offset((size - 16) as isize) as *const __m128i
        ))
    } {
        size += l;
        return if l > 0 {
            Ok(((), cursor.advance(size)))
        } else {
            Err(XmlError::ExpectedWhitespace)
        };
    } else {
        size += prefix.len();
        Ok(((), cursor.advance(size)))
    }
}

#[target_feature(enable = "ssse3")]
unsafe fn simd_ssse3_1(ptr: *const u8) -> usize {
    let v0 = _mm_loadu_si128(ptr as *const __m128i);

    let low_nibble_mask: __m128i = _mm_setr_epi8(
        -128, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, -128, -128, 0x0, 0x0, -128, 0x0, 0x0,
    );
    let high_nibble_mask: __m128i = _mm_setr_epi8(
        -128, 0x0, -128, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
    );

    let v_v0: __m128i = _mm_and_si128(
        _mm_shuffle_epi8(low_nibble_mask, v0),
        _mm_shuffle_epi8(
            high_nibble_mask,
            _mm_and_si128(_mm_srli_epi32::<4>(v0), _mm_set1_epi8(0x7f)),
        ),
    );

    _mm_movemask_epi8(v_v0).trailing_ones() as usize
}

fn simd_1(cursor: Cursor) -> Result<((), Cursor), XmlError> {
    let mut ptr = cursor.rest_bytes();

    let mut size = 0;
    while ptr.len() - size >= 16 {
        let l = unsafe { simd_ssse3_1(ptr.as_ptr().offset(size as isize)) };
        size += l;
        if l != 16 {
            return if size > 0 {
                Ok(((), cursor.advance(size)))
            } else {
                Err(XmlError::ExpectedWhitespace)
            };
        }
    }

    for c in &cursor.rest_bytes()[size..] {
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

const ONE_SPACE: &'static str = " <xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/>";
const INDENT: &'static str = "\n                        <xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/>";
const TAB_INDENT: &'static str = "\n\t\t\t<xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/><xml/>";

pub fn one_space(c: &mut Criterion) {
    let mut group = c.benchmark_group("one space");
    group.bench_with_input("take_while", ONE_SPACE, |b, i| {
        b.iter(|| take_while(Cursor::new(black_box(i))))
    });
    group.bench_with_input("table by hand", ONE_SPACE, |b, i| {
        b.iter(|| take_while_by_hand1(Cursor::new(black_box(i))))
    });
}

pub fn indent(c: &mut Criterion) {
    let mut group = c.benchmark_group("indent");
    let input = ONE_SPACE;
    group.bench_with_input("take_while", input, |b, i| {
        b.iter(|| take_while(Cursor::new(black_box(i))))
    });
    group.bench_with_input("SIMD", input, |b, i| {
        b.iter(|| simd(Cursor::new(black_box(i))))
    });
}

pub fn tab_indent(c: &mut Criterion) {
    let mut group = c.benchmark_group("tab indent");
    group.bench_with_input("take_while", TAB_INDENT, |b, i| {
        b.iter(|| take_while(Cursor::new(black_box(i))))
    });
    group.bench_with_input("table by hand 1", TAB_INDENT, |b, i| {
        b.iter(|| take_while_by_hand1(Cursor::new(black_box(i))))
    });
    group.bench_with_input("table by hand 2", TAB_INDENT, |b, i| {
        b.iter(|| take_while_by_hand2(Cursor::new(black_box(i))))
    });
}

criterion_group!(benches, indent);
criterion_main!(benches);
