//! simd.rs — P11 §6 `f64x4` struct-of-arrays (SoA) SIMD batch lane.
//!
//! **Design rule that guarantees bit-identity (BLUEPRINT-P11 §6):** vectorise
//! *ACROSS* the batch of independent rows, never *WITHIN* a single row's
//! reduction. A 4-wide register holds 4 independent rows' values at the same
//! column index; each lane replays the *exact* scalar op sequence for its own
//! row — row-max, `exp(x−max)`, the fixed left-to-right sum, and the divide —
//! so per-lane arithmetic order is unchanged and the lane is bit-identical to
//! the scalar single-row path (`attention.rs::softmax`).
//!
//! This is a pure property of the op order, not luck: max is exact (order-free),
//! `exp` is a per-element unary op (order-free), and the sum is accumulated
//! *per lane*, in column order, exactly like scalar `exps.iter().sum()`.
//!
//! Runtime detection mirrors `householder.rs` (`is_x86_feature_detected!("avx2")`
//! → AVX2 lane; scalar fallback otherwise). No new dependency. The bit-identity
//! holds in BOTH paths: the scalar fallback trivially matches, and the AVX2 lane
//! matches because per-lane order is unchanged.
//!
//! **Consumers:** `softmax_batch_lane` (the batch-of-rows softmax reduction from
//! `attention.rs`). The N-courier Kalman SoA consumer from §6 is a TODO — the
//! `f64x4` lane primitive here is exactly the substrate it needs; integrating
//! `kalman.rs` is deferred to avoid touching the per-courier filter authority
//! (noted, not done, per task scope).

/// Scalar reference softmax — mirror of `attention.rs::softmax` *exactly* (same
/// op order, same `exps.iter().sum()` left-to-right reduction). Used by the
/// bit-identity falsifier tests and as the scalar fallback path.
pub fn softmax_scalar(xs: &[f64]) -> Vec<f64> {
    if xs.is_empty() {
        return Vec::new();
    }
    let mut m = xs[0];
    for &x in &xs[1..] {
        if x > m {
            m = x;
        }
    }
    let exps: Vec<f64> = xs.iter().map(|&x| (x - m).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

/// Process up to 4 independent softmax rows per SIMD step (struct-of-arrays).
///
/// Bit-identical to calling [`softmax_scalar`] once per row. Only compiled on
/// x86_64 with the AVX2 target feature (caller must guarantee the CPU has AVX2
/// via `is_x86_feature_detected!`). `rows` has length 1..=4.
///
/// SAFETY/CORRECTNESS notes (no `unsafe` preconditions on the *caller* beyond
/// the AVX2 feature gate — all intrinsic use is in-bounds):
///   * `max` is computed first in a separate pass with `-inf` padding for
///     short/inactive lanes, so padding can never contaminate the row max.
///   * The exponential/sum pass zero-pads short/inactive lanes *before* the
///     per-lane add, so a padded lane contributes exactly 0.0 to its own sum
///     and is never written to output.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn softmax_lane4(rows: &[&[f64]]) -> Vec<Vec<f64>> {
    use core::arch::x86_64::*;
    let k = rows.len(); // 1..=4
    debug_assert!(k <= 4 && k >= 1);

    // Active lanes = non-empty rows. Inactive (empty) rows produce an empty
    // output and are masked out of every SIMD reduction (kept NaN-free).
    let mut active = [false; 4];
    let mut lens = [0usize; 4];
    let mut max_len = 0usize;
    for (lane, r) in rows.iter().enumerate() {
        active[lane] = !r.is_empty();
        lens[lane] = r.len();
        if r.len() > max_len {
            max_len = r.len();
        }
    }

    // ── Pass 1: per-lane row-max (exact, order-free). ──────────────────────
    // Init to -inf; max with -inf-padded short lanes leaves the real max intact.
    let mut max_arr = [f64::NEG_INFINITY; 4];
    let mut max_vec = _mm256_loadu_pd(max_arr.as_ptr());
    for j in 0..max_len {
        let mut vals = [f64::NEG_INFINITY; 4];
        for lane in 0..k {
            if active[lane] && j < lens[lane] {
                vals[lane] = rows[lane][j];
            }
        }
        let val_vec = _mm256_loadu_pd(vals.as_ptr());
        max_vec = _mm256_max_pd(max_vec, val_vec);
    }
    _mm256_storeu_pd(max_arr.as_mut_ptr(), max_vec);
    // Reload as a *constant* vector reused across the exp/sum pass.
    let max_vec = _mm256_loadu_pd(max_arr.as_ptr());

    // ── Pass 2: exp(x-max) + left-to-right per-lane sum, then divide. ───────
    let mut outs: Vec<Vec<f64>> = rows.iter().map(|r| vec![0.0f64; r.len()]).collect();
    // sum_vec holds 4 independent running accumulators (one per lane).
    let mut sum_vec = _mm256_setzero_pd();

    for j in 0..max_len {
        // Gather this column's 4 values (0.0 pad for short/inactive lanes).
        let mut vals = [0.0f64; 4];
        for lane in 0..k {
            if active[lane] && j < lens[lane] {
                vals[lane] = rows[lane][j];
            }
        }
        let val_vec = _mm256_loadu_pd(vals.as_ptr());
        // diff = val - max ; per-element subtraction → identical to scalar.
        let diff = _mm256_sub_pd(val_vec, max_vec);
        let mut diff_arr = [0.0f64; 4];
        _mm256_storeu_pd(diff_arr.as_mut_ptr(), diff);
        // exp is a per-element unary op (order-free); extract + call f64::exp.
        let mut exp_arr = [0.0f64; 4];
        for lane in 0..4 {
            exp_arr[lane] = diff_arr[lane].exp();
        }
        // Zero out short/inactive lanes so they add exactly 0.0 to their own
        // sum and are never written to output (keeps NaN-free + bit-clean).
        for lane in 0..k {
            if !(active[lane] && j < lens[lane]) {
                exp_arr[lane] = 0.0;
            }
        }
        let exp_vec = _mm256_loadu_pd(exp_arr.as_ptr());
        // Per-lane add, in COLUMN ORDER = scalar left-to-right sum. Identical.
        sum_vec = _mm256_add_pd(sum_vec, exp_vec);
        // Store the (real) exps into each active row's output buffer.
        for lane in 0..k {
            if active[lane] && j < lens[lane] {
                outs[lane][j] = exp_arr[lane];
            }
        }
    }

    // ── Divide each exp by its row's (per-lane) sum — same as scalar. ────────
    let mut sum_arr = [0.0f64; 4];
    _mm256_storeu_pd(sum_arr.as_mut_ptr(), sum_vec);
    for lane in 0..k {
        if !active[lane] {
            continue; // empty row → already empty output
        }
        let s = sum_arr[lane];
        for j in 0..lens[lane] {
            outs[lane][j] /= s;
        }
    }
    outs
}

/// Batch softmax over many independent rows, 4 rows per SIMD step.
///
/// Bit-identical to applying [`softmax_scalar`] to each row. Rows whose count is
/// not a multiple of 4 are handled by a scalar tail (also bit-identical). When
/// AVX2 is unavailable (or on non-x86_64) the entire batch falls back to the
/// scalar path.
pub fn softmax_batch_lane(rows: &[&[f64]]) -> Vec<Vec<f64>> {
    let mut out = Vec::with_capacity(rows.len());
    let mut i = 0;

    // AVX2 fast path: consume rows in chunks of 4 via the SoA lane.
    #[cfg(all(target_arch = "x86_64", feature = "std"))]
    {
        if std::is_x86_feature_detected!("avx2") {
            while i + 4 <= rows.len() {
                // SAFETY: CPU verified to have AVX2 via is_x86_feature_detected.
                let res = unsafe { softmax_lane4(&rows[i..i + 4]) };
                out.extend(res);
                i += 4;
            }
        }
    }

    // Scalar tail — covers both the <4 remainder AND every row when AVX2 is
    // not detected (the whole batch runs through here). Bit-identical either way.
    for r in &rows[i..] {
        out.push(softmax_scalar(r));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic LCG so the randomised battery is reproducible (zero-dep).
    fn lcg(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state
    }

    /// Borrow a `Vec<Vec<f64>>` as a slice of row-references for `softmax_batch_lane`.
    fn as_rows(battery: &[Vec<f64>]) -> Vec<&[f64]> {
        battery.iter().map(|r| r.as_slice()).collect()
    }

    /// Build `n_rows` random rows of random length (1..=max_len) with finite
    /// f64 values in roughly [-10, 10].
    fn random_battery(n_rows: usize, max_len: usize, seed: u64) -> Vec<Vec<f64>> {
        let mut state = seed;
        let mut rows = Vec::with_capacity(n_rows);
        for _ in 0..n_rows {
            let len = 1 + (lcg(&mut state) as usize) % max_len;
            let mut row = Vec::with_capacity(len);
            for _ in 0..len {
                let bits = lcg(&mut state);
                // map to [-10, 10], keep finite
                let frac = ((bits >> 11) as f64) / ((1u64 << 53) as f64);
                row.push(frac * 20.0 - 10.0);
            }
            rows.push(row);
        }
        rows
    }

    #[test]
    fn simd_softmax_bit_identical_to_scalar() {
        // 50 random rows of random length — exercises AVX2 chunks of 4 plus a
        // scalar tail (50 = 12*4 + 2). Asserts EXACT (bit-for-bit) equality.
        let battery = random_battery(50, 20, 0x9E3779B97F4A7C15);
        let refs: Vec<Vec<f64>> = battery.iter().map(|r| softmax_scalar(r)).collect();
        let got = softmax_batch_lane(&as_rows(&battery));
        assert_eq!(got.len(), refs.len());
        for (g, r) in got.iter().zip(refs.iter()) {
            assert_eq!(g.len(), r.len());
            for (a, b) in g.iter().zip(r.iter()) {
                assert_eq!(*a, *b, "bit-identical mismatch: {} vs {}", a, b);
            }
        }
    }

    #[test]
    fn simd_softmax_handles_non_multiple_of_four() {
        // Row counts deliberately not divisible by 4 (1, 2, 3, 5, 7, 11) to
        // hammer the scalar-tail path on every chunk boundary.
        for n in [1usize, 2, 3, 5, 7, 11, 13, 23] {
            let battery = random_battery(n, 12, 0x1234_ABCD + n as u64);
            let refs: Vec<Vec<f64>> = battery.iter().map(|r| softmax_scalar(r)).collect();
            let got = softmax_batch_lane(&as_rows(&battery));
            assert_eq!(got.len(), n);
            for (g, r) in got.iter().zip(refs.iter()) {
                assert_eq!(g, r, "n={} row mismatch (scalar tail)", n);
            }
        }
    }

    #[test]
    fn simd_softmax_empty_rows() {
        // Empty rows must round-trip as empty (scalar returns empty too).
        let battery: Vec<Vec<f64>> = vec![
            vec![],
            vec![1.0, 2.0, 3.0],
            vec![],
            vec![-0.5, 0.0, 0.5, 1.0],
        ];
        let refs: Vec<Vec<f64>> = battery.iter().map(|r| softmax_scalar(r)).collect();
        let got = softmax_batch_lane(&as_rows(&battery));
        assert_eq!(got, refs);
    }

    #[test]
    fn simd_softmax_uniform_and_hand_oracle() {
        // Uniform logits → uniform distribution (bit-identical to scalar ref).
        let u = softmax_batch_lane(&[&[0.0, 0.0, 0.0][..]]);
        for x in &u[0] {
            assert_eq!(*x, 1.0 / 3.0);
        }
        // Hand oracle: ln2 → [2/3, 1/3].
        let w = softmax_batch_lane(&[&[std::f64::consts::LN_2, 0.0][..]]);
        assert_eq!(w[0][0], 2.0 / 3.0);
        assert_eq!(w[0][1], 1.0 / 3.0);
    }
}
