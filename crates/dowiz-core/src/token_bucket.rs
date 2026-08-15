//! `token_bucket.rs` — P11 §4 / F33 compute-budget primitive (pure no_std core).
//!
//! The *pure* half: the integer GCRA decision package ([`gcra_decide`]), the lock-free
//! [`GcraTokenBucket`] (single `AtomicU64` TAT), and the monotonic [`TokenBucket`]
//! (a [`SpinLock<Inner>`] of `tokens: f64` + `last_refill_ns`, with `now_ns` injected).
//! NO `std` anywhere.
//!
//! The std side — stamping `now_ns` from the host's monotonic clock — lives in the kernel
//! shim, which re-exports these types and adds [`crate::clock::now_ns()`]-stamping free
//! functions (`token_bucket_try_acquire` / `token_bucket_available` / `gcra_*`).
//!
//! Poisoning (A6, fail-closed): the core [`SpinLock`] poisons only when a holder panics
//! while the lock is held (test builds). On a poisoned lock, `try_acquire`/`available`
//! degrade-closed (return `false`/`0.0`) rather than cascading the panic — the same
//! no-cascade guarantee as the old `Mutex` + `into_inner` recovery, but *refuse* instead
//! of *recover* (safe for a rate limiter: never grant from possibly-inconsistent state).
//!
//! Scope limit of the GCRA swap (verified, not assumed): valid ONLY for `refill_rate > 0.0`.
//! At `refill_rate <= 0.0` `GcraTokenBucket` grants exactly once, ever — callers using
//! `TokenBucket::new(_, 0.0)` as a drain-to-zero budget must keep using [`TokenBucket`].

use crate::autonomic::BoundedRate;
use crate::spinlock::SpinLock;
use core::sync::atomic::{AtomicU64, Ordering};

/// Pure GCRA (Generic Cell Rate Algorithm) transition — item 8's decision package.
///
/// `now_ns`/`tat_ns` are nanoseconds on some fixed monotonic base; `cost_ns` is this request's
/// cost; `burst_ns` is the burst allowance. Returns the new TAT to store on grant, `None` on
/// deny. Total (never panics) and degrade-closed: any addition that would overflow `u64` is
/// treated as "exceeds the burst limit" (`None`) via `checked_add`.
pub fn gcra_decide(now_ns: u64, tat_ns: u64, cost_ns: u64, burst_ns: u64) -> Option<u64> {
    let allow_at = tat_ns.max(now_ns);
    let new_tat = allow_at.checked_add(cost_ns)?;
    let limit = now_ns.checked_add(burst_ns)?;
    (new_tat <= limit).then_some(new_tat)
}

/// Lock-free GCRA token bucket — a single `AtomicU64` holds the theoretical-arrival-time (TAT).
/// The clock read happens OUTSIDE the CAS loop, so only the tiny integer CAS serializes under
/// contention. `now_ns` is injected by the caller (the kernel shim stamps
/// `crate::clock::now_ns()`), keeping this type `no_std`.
pub struct GcraTokenBucket {
    /// Nanoseconds per token, computed ONCE here — never re-derived inside the CAS loop.
    nanos_per_token: f64,
    /// Burst allowance in nanoseconds (`capacity * nanos_per_token`, saturating to `u64::MAX`).
    burst_nanos: u64,
    tat: AtomicU64,
}

impl GcraTokenBucket {
    /// Create a full bucket. `capacity` caps the burst; `refill_rate` is tokens/second and MUST
    /// be `> 0.0` for GCRA-equivalent semantics (see the module doc's scope limit).
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
        }
    }

    /// Grant iff the pure [`gcra_decide`] transition allows it; CAS-retries on contention.
    /// `now_ns` is the current monotonic nanosecond timestamp (injected by the caller).
    pub fn try_acquire(&self, n: f64, now_ns: u64) -> bool {
        let cost_ns = (n * self.nanos_per_token) as u64;
        loop {
            let tat = self.tat.load(Ordering::Relaxed);
            match gcra_decide(now_ns, tat, cost_ns, self.burst_nanos) {
                None => return false,
                Some(new_tat) => {
                    match self
                        .tat
                        .compare_exchange_weak(tat, new_tat, Ordering::Relaxed, Ordering::Relaxed)
                    {
                        Ok(_) => return true,
                        Err(_) => continue,
                    }
                }
            }
        }
    }

    /// Current available token budget (derived from the TAT, no stored token count to refill).
    pub fn available(&self, now_ns: u64) -> f64 {
        let tat = self.tat.load(Ordering::Relaxed);
        let spent_ns = tat.saturating_sub(now_ns);
        let available_ns = self.burst_nanos.saturating_sub(spent_ns);
        available_ns as f64 / self.nanos_per_token
    }
}

/// The refill state of a [`TokenBucket`].
struct Inner {
    tokens: f64,
    last_refill_ns: u64,
}

/// A monotonic-clock token bucket. `capacity` caps the burst; `refill_rate` is tokens/second.
/// `now_ns` (monotonic nanoseconds) is injected by the caller; the kernel shim stamps
/// `crate::clock::now_ns()`.
pub struct TokenBucket {
    capacity: f64,
    refill_rate: f64,
    inner: SpinLock<Inner>,
}

impl TokenBucket {
    /// Create a full bucket (starts at `capacity` tokens).
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        TokenBucket {
            capacity,
            refill_rate,
            inner: SpinLock::new(Inner {
                tokens: capacity,
                last_refill_ns: 0,
            }),
        }
    }

    /// Lazy monotonic refill: `tokens = min(capacity, tokens + refill_rate * elapsed_secs)`.
    /// Advances `last_refill_ns` to `now_ns` so sub-unit time is never lost. Underflow clamped
    /// at 0. Caller must hold the lock.
    fn refill_locked(&self, inner: &mut Inner, now_ns: u64) {
        let elapsed_secs = now_ns.saturating_sub(inner.last_refill_ns) as f64 / 1e9;
        if elapsed_secs > 0.0 {
            inner.tokens = (inner.tokens + self.refill_rate * elapsed_secs).min(self.capacity);
            if inner.tokens < 0.0 {
                inner.tokens = 0.0;
            }
            inner.last_refill_ns = now_ns;
        }
    }

    /// Refill lazily, then grant iff `tokens >= n` (decrement on success). Returns `true` iff
    /// granted; `false` ⇒ caller must degrade-closed. On a poisoned lock (a prior holder
    /// panicked), degrade-closed (`false`) — never a cascade panic (A6).
    pub fn try_acquire(&self, n: f64, now_ns: u64) -> bool {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return false, // poisoned → degrade-closed
        };
        self.refill_locked(&mut inner, now_ns);
        if inner.tokens >= n {
            inner.tokens -= n;
            true
        } else {
            false
        }
    }

    /// Current available token count (refills lazily first). `0.0` on a poisoned lock (A6).
    pub fn available(&self, now_ns: u64) -> f64 {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return 0.0, // poisoned → degrade-closed
        };
        self.refill_locked(&mut inner, now_ns);
        inner.tokens
    }

    /// Item 21: bounded refill-rate reconfiguration. `BoundedRate` is unconstructible outside
    /// `[MIN, MAX]`, so an out-of-bound rate is inexpressible. Re-fills under the OLD rate up
    /// to `now_ns`, then installs the NEW rate (holding the lock the whole time so no acquire
    /// can observe a stale `last_refill_ns` against the new rate). No-op on a poisoned lock.
    pub fn set_refill_rate(&mut self, r: BoundedRate, now_ns: u64) {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return, // poisoned → no-op
        };
        self.refill_locked(&mut inner, now_ns);
        self.refill_rate = r.get();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcra_decide_grants_within_burst_and_denies_beyond() {
        // burst = 100ns, cost = 30ns: three grants fit (30, 60, 90); the 4th (120) exceeds.
        assert_eq!(gcra_decide(0, 0, 30, 100), Some(30));
        assert_eq!(gcra_decide(0, 30, 30, 100), Some(60));
        assert_eq!(gcra_decide(0, 60, 30, 100), Some(90));
        // 4th: 90 + 30 = 120 > 0 + 100 = 100 → deny.
        assert_eq!(gcra_decide(0, 90, 30, 100), None);
    }

    #[test]
    fn gcra_decide_is_degrade_closed_on_overflow() {
        // allow_at(1) + cost(u64::MAX) overflows → None (never a wrapping grant).
        assert_eq!(gcra_decide(0, 1, u64::MAX, u64::MAX), None);
        // now + burst overflows → None.
        assert_eq!(gcra_decide(u64::MAX - 1, 0, 1, u64::MAX), None);
    }

    #[test]
    fn gcra_bucket_grants_within_capacity() {
        let b = GcraTokenBucket::new(10.0, 1.0);
        assert!(b.try_acquire(3.0, 0));
        assert!(b.try_acquire(3.0, 0));
        assert!(b.try_acquire(3.0, 0));
        assert!(!b.try_acquire(3.0, 0), "4th acquire of 3.0 must fail");
    }

    #[test]
    fn gcra_bucket_refills_over_time() {
        // 1 token/sec, capacity 1: after 2s (now_ns = 2e9), one token is available again.
        let b = GcraTokenBucket::new(1.0, 1.0);
        assert!(b.try_acquire(1.0, 0));
        assert!(!b.try_acquire(1.0, 0), "empty → refuse");
        assert!(b.try_acquire(1.0, 2_000_000_000), "refilled after 2s");
    }

    #[test]
    fn token_bucket_grants_within_capacity() {
        let b = TokenBucket::new(10.0, 1.0);
        assert!(b.try_acquire(3.0, 0));
        assert!(b.try_acquire(3.0, 0));
        assert!(b.try_acquire(3.0, 0));
        assert!(!b.try_acquire(3.0, 0), "4th acquire of 3.0 must fail");
    }

    #[test]
    fn token_bucket_refills_over_time() {
        // 100 tokens/sec, capacity 1: after 20ms (2e7 ns), one token is granted again.
        let b = TokenBucket::new(1.0, 100.0);
        assert!(b.try_acquire(1.0, 0), "first acquire drains the full bucket");
        assert!(!b.try_acquire(1.0, 0), "empty → refuse");
        assert!(b.try_acquire(1.0, 20_000_000), "after 20ms refill, granted again");
    }

    #[test]
    fn token_bucket_never_over_grants_under_refill() {
        // F33 falsifier: total granted ≤ capacity + rate·elapsed (deterministic now_ns sweep).
        let capacity = 5.0;
        let rate = 50.0;
        let b = TokenBucket::new(capacity, rate);
        let unit = 0.001;
        let mut granted = 0.0f64;
        let mut now_ns = 0u64;
        for _ in 0..5000 {
            now_ns += 1_000; // 1µs per step
            if b.try_acquire(unit, now_ns) {
                granted += unit;
            }
        }
        let elapsed = now_ns as f64 / 1e9;
        let ceiling = capacity + rate * elapsed + 1e-6;
        assert!(
            granted <= ceiling,
            "over-grant invariant violated: granted={granted} > ceiling={ceiling} (elapsed={elapsed}s)"
        );
    }

    #[test]
    fn token_bucket_poison_degrades_closed() {
        // A6: a panic while holding the lock poisons it (test build); the NEXT acquire
        // must NOT cascade the panic — it degrade-closes (returns false).
        use std::sync::Arc;
        let b = Arc::new(TokenBucket::new(10.0, 1.0));
        // Poison: panic while holding the lock.
        {
            let b2 = Arc::clone(&b);
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _guard = b2.inner.lock().expect("lock");
                panic!("chaos: panic mid-critical-section");
            }));
        }
        // The next acquire must not panic, and degrade-closed.
        let recovered =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.try_acquire(1.0, 0)));
        assert!(recovered.is_ok(), "post-poison try_acquire must not cascade");
        assert_eq!(recovered.unwrap(), false, "poisoned bucket degrade-closes (refuses)");
    }
}
