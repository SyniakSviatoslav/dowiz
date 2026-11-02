//! SHA-256 compression on the AArch64 SHA2 instructions (`sha256h/h2/su0/su1`, feature `sha2`).
//! The only `unsafe` of the sha256 path lives here: every entry is behind `have()`.

use super::scalar::K;
use core::arch::aarch64::*;

#[inline(always)]
fn have() -> bool {
    #[cfg(target_feature = "sha2")]
    {
        true
    }
    #[cfg(not(target_feature = "sha2"))]
    {
        std::arch::is_aarch64_feature_detected!("sha2")
    }
}

/// Four rounds: `wk` = W[4g..4g+4] + K[4g..4g+4]. Called only from `block`, itself only reached
/// after `have()` proved `sha2`.
#[inline]
#[target_feature(enable = "sha2")]
unsafe fn rounds4(abcd: &mut uint32x4_t, efgh: &mut uint32x4_t, wk: uint32x4_t) {
    let prev = *abcd;
    *abcd = vsha256hq_u32(prev, *efgh, wk);
    *efgh = vsha256h2q_u32(*efgh, prev, wk);
}

/// One 64-byte block at `p` into the state (abcd, efgh).
#[inline]
#[target_feature(enable = "sha2")]
unsafe fn block(st: (uint32x4_t, uint32x4_t), p: *const u8) -> (uint32x4_t, uint32x4_t) {
    let (mut abcd, mut efgh) = st;
    let mut m0 = vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(p)));
    let mut m1 = vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(p.add(16))));
    let mut m2 = vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(p.add(32))));
    let mut m3 = vreinterpretq_u32_u8(vrev32q_u8(vld1q_u8(p.add(48))));
    let k = K.as_ptr();
    // Groups 0..3 take the message as loaded; group g >= 4 derives its four words from
    // groups g-4 .. g-1 (su0 from g-4, g-3; su1 adds g-2, g-1), then the window rotates.
    for g in 0..16 {
        if g >= 4 {
            m0 = vsha256su1q_u32(vsha256su0q_u32(m0, m1), m2, m3);
        }
        rounds4(&mut abcd, &mut efgh, vaddq_u32(m0, vld1q_u32(k.add(4 * g))));
        let t = m0;
        m0 = m1;
        m1 = m2;
        m2 = m3;
        m3 = t;
    }
    (vaddq_u32(abcd, st.0), vaddq_u32(efgh, st.1))
}

#[target_feature(enable = "sha2")]
unsafe fn compress_hw(h: &mut [u32; 8], blocks: &[u8]) {
    let mut st = (vld1q_u32(h.as_ptr()), vld1q_u32(h.as_ptr().add(4)));
    for b in blocks.chunks_exact(64) {
        st = block(st, b.as_ptr());
    }
    vst1q_u32(h.as_mut_ptr(), st.0);
    vst1q_u32(h.as_mut_ptr().add(4), st.1);
}

/// Two independent messages of equal length, block by block side by side: the SHA unit's
/// latency on one chain is hidden by the other (the two hashes of one Merkle level).
#[target_feature(enable = "sha2")]
unsafe fn compress2_hw(ha: &mut [u32; 8], a: &[u8], hb: &mut [u32; 8], b: &[u8]) {
    let mut sa = (vld1q_u32(ha.as_ptr()), vld1q_u32(ha.as_ptr().add(4)));
    let mut sb = (vld1q_u32(hb.as_ptr()), vld1q_u32(hb.as_ptr().add(4)));
    for (x, y) in a.chunks_exact(64).zip(b.chunks_exact(64)) {
        sa = block(sa, x.as_ptr());
        sb = block(sb, y.as_ptr());
    }
    vst1q_u32(ha.as_mut_ptr(), sa.0);
    vst1q_u32(ha.as_mut_ptr().add(4), sa.1);
    vst1q_u32(hb.as_mut_ptr(), sb.0);
    vst1q_u32(hb.as_mut_ptr().add(4), sb.1);
}

/// Compress `blocks` (a multiple of 64 bytes) into `h`; `false` when this CPU has no `sha2`.
#[inline]
pub(super) fn compress(h: &mut [u32; 8], blocks: &[u8]) -> bool {
    if !have() {
        return false;
    }
    // SAFETY: `have()` proved the `sha2` feature on this CPU, which is all `compress_hw` needs;
    // its loads read `chunks_exact(64)` blocks and `h`'s eight words, all in bounds.
    unsafe { compress_hw(h, blocks) };
    true
}

/// Two equal-length messages side by side; `false` when this CPU has no `sha2`.
#[inline]
pub(super) fn compress2(ha: &mut [u32; 8], a: &[u8], hb: &mut [u32; 8], b: &[u8]) -> bool {
    if !have() {
        return false;
    }
    // SAFETY: as `compress`: the feature is proved, and every load is a whole 64-byte chunk.
    unsafe { compress2_hw(ha, a, hb, b) };
    true
}

/// `true` when `compress` runs on the instructions.
pub(super) fn active() -> bool {
    have()
}
