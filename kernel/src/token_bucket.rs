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

    /// Return `n` previously-acquired tokens to the bucket, saturating at `capacity`.
    ///
    /// The refund entry point the `EnvelopeMap` two-level check needs (§2.2): when the
    /// aggregate lane refuses after the per-peer envelope already granted, the peer's tokens
    /// must be given back rather than silently taxed. Interior-mutability `&self`, mirroring
    /// `try_acquire` — a sibling of acquire, not a new mutation discipline.
    ///
    /// F33 bound preserved *by construction*: the stored balance is clamped to `capacity` on
    /// write, so `release` can never mint tokens beyond what a prior `try_acquire` removed.
    /// Releasing more than was acquired, or releasing after a long idle refill has already
    /// topped the bucket up, saturates at the ceiling — it never overflows past `capacity`.
    pub fn release(&self, n: u64) {
        let add = n as f64;
        let mut cur = unpack(self.tokens.load(Ordering::Acquire));
        loop {
            let mut next = cur + add;
            if next > self.capacity {
                next = self.capacity; // saturate: never accumulate past the ceiling.
            }
            match self
                .tokens
                .compare_exchange_weak(pack(cur), pack(next), Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return,
                Err(c) => cur = unpack(c),
            }
        }
    }

    /// Raw internal token count WITHOUT the lazy refill/cap — test-only. Lets the F33
    /// saturation test observe that `release` itself caps *by construction*, independent of
    /// `refill`'s read-path cap (which has an `elapsed == 0` early-return hole and so cannot
    /// be relied on to enforce the ceiling).
    #[cfg(test)]
    fn raw_tokens(&self) -> f64 {
        unpack(self.tokens.load(Ordering::Acquire))
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

    #[test]
    fn release_returns_exact_tokens_when_below_capacity() {
        // §2.2 refund path (bounded case): a release that stays under `capacity` gives back
        // exactly `n` tokens — the aggregate-refusal path can restore the peer's lane precisely.
        let capacity = 10u64;
        let b = TokenBucket::new(capacity, 0.0); // rate 0: isolate `release` from clock refill.
        while b.try_acquire(1) {} // drain fully.
        let before = b.available();
        assert_eq!(before, 0, "drained bucket is empty before release");
        let n = 3u64;
        b.release(n);
        let after = b.available();
        assert_eq!(after - before, n, "release adds back exactly n when below capacity");
        assert!(after <= capacity, "F33 bound: available never exceeds capacity");
    }

    #[test]
    fn release_saturates_at_capacity() {
        // F33 falsifier for "release mints free budget": acquire n, then release 2n (past the
        // ceiling). Available MUST saturate at exactly `capacity`, never overflow it — a naive
        // unclamped add would land at capacity + n and this assertion catches it.
        let capacity = 10u64;
        let b = TokenBucket::new(capacity, 0.0); // rate 0: no refill in play.
        let n = 4u64;
        assert!(b.try_acquire(n)); // available now capacity - n = 6.
        b.release(2 * n); // 6 + 8 = 14 naively; must clamp to capacity (10).
        // Deterministic falsifier: `release` must cap the INTERNAL store, not lean on refill's
        // read-path cap. A naive unclamped add leaves raw_tokens == 14 here and fails this line.
        assert!(
            b.raw_tokens() <= capacity as f64,
            "release capped by construction: internal store never exceeds capacity (was {})",
            b.raw_tokens()
        );
        assert_eq!(
            b.available(),
            capacity,
            "release past capacity saturates at exactly capacity, never mints beyond it"
        );
        assert!(b.available() <= capacity, "F33 bound: available <= capacity, ever");
    }
}

