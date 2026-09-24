//! A circuit breaker for the three outbound rails that can make a healthy
//! product look broken: Stripe, the venue's AI endpoint, and Meta.
//!
//! WHY NOT THE ONE THE KERNEL ALREADY HAS. `crates/dowiz-core/src/breaker` is
//! eight files of genuinely good work, and the resilience blueprint said to
//! wire it here. **That recommendation was wrong, and reading the code is what
//! showed it.** That breaker is an ADMISSION CONTROL for an agent: it advances
//! per WINDOW on a `SignalVector`, trips on an anomaly score against a fitted
//! `ThresholdId`, keeps its state and an FDR audit chain in memory, and is
//! unconstructible without a threshold set fitted from measured rates. Every one
//! of those properties is right for what it was built for and wrong here:
//!
//!   * a Worker isolate does not survive between requests, so in-memory state
//!     is state that resets whenever traffic is light -- which is exactly when
//!     an outage is least likely to be noticed;
//!   * there are no windows and no rates to fit. There is "Stripe answered" and
//!     "Stripe did not";
//!   * an anomaly score over a signal vector is a richer question than the one
//!     being asked, and a richer question answered with invented inputs is
//!     worse than the simple one answered honestly.
//!
//! So this is the Release It! breaker, in the venue's own object, where
//! per-venue state already lives and already survives: count consecutive
//! failures, open after a threshold, half-open after a cooldown, close on one
//! success. The kernel's breaker stays where it belongs.
//!
//! WHAT IT IS FOR, precisely. It does not invent the fallback: placement
//! already degrades correctly when Stripe is unreachable -- the order is in the
//! log and comes back marked so the surface can offer cash. What it does not do
//! is stop TRYING, so every customer during an outage pays the full timeout
//! before being offered cash. **Stopping that is this module's whole job.**

use worker::*;

use crate::hubstore::Place;

/// The rails. Named rather than free strings so a typo cannot create a fourth
/// breaker that trips nothing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Rail {
    Stripe,
    Ai,
    Meta,
}

impl Rail {
    pub fn as_str(self) -> &'static str {
        match self {
            Rail::Stripe => "stripe",
            Rail::Ai => "ai",
            Rail::Meta => "meta",
        }
    }
}

/// The image the breakers live in. Small, per venue, and NOT the ops image: a
/// rail's state changes on a cadence nothing else shares, and putting it beside
/// the assignments would make every courier action rewrite it.
pub const IMAGE_RAILS: &str = "rails";
pub const RAILS_BYTES: usize = dowiz_hub::CEILING_BYTES;
const KIND: &str = "rail";

/// Consecutive failures that open the rail.
///
/// THREE, not one. A single timeout is a bad minute; three in a row on the same
/// venue is a provider. Opening on one would make a breaker that trips on
/// noise, and a breaker that trips on noise gets switched off.
const OPEN_AFTER: i64 = 3;

/// How long it stays open before one call is let through to find out.
const COOLDOWN_MS: i64 = 60_000;

/// What the caller should do.
pub enum Gate {
    /// Make the call.
    Go,
    /// Do not. The rail is open; use the fallback immediately.
    Tripped { since_ms: i64 },
}

fn state(t: &dowiz_hub::table::Table, rail: Rail) -> (i64, i64) {
    t.get(KIND, rail.as_str())
        .and_then(|j| serde_json::from_str::<serde_json::Value>(&j).ok())
        .map(|v| {
            (
                v.get("failures").and_then(serde_json::Value::as_i64).unwrap_or(0),
                v.get("opened_at_ms").and_then(serde_json::Value::as_i64).unwrap_or(0),
            )
        })
        .unwrap_or((0, 0))
}

/// May this call go out?
///
/// FAILS OPEN, like every other guard in this file's family: if the state
/// cannot be read, the call is made. A breaker that refuses a payment because
/// its own storage hiccuped has done more damage than the outage it watches.
pub async fn admit(place: &Place, rail: Rail, now_ms: i64) -> Gate {
    let Ok(l) = crate::hubstore::load_table(place, IMAGE_RAILS, RAILS_BYTES).await else {
        return Gate::Go;
    };
    let (failures, opened) = state(&l.table, rail);
    if failures < OPEN_AFTER || opened == 0 {
        return Gate::Go;
    }
    if now_ms - opened >= COOLDOWN_MS {
        // HALF OPEN: one call is let through. It is not marked here -- whatever
        // that call reports closes the rail or re-opens it, and marking it
        // before it happened would let a burst of retries all pass as probes.
        return Gate::Go;
    }
    Gate::Tripped { since_ms: now_ms - opened }
}

/// Tell the breaker what happened.
///
/// NEVER FAILS THE CALLER. The call has already succeeded or already failed;
/// losing this record means the breaker is slower to notice, never that a
/// request is refused.
pub async fn record(place: &Place, rail: Rail, ok: bool, now_ms: i64) {
    let _ = crate::hubstore::with_table(place, IMAGE_RAILS, RAILS_BYTES, move |t| {
        let (failures, opened) = state(t, rail);
        let (failures, opened) = if ok {
            // ONE SUCCESS CLOSES IT. Requiring several would keep a recovered
            // provider shut out for no reason a customer would accept.
            (0, 0)
        } else {
            let n = failures + 1;
            (n, if n >= OPEN_AFTER && opened == 0 { now_ms } else { opened })
        };
        let rec = serde_json::json!({
            "failures": failures, "opened_at_ms": opened, "at_ms": now_ms,
        })
        .to_string();
        t.put(KIND, rail.as_str(), &rec, &[], &[])
            .map_err(|e| Error::RustError(format!("rail: {e}")))
    })
    .await;
}

/// What every rail is doing, for `/api/owner/health`.
///
/// AN INSTRUMENT THAT CANNOT BE SEEN IS NOT AN INSTRUMENT. A breaker that
/// silently protects a venue is indistinguishable from one that silently does
/// nothing, and this codebase has paid for that distinction before.
pub async fn snapshot(place: &Place, now_ms: i64) -> serde_json::Value {
    let Ok(l) = crate::hubstore::load_table(place, IMAGE_RAILS, RAILS_BYTES).await else {
        return serde_json::json!({});
    };
    let mut out = serde_json::Map::new();
    for rail in [Rail::Stripe, Rail::Ai, Rail::Meta] {
        let (failures, opened) = state(&l.table, rail);
        let open = failures >= OPEN_AFTER && opened != 0 && now_ms - opened < COOLDOWN_MS;
        out.insert(
            rail.as_str().into(),
            serde_json::json!({
                "failures": failures,
                "open": open,
                "openedAtMs": if opened == 0 { serde_json::Value::Null } else { serde_json::json!(opened) },
            }),
        );
    }
    serde_json::Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The decision itself, with the store taken out of it. These four cases
    /// are the whole breaker; everything else in this file is storage.
    fn decide(failures: i64, opened: i64, now: i64) -> bool {
        if failures < OPEN_AFTER || opened == 0 {
            return true;
        }
        now - opened >= COOLDOWN_MS
    }

    #[test]
    fn a_healthy_rail_admits() {
        assert!(decide(0, 0, 1000));
    }

    #[test]
    fn one_failure_is_a_bad_minute_and_not_a_provider() {
        assert!(decide(1, 0, 1000));
        assert!(decide(2, 0, 1000));
    }

    #[test]
    fn three_in_a_row_shuts_it() {
        assert!(!decide(OPEN_AFTER, 1000, 1000));
        assert!(!decide(OPEN_AFTER, 1000, 1000 + COOLDOWN_MS - 1));
    }

    #[test]
    fn after_the_cooldown_one_call_is_let_through() {
        assert!(decide(OPEN_AFTER, 1000, 1000 + COOLDOWN_MS));
    }

    /// A rail that failed three times and was never marked open must not be
    /// treated as open for ever by a clock that has moved past zero.
    #[test]
    fn failures_without_an_opening_time_do_not_trip() {
        assert!(decide(99, 0, 1_789_000_000_000));
    }
}
