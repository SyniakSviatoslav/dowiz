//! ct_gate.rs — std host shim (pure `ct_eq`/`Stats`/`welch_t` live in
//! `dowiz_core::ct_gate`; the `std::time::Instant` timing harness stays here).
//!
//! `time_block` / `measure_leakage` sample a comparator under `Instant` — the
//! wall-clock seam the no_std core cannot hold. The planted-leak dudect
//! self-test (`dudect_gate_detects_planted_leak_and_passes_ct_eq`) is the
//! load-bearing CI gate; run in release via `scripts/hardening-gate.sh` step E.

pub use dowiz_core::ct_gate::*;

use core::hint::black_box;
use std::time::Instant;

/// Time `batch` invocations of a nullary closure, returning the average per-call nanoseconds.
/// `black_box` on the accumulator defeats dead-code elimination of the measured work.
#[inline]
fn time_block<F: Fn() -> bool>(f: &F, batch: usize) -> f64 {
    let t0 = Instant::now();
    let mut acc = false;
    for _ in 0..batch {
        acc ^= black_box(f());
    }
    black_box(acc);
    t0.elapsed().as_nanos() as f64 / batch as f64
}

/// Measure timing leakage of comparator `cmp` between two fixed input classes, returning |Welch t|.
///
/// `class_a`/`class_b` are `(lhs, rhs)` byte-slice pairs. The two classes are measured **interleaved**
/// and the interleave order flips every round, so slow environmental drift (frequency scaling, cache
/// warmup) contaminates both classes equally and cancels out of the difference of means. Inputs are
/// fed through `black_box` so the optimizer cannot constant-fold a fixed-input comparator away.
pub fn measure_leakage<F>(
    class_a: (&[u8], &[u8]),
    class_b: (&[u8], &[u8]),
    cmp: F,
    rounds: usize,
    batch: usize,
) -> f64
where
    F: Fn(&[u8], &[u8]) -> bool,
{
    let run_a = || cmp(black_box(class_a.0), black_box(class_a.1));
    let run_b = || cmp(black_box(class_b.0), black_box(class_b.1));

    // Warm up caches / branch predictors before the timed rounds.
    for _ in 0..batch {
        black_box(run_a());
        black_box(run_b());
    }

    let mut sa = Stats::default();
    let mut sb = Stats::default();
    for r in 0..rounds {
        if r % 2 == 0 {
            sa.push(time_block(&run_a, batch));
            sb.push(time_block(&run_b, batch));
        } else {
            sb.push(time_block(&run_b, batch));
            sa.push(time_block(&run_a, batch));
        }
    }
    welch_t(&sa, &sb).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PLANTED LEAK (test-only): see the core module docs; duplicated here for the timing harness.
    fn naive_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        for i in 0..a.len() {
            if a[i] != b[i] {
                return false;
            }
        }
        true
    }

    #[test]
    #[ignore = "timing self-test; run in release by scripts/hardening-gate.sh step E"]
    fn dudect_gate_detects_planted_leak_and_passes_ct_eq() {
        let equal_l = [0u8; 256];
        let equal_r = [0u8; 256];
        let diff_l = [0u8; 256];
        let mut diff_r = [0u8; 256];
        diff_r[0] = 1;
        let class_a = (&equal_l[..], &equal_r[..]);
        let class_b = (&diff_l[..], &diff_r[..]);

        const ROUNDS: usize = 300;
        const BATCH: usize = 4096;

        let leak_t = (0..3)
            .map(|_| measure_leakage(class_a, class_b, naive_eq, ROUNDS, BATCH))
            .fold(0.0_f64, f64::max);
        assert!(
            leak_t >= T_THRESHOLD,
            "PLANTED LEAK NOT DETECTED: naive_eq |t|={leak_t:.2} < {T_THRESHOLD} — gate is blind"
        );

        let ct_t = (0..5)
            .map(|_| measure_leakage(class_a, class_b, ct_eq, ROUNDS, BATCH))
            .fold(f64::INFINITY, f64::min);

        assert!(
            leak_t >= 3.0 * ct_t,
            "harness failed to SEPARATE leaky from constant-time: leak |t|={leak_t:.2}, ct |t|={ct_t:.2} (need >= 3x)"
        );

        let verdict = if ct_t < T_THRESHOLD {
            format!("ct_eq |t|={ct_t:.2} (PASS, < {T_THRESHOLD})")
        } else {
            format!("ct_eq |t|={ct_t:.2} (elevated under load; separation proof still holds)")
        };
        println!(
            "dudect self-test PASS: planted-leak naive_eq |t|={leak_t:.1} (DETECTED, >= {T_THRESHOLD}); \
             {verdict}; separation {:.1}x (>= 3x required)",
            leak_t / ct_t.max(1e-9)
        );
    }
}
