use std::arch::x86_64::{
    __m128i, _mm_and_si128, _mm_cmpeq_epi8, _mm_load_si128, _mm_loadu_si128, _mm_movemask_epi8,
    _mm_set1_epi8, _mm_setr_epi8, _mm_shuffle_epi8, _mm_srli_epi32,
};
use std::marker::PhantomData;
use std::mem::transmute;
use std::ptr;
use xrs_chars::XmlAsciiChar;

#[allow(non_camel_case_types)]
pub type m128i = __m128i;

pub trait ShuftiConfig {
    fn low_tbl() -> [i8; 16];
    fn high_tbl() -> [i8; 16];
    fn fallback(data: u8) -> bool;

    fn fallback_block(data: &[u8]) -> usize {
        data.iter().take_while(|c| Self::fallback(**c)).count()
    }
}

pub trait SimdAlgo {
    type VectorType: SimdVector;

    unsafe fn block(&self, data: Self::VectorType) -> i32;
    fn fallback_block(&self, data: &[u8]) -> i32;

    #[target_feature(enable = "ssse3")]
    unsafe fn mini_block(&self, data: &[u8]) -> i32 {
        unsafe {
            let mut padded_data = Self::VectorType::zeros();
            ptr::copy_nonoverlapping(
                data.as_ptr(),
                &mut padded_data as *mut _ as *mut u8,
                data.len(),
            );
            let index = self.block(padded_data);
            if index >= data.len() as i32 {
                -1
            } else {
                index
            }
        }
    }
}

pub trait SimdVector: Copy {
    const BLOCK_SIZE: usize;

    unsafe fn zeros() -> Self;
    unsafe fn load_unaligned(data: *const u8) -> Self;
    unsafe fn load_aligned(data: *const u8) -> Self;
}

impl SimdVector for __m128i {
    const BLOCK_SIZE: usize = 16;

    #[target_feature(enable = "ssse3")]
    unsafe fn zeros() -> Self {
        _mm_set1_epi8(0)
    }

    #[target_feature(enable = "ssse3")]
    unsafe fn load_unaligned(data: *const u8) -> Self {
        _mm_loadu_si128(data as *const Self)
    }

    #[target_feature(enable = "ssse3")]
    unsafe fn load_aligned(data: *const u8) -> Self {
        _mm_load_si128(data as *const Self)
    }
}

pub struct ShuftiSSSE3 {
    low_tbl: __m128i,
    high_tbl: __m128i,
}

impl ShuftiSSSE3 {
    pub fn new(low_tbl: __m128i, high_tbl: __m128i) -> Self {
        Self { low_tbl, high_tbl }
    }

    pub fn from_data<T: ShuftiConfig>() -> Self {
        let l = T::low_tbl();
        let h = T::high_tbl();
        unsafe {
            Self {
                low_tbl: _mm_setr_epi8(
                    l[0], l[1], l[2], l[3], l[4], l[5], l[6], l[7], l[8], l[9], l[10], l[11],
                    l[12], l[13], l[14], l[15],
                ),
                high_tbl: _mm_setr_epi8(
                    h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], h[8], h[9], h[10], h[11],
                    h[12], h[13], h[14], h[15],
                ),
            }
        }
    }
}

impl SimdAlgo for ShuftiSSSE3 {
    type VectorType = __m128i;

    #[target_feature(enable = "ssse3")]
    unsafe fn block(&self, v0: Self::VectorType) -> i32 {
        let zero = Self::VectorType::zeros();

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
}

fn find_index<D: ShuftiConfig>(data: &[u8]) -> usize {
    if is_x86_feature_detected!("ssse3") {
        unsafe { UfExecutor(ShuftiSSSE3::from_data::<D>()).find_index(data) }
    } else {
        D::fallback_block(data)
    }
}

pub trait SimdExecutor {
    unsafe fn find_index(&self, data: &[u8]) -> usize;
}

/// Unaligned-Aligned-Unaligned
pub struct UauExecutor<T: SimdAlgo>(pub T);

impl<T: SimdAlgo> SimdExecutor for UauExecutor<T> {
    #[target_feature(enable = "ssse3")]
    unsafe fn find_index(&self, ptr: &[u8]) -> usize {
        let BLOCK_SIZE: usize = T::VectorType::BLOCK_SIZE;

        let mut skipped = 0usize;
        if ptr.len() < BLOCK_SIZE {
            let len = self.0.mini_block(ptr);
            return if len >= 0 { len as usize } else { ptr.len() };
        }

        let (prefix, middle, suffix) = unsafe { ptr.align_to::<T::VectorType>() };

        // iter prefix
        let prefix_len = unsafe { self.0.block(T::VectorType::load_unaligned(ptr.as_ptr())) };
        if prefix_len >= 0 {
            return prefix_len as usize;
        } else {
            skipped += prefix.len();
        }

        // iter aligned
        while (prefix.len() + middle.len()) < skipped {
            let len = unsafe {
                self.0
                    .block(T::VectorType::load_aligned(ptr.as_ptr().add(skipped)))
            };
            if len >= 0 {
                return skipped + len as usize;
            } else {
                skipped += BLOCK_SIZE;
            }
        }

        // iter suffix
        let suffix_len = unsafe {
            self.0.block(T::VectorType::load_unaligned(
                ptr.as_ptr().add(ptr.len() - BLOCK_SIZE),
            ))
        };
        if suffix_len >= 0 {
            ptr.len() - BLOCK_SIZE + suffix_len as usize
        } else {
            ptr.len()
        }
    }
}

/// Unaligned-Miniblock
pub struct UmExecutor<T: SimdAlgo>(pub T);

impl<T: SimdAlgo> SimdExecutor for UmExecutor<T> {
    #[target_feature(enable = "ssse3")]
    unsafe fn find_index(&self, ptr: &[u8]) -> usize {
        let BLOCK_SIZE: usize = T::VectorType::BLOCK_SIZE;

        let mut skipped = 0usize;
        while ptr.len() >= skipped + BLOCK_SIZE {
            let len = unsafe {
                self.0
                    .block(T::VectorType::load_unaligned(ptr.as_ptr().add(skipped)))
            };
            if len >= 0 {
                return skipped + len as usize;
            } else {
                skipped += BLOCK_SIZE;
            }
        }

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
    unsafe fn find_index(&self, ptr: &[u8]) -> usize {
        let BLOCK_SIZE: usize = T::VectorType::BLOCK_SIZE;

        let mut skipped = 0usize;
        while ptr.len() >= skipped + BLOCK_SIZE {
            let len = unsafe {
                self.0
                    .block(T::VectorType::load_unaligned(ptr.as_ptr().add(skipped)))
            };
            if len >= 0 {
                return skipped + len as usize;
            } else {
                skipped += BLOCK_SIZE;
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
    unsafe fn find_index(&self, ptr: &[u8]) -> usize {
        let BLOCK_SIZE: usize = T::VectorType::BLOCK_SIZE;

        if ptr.len() < BLOCK_SIZE {
            let len = self.0.mini_block(ptr);
            return if len >= 0 { len as usize } else { ptr.len() };
        }

        let mut skipped = 0usize;
        while ptr.len() - BLOCK_SIZE >= skipped {
            let len = unsafe {
                self.0
                    .block(T::VectorType::load_unaligned(ptr.as_ptr().add(skipped)))
            };
            if len >= 0 {
                return skipped + len as usize;
            } else {
                skipped += BLOCK_SIZE;
            }
        }

        // suffix
        let suffix_len = unsafe {
            self.0.block(T::VectorType::load_unaligned(
                ptr.as_ptr().add(ptr.len() - BLOCK_SIZE),
            ))
        };
        if suffix_len >= 0 {
            ptr.len() - BLOCK_SIZE + suffix_len as usize
        } else {
            ptr.len()
        }
    }
}

/// Fallback-Aligned-Fallback
pub struct FafExecutor<T: SimdAlgo>(pub T);

impl<T: SimdAlgo> SimdExecutor for FafExecutor<T> {
    #[target_feature(enable = "ssse3")]
    unsafe fn find_index(&self, ptr: &[u8]) -> usize {
        let BLOCK_SIZE: usize = T::VectorType::BLOCK_SIZE;

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
                return prefix.len() + block_idx * BLOCK_SIZE + len as usize;
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

#[cfg(test)]
mod tests {
    use super::*;
    use xrs_chars::XmlAsciiChar;

    const BIG_INPUT: &[u8] = b" \t\n\r<root\r\n        attr=\"#value\"  > inner\tvalue </root > ";
    const SHORT_INPUT: &[u8] = b" \t\n\r</root>";
    const LONG_SEQ_INPUT: &[u8] = b"                                                              ";

    pub struct WhitespaceShufti;

    impl ShuftiConfig for WhitespaceShufti {
        fn low_tbl() -> [i8; 16] {
            [
                0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1, 0x1, 0x0, 0x0, 0x1, 0x0, 0x0,
            ]
        }

        fn high_tbl() -> [i8; 16] {
            [
                0x1, 0x0, 0x2, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0,
            ]
        }

        fn fallback(c: u8) -> bool {
            c.is_xml_whitespace()
        }
    }

    fn ssse3_skipper() -> ShuftiSSSE3 {
        ShuftiSSSE3::from_data::<WhitespaceShufti>()
    }

    #[test]
    fn uau_big() {
        let executor = UauExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(BIG_INPUT) };
        assert_eq!(4, res);
    }

    #[test]
    fn uf_big() {
        let executor = UfExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(BIG_INPUT) };
        assert_eq!(4, res);
    }

    #[test]
    fn u_big() {
        let executor = UExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(BIG_INPUT) };
        assert_eq!(4, res);
    }

    #[test]
    fn faf_big() {
        let executor = FafExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(BIG_INPUT) };
        assert_eq!(4, res);
    }

    #[test]
    fn uau_short() {
        let executor = UauExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(SHORT_INPUT) };
        assert_eq!(4, res);
    }

    #[test]
    fn uf_short() {
        let executor = UfExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(SHORT_INPUT) };
        assert_eq!(4, res);
    }

    #[test]
    fn u_short() {
        let executor = UExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(SHORT_INPUT) };
        assert_eq!(4, res);
    }

    #[test]
    fn faf_short() {
        let executor = FafExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(SHORT_INPUT) };
        assert_eq!(4, res);
    }

    #[test]
    fn uau_long() {
        let executor = UauExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(LONG_SEQ_INPUT) };
        assert_eq!(62, res);
    }

    #[test]
    fn uf_long() {
        let executor = UfExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(LONG_SEQ_INPUT) };
        assert_eq!(62, res);
    }

    #[test]
    fn u_long() {
        let executor = UExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(LONG_SEQ_INPUT) };
        assert_eq!(62, res);
    }

    #[test]
    fn faf_long() {
        let executor = FafExecutor(ssse3_skipper());
        let res = unsafe { executor.find_index(LONG_SEQ_INPUT) };
        assert_eq!(62, res);
    }
}
