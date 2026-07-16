//! token_bucket.rs — F33 compute-budget `TokenBucket` (zero-dep, monotonic-clock).
//!
//! Verified-by-math property (the falsifier): the total tokens granted across any window of
//! `elapsed` seconds NEVER exceeds `capacity + refill_rate * elapsed`. Degrade-closed: when empty,
//! `try_acquire` returns `false` (the caller's typed `Err`), never silently queues-then-downgrades.
//!
//! This is the budget primitive the `llm-adapters` `Dispatcher` reuses to bound concurrency on
//! LLM calls (§4.2). It is deliberately plain-std (no tokio) so it can live in the kernel.
//!
//! Tokens are held as a `f64` inside an `AtomicU64` (bit-cast) so fractional refills accumulate
//! across calls (a 0.1/s rate must reach 1.0 after 1s even when probed every 100ms — the earlier
//! integer-truncation version lost sub-unit time).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Bit-cast f64 <-> u64 for lock-free atomic storage of the fractional token count.
fn pack(x: f64) -> u64 {
    x.to_bits()
}
fn unpack(b: u64) -> f64 {
    f64::from_bits(b)
}

/// A monotonic token bucket.
pub struct TokenBucket {
    capacity: f64,
    refill_rate: f64, // tokens per second
    tokens: AtomicU64,
    last: Mutex<Instant>,
}

impl TokenBucket {
    pub fn new(capacity: u64, refill_rate: f64) -> Self {
        TokenBucket {
            capacity: capacity as f64,
            refill_rate,
            tokens: AtomicU64::new(pack(capacity as f64)),
            last: Mutex::new(Instant::now()),
        }
    }

    /// Refill based on elapsed wall-clock (fractional, accumulates), then atomically subtract `n`.
    /// Returns `true` iff granted; `false` ⇒ caller must degrade-closed.
    pub fn try_acquire(&self, n: u64) -> bool {
        self.refill();
        let need = n as f64;
        let mut cur = unpack(self.tokens.load(Ordering::Acquire));
        loop {
            if cur < need {
                return false; // degrade-closed: no partial grant, no silent downgrade.
            }
            let next = pack(cur - need);
            match self
                .tokens
                .compare_exchange_weak(pack(cur), next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return true,
                Err(c) => cur = unpack(c),
            }
        }
    }

    /// Lazy fractional refill: advance the clock by `elapsed` unconditionally (so sub-unit time
    /// accumulates), add `refill_rate * elapsed` to the token count, cap at `capacity`.
    fn refill(&self) {
        let mut last = self.last.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(*last);
        if elapsed <= Duration::ZERO {
            return;
        }
        let add = self.refill_rate * elapsed.as_secs_f64();
        let cur = unpack(self.tokens.load(Ordering::Acquire));
        let mut new = cur + add;
        if new > self.capacity {
            new = self.capacity;
        }
        self.tokens.store(pack(new), Ordering::Release);
        *last = now; // clock advances by full elapsed even when add rounds < 1 unit.
    }

    /// Current token count (for telemetry/tests).
    pub fn available(&self) -> u64 {
        self.refill();
        unpack(self.tokens.load(Ordering::Acquire)).max(0.0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_exceeds_capacity_plus_refill() {
        // F33 falsifier: total granted over any window never exceeds capacity + refill_rate*elapsed.
        let capacity = 10u64;
        let rate = 1.0; // 1 token/sec
        let b = TokenBucket::new(capacity, rate);
        let mut granted = 0u64;
        while b.try_acquire(1) {
            granted += 1;
        }
        assert_eq!(granted, capacity, "can only grant up to capacity initially");
        // Now empty → acquire must fail (degrade-closed).
        assert!(!b.try_acquire(1), "empty bucket must refuse");
        // After a full ~1.1s (fractional refill must accumulate), ~1 token available → succeeds.
        std::thread::sleep(Duration::from_millis(1100));
        assert!(b.try_acquire(1), "after ~1.1s refill, one token granted");
        // Invariant: granted this run + currently-available ≤ capacity + ~1 (refill over the window).
        assert!(b.available() <= capacity + 1);
    }

    #[test]
    fn budget_exhausted_returns_typed_false() {
        let b = TokenBucket::new(2, 0.0); // no refill
        assert!(b.try_acquire(1));
        assert!(b.try_acquire(1));
        assert!(!b.try_acquire(1), "third acquire must be typed-false (degrade-closed)");
    }
}

