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
//! That swap is left OPERATOR-GATED (see `docs/research/OPUS-PERF-CONTENTION-BENCH-RESULTS-2026-07-18.md`),
//! not shipped here: the realistic dispatch path (one acquire per request between long network
//! calls) exhibits low real contention, so correctness-first favors the minimal safe change.

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
}
