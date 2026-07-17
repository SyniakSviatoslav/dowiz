//! bounded_drainer.rs — P11 §3 bounded work-unit drainer (zero-dep).
//!
//! The CPU-side analogue of the fixed-timestep loop's spiral-of-death guard
//! (`engine/src/loop_.rs::MAX_SUBSTEPS`, lines 25/68–77). For operations that are
//! heavy *once* rather than *per frame* — a full numeric eigendecomposition on a
//! large graph, a backup, a re-index — a heavy op must never monopolise a tick or
//! blow the compute budget in one burst. `BoundedDrainer` consumes AT MOST `k`
//! work-units per `tick` and yields; each drained unit debits a [`TokenBucket`]
//! (§4), tying heavy compute to the spend rail. If the bucket is empty the
//! drainer stops early (degrade-closed) — it never over-spends and never runs a
//! unit it could not pay for.
//!
//! Verified-by-math falsifiers (tests): (1) a tick never runs more than `k`
//! units; (2) the total units run across a full drain equals the queued count
//! and never exceeds it; (3) when the TokenBucket cannot pay, the tick stops
//! early and no further units run (degrade-closed); (4) the units-run total is
//! bounded by the tokens granted (each unit debits exactly one token).
//!
//! ZERO new dependencies (plain `std`), consumes only the existing kernel
//! `TokenBucket`.

use crate::token_bucket::TokenBucket;

/// A bounded, budget-aware drainer over a queue of `remaining` work-units.
/// Each `tick` runs at most `k` units, debiting `cost_per_unit` tokens from the
/// bound `TokenBucket` per unit and stopping early if the budget is exhausted.
pub struct BoundedDrainer {
    /// Work-units still to run.
    remaining: u64,
    /// Hard cap on units consumed per `tick` (the `MAX_SUBSTEPS` analogue).
    k: u32,
    /// Tokens debited from the bucket for each unit run.
    cost_per_unit: f64,
    /// Total units run across all ticks so far (falsifier evidence).
    total_run: u64,
}

impl BoundedDrainer {
    /// New drainer for `units` work-units, at most `k` per tick, each debiting
    /// `cost_per_unit` tokens. `k` is clamped to at least 1 so a tick always
    /// makes progress when budget allows.
    pub fn new(units: u64, k: u32, cost_per_unit: f64) -> Self {
        BoundedDrainer {
            remaining: units,
            k: k.max(1),
            cost_per_unit,
            total_run: 0,
        }
    }

    /// Units still queued.
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Total units run so far across every `tick`.
    pub fn total_run(&self) -> u64 {
        self.total_run
    }

    /// True once every queued unit has been run.
    pub fn is_done(&self) -> bool {
        self.remaining == 0
    }

    /// Run up to `k` work-units, invoking `run_unit` for each. Before each unit
    /// it debits `cost_per_unit` from `bucket`; if the bucket refuses (empty),
    /// the tick STOPS EARLY (degrade-closed) — that unit does not run and no
    /// token is spent for it. Returns the number of units actually run this tick.
    pub fn tick<F: FnMut()>(&mut self, bucket: &TokenBucket, mut run_unit: F) -> u32 {
        let mut ran = 0u32;
        while ran < self.k && self.remaining > 0 {
            if !bucket.try_acquire(self.cost_per_unit) {
                break; // degrade-closed: cannot pay → stop, do not run unpaid work
            }
            run_unit();
            self.remaining -= 1;
            self.total_run += 1;
            ran += 1;
        }
        ran
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // (1) A tick never runs more than `k` units (spiral-of-death guard).
    #[test]
    fn tick_caps_at_k_units() {
        // Huge budget so the cap, not the bucket, is what bounds the tick.
        let bucket = TokenBucket::new(1_000_000.0, 0.0);
        let mut d = BoundedDrainer::new(100, 5, 1.0);
        let ran = d.tick(&bucket, || {});
        assert_eq!(ran, 5, "tick must run exactly k=5 units, not more");
        assert_eq!(d.remaining(), 95);
    }

    // (2) Across a full drain the total units run equals the queued count exactly.
    #[test]
    fn full_drain_runs_every_unit_no_more() {
        let bucket = TokenBucket::new(1_000_000.0, 0.0);
        let mut d = BoundedDrainer::new(23, 5, 1.0);
        let mut count = 0u64;
        while !d.is_done() {
            d.tick(&bucket, || count += 1);
        }
        assert_eq!(count, 23, "closure must fire once per queued unit");
        assert_eq!(d.total_run(), 23);
        assert_eq!(d.remaining(), 0);
        // A further tick on an empty queue runs nothing.
        assert_eq!(d.tick(&bucket, || count += 1), 0);
        assert_eq!(count, 23, "no over-run past the queued total");
    }

    // (3) Degrade-closed: when the bucket cannot pay, the tick stops early and
    //     runs no unpaid work.
    #[test]
    fn degrade_closed_when_budget_exhausted() {
        // Bucket holds exactly 3 tokens, no refill. Each unit costs 1 token.
        let bucket = TokenBucket::new(3.0, 0.0);
        let mut d = BoundedDrainer::new(100, 10, 1.0);
        let mut count = 0u64;
        let ran = d.tick(&bucket, || count += 1);
        assert_eq!(ran, 3, "only 3 units affordable → tick stops early at 3");
        assert_eq!(count, 3, "no unpaid unit ran");
        assert_eq!(d.remaining(), 97, "unaffordable units stay queued");
        // Next tick with the empty bucket runs nothing at all.
        assert_eq!(d.tick(&bucket, || count += 1), 0);
        assert_eq!(count, 3);
    }

    // (4) Each unit debits exactly one token: units-run ≤ tokens available.
    #[test]
    fn units_run_bounded_by_tokens_debited() {
        let bucket = TokenBucket::new(7.0, 0.0);
        let mut d = BoundedDrainer::new(100, 100, 1.0);
        let ran = d.tick(&bucket, || {});
        assert_eq!(ran, 7, "7 tokens ⇒ at most 7 units run");
        assert!(
            bucket.available() < 1.0,
            "budget fully spent on the 7 units"
        );
    }
}
