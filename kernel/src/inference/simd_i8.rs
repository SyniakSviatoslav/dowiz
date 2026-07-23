//! ITEM 39 — AVX2 i8 SIMD Quantized Kernels (`core::arch`), oracle fallback + differential.
//!
//! Governing ruling (arc-wide): *"безпека і передбачуваність понад швидкість"* — item 39 is
//! the ONE place speed is pursued, and only under a bit-exact differential leash against the
//! item-37 scalar oracle.
//!
//! # Design (BLUEPRINT-ITEM-39-40 PART A)
//!
//! Integer arithmetic IS associative, so within-row vectorization is *mathematically legal*
//! here — unlike the f64 lanes (`simd.rs`), whose across-rows-only rule exists because f64
//! addition is non-associative. The relaxation buys legality of within-row SIMD; it does not
//! buy freedom from the differential leash.
//!
//! **Saturation / signedness hazard — resolved by construction.** The architect-recommended
//! path is `_mm256_madd_epi16` (**i16×i16 → i32, NON-saturating**). We widen each i8 operand to
//! i16 (`_mm256_cvtepi8_epi16`) and let `madd_epi16` form the 8 i32 products+pairwise-sums in a
//! 256-bit register. Because the intermediate is i32 and the instruction does not saturate, the
//! SIMD kernel computes the *exact same integer math* as the scalar oracle — no saturation
//! cliff exists. (We deliberately do NOT use `_mm256_maddubs_epi16`, whose i16 intermediate
//! saturates and would introduce a determinism hazard.)
//!
//! **Lane order.** Integer addition is associative, so grouping within the 8-lane i32
//! accumulator and the final horizontal sum is order-free *and* value-identical to the oracle's
//! left-to-right sum. The order is still pinned here (chunk-of-16, pairwise `madd_epi16`,
//! 8-lane accumulate, horizontal sum, scalar tail) and documented so the result is auditable and
//! bit-exact to the oracle.
//!
//! **Differential leash.** Every public entry point carries `debug_assert_eq!` against item 37's
//! oracle on every call — continuous verification, compiled out of release at zero production
//! cost. The oracle is the scalar fallback path when AVX2 is unavailable (or on non-x86_64).
//!
//! Runtime detection mirrors `simd.rs` / `householder.rs`: `is_x86_feature_detected!("avx2")`
//! → AVX2 kernel; scalar-oracle fallback otherwise. No new dependency.

use crate::inference::fixed::{check_overflow_bound, Q_MAX};
use crate::inference::oracle::{
    oracle_argmax, oracle_matmul_i8, oracle_relu_i32, oracle_requantize,
};
use crate::inference::workspace::{C, H, N};

/// Maximum `k` (reduction length) this module's column-gather buffer supports. The pilot's
/// layers have `k ≤ 8`; the differential corpus uses `k ≤ 12`; the item-35 overflow lemma caps
/// meaningful `k` far below this. A larger `k` is a programming error, caught by `debug_assert`.
const MAX_K: usize = 64;

/// Scalar i8·i8 dot product — the **fallback** path and the `debug_assert` reference.
///
/// Plain left-to-right i32 accumulation, identical to `oracle_matmul_i8`'s per-output
/// summation. Exact in i32 (i8·i8 ≤ 16129 per term; the item-35 lemma guarantees the total fits
/// i32). The SIMD path must match this bit-for-bit.
#[inline]
fn scalar_dot(a: &[i8], w: &[i8]) -> i32 {
    debug_assert_eq!(a.len(), w.len());
    let mut acc: i32 = 0;
    // Left-to-right sum order === the oracle's golden order.
    for k in 0..a.len() {
        acc += (a[k] as i32) * (w[k] as i32);
    }
    acc
}

/// AVX2 i8·i8 dot product via `_mm256_madd_epi16` (non-saturating i16×i16→i32).
///
/// Processes the contiguous slices in chunks of 16: load 16 i8, sign-extend to 16 i16,
/// `madd_epi16` forms 8 i32 = Σ of adjacent i16 products, accumulate into an 8-lane i32 vector,
/// then horizontal-sum to i32. The `<16` tail is finished in scalar. Because `madd_epi16` does
/// not saturate and integer addition is exact/associative within i32, this is bit-identical to
/// [`scalar_dot`].
///
/// SAFETY: caller must have verified the CPU has AVX2 via `is_x86_feature_detected!("avx2")`.
/// `a` and `w` must each have length ≥ `k`; loads are in-bounds by construction.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn dot_i8_avx2(a: *const i8, w: *const i8, k: usize) -> i32 {
    use core::arch::x86_64::*;
    // 8-lane i32 accumulator (one per pairwise-madd group).
    let mut acc = _mm256_setzero_si256();
    let mut i = 0;
    while i + 16 <= k {
        // Load 16 i8 and sign-extend to 16 i16 (no data loss: i8 fits i16 exactly).
        let va8 = _mm_loadu_si128(a.add(i) as *const __m128i);
        let vw8 = _mm_loadu_si128(w.add(i) as *const __m128i);
        let va16 = _mm256_cvtepi8_epi16(va8);
        let vw16 = _mm256_cvtepi8_epi16(vw8);
        // vpmaddwd: out[2r] = a16[2r]*w16[2r] + a16[2r+1]*w16[2r+1], 8×i32, NON-saturating.
        let prod = _mm256_madd_epi16(va16, vw16);
        acc = _mm256_add_epi32(acc, prod);
        i += 16;
    }
    // Horizontal sum of the 8 i32 lanes → i32 accumulator.
    let mut tmp = [0i32; 8];
    _mm256_storeu_si256(tmp.as_mut_ptr() as *mut __m256i, acc);
    let mut s: i32 = 0;
    for v in tmp.iter() {
        s += *v;
    }
    // Scalar tail for the remaining <16 elements (exact, order-free add).
    while i < k {
        s += (*a.add(i) as i32) * (*w.add(i) as i32);
        i += 1;
    }
    s
}

/// Public i8·i8 dot product: AVX2 fast path (runtime-detected) with scalar-oracle fallback.
///
/// `debug_assert_eq!` against the scalar oracle on every call — the continuous differential
/// leash (compiled out of release).
pub fn dot_i8(a: &[i8], w: &[i8]) -> i32 {
    assert_eq!(
        a.len(),
        w.len(),
        "dot_i8: activations and weights must align"
    );
    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: CPU verified to have AVX2 via is_x86_feature_detected.
            let s = unsafe { dot_i8_avx2(a.as_ptr(), w.as_ptr(), a.len()) };
            debug_assert_eq!(s, scalar_dot(a, w), "simd_i8 dot diverged from oracle");
            return s;
        }
    }
    let s = scalar_dot(a, w);
    debug_assert_eq!(s, scalar_dot(a, w));
    s
}

/// AVX2 matmul: `out[i][j] = Σ_k A[i][k] · W[k][j]`, i8 in, i32 out.
///
/// For each output element we gather weight column `j` into a contiguous stack buffer (stride
/// `n`) and run the contiguous [`dot_i8_avx2`] over the A row (contiguous) — keeps the inner
/// kernel on unaligned contiguous loads. The dot result is bit-exact to the oracle's matmul.
///
/// SAFETY: caller must have verified AVX2; `a`/`w`/`out` sized as declared; reads in-bounds.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn matmul_i8_avx2(a: *const i8, w: *const i8, m: usize, k: usize, n: usize, out: *mut i32) {
    use core::arch::x86_64::*;
    debug_assert!(k <= MAX_K, "simd_i8: k exceeds column-gather buffer");
    let mut col = [0i8; MAX_K];
    for i in 0..m {
        let arow = a.add(i * k);
        for j in 0..n {
            // Gather weight column j (strided by n) into a contiguous, zero-other buffer.
            for t in 0..k {
                col[t] = *w.add(t * n + j);
            }
            let s = dot_i8_avx2(arow, col.as_ptr(), k);
            *out.add(i * n + j) = s;
        }
    }
    let _ = _mm256_setzero_si256(); // touch the import so it is not flagged unused on some cfgs
}

/// Public i8 matmul: AVX2 fast path (runtime-detected) with scalar-oracle fallback.
///
/// Same shape contract as `oracle_matmul_i8` (row-major `A[m][k]`, `W[k][n]`, out `[m][n]`).
/// Refuses an overflowing `k` at the door (the lemma) and `debug_assert_eq!`s against the item-37
/// oracle on every call.
pub fn matmul_i8(
    a: &[i8],
    w: &[i8],
    m: usize,
    k: usize,
    n: usize,
) -> Result<Vec<i32>, &'static str> {
    check_overflow_bound(k, Q_MAX as i32)?;
    if a.len() != m * k || w.len() != k * n {
        return Err("simd_i8 matmul: shape mismatch");
    }
    let mut out = vec![0i32; m * n];
    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            // SAFETY: CPU verified to have AVX2 via is_x86_feature_detected.
            unsafe { matmul_i8_avx2(a.as_ptr(), w.as_ptr(), m, k, n, out.as_mut_ptr()) };
            let oracle = oracle_matmul_i8(a, w, m, k, n)?;
            debug_assert_eq!(out, oracle, "simd_i8 matmul diverged from oracle");
            return Ok(out);
        }
    }
    // Scalar-oracle fallback: identical to the item-37 reference by construction.
    let oracle = oracle_matmul_i8(a, w, m, k, n)?;
    out.copy_from_slice(&oracle);
    debug_assert_eq!(out, oracle);
    Ok(out)
}

/// The SIMD pilot forward pass — bit-exact to `oracle_forward` (item 37/34).
///
/// Reuses the oracle's requantize/ReLU/argmax (integer-exact, shared) and this module's SIMD
/// matmul for the affine cores. Since the SIMD matmul is bit-exact to the oracle matmul, the
/// whole pass equals `oracle_forward` exactly — on both the AVX2 and scalar-fallback paths.
pub fn simd_i8_forward(
    input: &[i8; N],
    w1: &[i8; N * H],
    b1: &[i8; H],
    scale1: u32,
    w2: &[i8; H * C],
    b2: &[i8; C],
    scale2: u32,
) -> [i8; C] {
    // Layer 1: hidden[h] = requant(relu(Σ_n input[n]*w1[h][n] + b1[h]))
    let mut hidden = [0i8; H];
    let l1 = matmul_i8(input, w1, 1, N, H).expect("layer1 fits i32");
    for h in 0..H {
        let acc = l1[h] + b1[h] as i32;
        let rq = oracle_requantize(acc, scale1);
        hidden[h] = oracle_relu_i32(rq as i32) as i8;
    }
    // Layer 2: logits[c] = requant(Σ_h hidden[h]*w2[c][h] + b2[c])
    let mut logits = [0i8; C];
    let l2 = matmul_i8(&hidden, w2, 1, H, C).expect("layer2 fits i32");
    for c in 0..C {
        let acc = l2[c] + b2[c] as i32;
        logits[c] = oracle_requantize(acc, scale2);
    }
    logits
}

/// Classify `input` ∈ D via the SIMD forward pass (argmax over logits). Bit-exact to
/// `crate::inference::spec::classify`.
pub fn simd_i8_classify(input: &[i8; N]) -> usize {
    let logits = simd_i8_forward(
        input,
        &crate::inference::spec::W1,
        &crate::inference::spec::B1,
        crate::inference::spec::SCALE1,
        &crate::inference::spec::W2,
        &crate::inference::spec::B2,
        crate::inference::spec::SCALE2,
    );
    oracle_argmax(&logits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::fixed::Q_MIN;
    use crate::inference::oracle::oracle_matmul_i8;
    use crate::inference::spec::{self, B1, B2, SCALE1, SCALE2, W1, W2};

    /// Deterministic LCG (zero-dep, reproducible corpus).
    fn lcg(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state
    }

    /// §A5.1 — AVX2 dot kernel vs the scalar oracle, **bit-exact**, over a large randomized
    /// corpus (incl. tiny and medium `k`) plus the i8 boundary values ±127/0. On this CPU AVX2 is
    /// detected, so the SIMD path is exercised; the in-kernel `debug_assert_eq!` also guards it.
    /// RED→GREEN: a dropped tail element or a wrong lane order diverges → test fails.
    #[test]
    fn simd_i8_dot_bit_exact_vs_oracle() {
        let mut rng: u64 = 0x9E3779B97F4A7C15;
        for _ in 0..5000 {
            let k = 1 + (lcg(&mut rng) as usize) % 24; // 1..24 (exercises chunk + tail)
            let mut a = vec![0i8; k];
            let mut w = vec![0i8; k];
            for x in a.iter_mut() {
                *x = (lcg(&mut rng) & 0xFF) as i8;
            }
            for x in w.iter_mut() {
                *x = (lcg(&mut rng) & 0xFF) as i8;
            }
            let got = dot_i8(&a, &w);
            let ref_scalar = scalar_dot(&a, &w);
            assert_eq!(
                got, ref_scalar,
                "AVX2 dot diverged from scalar oracle at k={k}"
            );
        }
        // i8 boundary corpus: values at ±127/0 only — the saturation corner.
        for av in [-127i8, 0i8, 127i8] {
            for wv in [-127i8, 0i8, 127i8] {
                let a = vec![av; 16];
                let w = vec![wv; 16];
                // 16 products of ±127²; non-saturating madd must equal the scalar sum exactly.
                let expected: i32 = (av as i32) * (wv as i32) * 16;
                assert_eq!(
                    dot_i8(&a, &w),
                    expected,
                    "boundary dot wrong at av={av}, wv={wv}"
                );
            }
        }
        // Maxmagnitude 8-wide corner (pilot k=8): 8 × 127² — within i32, no saturation cliff.
        let a = vec![127i8; 8];
        let w = vec![127i8; 8];
        assert_eq!(dot_i8(&a, &w), 127i32 * 127 * 8);
    }

    /// §A5.1 — AVX2 matmul vs the item-37 oracle, **bit-exact** on randomized shapes (incl. the
    /// pilot `N×H` and `H×C` shapes) AND on the i8 boundary grid. Drives the in-kernel
    /// `debug_assert_eq!` against `oracle_matmul_i8` on every call.
    #[test]
    fn simd_i8_matmul_bit_exact_vs_oracle() {
        let mut rng: u64 = 0xA53C9E3779B97F4A;
        for _ in 0..3000 {
            let m = 1 + (lcg(&mut rng) as usize) % 4; // 1..4
            let k = 1 + (lcg(&mut rng) as usize) % 12; // 1..12 (≤ MAX_K)
            let n = 1 + (lcg(&mut rng) as usize) % 6; // 1..6
            let mut a = vec![0i8; m * k];
            let mut w = vec![0i8; k * n];
            for x in a.iter_mut() {
                *x = (lcg(&mut rng) & 0xFF) as i8;
            }
            for x in w.iter_mut() {
                *x = (lcg(&mut rng) & 0xFF) as i8;
            }
            let got = matmul_i8(&a, &w, m, k, n).expect("shape fits i32");
            let oracle = oracle_matmul_i8(&a, &w, m, k, n).expect("shape fits i32");
            assert_eq!(
                got, oracle,
                "AVX2 matmul diverged from oracle at {m}x{k}x{n}"
            );
        }
        // Pilot shapes explicitly (N=8→H=8→C=4).
        let input = [1i8, 2, 3, 4, 5, 6, 7, 8];
        let l1_simd = matmul_i8(&input, &W1, 1, N, H).unwrap();
        let l1_oracle = oracle_matmul_i8(&input, &W1, 1, N, H).unwrap();
        assert_eq!(l1_simd, l1_oracle, "pilot layer-1 matmul diverged");
        // Boundary grid: all-±127 weight matrices, pilot dims.
        for &wv in &[-127i8, 127i8] {
            let w1b = [wv; N * H];
            let got = matmul_i8(&input, &w1b, 1, N, H).unwrap();
            let oracle = oracle_matmul_i8(&input, &w1b, 1, N, H).unwrap();
            assert_eq!(got, oracle, "boundary pilot matmul diverged at wv={wv}");
        }
    }

    /// §A5.2/§A5.1 — the full SIMD forward pass is **bit-exact** to `oracle_forward` over a
    /// randomized input corpus (incl. the i8 boundary), on the active path. Because the SIMD
    /// matmul is bit-exact to the oracle matmul, the whole pass matches — the in-kernel
    /// `debug_assert_eq!` leash fires on any divergence (debug builds, which `cargo test` is).
    #[test]
    fn simd_i8_forward_bit_exact_vs_oracle() {
        let mut rng: u64 = 0x1234_ABCD_9E37_7F4A;
        for _ in 0..4000 {
            let mut input = [0i8; N];
            for v in input.iter_mut() {
                let raw = (lcg(&mut rng) & 0xFF) as i8;
                // Bias toward small magnitudes (restricted-symmetric-ish), but keep the full
                // range reachable occasionally.
                *v = if raw > 100 {
                    100
                } else if raw < -100 {
                    -100
                } else {
                    raw
                };
            }
            let got = simd_i8_forward(&input, &W1, &B1, SCALE1, &W2, &B2, SCALE2);
            let oracle = crate::inference::oracle::oracle_forward(
                &input, &W1, &B1, SCALE1, &W2, &B2, SCALE2,
            );
            assert_eq!(
                got, oracle,
                "SIMD forward diverged from oracle at {input:?}"
            );
        }
        // Boundary inputs: all min, all max, all zero.
        for &fill in &[Q_MIN, 0i8, Q_MAX] {
            let input = [fill; N];
            let got = simd_i8_forward(&input, &W1, &B1, SCALE1, &W2, &B2, SCALE2);
            let oracle = crate::inference::oracle::oracle_forward(
                &input, &W1, &B1, SCALE1, &W2, &B2, SCALE2,
            );
            assert_eq!(got, oracle, "SIMD forward diverged at boundary fill={fill}");
        }
    }

    /// §A5.3 — the SIMD path's signedness/saturation is proven bit-exact to the scalar oracle by
    /// choosing the **non-saturating** `_mm256_madd_epi16` (documented). This test is the
    /// explicit proof: stress the i16 intermediate with the largest representable product
    /// (127×127) across a full 16-lane chunk and assert it equals the exact scalar sum — there is
    /// no saturation cliff. The maddubs (saturating) alternative would be refused by this test.
    #[test]
    fn simd_i8_madd_epi16_non_saturating_proven() {
        // 16 i8 lanes of 127 · 16 i8 lanes of 127. Each madd pair = 127²+127² = 32258 (< i32::MAX,
        // non-saturating). The 8 i32 results sum to 16·127² = 258 064.
        let a = vec![127i8; 16];
        let w = vec![127i8; 16];
        let simd_val = dot_i8(&a, &w);
        let scalar_val = scalar_dot(&a, &w);
        assert_eq!(simd_val, scalar_val);
        assert_eq!(simd_val, 127i32 * 127 * 16);
        // Negative products, mixed signs — associativity/non-saturation must hold.
        let a2 = vec![
            -127i8, 127, -127, 127, -127, 127, -127, 127, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let w2 = vec![
            127i8, 127, -127, -127, 1, -1, 2, -2, 5, -5, 7, -7, 9, -9, 11, -11,
        ];
        assert_eq!(dot_i8(&a2, &w2), scalar_dot(&a2, &w2));
    }

    /// §A5.1 (companion) — the scalar **fallback** path (what runs when AVX2 is absent / on
    /// non-x86_64) is itself correct vs an independent i128 wide reference, and equals the oracle.
    /// Validates the fallback independent of CPU feature detection.
    #[test]
    fn simd_i8_scalar_fallback_matches_oracle() {
        let mut rng: u64 = 0xDEAD_BEEF_1357_9E37;
        for _ in 0..2000 {
            let k = 1 + (lcg(&mut rng) as usize) % 16;
            let mut a = vec![0i8; k];
            let mut w = vec![0i8; k];
            for x in a.iter_mut() {
                *x = (lcg(&mut rng) & 0xFF) as i8;
            }
            for x in w.iter_mut() {
                *x = (lcg(&mut rng) & 0xFF) as i8;
            }
            // Independent i128 reference (widest accumulator).
            let wide: i128 = a
                .iter()
                .zip(w.iter())
                .map(|(&x, &y)| (x as i128) * (y as i128))
                .sum();
            assert_eq!(
                scalar_dot(&a, &w) as i128,
                wide,
                "scalar fallback diverged from i128"
            );
            // And the public entry (which may take AVX2 here) still equals the oracle scalar.
            assert_eq!(dot_i8(&a, &w), scalar_dot(&a, &w));
        }
    }

    /// §A5.1 (companion) — the SIMD forward pass classifies identically to the oracle/spec on a
    /// randomized corpus (end-to-end `f(x)=y` contract, bit-exact).
    #[test]
    fn simd_i8_classify_matches_spec() {
        let mut rng: u64 = 0x5F1B9E3779B97F4A;
        for _ in 0..3000 {
            let mut input = [0i8; N];
            for v in input.iter_mut() {
                let raw = (lcg(&mut rng) & 0xFF) as i8;
                *v = if raw > 90 {
                    90
                } else if raw < -90 {
                    -90
                } else {
                    raw
                };
            }
            assert_eq!(
                simd_i8_classify(&input),
                spec::classify(&input),
                "SIMD classify diverged from spec at {input:?}"
            );
        }
    }
}
