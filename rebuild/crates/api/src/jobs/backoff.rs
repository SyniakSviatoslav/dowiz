//! Retry backoff (`docs/design/rebuild-jobs-s8-council/proposal.md` §3.3, Q-BARE-DEFAULTS) —
//! FIX-IN-PORT: the census found only 6/30 Node queues run with real backoff+DLQ (the backup
//! family); the other 24 run bare pg-boss v10 defaults (`retryLimit=2`, **0s** backoff, no DLQ) —
//! a transient failure gets hammered twice in milliseconds, then lands in `failed` with no
//! salvage path. This port ships backoff+DLQ as the **baseline for every queue**, documented here
//! as a deliberate reliability improvement, not a silent parity break.
//!
//! Exponential with full jitter, capped — the same shape `crate::ws::pg_fanout::reconnect_backoff`
//! already uses for the S6 listener reconnect (`Math.min(1000 * 2**attempts, 30000)` parity), but
//! this is a DIFFERENT constant (retry delay between job attempts, not a socket-reconnect delay)
//! and adds jitter — a `SKIP LOCKED` fleet retrying every failed job at the exact same clock tick
//! is a self-inflicted thundering herd the reconnect case doesn't have (one listener, not N
//! competing job attempts).

use rand::Rng;
use std::time::Duration;

/// Base delay for attempt 1. Doubles per attempt thereafter (`base * 2^(attempts-1)`).
const BASE: Duration = Duration::from_secs(2);
/// Ceiling — no job waits longer than this between attempts, however high `attempts` climbs.
const CAP: Duration = Duration::from_secs(15 * 60);

/// The default retry ceiling for every queue (Q-BARE-DEFAULTS) — past this, a job moves to the
/// DLQ (`crate::jobs::runner`) instead of being requeued again.
pub const DEFAULT_MAX_ATTEMPTS: i32 = 8;

/// `attempts` is 1-indexed (the count AFTER the failing attempt that triggered this call, i.e.
/// `jobs.attempts` post-increment — see `crate::jobs::runner`'s claim SQL, which increments
/// `attempts` as part of the claim itself). Full jitter (`rand::thread_rng().gen_range(0..=delay)`)
/// rather than equal/decorrelated jitter — the simplest jitter shape that still breaks a
/// thundering herd, and this fleet's job volume (~1-2k/day, proposal §2 back-of-envelope) doesn't
/// need a fancier one.
pub fn backoff_for_attempt(attempts: i32) -> Duration {
    let exponent = u32::try_from(attempts.saturating_sub(1).clamp(0, 32)).unwrap_or(32);
    let uncapped_ms = BASE
        .as_millis()
        .saturating_mul(1u128.checked_shl(exponent).unwrap_or(u128::MAX).max(1));
    let capped_ms = uncapped_ms.min(CAP.as_millis());
    #[allow(
        clippy::as_conversions,
        reason = "capped_ms is bounded by CAP.as_millis() above (900_000), always within u64 range"
    )]
    let capped_ms = capped_ms as u64;
    let jittered_ms = rand::thread_rng().gen_range(0..=capped_ms);
    Duration::from_millis(jittered_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_exponentially_before_the_cap() {
        // Jitter makes exact values non-deterministic — assert the CEILING per attempt instead
        // (the value backoff_for_attempt can never exceed), which IS deterministic.
        let ceiling_for = |attempts: i32| -> Duration {
            let exponent = u32::try_from((attempts - 1).clamp(0, 32)).unwrap_or(32);
            (BASE * 2u32.pow(exponent)).min(CAP)
        };
        assert_eq!(ceiling_for(1), Duration::from_secs(2));
        assert_eq!(ceiling_for(2), Duration::from_secs(4));
        assert_eq!(ceiling_for(3), Duration::from_secs(8));
        assert_eq!(ceiling_for(4), Duration::from_secs(16));

        for attempts in 1..=6 {
            let ceiling = ceiling_for(attempts);
            for _ in 0..20 {
                let sample = backoff_for_attempt(attempts);
                assert!(
                    sample <= ceiling,
                    "attempt {attempts}: {sample:?} exceeded ceiling {ceiling:?}"
                );
            }
        }
    }

    #[test]
    fn backoff_never_exceeds_the_cap_however_high_attempts_climbs() {
        for attempts in [10, 20, 32, 100, i32::MAX] {
            for _ in 0..20 {
                assert!(backoff_for_attempt(attempts) <= CAP);
            }
        }
    }

    #[test]
    fn backoff_can_be_zero_full_jitter_includes_the_floor() {
        // Full jitter's range is [0, ceiling] inclusive — this is a property test over many
        // samples rather than asserting a specific draw hits exactly 0 (that would be flaky).
        let saw_a_small_value =
            (0..500).any(|_| backoff_for_attempt(1) < Duration::from_millis(200));
        assert!(
            saw_a_small_value,
            "full jitter should occasionally draw values near the floor"
        );
    }

    #[test]
    fn default_max_attempts_is_a_real_ceiling_not_unbounded() {
        // A `const { assert!(..) }` block (clippy::assertions_on_constants' own suggestion, since
        // both sides ARE compile-time constants here) — stronger than a runtime test assertion:
        // this fails the BUILD, not just `cargo test`, the moment the constant is set to
        // something silly.
        const { assert!(DEFAULT_MAX_ATTEMPTS > 0) };
        const {
            assert!(
                DEFAULT_MAX_ATTEMPTS < 100,
                "an unbounded-looking retry ceiling defeats the DLQ's purpose"
            )
        };
    }
}
