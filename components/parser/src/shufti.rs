use std::arch::x86_64::{
    __m128i, _mm_and_si128, _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8,
    _mm_setr_epi8, _mm_shuffle_epi8, _mm_srli_epi32,
};
use std::mem::transmute;

#[allow(non_camel_case_types)]
pub type m128i = __m128i;

pub struct SSEInput {
    v0: m128i,
    // v1: m128,
    // v2: m128,
    // v3: m128,
}

fn m128i_as_slice(v: &m128i) -> &[u8; 16] {
    unsafe { transmute(v) }
}

fn format_m128_chars(prefix: &str, v: m128i) {
    unsafe {
        print!("{:12}: ", prefix);
        for b in m128i_as_slice(&v) {
            if b.is_ascii_graphic() {
                print!(" {} ", char::from(*b));
            } else {
                print!(" . ");
            }
        }
        println!()
    }
}

fn format_m128_bytes(prefix: &str, v: m128i) {
    unsafe {
        print!("{:12}: ", prefix);
        for b in m128i_as_slice(&v) {
            print!("{:02x} ", b);
        }
        println!()
    }
}

fn format_bits(prefix: &str, v: i32) {
    unsafe {
        print!("{:12}: ", prefix);
        for i in 0..16 {
            print!("{:02x} ", (v >> i) & 1);
        }
        println!()
    }
}

impl SSEInput {
    pub(crate) fn new(ptr: &[u8]) -> Self {
        assert!(ptr.len() >= 16);
        unsafe {
            format_m128_chars("input", _mm_loadu_si128(ptr.as_ptr() as *const m128i));

            Self {
                v0: _mm_loadu_si128(ptr.as_ptr() as *const m128i),
                // v1: _mm_loadu_si128(ptr.as_ptr().add(16) as *const m128),
                // v2: _mm_loadu_si128(ptr.as_ptr().add(32) as *const m128),
                // v3: _mm_loadu_si128(ptr.as_ptr().add(48) as *const m128),
            }
        }
    }

    /// shufti algorithm stealed from hyperscan
    fn scan_whitespace(&self) -> u64 {
        unsafe {
            let low_nibble_mask: __m128i = _mm_setr_epi8(
                0x1, 0x0, 0x8, 0x0, 0x0, 0x0, 0x4, 0x0, 0x0, 0x1, 0x1, 0x4, 0x2, 0x11, 0x2, 0x0,
            );
            let high_nibble_mask: __m128i = _mm_setr_epi8(
                0x1, 0x0, 0xd, 0x16, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            );

            let shufti_mask: __m128i = _mm_set1_epi8(0b1000);

            format_m128_bytes("self.v0", self.v0);
            let v_v0: __m128i = _mm_and_si128(
                _mm_shuffle_epi8(low_nibble_mask, self.v0),
                _mm_shuffle_epi8(
                    high_nibble_mask,
                    _mm_and_si128(_mm_srli_epi32(self.v0, 4), _mm_set1_epi8(0x7f)),
                ),
            );
            format_m128_bytes("v_v0_shuflo", _mm_shuffle_epi8(low_nibble_mask, self.v0));
            format_m128_bytes(
                "v_v0_shufhi",
                _mm_shuffle_epi8(
                    high_nibble_mask,
                    _mm_and_si128(_mm_srli_epi32(self.v0, 4), _mm_set1_epi8(0x7f)),
                ),
            );
            format_m128_bytes("v_v0", v_v0);
            let tmp_ws_v0: __m128i =
                _mm_cmpeq_epi8(_mm_and_si128(v_v0, shufti_mask), _mm_set1_epi8(0));
            format_m128_bytes("tmp_ws_v0", tmp_ws_v0);
            format_bits("result", _mm_movemask_epi8(tmp_ws_v0));
            _mm_movemask_epi8(tmp_ws_v0) as u64
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::shufti::SSEInput;

    #[test]
    fn it_works() {
        let input = SSEInput::new(b"<root \t\n\rattr=\"#value\"  > inner\tvalue </root > ");
        println!("{:08b}", input.scan_whitespace().reverse_bits());
    }
}
