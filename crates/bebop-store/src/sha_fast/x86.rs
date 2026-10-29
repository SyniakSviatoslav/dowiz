//! SHA-256 compression on x86_64 SHA-NI (`sha256rnds2/msg1/msg2`, features `sha` + SSSE3 + SSE4.1).
//! The only `unsafe` of the x86 sha256 path lives here: every entry is behind `have()`.

use super::scalar::K;
use core::arch::x86_64::*;

#[inline(always)]
fn have() -> bool {
    #[cfg(all(target_feature = "sha", target_feature = "ssse3", target_feature = "sse4.1"))]
    {
        true
    }
    #[cfg(not(all(target_feature = "sha", target_feature = "ssse3", target_feature = "sse4.1")))]
    {
        std::arch::is_x86_feature_detected!("sha")
            && std::arch::is_x86_feature_detected!("ssse3")
            && std::arch::is_x86_feature_detected!("sse4.1")
    }
}

#[target_feature(enable = "sha,sse2,ssse3,sse4.1")]
unsafe fn compress_hw(h: &mut [u32; 8], blocks: &[u8]) {
    // Big-endian word loads.
    let mask = _mm_set_epi64x(0x0c0d_0e0f_0809_0a0bu64 as i64, 0x0405_0607_0001_0203u64 as i64);
    // The SHA-NI state is (ABEF, CDGH), not (ABCD, EFGH).
    let dcba = _mm_loadu_si128(h.as_ptr() as *const __m128i);
    let hgfe = _mm_loadu_si128(h.as_ptr().add(4) as *const __m128i);
    let cdab = _mm_shuffle_epi32(dcba, 0xB1);
    let efgh = _mm_shuffle_epi32(hgfe, 0x1B);
    let mut abef = _mm_alignr_epi8(cdab, efgh, 8);
    let mut cdgh = _mm_blend_epi16(efgh, cdab, 0xF0);
    for block in blocks.chunks_exact(64) {
        let (s_abef, s_cdgh) = (abef, cdgh);
        let p = block.as_ptr() as *const __m128i;
        let mut m0 = _mm_shuffle_epi8(_mm_loadu_si128(p), mask);
        let mut m1 = _mm_shuffle_epi8(_mm_loadu_si128(p.add(1)), mask);
        let mut m2 = _mm_shuffle_epi8(_mm_loadu_si128(p.add(2)), mask);
        let mut m3 = _mm_shuffle_epi8(_mm_loadu_si128(p.add(3)), mask);
        for g in 0..16 {
            if g >= 4 {
                let t = _mm_add_epi32(_mm_sha256msg1_epu32(m0, m1), _mm_alignr_epi8(m3, m2, 4));
                m0 = _mm_sha256msg2_epu32(t, m3);
            }
            let wk = _mm_add_epi32(m0, _mm_loadu_si128(K.as_ptr().add(4 * g) as *const __m128i));
            cdgh = _mm_sha256rnds2_epu32(cdgh, abef, wk);
            abef = _mm_sha256rnds2_epu32(abef, cdgh, _mm_shuffle_epi32(wk, 0x0E));
            let t = m0;
            m0 = m1;
            m1 = m2;
            m2 = m3;
            m3 = t;
        }
        abef = _mm_add_epi32(abef, s_abef);
        cdgh = _mm_add_epi32(cdgh, s_cdgh);
    }
    let feba = _mm_shuffle_epi32(abef, 0x1B);
    let dchg = _mm_shuffle_epi32(cdgh, 0xB1);
    let dcba = _mm_blend_epi16(feba, dchg, 0xF0);
    let hgef = _mm_alignr_epi8(dchg, feba, 8);
    _mm_storeu_si128(h.as_mut_ptr() as *mut __m128i, dcba);
    _mm_storeu_si128(h.as_mut_ptr().add(4) as *mut __m128i, hgef);
}

/// Compress `blocks` (a multiple of 64 bytes) into `h`; `false` when this CPU has no SHA-NI.
#[inline]
pub(super) fn compress(h: &mut [u32; 8], blocks: &[u8]) -> bool {
    if !have() {
        return false;
    }
    // SAFETY: `have()` proved sha + ssse3 + sse4.1 on this CPU; the unaligned loads read
    // `chunks_exact(64)` blocks and `h`'s eight words, all in bounds.
    unsafe { compress_hw(h, blocks) };
    true
}

/// `true` when `compress` runs on the instructions.
pub(super) fn active() -> bool {
    have()
}
