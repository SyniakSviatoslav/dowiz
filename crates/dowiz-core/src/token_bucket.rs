//! `token_bucket.rs` — P11 §4 / F33 compute-budget primitive (pure no_std core).
//!
//! The *pure* half: the integer GCRA (Generic Cell Rate Algorithm) decision package
//! ([`gcra_decide`], item 8 / `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` §5) and the
//! lock-free [`GcraTokenBucket`] (a single `AtomicU64` holding the theoretical-arrival-time).
//! Integer nanoseconds throughout: NO `f64` in `gcra_decide`, NO `std` anywhere.
//!
//! The std side — [`TokenBucket`] (a `Mutex<Inner>` of `tokens: f64` + `Instant`, with the
//! mutex-poison recovery and the `chaos` seam) — stays in the kernel shim, which re-exports
//! these pure types and adds [`crate::clock::now_ns()`]-stamping free functions.
//!
//! Scope limit of the GCRA swap (verified, not assumed): valid ONLY for `refill_rate > 0.0`
//! (the continuous-refill case). At `refill_rate <= 0.0`, `nanos_per_token` is infinite and
//! `gcra_decide` grants exactly once, ever — it cannot drain a one-shot budget across many
//! partial acquires the way `TokenBucket` does. Callers using `TokenBucket::new(_, 0.0)` as
//! a drain-to-zero budget must keep using `TokenBucket`.

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
}
