use crate::XmlError;
use std::arch::x86_64::{
    __m128i, _mm_and_si128, _mm_cmpeq_epi8, _mm_load_si128, _mm_loadu_si128, _mm_movemask_epi8,
    _mm_set1_epi8, _mm_setr_epi8, _mm_shuffle_epi8, _mm_srli_epi32,
};
use std::mem::transmute;
use std::ptr;
use xrs_chars::XmlAsciiChar;

#[allow(non_camel_case_types)]
pub type m128i = __m128i;

pub trait SimdAlgo {
    const BLOCK_SIZE: usize;
    type VectorType: Copy;

    unsafe fn block(&self, data: Self::VectorType) -> i32;
    fn fallback_block(&self, data: &[u8]) -> i32;

    unsafe fn zeros() -> Self::VectorType;
    unsafe fn load_unaligned(data: *const u8) -> Self::VectorType;
    unsafe fn load_aligned(data: *const u8) -> Self::VectorType;

    fn mini_block(&self, data: &[u8]) -> i32 {
        unsafe {
            let mut padded_data = Self::zeros();
            ptr::copy_nonoverlapping(
                data.as_ptr(),
                &mut padded_data as *mut _ as *mut u8,
                data.len(),
            );
            self.block(padded_data)
        }
    }
}

pub struct ShuftiSEE3 {
    low_tbl: __m128i,
    high_tbl: __m128i,
}

impl ShuftiSEE3 {
    pub fn new(low_tbl: __m128i, high_tbl: __m128i) -> Self {
        Self { low_tbl, high_tbl }
    }
}

impl SimdAlgo for ShuftiSEE3 {
    const BLOCK_SIZE: usize = 16;
    type VectorType = __m128i;

    #[target_feature(enable = "ssse3")]
    unsafe fn block(&self, v0: Self::VectorType) -> i32 {
        let zero = Self::zeros();

        let v_v0: __m128i = _mm_and_si128(
            _mm_shuffle_epi8(self.low_tbl, v0),
            _mm_shuffle_epi8(
                self.high_tbl,
                _mm_and_si128(_mm_srli_epi32::<4>(v0), _mm_set1_epi8(0x0f)),
            ),
        );
        // _mm_cmpeq_epi8_mask can make sense but requires AVX512BW + AVX512VL
        let tmp_ws_v0: __m128i = _mm_cmpeq_epi8(v_v0, zero);
        let mask = _mm_movemask_epi8(tmp_ws_v0) as u16;
        if mask == 0 {
            -1
        } else {
            mask.trailing_zeros() as i32
        }
    }

    fn fallback_block(&self, data: &[u8]) -> i32 {
        let len = data.iter().take_while(|c| c.is_xml_whitespace()).count();
        if len == data.len() {
            -1
        } else {
            len as i32
        }
    }

    unsafe fn zeros() -> Self::VectorType {
        _mm_set1_epi8(0)
    }

    unsafe fn load_unaligned(data: *const u8) -> Self::VectorType {
        _mm_loadu_si128(data as *const Self::VectorType)
    }

    unsafe fn load_aligned(data: *const u8) -> Self::VectorType {
        _mm_load_si128(data as *const Self::VectorType)
    }
}

pub trait SimdExecutor {
    unsafe fn ssse3_skip(&self, data: &[u8]) -> usize;

    fn skip(&self, data: &[u8]) -> usize {
        if is_x86_feature_detected!("ssse3") {
            unsafe { self.ssse3_skip(data) }
        } else {
            panic!("SSSE3 cpu feature is required")
        }
    }
}

/// Unaligned-Aligned-Unaligned
pub struct UauExecutor<T: SimdAlgo>(pub T);

impl<T: SimdAlgo> SimdExecutor for UauExecutor<T> {
    #[target_feature(enable = "ssse3")]
    unsafe fn ssse3_skip(&self, ptr: &[u8]) -> usize {
        let mut skipped = 0usize;

        if ptr.len() < T::BLOCK_SIZE {
            let len = self.0.mini_block(ptr);
            return if len >= 0 { len as usize } else { ptr.len() };
        }

        let (prefix, middle, suffix) = unsafe { ptr.align_to::<T::VectorType>() };

        // iter prefix
        let prefix_len = unsafe { self.0.block(T::load_unaligned(ptr.as_ptr())) };
        if prefix_len >= 0 {
            return prefix_len as usize;
        } else {
            skipped += prefix.len();
        }

        // iter aligned
        while (prefix.len() + middle.len()) < skipped {
            let len = unsafe { self.0.block(T::load_aligned(ptr.as_ptr().add(skipped))) };
            if len >= 0 {
                return skipped + len as usize;
            } else {
                skipped += T::BLOCK_SIZE;
            }
        }

        // iter suffix
        let suffix_len = unsafe {
            self.0.block(T::load_unaligned(
                ptr.as_ptr().add(ptr.len() - T::BLOCK_SIZE),
            ))
        };
        if suffix_len >= 0 {
            ptr.len() - T::BLOCK_SIZE + suffix_len as usize
        } else {
            ptr.len()
        }
    }
}

/// Unaligned-Miniblock
pub struct UmExecutor<T: SimdAlgo>(pub T);

impl<T: SimdAlgo> SimdExecutor for UmExecutor<T> {
    #[target_feature(enable = "ssse3")]
    unsafe fn ssse3_skip(&self, ptr: &[u8]) -> usize {
        for i in (0..ptr.len()).step_by(T::BLOCK_SIZE) {
            let len = unsafe { self.0.block(T::load_unaligned(ptr.as_ptr().add(i))) };
            if len >= 0 {
                return i + len as usize;
            }
        }

        let skipped = ptr.len() - (ptr.len() % T::BLOCK_SIZE);
        let len = self.0.mini_block(ptr.get_unchecked(skipped..));
        if len >= 0 {
            skipped + len as usize
        } else {
            ptr.len()
        }
    }
}

/// Unaligned-Fallback
pub struct UfExecutor<T: SimdAlgo>(pub T);

impl<T: SimdAlgo> SimdExecutor for UfExecutor<T> {
    #[target_feature(enable = "ssse3")]
    unsafe fn ssse3_skip(&self, ptr: &[u8]) -> usize {
        let mut skipped = 0usize;
        while ptr.len() >= skipped + T::BLOCK_SIZE {
            let len = unsafe { self.0.block(T::load_unaligned(ptr.as_ptr().add(skipped))) };
            if len >= 0 {
                return skipped + len as usize;
            } else {
                skipped += T::BLOCK_SIZE;
            }
        }

        let len = self
            .0
            .fallback_block(unsafe { ptr.get_unchecked(skipped..) });
        if len >= 0 {
            len as usize
        } else {
            ptr.len()
        }
    }
}

/// Unaligned
pub struct UExecutor<T: SimdAlgo>(pub T);

impl<T: SimdAlgo> SimdExecutor for UExecutor<T> {
    #[target_feature(enable = "ssse3")]
    unsafe fn ssse3_skip(&self, ptr: &[u8]) -> usize {
        if ptr.len() < T::BLOCK_SIZE {
            let len = self.0.mini_block(ptr);
            return if len >= 0 { len as usize } else { ptr.len() };
        }

        let mut skipped = 0usize;
        while ptr.len() - T::BLOCK_SIZE >= skipped {
            let len = unsafe { self.0.block(T::load_unaligned(ptr.as_ptr().add(skipped))) };
            if len >= 0 {
                return skipped + len as usize;
            } else {
                skipped += T::BLOCK_SIZE;
            }
        }

        // suffix
        let suffix_len = unsafe {
            self.0.block(T::load_unaligned(
                ptr.as_ptr().add(ptr.len() - T::BLOCK_SIZE),
            ))
        };
        if suffix_len >= 0 {
            ptr.len() - T::BLOCK_SIZE + suffix_len as usize
        } else {
            ptr.len()
        }
    }
}

/// Fallback-Aligned-Fallback
pub struct FafExecutor<T: SimdAlgo>(pub T);

impl<T: SimdAlgo> SimdExecutor for FafExecutor<T> {
    #[target_feature(enable = "ssse3")]
    unsafe fn ssse3_skip(&self, ptr: &[u8]) -> usize {
        let (prefix, aligned, suffix) = unsafe { ptr.align_to::<T::VectorType>() };

        // iter prefix
        let len = self.0.fallback_block(prefix);
        if len >= 0 {
            return len as usize;
        }

        // iter aligned
        for block_idx in 0..aligned.len() {
            let len = unsafe { self.0.block(*aligned.get_unchecked(block_idx)) };
            if len >= 0 {
                return prefix.len() + block_idx * T::BLOCK_SIZE + len as usize;
            }
        }

        // iter suffix
        let len = self.0.fallback_block(suffix);
        if len >= 0 {
            (ptr.len() - suffix.len()) + len as usize
        } else {
            ptr.len()
        }
    }
}

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

    /// shufti algorithm from hyperscan
    fn scan_whitespace(&self) -> u64 {
        unsafe {
            let low_nibble_mask: __m128i = _mm_setr_epi8(
                0x1, 0x0, 0x8, 0x0, 0x0, 0x0, 0x4, 0x0, 0x0, 0x1, 0x1, 0x4, 0x2, 0x11, 0x2, 0x0,
            );
            let high_nibble_mask: __m128i = _mm_setr_epi8(
                0x1, 0x0, 0xd, 0x16, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            );

            let shufti_mask: __m128i = _mm_set1_epi8(0b0001);

            format_m128_bytes("self.v0", self.v0);
            let v_v0: __m128i = _mm_and_si128(
                _mm_shuffle_epi8(low_nibble_mask, self.v0),
                _mm_shuffle_epi8(
                    high_nibble_mask,
                    _mm_and_si128(_mm_srli_epi32::<4>(self.v0), _mm_set1_epi8(0x7f)),
                ),
            );
            format_m128_bytes("v_v0_shuflo", _mm_shuffle_epi8(low_nibble_mask, self.v0));
            format_m128_bytes(
                "v_v0_shufhi",
                _mm_shuffle_epi8(
                    high_nibble_mask,
                    _mm_and_si128(_mm_srli_epi32::<4>(self.v0), _mm_set1_epi8(0x7f)),
                ),
            );
            format_m128_bytes("v_v0_shufhi'", _mm_srli_epi32::<4>(self.v0));
            format_m128_bytes(
                "v_v0_shufhi''",
                _mm_and_si128(_mm_srli_epi32::<4>(self.v0), _mm_set1_epi8(0x7f)),
            );
            format_m128_bytes("v_v0", v_v0);
            let tmp_ws_v0: __m128i =
                _mm_cmpeq_epi8(_mm_and_si128(v_v0, shufti_mask), _mm_set1_epi8(0));
            format_m128_bytes("tmp_ws_v0", tmp_ws_v0);
            format_bits("result", _mm_movemask_epi8(tmp_ws_v0));
            (_mm_movemask_epi8(tmp_ws_v0) as u64).trailing_zeros() as u64
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::shufti::{
        FafExecutor, SSEInput, ShuftiSEE3, SimdExecutor, UExecutor, UauExecutor, UfExecutor,
    };
    use std::arch::x86_64::_mm_setr_epi8;
    const BIG_INPUT: &[u8] = b" \t\n\r<root\r\n        attr=\"#value\"  > inner\tvalue </root > ";
    const SHORT_INPUT: &[u8] = b" \t\n\r</root>";
    const LONG_SEQ_INPUT: &[u8] = b"                                                              ";

    fn new_whitespace_skipper() -> ShuftiSEE3 {
        unsafe {
            ShuftiSEE3 {
                low_tbl: _mm_setr_epi8(
                    0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x1, 0x0, 0x0, 0x1, 0x0, 0x0,
                ),
                high_tbl: _mm_setr_epi8(
                    0x1, 0x0, 0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
                ),
            }
        }
    }

    #[test]
    fn it_works() {
        let input = SSEInput::new(BIG_INPUT);
        assert_eq!(4, input.scan_whitespace());
    }

    #[test]
    fn uau_big() {
        let executor = UauExecutor(new_whitespace_skipper());
        let res = executor.skip(BIG_INPUT);
        assert_eq!(4, res);
    }

    #[test]
    fn uf_big() {
        let executor = UfExecutor(new_whitespace_skipper());
        let res = executor.skip(BIG_INPUT);
        assert_eq!(4, res);
    }

    #[test]
    fn u_big() {
        let executor = UExecutor(new_whitespace_skipper());
        let res = executor.skip(BIG_INPUT);
        assert_eq!(4, res);
    }

    #[test]
    fn faf_big() {
        let executor = FafExecutor(new_whitespace_skipper());
        let res = executor.skip(BIG_INPUT);
        assert_eq!(4, res);
    }

    #[test]
    fn uau_short() {
        let executor = UauExecutor(new_whitespace_skipper());
        let res = executor.skip(SHORT_INPUT);
        assert_eq!(4, res);
    }

    #[test]
    fn uf_short() {
        let executor = UfExecutor(new_whitespace_skipper());
        let res = executor.skip(SHORT_INPUT);
        assert_eq!(4, res);
    }

    #[test]
    fn u_short() {
        let executor = UExecutor(new_whitespace_skipper());
        let res = executor.skip(SHORT_INPUT);
        assert_eq!(4, res);
    }

    #[test]
    fn faf_short() {
        let executor = FafExecutor(new_whitespace_skipper());
        let res = executor.skip(SHORT_INPUT);
        assert_eq!(4, res);
    }

    #[test]
    fn uau_long() {
        let executor = UauExecutor(new_whitespace_skipper());
        let res = executor.skip(LONG_SEQ_INPUT);
        assert_eq!(62, res);
    }

    #[test]
    fn uf_long() {
        let executor = UfExecutor(new_whitespace_skipper());
        let res = executor.skip(LONG_SEQ_INPUT);
        assert_eq!(62, res);
    }

    #[test]
    fn u_long() {
        let executor = UExecutor(new_whitespace_skipper());
        let res = executor.skip(LONG_SEQ_INPUT);
        assert_eq!(62, res);
    }

    #[test]
    fn faf_long() {
        let executor = FafExecutor(new_whitespace_skipper());
        let res = executor.skip(LONG_SEQ_INPUT);
        assert_eq!(62, res);
    }
}
