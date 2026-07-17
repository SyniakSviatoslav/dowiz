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

/// OTP MaxR/MaxT restart-intensity bound as a PURE LAUNCH-PATH PREDICATE.
/// (Synthesis §7 T-6 / V2 W3-L4: "a monotone relaunch fact checked by a pure
/// predicate IN the launch path — degrade-closed refuse-to-launch; a standing
/// sampler process is never built.")
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartBudget {
    /// Max launches permitted inside any sliding window (OTP MaxR).
    pub max_restarts: u32,
    /// Sliding window length, milliseconds (OTP MaxT).
    pub window_ms: u64,
}

/// Kernel default: 5 relaunches per rolling 60 s. A sustained crash loop
/// (mean time-to-crash < 12 s) is stopped within one minute; slower periodic
/// restarts (< 5/min) are legitimate. The systemd substrate mirror, where a
/// unit exists, MUST copy these numbers (StartLimitBurst=5,
/// StartLimitIntervalSec=60) so both planes enforce the same physics —
/// Phase 27 §3.4 leaves unit-existence (unverified); resolve at implementation.
pub const DRAINER_RESTART_BUDGET: RestartBudget =
    RestartBudget { max_restarts: 5, window_ms: 60_000 };

const _: () = assert!(DRAINER_RESTART_BUDGET.max_restarts >= 1);
const _: () = assert!(DRAINER_RESTART_BUDGET.window_ms > 0);

/// Proof-of-admission token. The ONLY constructor is `launch_permitted` (the
/// field is module-private). A launcher entry point written as
/// `fn run_drainer(token: LaunchToken, ...)` therefore CANNOT be invoked
/// without the predicate having run — bypass is a compile error, not a
/// runtime gap (doc 19 §2.3 axis 1: absence-is-visible).
pub struct LaunchToken { _private: () }

#[derive(Debug, PartialEq, Eq)]
pub enum LaunchRefused {
    /// MaxR launches already inside the MaxT window.
    IntensityExceeded { attempts_in_window: u32, max_restarts: u32, window_ms: u64 },
    /// `now_ms` earlier than the last recorded launch. A rewound clock could
    /// smuggle launches past the window; unprovable headroom refuses (fail-closed).
    ClockRewound { last_launch_ms: u64, now_ms: u64 },
}

impl std::fmt::Display for LaunchRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchRefused::IntensityExceeded {
                attempts_in_window,
                max_restarts,
                window_ms,
            } => write!(
                f,
                "BLOCKER: launch refused — {} launches in {} ms (max {}); lane stays down",
                attempts_in_window, window_ms, max_restarts
            ),
            LaunchRefused::ClockRewound {
                last_launch_ms,
                now_ms,
            } => write!(
                f,
                "BLOCKER: launch refused — clock rewound (now={} ms < last={} ms); lane stays down",
                now_ms, last_launch_ms
            ),
        }
    }
}

/// The predicate. PURE and TOTAL over the monotone launch-attempt history:
/// `prior_launches_ms` is append-only with non-decreasing timestamps (the
/// launcher appends the grant time before exec; entries are never edited).
/// An entry `t` is in-window iff `now_ms − t < window_ms` (strict).
pub fn launch_permitted(
    budget: &RestartBudget,
    prior_launches_ms: &[u64],
    now_ms: u64,
) -> Result<LaunchToken, LaunchRefused> {
    if let Some(&last) = prior_launches_ms.last() {
        if now_ms < last {
            return Err(LaunchRefused::ClockRewound { last_launch_ms: last, now_ms });
        }
    }
    let attempts_in_window = prior_launches_ms.iter().rev()
        .take_while(|&&t| now_ms - t < budget.window_ms)
        .count() as u32;
    if attempts_in_window >= budget.max_restarts {
        Err(LaunchRefused::IntensityExceeded {
            attempts_in_window,
            max_restarts: budget.max_restarts,
            window_ms: budget.window_ms,
        })
    } else {
        Ok(LaunchToken { _private: () })
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

    // P-C §7 T5 — the constructed "unsafe launch": 5 grants inside the window
    // then a 6th attempt is refused (IntensityExceeded). One tick past window
    // (strict inequality) the oldest entry ages out and launch is permitted.
    #[test]
    fn restart_gate_refuses_sixth_launch_in_window() {
        let b = DRAINER_RESTART_BUDGET;
        let hist = [0u64, 10_000, 20_000, 30_000, 40_000];
        // 6th launch at now=59_999: all 5 still in-window → refused.
        let r = launch_permitted(&b, &hist, 59_999);
        assert!(
            matches!(
                r,
                Err(LaunchRefused::IntensityExceeded {
                    attempts_in_window: 5,
                    max_restarts: 5,
                    window_ms: 60_000,
                })
            ),
            "the 6th launch inside the window must be refused"
        );
        // At now=60_000 the t=0 entry has aged out (60_000-0 = 60_000, not < 60_000).
        assert!(
            launch_permitted(&b, &hist, 60_000).is_ok(),
            "strict-inequality boundary: oldest entry expires exactly at window_ms"
        );
    }

    // P-C §7 T6 — adversarial clock rewind fails closed.
    #[test]
    fn restart_gate_clock_rewind_fails_closed() {
        let b = DRAINER_RESTART_BUDGET;
        let hist = [10_000u64, 50_000];
        let r = launch_permitted(&b, &hist, 49_999);
        assert!(
            matches!(
                r,
                Err(LaunchRefused::ClockRewound {
                    last_launch_ms: 50_000,
                    now_ms: 49_999,
                })
            ),
            "a rewound clock cannot smuggle a launch past the window"
        );
    }

    // P-C §7 T7 — slow legitimate periodic restart (< 5/min) is always permitted.
    #[test]
    fn restart_gate_slow_periodic_crash_is_legitimate() {
        let b = DRAINER_RESTART_BUDGET;
        let mut hist: Vec<u64> = Vec::new();
        for i in 0..20 {
            let now = i * 15_000; // one launch every 15 s (4/min)
            let r = launch_permitted(&b, &hist, now);
            assert!(r.is_ok(), "legitimate launch #{i} at t={now} must be permitted");
            hist.push(now);
        }
        assert_eq!(hist.len(), 20);
    }

    // P-C §7 T8 — first launch with empty history is always permitted.
    #[test]
    fn restart_gate_first_launch_always_permitted() {
        let b = DRAINER_RESTART_BUDGET;
        let hist: [u64; 0] = [];
        assert!(
            launch_permitted(&b, &hist, 0).is_ok(),
            "empty history ⇒ first launch permitted"
        );
    }
}
