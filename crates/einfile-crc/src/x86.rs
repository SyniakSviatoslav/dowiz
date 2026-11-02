//! x86_64 PCLMULQDQ folding of the zlib polynomial (Intel 2009; zlib / crc32fast constants). NOT
//! SSE4.2 `crc32`: that instruction is CRC-32C. The crate's only x86_64 `unsafe`.

use core::arch::x86_64::*;

/// Below this the folding set-up costs more than the tables; 64 bytes is four lanes of 16.
pub(crate) const MIN: usize = 64;
/// Cells gathered per fold by `cells_by`.
pub(crate) const GATHER: usize = 128;

// x^k mod P(x) for the reflected zlib polynomial.
const K1: i64 = 0x1_5444_2bd4; // x^(4*128+32) mod P, reflected
const K2: i64 = 0x1_c6e4_1596; // x^(4*128-32) mod P
const K3: i64 = 0x1_7519_97d0; // x^(128+32) mod P
const K4: i64 = 0x0_ccaa_009e; // x^(128-32) mod P
const K5: i64 = 0x1_63cd_6124; // x^64 mod P
const P_X: i64 = 0x1_DB71_0641; // P(x), reflected, with the x^32 term
const U_PRIME: i64 = 0x1_F701_1641; // floor(x^64 / P(x)), reflected

/// pclmulqdq + sse4.1: on at compile time, else (with `std`, and always under test) detected at
/// run time.
#[inline(always)]
pub(crate) fn active() -> bool {
    #[cfg(all(target_feature = "pclmulqdq", target_feature = "sse4.1"))]
    {
        true
    }
    #[cfg(all(not(all(target_feature = "pclmulqdq", target_feature = "sse4.1")), any(feature = "std", test)))]
    {
        std::arch::is_x86_feature_detected!("pclmulqdq") && std::arch::is_x86_feature_detected!("sse4.1")
    }
    #[cfg(all(not(all(target_feature = "pclmulqdq", target_feature = "sse4.1")), not(any(feature = "std", test))))]
    {
        false
    }
}

/// Sixteen bytes from the front of `d`, which the caller proved holds at least sixteen.
#[inline(always)]
unsafe fn load(d: &mut &[u8]) -> __m128i {
    let r = _mm_loadu_si128(d.as_ptr() as *const __m128i);
    *d = &d[16..];
    r
}

#[inline(always)]
unsafe fn fold(a: __m128i, b: __m128i, k: __m128i) -> __m128i {
    let lo = _mm_clmulepi64_si128(a, k, 0x00);
    let hi = _mm_clmulepi64_si128(a, k, 0x11);
    _mm_xor_si128(_mm_xor_si128(b, lo), hi)
}

/// `b.len() >= MIN`. The register goes in and comes out un-inverted.
#[target_feature(enable = "pclmulqdq,sse2,sse4.1")]
unsafe fn bytes_hw(c: u32, b: &[u8]) -> u32 {
    let mut d = b;
    let mut x3 = load(&mut d);
    let mut x2 = load(&mut d);
    let mut x1 = load(&mut d);
    let mut x0 = load(&mut d);
    x3 = _mm_xor_si128(x3, _mm_cvtsi32_si128(c as i32));

    let k1k2 = _mm_set_epi64x(K2, K1);
    while d.len() >= 64 {
        x3 = fold(x3, load(&mut d), k1k2);
        x2 = fold(x2, load(&mut d), k1k2);
        x1 = fold(x1, load(&mut d), k1k2);
        x0 = fold(x0, load(&mut d), k1k2);
    }

    let k3k4 = _mm_set_epi64x(K4, K3);
    let mut x = fold(x3, x2, k3k4);
    x = fold(x, x1, k3k4);
    x = fold(x, x0, k3k4);
    while d.len() >= 16 {
        x = fold(x, load(&mut d), k3k4);
    }

    // 128 -> 64 bits.
    let lo32 = _mm_set_epi32(0, 0, 0, !0);
    let x = _mm_xor_si128(_mm_clmulepi64_si128(x, k3k4, 0x10), _mm_srli_si128(x, 8));
    let x = _mm_xor_si128(
        _mm_clmulepi64_si128(_mm_and_si128(x, lo32), _mm_set_epi64x(0, K5), 0x00),
        _mm_srli_si128(x, 4),
    );
    // Barrett: 64 -> 32 bits.
    let pu = _mm_set_epi64x(U_PRIME, P_X);
    let t1 = _mm_clmulepi64_si128(_mm_and_si128(x, lo32), pu, 0x10);
    let t2 = _mm_clmulepi64_si128(_mm_and_si128(t1, lo32), pu, 0x00);
    let r = _mm_extract_epi32(_mm_xor_si128(x, t2), 1) as u32;

    // The tail under 16 bytes goes through the tables on the same register.
    crate::step_bytes(r, d)
}

/// `None` when this CPU lacks the features or `b` is shorter than `MIN`.
#[inline]
pub(crate) fn bytes(c: u32, b: &[u8]) -> Option<u32> {
    if b.len() < MIN || !active() {
        return None;
    }
    // SAFETY: `active()` proved pclmulqdq + sse4.1 (sse2 is x86_64's baseline), and every `load`
    // in `bytes_hw` is guarded by a length check: four up front (b.len() >= MIN = 64), four per
    // 64-byte round, one per 16-byte round; the rest goes through the safe tables.
    Some(unsafe { bytes_hw(c, b) })
}

/// `None` when this CPU lacks the features or `v` is shorter than `MIN` bytes.
#[inline]
pub(crate) fn cells(c: u32, v: &[i64]) -> Option<u32> {
    if v.len() * 8 < MIN || !active() {
        return None;
    }
    // SAFETY: an `&[i64]` is `len * 8` initialised bytes, `u8` has alignment 1, and x86_64 is
    // little-endian, so this view IS the cells' little-endian bytes, borrowed for this call only.
    let b = unsafe { core::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 8) };
    bytes(c, b)
}
