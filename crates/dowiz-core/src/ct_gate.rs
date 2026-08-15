#![allow(unused)]
//! ct_gate.rs — minimal, zero-dependency dudect-style constant-time gate.
//!
//! The no_std core ships the reusable primitives: `ct_eq` (constant-time byte
//! equality), `Stats` (Welford accumulator), and `welch_t` (Welch's two-sample t
//! statistic). The timing harness (`time_block` / `measure_leakage`, which need
//! `std::time::Instant`) lives in the kernel held-handle shim.
//!
//! `ct_eq` is the kernel's reusable CT-equality primitive — the first intended
//! production caller is the P91.2 constant-time fix for the `pq/kem.rs` /
//! `pq/hybrid.rs` tag compares.

/// The standard dudect acceptance threshold: |t| below this is indistinguishable-from-constant-time
/// at the sample sizes used here; at/above it, a secret-dependent timing channel is detectable.
pub const T_THRESHOLD: f64 = 4.5;

/// Constant-time byte-slice equality. Branch-free over the byte content: every byte of the (equal-
/// length) inputs is XOR-accumulated regardless of value, so the run-time does not depend on *where*
/// the inputs first differ. Length is public (an attacker already knows tag/key sizes), so the
/// length pre-check is an allowed public branch — the secret-dependent part is the byte loop, and it
/// has no early exit. The final `acc == 0` is a single O(1) reduction, not a per-byte branch.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc: u8 = 0;
    for i in 0..a.len() {
        acc |= a[i] ^ b[i];
    }
    acc == 0
}

/// Online mean/variance accumulator (Welford, 1962) — zero-alloc, one pass, numerically stable.
#[derive(Clone, Copy, Default)]
pub struct Stats {
    n: f64,
    mean: f64,
    m2: f64,
}

impl Stats {
    #[inline]
    pub fn push(&mut self, x: f64) {
        self.n += 1.0;
        let d = x - self.mean;
        self.mean += d / self.n;
        let d2 = x - self.mean;
        self.m2 += d * d2;
    }
    #[inline]
    pub fn n(&self) -> f64 {
        self.n
    }
    #[inline]
    pub fn mean(&self) -> f64 {
        self.mean
    }
    /// Sample variance (Bessel-corrected). Zero for n < 2.
    #[inline]
    pub fn var(&self) -> f64 {
        if self.n < 2.0 {
            0.0
        } else {
            self.m2 / (self.n - 1.0)
        }
    }
}

/// Welch's two-sample t statistic (unequal variances) between two timing classes.
/// Returns 0 when both classes have zero variance (identical, degenerate samples).
pub fn welch_t(a: &Stats, b: &Stats) -> f64 {
    if a.n() < 2.0 || b.n() < 2.0 {
        return 0.0;
    }
    let denom = crate::math::sqrt(a.var() / a.n() + b.var() / b.n());
    if denom == 0.0 {
        return 0.0;
    }
    (a.mean() - b.mean()) / denom
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PLANTED LEAK (test-only): the classic variable-time compare. Returns as soon as it finds a
    /// differing byte, so its run-time leaks the position of the first difference — exactly the
    /// timing channel a dudect gate must catch. The gate's whole credibility rests on rejecting
    /// this one with the same machinery it uses to accept `ct_eq`.
    fn naive_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        for i in 0..a.len() {
            if a[i] != b[i] {
                return false; // early return — the leak
            }
        }
        true
    }

    // ── pure-logic unit tests (run in the default suite; no timing, no flakiness) ──────────────

    #[test]
    fn ct_eq_matches_naive_eq_on_semantics() {
        let cases: &[(&[u8], &[u8])] = &[
            (b"", b""),
            (b"abc", b"abc"),
            (b"abc", b"abd"),
            (b"abc", b"ab"),
            (b"\x00\x00", b"\x00\x00"),
            (b"\xff\x00\xff", b"\xff\x00\xfe"),
        ];
        for (a, b) in cases {
            assert_eq!(
                ct_eq(a, b),
                naive_eq(a, b),
                "ct_eq disagrees on {a:?} vs {b:?}"
            );
        }
    }

    #[test]
    fn welch_t_is_zero_for_identical_classes() {
        let mut s = Stats::default();
        for x in [1.0, 2.0, 3.0, 4.0] {
            s.push(x);
        }
        // same distribution twice → mean difference 0 → t = 0
        assert_eq!(welch_t(&s, &s), 0.0);
    }

    #[test]
    fn welch_t_is_large_for_separated_classes() {
        let mut lo = Stats::default();
        let mut hi = Stats::default();
        for i in 0..100 {
            // small nonzero variance in each class, huge mean gap between them
            lo.push(1.0 + (i % 2) as f64 * 0.1);
            hi.push(100.0 + (i % 2) as f64 * 0.1);
        }
        // tiny within-class variance, huge mean gap → |t| explodes
        assert!(welch_t(&lo, &hi).abs() > 4.5);
    }

    // ── stats / welch-t edge cases ────────────────────────────────────────────

    #[test]
    fn stats_var_zero_for_single_sample() {
        let mut s = Stats::default();
        s.push(5.0);
        assert!((s.var() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn stats_var_positive_for_multiple_samples() {
        let mut s = Stats::default();
        s.push(1.0);
        s.push(3.0);
        assert!(s.var() > 0.0);
    }

    #[test]
    fn welch_t_zero_for_small_samples() {
        let mut a = Stats::default();
        a.push(1.0);
        let mut b = Stats::default();
        b.push(2.0);
        assert_eq!(welch_t(&a, &b), 0.0);
    }

    #[test]
    fn welch_t_zero_for_zero_variance() {
        let mut a = Stats::default();
        a.push(5.0);
        a.push(5.0);
        let mut b = Stats::default();
        b.push(5.0);
        b.push(5.0);
        assert_eq!(welch_t(&a, &b), 0.0);
    }

    #[test]
    fn ct_eq_diff_len() {
        assert!(!ct_eq(b"hello", b"hell"));
    }

    #[test]
    fn ct_eq_same() {
        assert!(ct_eq(b"abc", b"abc"));
    }

    #[test]
    fn ct_eq_diff_same_len() {
        assert!(!ct_eq(b"\x00\x01", b"\x00\x02"));
    }
}
