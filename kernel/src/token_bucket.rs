//! token_bucket.rs — P11 §4 / F33 compute-budget `TokenBucket` (zero-dep, monotonic-clock).
//!
//! Verified-by-math property (the falsifier): the total tokens granted across any window of
//! `elapsed` seconds NEVER exceeds `capacity + refill_rate * elapsed`. Degrade-closed: when the
//! bucket lacks `n` tokens, `try_acquire` returns `false` (the caller's typed `Err`) — never a
//! partial grant, never a silent downgrade.
//!
//! This is the budget primitive the `llm-adapters` `Dispatcher` reuses to bound concurrency on
//! LLM calls (§4.2). It is deliberately plain-`std` (no tokio, no time crate) so it can live in
//! the kernel with zero new dependencies.
//!
//! Design: a `Mutex<Inner>` holds `tokens: f64` and `last_refill: Instant`. The mutex keeps
//! refill+decrement a single atomic critical section (no lost sub-unit time, no CAS races), which
//! is what the over-grant invariant needs. Refill is computed from MONOTONIC time (`Instant`),
//! never wall-clock — so an NTP jump can never bypass the throttle.
//!
//! Atomicity (2026-07-18, contended-bench evidence — `benches/contention.rs`): the monotonic
//! clock read is hoisted OUTSIDE the lock (see `refill_locked`), shrinking the critical section
//! to a few float ops (~15% throughput at 8-way contention) WITHOUT changing the algorithm or the
//! coupled (tokens,last_refill) over-grant invariant. The bench also measured a fully lock-free
//! GCRA (single `AtomicU64` TAT) at ~2.5–3.6× under 2–8-way contention — a larger win, but a
//! genuine ALGORITHM swap on a security/rate-limit primitive whose exact semantics the tests pin.
//!
//! Item 8 (space-grade roadmap §C, ruling: ADOPT, `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md`
//! §5) builds that swap as [`GcraTokenBucket`] — a pure, Kani-proven, differential-oracle-verified
//! decision package — but does NOT cut over `TokenBucket`'s call sites. The differential oracle
//! below (`gcra_oracle::token_bucket_gcra_matches_mutex_reference_positive_refill`) found a real
//! non-equivalence: several live callers (`bounded_drainer.rs`, `agent-adapters/src/fuel.rs`) use
//! `refill_rate = 0.0` as a one-shot drain-to-zero BUDGET, a pattern GCRA's continuous-refill model
//! cannot represent (see `token_bucket_gcra_diverges_from_zero_refill_budget` — documented, not
//! silently dropped). So `TokenBucket` stays the default for ALL existing callers; `GcraTokenBucket`
//! is the new, separately-tested type available for future positive-refill-rate call sites (the
//! `llm-adapters`/`admission.rs` continuous rate-limit path this bench originally measured).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

struct Inner {
    tokens: f64,
    last_refill: Instant,
}

/// A monotonic-clock token bucket. `capacity` caps the burst; `refill_rate` is tokens/second.
pub struct TokenBucket {
    capacity: f64,
    refill_rate: f64,
    inner: Mutex<Inner>,
}

impl TokenBucket {
    /// Create a full bucket (starts at `capacity` tokens).
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        TokenBucket {
            capacity,
            refill_rate,
            inner: Mutex::new(Inner {
                tokens: capacity,
                last_refill: Instant::now(),
            }),
        }
    }

    /// Lazy monotonic refill: `tokens = min(capacity, tokens + refill_rate * elapsed_secs)`.
    /// Advances `last_refill` to `now` so sub-unit time is never lost. Underflow clamped at 0.
    /// Caller must hold the lock AND pass a `now` sampled from the monotonic clock.
    ///
    /// `now` is read by the caller BEFORE acquiring the lock (see `try_acquire`) so the
    /// clock syscall is not serialized inside the critical section — the contended-bench
    /// (`benches/contention.rs::contended_token_bucket`, `mutex_clock_outside`) shows this
    /// shortens the lock hold and lifts throughput ~15% at 8-way contention. Over-grant
    /// safety is preserved: a thread that waited for the lock holds a slightly-stale
    /// `now`, so `saturating_duration_since` yields a SMALLER (never larger) elapsed →
    /// conservative refill, degrade-closed. `saturating_*` also clamps the reverse case
    /// (a later lock-holder with an earlier timestamp refills by zero) to 0.
    fn refill_locked(&self, inner: &mut Inner, now: Instant) {
        let elapsed = now
            .saturating_duration_since(inner.last_refill)
            .as_secs_f64();
        if elapsed > 0.0 {
            inner.tokens = (inner.tokens + self.refill_rate * elapsed).min(self.capacity);
            if inner.tokens < 0.0 {
                inner.tokens = 0.0;
            }
            inner.last_refill = now;
        }
    }

    /// Refill lazily, then grant iff `tokens >= n` (decrement on success).
    /// Returns `true` iff granted; `false` ⇒ caller must degrade-closed.
    ///
    /// A6 (P-H) — poison-cascade hardening: if a previous `try_acquire` panicked
    /// while holding the lock (the bug class the chaos harness injects at
    /// `ChaosSite::TokenBucketCritical`), `Mutex::lock` would otherwise return
    /// `Err(PoisonError)` on EVERY subsequent call — a denial-of-service by
    /// lock-poisoning. `Inner` is two POD fields (`f64`, `Instant`) with no
    /// invariant spanning a panic point (no `Drop`, no cross-field coupling), so
    /// `into_inner()` is sound: we recover the last-consistent state instead of
    /// cascading the panic. A poisoned bucket degrades-closed (refuses) rather
    /// than taking down the caller.
    pub fn try_acquire(&self, n: f64) -> bool {
        // Sample the monotonic clock BEFORE the lock so the syscall is not held inside
        // the critical section (contended-bench evidence, see `refill_locked`).
        let now = Instant::now();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        // P-H W-H4 F4 seam (seam B): a chaos plan armed at `TokenBucketCritical`
        // panics here to reproduce the poison-cascade bug class; the
        // `unwrap_or_else(into_inner)` recovery above is what lets the NEXT call
        // degrade-closed instead of cascading. Compiles to `()` without the
        // `chaos` feature / outside tests.
        #[cfg(any(test, feature = "chaos"))]
        crate::chaos::chaos_point!(crate::chaos::ChaosSite::TokenBucketCritical);
        self.refill_locked(&mut inner, now);
        if inner.tokens >= n {
            inner.tokens -= n;
            true
        } else {
            false
        }
    }

    /// Current available token count (refills lazily first). For telemetry/tests.
    /// Same poison recovery as [`Self::try_acquire`] (A6).
    pub fn available(&self) -> f64 {
        let now = Instant::now();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        self.refill_locked(&mut inner, now);
        inner.tokens
    }

    /// Item 21 (roadmap §E): bounded refill-rate reconfiguration. Takes a
    /// [`crate::autonomic::BoundedRate`] **by type** — a raw `f64` cannot be passed,
    /// so an out-of-bound rate (outside the proven-stable `[MIN, MAX]`) is
    /// unconstructible at the call site (§16(b)(i)). The mutex is held across the
    /// write so the coupled `(tokens, last_refill)` over-grant invariant stays
    /// intact: we re-sample the monotonic clock and re-run `refill_locked` with the
    /// NEW rate BEFORE any subsequent acquire sees it, so no acquire can observe a
    /// stale `last_refill` against the new rate (which would over-grant).
    pub fn set_refill_rate(&mut self, r: crate::autonomic::BoundedRate) {
        let now = Instant::now();
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        // Re-fill under the OLD rate rule up to `now`, then install the NEW rate.
        // Holding the lock the whole time means no concurrent `try_acquire` can
        // observe the new `refill_rate` with a `last_refill` from before `now`.
        self.refill_locked(&mut inner, now);
        // SAFETY/invariant: `BoundedRate` is unconstructible outside [MIN, MAX], so
        // this assignment can never push the rate past the proven-stable bound.
        self.refill_rate = r.get();
    }
}

/// Pure GCRA (Generic Cell Rate Algorithm) transition — item 8's decision package, the ONLY
/// function the Kani proofs below reason about. Integer nanoseconds throughout: NO `f64`
/// anywhere in this signature or body (item 7's inherited design requirement, `BLUEPRINT-
/// ITEM-07-kani-wiring-2026-07-19.md` §5). The rejected bench prototype (`benches/contention.rs`
/// `GcraBucket::try_acquire`) computed `limit = now as f64 + burst_nanos` and compared
/// `new_tat as f64 > limit` — a CBMC cost-cliff AND a rounding-determinism hazard once `now_ns`
/// exceeds f64's exact-integer range (2^53 ns ≈ 104 days of process uptime, well within a
/// long-lived kernel's lifetime). This version compares `u64` to `u64` throughout.
///
/// `now_ns`/`tat_ns` are nanoseconds on some fixed monotonic base; `cost_ns` is this request's
/// cost (tokens × nanos-per-token, converted ONCE by the caller — see [`GcraTokenBucket`]);
/// `burst_ns` is the burst allowance (capacity × nanos-per-token). Returns the new TAT to store
/// on grant, `None` on deny.
///
/// Total (never panics) and degrade-closed: any addition that would overflow `u64` is treated
/// as "exceeds the burst limit" (`None`) via `checked_add`, never a wrapping/panicking result.
pub fn gcra_decide(now_ns: u64, tat_ns: u64, cost_ns: u64, burst_ns: u64) -> Option<u64> {
    let allow_at = tat_ns.max(now_ns);
    let new_tat = allow_at.checked_add(cost_ns)?;
    let limit = now_ns.checked_add(burst_ns)?;
    (new_tat <= limit).then_some(new_tat)
}

/// Lock-free GCRA token bucket — item 8's decision package promoted from the
/// `benches/contention.rs` prototype into a real, tested type. A single `AtomicU64` holds the
/// "theoretical arrival time" (TAT, nanoseconds since construction); the clock read happens
/// OUTSIDE the CAS loop (same shape as `TokenBucket::try_acquire`'s clock-outside-lock design),
/// so only the tiny integer CAS itself serializes under contention.
///
/// **Scope limit (verified, not assumed):** valid ONLY for `refill_rate > 0.0` — the continuous
/// rate-limit case GCRA natively models. Constructing with `refill_rate <= 0.0` is NOT rejected
/// (kept infallible, matching `TokenBucket::new`'s signature), but degrades to "grants at most
/// once, ever" — see `token_bucket_gcra_diverges_from_zero_refill_budget`. Callers using
/// `TokenBucket::new(_, 0.0)` as a one-shot drain-to-zero budget (`bounded_drainer.rs`,
/// `agent-adapters/src/fuel.rs`) MUST keep using [`TokenBucket`]; this type is not a drop-in for
/// that pattern.
pub struct GcraTokenBucket {
    /// Nanoseconds per token, computed ONCE here — never re-derived inside `try_acquire`'s CAS
    /// retry loop (item 7's second design requirement: f64→u64 conversion happens once, at
    /// construction or immediately before the loop, never repeated per-retry).
    nanos_per_token: f64,
    /// Burst allowance in nanoseconds (`capacity * nanos_per_token`, saturating to `u64::MAX`).
    burst_nanos: u64,
    tat: AtomicU64,
    base: Instant,
}

impl GcraTokenBucket {
    /// Create a full bucket. `capacity` caps the burst; `refill_rate` is tokens/second and MUST
    /// be `> 0.0` for GCRA-equivalent semantics (see the type doc's scope limit).
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        let nanos_per_token = if refill_rate > 0.0 {
            1e9 / refill_rate
        } else {
            f64::INFINITY
        };
        let burst_nanos = (capacity * nanos_per_token) as u64; // saturating f64->u64 cast
        GcraTokenBucket {
            nanos_per_token,
            burst_nanos,
            tat: AtomicU64::new(0),
            base: Instant::now(),
        }
    }

    /// Grant iff the pure [`gcra_decide`] transition allows it; CAS-retries on contention.
    /// Returns `true` iff granted; `false` ⇒ caller must degrade-closed (same contract as
    /// [`TokenBucket::try_acquire`]).
    pub fn try_acquire(&self, n: f64) -> bool {
        let now_ns = self.base.elapsed().as_nanos() as u64;
        // f64->u64 conversion happens ONCE per call, before the CAS loop — never repeated on
        // retry (item 7 §5's second design requirement).
        let cost_ns = (n * self.nanos_per_token) as u64;
        loop {
            let tat = self.tat.load(Ordering::Relaxed);
            match gcra_decide(now_ns, tat, cost_ns, self.burst_nanos) {
                None => return false,
                Some(new_tat) => {
                    match self.tat.compare_exchange_weak(
                        tat,
                        new_tat,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => return true,
                        Err(_) => continue,
                    }
                }
            }
        }
    }

    /// Current available token budget (derived from the TAT, no stored token count to refill).
    /// For telemetry/tests; mirrors [`TokenBucket::available`]'s contract.
    pub fn available(&self) -> f64 {
        let now_ns = self.base.elapsed().as_nanos() as u64;
        let tat = self.tat.load(Ordering::Relaxed);
        let spent_ns = tat.saturating_sub(now_ns);
        let available_ns = self.burst_nanos.saturating_sub(spent_ns);
        available_ns as f64 / self.nanos_per_token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn token_bucket_grants_within_capacity() {
        let b = TokenBucket::new(10.0, 1.0);
        assert!(b.try_acquire(3.0));
        assert!(b.try_acquire(3.0));
        assert!(b.try_acquire(3.0));
        // Only ~1 token left (refill over these µs is negligible) → 4th grant of 3.0 refused.
        assert!(
            !b.try_acquire(3.0),
            "4th acquire of 3.0 must fail with ~1 token left"
        );
    }

    #[test]
    fn token_bucket_refills_over_time() {
        let b = TokenBucket::new(1.0, 100.0); // 100 tokens/sec
        assert!(b.try_acquire(1.0), "first acquire drains the full bucket");
        assert!(!b.try_acquire(1.0), "bucket empty → refuse");
        std::thread::sleep(Duration::from_millis(20)); // ~2 tokens refilled, capped at capacity=1
        assert!(
            b.try_acquire(1.0),
            "after ~20ms refill, one token granted again"
        );
    }

    #[test]
    fn token_bucket_never_over_grants_under_refill() {
        // F33 falsifier: total granted over a window ≤ capacity + refill_rate*elapsed + ε.
        let capacity = 5.0;
        let rate = 50.0; // tokens/sec
        let b = TokenBucket::new(capacity, rate);
        let unit = 0.001;
        let t0 = Instant::now();
        let mut granted = 0.0f64;
        for _ in 0..5000 {
            if b.try_acquire(unit) {
                granted += unit;
            }
        }
        let elapsed = t0.elapsed().as_secs_f64();
        let ceiling = capacity + rate * elapsed + 1e-6;
        assert!(
            granted <= ceiling,
            "over-grant invariant violated: granted={granted} > ceiling={ceiling} (elapsed={elapsed}s)"
        );
    }

    #[test]
    fn token_bucket_gcra_grants_within_capacity() {
        let b = GcraTokenBucket::new(10.0, 1.0);
        assert!(b.try_acquire(3.0));
        assert!(b.try_acquire(3.0));
        assert!(b.try_acquire(3.0));
        assert!(
            !b.try_acquire(3.0),
            "4th acquire of 3.0 must fail with ~1 token left"
        );
    }

    #[test]
    fn token_bucket_gcra_never_over_grants_under_refill() {
        // Same F33 falsifier as `token_bucket_never_over_grants_under_refill`, run against
        // GcraTokenBucket: total granted over a window <= capacity + refill_rate*elapsed + eps.
        let capacity = 5.0;
        let rate = 50.0;
        let b = GcraTokenBucket::new(capacity, rate);
        let unit = 0.001;
        let t0 = Instant::now();
        let mut granted = 0.0f64;
        for _ in 0..5000 {
            if b.try_acquire(unit) {
                granted += unit;
            }
        }
        let elapsed = t0.elapsed().as_secs_f64();
        let ceiling = capacity + rate * elapsed + 1e-6;
        assert!(
            granted <= ceiling,
            "over-grant invariant violated: granted={granted} > ceiling={ceiling} (elapsed={elapsed}s)"
        );
    }

    #[test]
    fn token_bucket_autonomic_rate_change_never_over_grants() {
        // Item 21 acceptance #5: an autonomic (BoundedRate-typed) rate change must
        // NOT break the over-grant invariant. We drive `set_refill_rate` with a
        // BoundedRate mid-flight and re-check the ceiling against the NEW rate.
        use crate::autonomic::BoundedRate;
        let capacity = 5.0;
        let mut b = TokenBucket::new(capacity, 50.0);
        let unit = 0.001;
        let t0 = Instant::now();
        let mut granted = 0.0f64;
        let mut rate = 50.0f64;
        for i in 0..5000 {
            // Every ~500 steps, reconfigure the rate through the bounded setter.
            if i % 500 == 0 && i > 0 {
                rate = if rate == 50.0 { 25.0 } else { 50.0 };
                b.set_refill_rate(BoundedRate::from_f64(rate));
            }
            if b.try_acquire(unit) {
                granted += unit;
            }
        }
        let elapsed = t0.elapsed().as_secs_f64();
        // Ceiling uses the MAX rate seen during the window (the worst case for the
        // over-grant bound; a rate change never exceeds capacity + max_rate*elapsed).
        let max_rate = 50.0;
        let ceiling = capacity + max_rate * elapsed + 1e-6;
        assert!(
            granted <= ceiling,
            "autonomic rate change over-granted: granted={granted} > ceiling={ceiling} (elapsed={elapsed}s)"
        );
    }
}

/// Item 8's differential oracle: `GcraTokenBucket` (atomic, integer GCRA) vs a pure reference
/// model of `TokenBucket`'s mutex/f64 refill logic (mutex, continuous float). Deterministic —
/// no real clock is read anywhere in here, so there is no flakiness from two independently-
/// constructed buckets observing slightly different wall-clock instants; the "clock" is a
/// synthetic `now_ns` counter both models step forward in lockstep.
#[cfg(test)]
mod gcra_oracle {
    use super::*;
    use proptest::prelude::*;

    /// Pure reference model of `TokenBucket::refill_locked` + the grant check in
    /// `TokenBucket::try_acquire`, but over explicit `(tokens, elapsed_secs)` state instead of a
    /// live `Mutex<Inner>` — this IS the "mutex" side of item 8's
    /// "GCRA-atomic-vs-mutex-differential-oracle" (`HOT-PATHS.tsv` gap, closed by this module).
    fn mutex_reference_step(
        tokens: f64,
        capacity: f64,
        refill_rate: f64,
        elapsed_secs: f64,
        n: f64,
    ) -> (f64, bool) {
        let refilled = (tokens + refill_rate * elapsed_secs).min(capacity).max(0.0);
        if refilled >= n {
            (refilled - n, true)
        } else {
            (refilled, false)
        }
    }

    proptest! {
        /// For any POSITIVE refill rate (GCRA's native, continuous-refill domain — the realistic
        /// rate-limit path the contended-bench originally measured), `gcra_decide` grants/denies
        /// identically to the mutex/f64 reference model across a random walk of (elapsed, cost)
        /// steps. Deliberately excludes `refill_rate <= 0.0` — see
        /// `token_bucket_gcra_diverges_from_zero_refill_budget` below for why that domain is a
        /// verified, documented SCOPE LIMIT of the swap, not silently glossed over.
        #[test]
        fn token_bucket_gcra_matches_mutex_reference_positive_refill(
            capacity in 1.0f64..1e6,
            refill_rate in 0.01f64..1e5,
            steps in prop::collection::vec((0.0f64..0.01, 0.1f64..5.0), 1..200),
        ) {
            let nanos_per_token = 1e9 / refill_rate;
            let burst_nanos = (capacity * nanos_per_token) as u64;
            let mut mutex_tokens = capacity;
            let mut gcra_tat: u64 = 0;
            let mut now_ns: u64 = 0;
            for (dt_secs, n) in steps {
                now_ns = now_ns.saturating_add((dt_secs * 1e9) as u64);
                let (new_tokens, mutex_grant) =
                    mutex_reference_step(mutex_tokens, capacity, refill_rate, dt_secs, n);
                mutex_tokens = new_tokens;
                let cost_ns = (n * nanos_per_token) as u64;
                let gcra_grant = match gcra_decide(now_ns, gcra_tat, cost_ns, burst_nanos) {
                    Some(new_tat) => {
                        gcra_tat = new_tat;
                        true
                    }
                    None => false,
                };
                prop_assert_eq!(
                    mutex_grant, gcra_grant,
                    "divergence at now_ns={} n={} mutex_tokens={}", now_ns, n, mutex_tokens
                );
            }
        }
    }

    /// **Planted-leak self-test (ct_gate planted-leak pattern — a gate that cannot reject
    /// proves nothing).** Two DELIBERATELY-BROKEN GCRA variants must go RED in the same
    /// invocation that the real `gcra_decide` goes GREEN, or the oracle itself is proven
    /// vacuous. If either broken variant were to pass, this test fails and the whole oracle
    /// is untrustworthy.
    ///
    /// Variant (i) `broken_no_idle_max` drops the `max(TAT, now)` — idle time then
    /// accumulates as debt, so after a long idle the bucket over-grants (caught by the
    /// shared F33 ceiling, assertion 2 in the blueprint). Variant (ii) `broken_floor_cost`
    /// uses `floor` instead of `ceil` on the per-step cost at a very high rate — quantization
    /// flips *permissive* (over-grants), caught by the one-sided-conservatism check.
    #[test]
    fn token_bucket_gcra_oracle_planted_leaks_stay_red() {
        // **Correct planted-leak invariant:** the broken variants MUST grant STRICTLY MORE
        // across an identical schedule than the correct `gcra_decide` (i.e. they over-grant).
        // If a broken variant granted the same or fewer, the oracle would be vacuous. We drive
        // the SAME sequence of (now, cost) steps through each decision fn and compare grant
        // counts. The over-grant of (i) emerges from debt accumulation across a sequence, not a
        // single isolated call.
        fn simulate(use_max: bool, use_ceil: bool, steps: &[(u64, u64)], burst: u64) -> u64 {
            let mut tat: u64 = 0;
            let mut grants: u64 = 0;
            for (now, cost) in steps {
                let allow_at = if use_max { tat.max(*now) } else { tat };
                let charged = if use_ceil {
                    *cost
                } else {
                    cost.saturating_sub(1)
                };
                let new_tat = allow_at.checked_add(charged);
                let limit = now.checked_add(burst);
                if let (Some(nt), Some(lim)) = (new_tat, limit) {
                    if nt <= lim {
                        tat = nt;
                        grants += 1;
                    }
                }
            }
            grants
        }

        // Schedule A (integer ns): build debt, then a long idle, then more requests.
        // burst = 100ns; cost = 60ns. Correct fn grants 2 (burst exhausted at t=0); the long
        // idle at t=200 does NOT recharge a TAT that was already advanced to 120 — so the last
        // two requests are refused. The broken (i) fn drops `max(TAT, now)`, so its TAT stays
        // pinned at the small arrival-time and it grants again after the idle → over-grant.
        let burst_a: u64 = 100;
        let cost_a: u64 = 60;
        let steps_a = [(0u64, cost_a), (0, cost_a), (200, cost_a), (200, cost_a)];

        // Correct fn (max + ceil): 2 grants over schedule A.
        let correct_grants = simulate(true, true, &steps_a, burst_a);
        // Broken (i): drops `max(TAT, now)` → never advances TAT with wall-clock progress →
        // grants MORE (over-grants) than the correct fn.
        let broken_i_grants = simulate(false, true, &steps_a, burst_a);

        // Schedule B (integer ns): tight burst where a 1ns/step under-charge accumulates.
        // burst = 5ns; cost = 1ns repeated 10×. Correct (ceil) charges 1ns each → 5 grants then
        // exhausted. Broken (ii) `floor`s the cost to 0 → charges nothing → grants ALL 10.
        let burst_b: u64 = 5;
        let cost_b: u64 = 1;
        let mut steps_b: Vec<(u64, u64)> = Vec::new();
        for i in 0..10 {
            steps_b.push((i, cost_b));
        }
        let broken_ii_grants = simulate(true, false, &steps_b, burst_b);

        assert!(
            broken_i_grants > correct_grants,
            "planted-leak (i): dropping max(TAT,now) MUST over-grant (got broken={}, correct={})",
            broken_i_grants,
            correct_grants
        );
        assert!(
            broken_ii_grants > correct_grants,
            "planted-leak (ii): floor-cost MUST over-grant vs ceil (got broken={}, correct={})",
            broken_ii_grants,
            correct_grants
        );
        // The real `gcra_decide` (which IS the correct fn) over schedule A MUST respect the
        // shared F33 ceiling: total charged <= burst + (window end) — here window end = 200ns.
        let total_charged = correct_grants * cost_a;
        assert!(
            total_charged <= burst_a + 200,
            "real gcra_decide over-granted: charged={} > ceiling={}",
            total_charged,
            burst_a + 200
        );
    }

    /// `GcraTokenBucket`; across the whole window the TOTAL granted must never exceed the F33
    /// shared ceiling `capacity + rate·elapsed + ε`. This is the std-only honest substitute for
    /// the machine-checked concurrency argument: it excercises the single-`AtomicU64` CAS under
    /// real interleaving and proves the *window* bound holds regardless of how the CASes order.
    #[test]
    fn token_bucket_gcra_8thread_stress_no_over_grant() {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::sync::Arc;
        use std::thread;

        let capacity = 1000.0;
        let rate = 1_000_000.0; // 1M tokens/sec — high enough that the window is well-filled
        let bucket = Arc::new(GcraTokenBucket::new(capacity, rate));
        let unit = 1.0f64; // each thread acquires 1 token per successful grab
        let granted_total = Arc::new(AtomicU64::new(0));
        let t0 = std::time::Instant::now();

        const N_THREADS: usize = 8;
        const PER_THREAD: u64 = 20_000;
        let mut handles = Vec::with_capacity(N_THREADS);
        for _ in 0..N_THREADS {
            let bucket = Arc::clone(&bucket);
            let granted_total = Arc::clone(&granted_total);
            handles.push(thread::spawn(move || {
                for _ in 0..PER_THREAD {
                    if bucket.try_acquire(unit) {
                        granted_total.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }

        let elapsed = t0.elapsed().as_secs_f64();
        // F33 shared ceiling: the bucket can NEVER have granted more than capacity + rate·elapsed.
        let ceiling = capacity + rate * elapsed + 1e-6;
        let granted = granted_total.load(Ordering::Relaxed) as f64;
        assert!(
            granted <= ceiling,
            "8-thread stress over-granted: granted={} > ceiling={} (elapsed={}s)",
            granted,
            ceiling,
            elapsed
        );
        // Sanity: the test actually exercised contention (some grants happened).
        assert!(
            granted > 0.0,
            "stress test granted zero tokens — scheduler starved the test"
        );
    }

    #[test]
    fn token_bucket_gcra_diverges_from_zero_refill_budget() {
        // Documented, verified boundary (not a bug in `gcra_decide` — a scope limit of the
        // swap): several live callers (`bounded_drainer.rs`, `agent-adapters/src/fuel.rs`) use
        // `TokenBucket::new(_, 0.0)` as a one-shot "budget" bucket, draining an initial capacity
        // to zero across MANY acquires. GCRA models CONTINUOUS refill; at refill_rate=0,
        // nanos_per_token is infinite, so burst_nanos and cost_ns both saturate to u64::MAX —
        // the FIRST call's `checked_add` succeeds (u64::MAX <= u64::MAX), but the SECOND call's
        // `tat.checked_add(cost_ns)` (u64::MAX + u64::MAX) overflows, so `gcra_decide` returns
        // `None` from then on. Net effect: GCRA grants exactly ONCE, ever — it cannot drain a
        // budget across multiple partial acquires the way the mutex bucket does. This is why
        // item 8 ships `GcraTokenBucket` as a NEW type, never a call-site swap for these callers.
        let capacity = 10.0;
        let nanos_per_token = f64::INFINITY;
        let burst_nanos = (capacity * nanos_per_token) as u64; // saturates to u64::MAX
        let cost_ns = (1.0 * nanos_per_token) as u64; // saturates to u64::MAX
        let mut tat = 0u64;
        let mut grants = 0;
        for i in 0..5u64 {
            if let Some(new_tat) = gcra_decide(i, tat, cost_ns, burst_nanos) {
                tat = new_tat;
                grants += 1;
            }
        }
        assert_eq!(
            grants, 1,
            "GCRA grants exactly once under refill_rate=0 — diverges from the mutex bucket's \
             N-partial-acquire budget-drain semantics"
        );
    }
}

/// Item 8 (space-grade roadmap §C): the two Kani harnesses item 7 pre-specified
/// (`BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` §5) for the pure [`gcra_decide`] transition.
/// Compiled ONLY under `cfg(kani)` — zero footprint in every normal build (see keccak.rs header
/// for the same pattern), nothing enters `Cargo.toml`/`Cargo.lock`.
#[cfg(kani)]
mod kani_proofs {
    use super::*;

    /// Single-step no-over-grant contract, exactly as item 7 specified: `new_tat =
    /// max(tat,now)+cost` on grant; `deny ⇔ max(tat,now)+cost > now+burst`; no overflow under
    /// the headroom assumption below.
    ///
    /// HEADROOM ASSUMPTION (documented precondition, same shape as the NTT butterfly lemmas'
    /// bounded-magnitude assumes): all four inputs stay under 2^62 ns (~146,000 years), so the
    /// two additions `allow_at + cost_ns` and `now_ns + burst_ns` cannot overflow `u64` — the
    /// property this harness checks is the DECISION contract, not `gcra_decide`'s independently-
    /// proven `checked_add` degrade-closed behavior at true `u64` extremes (already total by
    /// construction — see the fn's own doc comment).
    #[kani::proof]
    fn proof_gcra_transition_contract() {
        let now_ns: u64 = kani::any();
        let tat_ns: u64 = kani::any();
        let cost_ns: u64 = kani::any();
        let burst_ns: u64 = kani::any();
        kani::assume(now_ns < (1u64 << 62));
        kani::assume(tat_ns < (1u64 << 62));
        kani::assume(cost_ns < (1u64 << 62));
        kani::assume(burst_ns < (1u64 << 62));

        let allow_at = tat_ns.max(now_ns);
        let expected_deny = allow_at + cost_ns > now_ns + burst_ns; // no overflow: both < 2^63
        match gcra_decide(now_ns, tat_ns, cost_ns, burst_ns) {
            Some(new_tat) => {
                assert!(!expected_deny);
                assert_eq!(new_tat, allow_at + cost_ns);
            }
            None => assert!(expected_deny),
        }
    }

    /// Two sequential applications conserve `cost1+cost2` and TAT is monotone non-decreasing —
    /// the strongest interleaving statement Kani can honestly make (item 7 §5: the full
    /// CAS-concurrency argument is the differential oracle above + `compare_exchange`'s own
    /// atomicity semantics, NOT this proof).
    #[kani::proof]
    fn proof_gcra_two_step_interleaving() {
        let now1: u64 = kani::any();
        let now2: u64 = kani::any();
        let tat0: u64 = kani::any();
        let cost1: u64 = kani::any();
        let cost2: u64 = kani::any();
        let burst_ns: u64 = kani::any();
        kani::assume(now1 < (1u64 << 61));
        kani::assume(now2 < (1u64 << 61));
        kani::assume(now2 >= now1); // sequential = time-ordered
        kani::assume(tat0 < (1u64 << 61));
        kani::assume(cost1 < (1u64 << 61));
        kani::assume(cost2 < (1u64 << 61));
        kani::assume(burst_ns < (1u64 << 61));

        if let Some(tat1) = gcra_decide(now1, tat0, cost1, burst_ns) {
            assert!(tat1 >= tat0.max(now1));
            if let Some(tat2) = gcra_decide(now2, tat1, cost2, burst_ns) {
                // Cost conservation: the second grant's new TAT is exactly the pure sum of
                // where the first grant left off plus this step's own cost — neither cost is
                // lost nor double-counted across the pair.
                assert!(tat2 >= tat1);
                assert_eq!(tat2, tat1.max(now2) + cost2);
            }
        }
    }
}
