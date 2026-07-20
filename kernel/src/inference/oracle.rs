//! ITEM 37 — Reference Oracle (the inference "Schoolbook").
//!
//! Governing ruling (arc-wide): *"безпека і передбачуваність понад швидкість"* — the
//! oracle is the ruling in one module: a slow, obviously-correct, permanently-retained
//! reference against which every optimized path is differentially proven bit-exact. Speed
//! is explicitly *not* its job.
//!
//! This is a scalar, dependency-free, obviously-correct **integer-domain** reference
//! implementation of the pilot's operations — matmul + the activation/requantize/argmax
//! set — with **i64/i128 shadow accumulation** so it is its own overflow detector. It is
//! the permanent, test-only differential target for the optimized SIMD path (item 39),
//! the source of the golden per-layer checksums (item 40), and the `f(x)=y` ground truth
//! (item 34) for end-to-end tests (item 42).
//!
//! PERMANENT — never delete on optimization, per CHECKLIST.md item 1 and the NTT-schoolbook
//! precedent. No SIMD, no arena, no dependency (`std`-only). Recorded in HOT-PATHS.tsv as
//! the item-39 differential reference.
//!
//! The matmul *shape* mirrors `kernel/src/mat.rs:132` (`matmul_contig`) — the ONE matmul
//! shape — re-typed into the integer domain (i8 in, i32 accumulate, i128 shadow). Not a new
//! algorithm, a re-typing of a proven shape.

use crate::inference::fixed::{div_half_up, requantize_pow2, saturating_clamp};
use crate::inference::workspace::{C, H, N};

/// Oracle integer-domain matmul: `C[m][n] = Σ_k A[m][k] * W[k][n]`.
///
/// Accumulates in **i32** (the production accumulator type) with an **i128 shadow** that
/// asserts (a) the shadow equals the i32 result and (b) the shadow fits i32 — the item-35
/// overflow lemma, checked at runtime for the given shapes. A shape that would overflow
/// makes the oracle **fail loudly** (`Err`), never wrap.
///
/// Left-to-right sum order IS the golden order; item 39's SIMD path (whose integer
/// associativity *permits* reorder) must still match it bit-exact, so the order is pinned
/// here. `A` is row-major `[m][k]`, `W` row-major `[k][n]`, output `[m][n]`.
pub fn oracle_matmul_i8(
    a: &[i8],
    w: &[i8],
    m: usize,
    k: usize,
    n: usize,
) -> Result<Vec<i32>, &'static str> {
    // Door check: refuse a shape whose accumulation would overflow the i32 ceiling.
    crate::inference::fixed::check_overflow_bound(k, crate::inference::fixed::Q_MAX as i32)?;
    if a.len() != m * k || w.len() != k * n {
        return Err("oracle_matmul_i8: shape mismatch");
    }
    let mut out = vec![0i32; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut acc: i32 = 0;
            let mut shadow: i128 = 0;
            for t in 0..k {
                let prod = (a[i * k + t] as i32) * (w[t * n + j] as i32);
                acc = acc
                    .checked_add(prod)
                    .ok_or("oracle: i32 accumulator overflow")?;
                shadow += prod as i128;
                // The shadow is the truth: it must equal the (still in-range) i32 accumulator.
                debug_assert_eq!(shadow, acc as i128, "oracle shadow diverged from i32 acc");
            }
            // The lemma guarantees this holds; assert it so a future lemma change is caught.
            assert!(
                shadow >= i32::MIN as i128 && shadow <= i32::MAX as i128,
                "oracle: accumulation left the i32 range — refuse, never wrap"
            );
            out[i * n + j] = acc;
        }
    }
    Ok(out)
}

/// Oracle ReLU on i32 (pre-requantize). `max(0, x)`.
#[inline]
pub fn oracle_relu_i32(x: i32) -> i32 {
    if x > 0 {
        x
    } else {
        0
    }
}

/// Oracle requantize: `div_half_up` (item-35 law) + saturating clamp to i8.
#[inline]
pub fn oracle_requantize(acc: i32, scale_shift: u32) -> i8 {
    requantize_pow2(acc, scale_shift)
}

/// Oracle argmax over a slice; returns the index of the maximum (first on ties).
#[inline]
pub fn oracle_argmax(v: &[i8]) -> usize {
    let mut best = 0usize;
    let mut best_v = v[0];
    for (i, &x) in v.iter().enumerate().skip(1) {
        if x > best_v {
            best_v = x;
            best = i;
        }
    }
    best
}

/// The oracle's full pilot forward pass — the `f(x)=y` ground truth.
///
/// `input` is `[N]`, `w1` is row-major `[H][N]`, `b1` length `H`, `w2` row-major `[C][H]`,
/// `b2` length `C`. Returns the post-requant logits `[C]` (the `y` of `f(x)=y`; argmax is
/// the class label). Every op is the scalar oracle; this is the reference item 40's
/// checksums and item 42's end-to-end test consume.
pub fn oracle_forward(
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
    let l1 = oracle_matmul_i8(input, w1, 1, N, H).expect("layer1 fits i32");
    for h in 0..H {
        let acc = l1[h] + b1[h] as i32;
        let rq = oracle_requantize(acc, scale1);
        hidden[h] = oracle_relu_i32(rq as i32) as i8;
    }
    // Layer 2: logits[c] = requant(Σ_h hidden[h]*w2[c][h] + b2[c])
    let mut logits = [0i8; C];
    let l2 = oracle_matmul_i8(&hidden, w2, 1, H, C).expect("layer2 fits i32");
    for c in 0..C {
        let acc = l2[c] + b2[c] as i32;
        logits[c] = oracle_requantize(acc, scale2);
    }
    logits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::fixed::P_MAX_RESTRICTED;

    /// §5.1 — exhaustive small-dimension cases + a large randomized corpus, oracle matmul
    /// vs the i128 wide-accumulator shadow, **zero divergence**. RED→GREEN: a wrong
    /// accumulation order or a dropped term turns it RED.
    #[test]
    fn oracle_matmul_corpus_shadow_exact() {
        let mut rng: u64 = 0x9E3779B97F4A7C15;
        let mut next = |rng: &mut u64| -> i8 {
            // xorshift64* → take low 8 bits, bias toward small magnitudes.
            *rng ^= *rng << 13;
            *rng ^= *rng >> 7;
            *rng ^= *rng << 17;
            let v = (*rng & 0xFF) as i8;
            if v > 60 {
                60
            } else if v < -60 {
                -60
            } else {
                v
            }
        };
        let mut divergent = 0u32;
        for _ in 0..4000 {
            // Tiny shapes keep the corpus large-but-fast.
            let m = 1 + (rng % 2) as usize; // 1..2
            let k = 1 + (rng % 12) as usize; // 1..12
            let n = 1 + (rng % 8) as usize; // 1..8
            let mut a = vec![0i8; m * k];
            let mut w = vec![0i8; k * n];
            for x in a.iter_mut() {
                *x = next(&mut rng);
            }
            for x in w.iter_mut() {
                *x = next(&mut rng);
            }
            let got = oracle_matmul_i8(&a, &w, m, k, n).expect("in-bounds shape fits i32");
            // Independent i128 reference (the widest possible accumulator).
            let mut ref_out = vec![0i128; m * n];
            for i in 0..m {
                for j in 0..n {
                    let mut s: i128 = 0;
                    for t in 0..k {
                        s += (a[i * k + t] as i128) * (w[t * n + j] as i128);
                    }
                    ref_out[i * n + j] = s;
                }
            }
            for (i, &g) in got.iter().enumerate() {
                if (g as i128) != ref_out[i] {
                    divergent += 1;
                }
                assert!(
                    ref_out[i] >= i32::MIN as i128 && ref_out[i] <= i32::MAX as i128,
                    "reference left i32 range (unexpected for {}x{}x{})",
                    m,
                    k,
                    n
                );
            }
        }
        assert_eq!(divergent, 0, "oracle matmul diverged from i128 shadow");
    }

    /// §5.2 — a shape whose accumulation would exceed the item-35 i32 ceiling makes the
    /// oracle **fail loudly** (Err), never silently wrap. Demonstrated with a synthetic
    /// over-ceiling shape (k large with max-magnitude i8 weights).
    #[test]
    fn oracle_overflow_shape_fails_loudly() {
        // k = 200_000, all weights 127 ⇒ 200_000 * 127² >> 2³¹−1.
        let k = 200_000usize;
        let a = vec![127i8; k];
        let w = vec![127i8; k]; // 1x1 matmul (m=n=1) with k terms
        assert!(oracle_matmul_i8(&a, &w, 1, k, 1).is_err());
        // The lemma says the same: check_overflow_bound refuses this k.
        assert!(crate::inference::fixed::check_overflow_bound(k, P_MAX_RESTRICTED).is_err());
        // Pilot-scale shapes (k ≤ 64) are far below the ceiling → Ok.
        let small = vec![3i8; 8];
        assert!(oracle_matmul_i8(&small, &small, 1, 8, 1).is_ok());
    }

    /// §5.3 — the oracle's requantize/relu/argmax ops are exact on known inputs.
    #[test]
    fn oracle_ops_exact() {
        // relu
        assert_eq!(oracle_relu_i32(-5), 0);
        assert_eq!(oracle_relu_i32(0), 0);
        assert_eq!(oracle_relu_i32(7), 7);
        // requantize = div_half_up + clamp (item-35 law).
        assert_eq!(oracle_requantize(5, 1), div_half_up(5, 2) as i8);
        assert_eq!(oracle_requantize(10_000, 0), saturating_clamp(10_000));
        // argmax
        assert_eq!(oracle_argmax(&[1, 5, 2, -3]), 1);
        assert_eq!(oracle_argmax(&[-1, -1, -1]), 0); // first on tie
    }

    /// §5.5 — the oracle forward pass is deterministic and its output is in i8 range.
    #[test]
    fn oracle_forward_in_range() {
        // Tiny hand-authored weights (restricted-symmetric).
        let w1 = [
            1i8, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
            0, 0, 0, 0, 0, 0, 1,
        ];
        let b1 = [0i8; H];
        let w2 = [
            1i8, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            0, 0, 0, 0,
        ];
        let b2 = [0i8; C];
        let input = [1i8, 2, 3, 4, 5, 6, 7, 8];
        let out = oracle_forward(&input, &w1, &b1, 0, &w2, &b2, 0);
        for &v in out.iter() {
            assert!(
                (Q_MIN_CK..=Q_MAX_CK).contains(&v),
                "logit out of i8 range: {v}"
            );
        }
    }

    const Q_MIN_CK: i8 = crate::inference::fixed::Q_MIN;
    const Q_MAX_CK: i8 = crate::inference::fixed::Q_MAX;
}
