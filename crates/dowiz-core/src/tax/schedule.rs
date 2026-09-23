//! The rate IN FORCE at a moment, and the owner's short list of future changes.
//!
//! STAMPING, NOT LOOKING UP (blueprint §2.6). An order carries the rate it was
//! charged at, and the content-chained log proves it; nothing is looked up
//! later. What stamping cannot do is change the rate at 00:00 on the day a law
//! changes while the owner is asleep — Germany 2026-01-01 is the case. So the
//! settings hold the current rate (`tax.default_ppm`) and this optional list of
//! FUTURE changes (`tax.schedule`, `[{"since_ms":…,"ppm":…}]`). Past changes
//! are not kept here; the log has them.
//!
//! NO CLOCK. `now_ms` is passed in — the one clock is `Req { now_ms }`, read
//! once per request, and `tools/gates/clock.sh` stays at zero.

use super::RatePpm;
use alloc::vec::Vec;

/// At most this many scheduled changes. A longer list is a rate HISTORY, and
/// the history lives in the log, not in a setting.
pub const MAX_CHANGES: usize = 8;

/// One scheduled change: from `since_ms` (inclusive) the rate is `ppm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    pub since_ms: i64,
    pub ppm: RatePpm,
}

/// Parse `tax.schedule`. Empty text is an empty schedule. Integers only: a
/// float `ppm` or `since_ms` is refused, as is any key but those two.
pub fn parse(s: &str) -> Result<Vec<Change>, &'static str> {
    use crate::json::Value;
    if s.trim().is_empty() {
        return Ok(Vec::new());
    }
    let v = crate::json::parse(s).map_err(|_| "tax.schedule: not JSON")?;
    let Some(items) = v.as_array() else {
        return Err("tax.schedule: must be a list [{\"since_ms\":…,\"ppm\":…}]");
    };
    if items.len() > MAX_CHANGES {
        return Err("tax.schedule: at most 8 future changes; the log keeps the history");
    }
    let mut out = Vec::with_capacity(items.len());
    for it in items {
        let Value::Object(members) = it else {
            return Err("tax.schedule: every entry is an object {\"since_ms\",\"ppm\"}");
        };
        if members.iter().any(|(k, _)| k != "since_ms" && k != "ppm") {
            return Err("tax.schedule: an entry has a key other than since_ms and ppm");
        }
        let since_ms = match it.get("since_ms") {
            Some(Value::Int(n)) => *n,
            Some(_) => return Err("tax.schedule: since_ms must be an integer of milliseconds"),
            None => return Err("tax.schedule: an entry has no since_ms"),
        };
        let ppm = match it.get("ppm") {
            Some(Value::Int(n)) if (0..=i64::from(RatePpm::MAX)).contains(n) => RatePpm(*n as u32),
            Some(Value::Int(_)) => return Err("tax.schedule: ppm must be 0..=1000000"),
            Some(_) => return Err("tax.schedule: ppm must be an integer (20% is 200000, never 0.20)"),
            None => return Err("tax.schedule: an entry has no ppm"),
        };
        out.push(Change { since_ms, ppm });
    }
    Ok(out)
}

/// The rate in force at `now_ms`: the latest change whose `since_ms` has been
/// reached, else `default`. `None` means the venue has no rate — it is not
/// configured for tax, and that is a named state, never a silent zero.
pub fn in_force(schedule: &[Change], default: Option<RatePpm>, now_ms: i64) -> Option<RatePpm> {
    schedule
        .iter()
        .filter(|c| c.since_ms <= now_ms)
        .max_by_key(|c| c.since_ms)
        .map(|c| c.ppm)
        .or(default)
}
