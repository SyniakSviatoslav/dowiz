//! WHAT A STOCK RECORD SAYS BESIDES ITS MOVEMENT (research 2026-09-26 §3.3,
//! rows R2, R3, R5, R7, R8): when it happened, which lot, from whom, on which
//! paper, until when it keeps, what it cost, which count it belonged to.
//!
//! THE SHELF'S FOLD NEVER READS ANY OF THIS. Every field rides as an extra
//! JSON key appended INSIDE the record `encode` wrote -- the pattern
//! `cost.rs` set for prices -- so `decode` sees the same `item`/`qty`/`order`
//! it always did, and a record written before these keys existed reads with
//! every one of them absent. That is the old-image rule: a venue's shelf is the
//! fold of its whole history, and none of it may stop folding.
//!
//! Integers only: `at` is the Worker's one clock in ms; `expiry` a LOCAL day as
//! `yyyymmdd` (20261003), so "is it past" is an integer compare against a day
//! the Worker computed in the venue's zone; money is minor units.

use super::{encode, StockError, StockEvent, StockLog};
use crate::minijson::{esc, int_field, str_field};

/// Everything optional. `Meta::default()` writes nothing extra.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Meta {
    /// ms, the request clock. Absent on every record written before 2026-09-26.
    pub at: Option<i64>,
    /// The label's lot code (the Albanian "L" code) or a hub-minted one.
    pub lot: Option<String>,
    pub supplier: Option<String>,
    /// The invoice / delivery note number.
    pub doc: Option<String>,
    /// Best-before, a local day `yyyymmdd`.
    pub expiry: Option<i64>,
    /// Minor units per `per` base units (a receipt's price).
    pub unit_cost: Option<i64>,
    pub per: Option<i64>,
    /// One id for every line of one stocktake.
    pub session: Option<String>,
    /// What the ledger held when the count was written, so drift is a row.
    pub expected: Option<i64>,
    /// Minor units this record is worth at the cost book's average (waste).
    pub value: Option<i64>,
    /// Who acted, on a record whose event has no signer of its own (a receipt).
    pub by: Option<String>,
}

impl Meta {
    pub fn at(ms: i64) -> Self {
        Meta { at: Some(ms), ..Meta::default() }
    }

    /// The keys, as `,"k":v` pairs in a fixed order.
    fn pairs(&self) -> String {
        let mut s = String::new();
        let mut int = |k: &str, v: Option<i64>| {
            if let Some(v) = v {
                s.push_str(&format!(r#","{k}":{v}"#));
            }
        };
        int("at", self.at);
        int("expiry", self.expiry);
        int("unit_cost", self.unit_cost);
        int("per", self.per);
        int("expected", self.expected);
        int("value", self.value);
        for (k, v) in [
            ("lot", &self.lot),
            ("supplier", &self.supplier),
            ("doc", &self.doc),
            ("session", &self.session),
            ("by_", &self.by),
        ] {
            if let Some(v) = v.as_ref().filter(|v| !v.trim().is_empty()) {
                s.push_str(&format!(r#","{k}":"{}""#, esc(v.trim())));
            }
        }
        s
    }
}

/// `payload` (one JSON object) with `meta`'s keys appended inside it.
pub fn with_meta(payload: &str, meta: &Meta) -> String {
    let extra = meta.pairs();
    if extra.is_empty() {
        return payload.to_string();
    }
    let body = payload.strip_suffix('}').unwrap_or(payload);
    format!("{body}{extra}}}")
}

/// What a raw record says besides its movement. Keys a record does not carry
/// are `None` -- every record written before this file existed.
///
/// `by_` (not `by`): `Wasted` and `Stocktake` already carry their signer as
/// `by`, and `str_field` answers the FIRST key of a name.
pub fn meta_of(rec: &str) -> Meta {
    Meta {
        at: int_field(rec, "at"),
        lot: str_field(rec, "lot"),
        supplier: str_field(rec, "supplier"),
        doc: str_field(rec, "doc"),
        expiry: int_field(rec, "expiry").filter(|d| valid_day(*d)),
        unit_cost: int_field(rec, "unit_cost"),
        per: int_field(rec, "per"),
        session: str_field(rec, "session"),
        expected: int_field(rec, "expected"),
        value: int_field(rec, "value"),
        by: str_field(rec, "by_"),
    }
}

/// A `yyyymmdd` that could be a day (month 1-12, day 1-31, years 2000-2999).
pub fn valid_day(d: i64) -> bool {
    let (y, m, dd) = (d / 10_000, d / 100 % 100, d % 100);
    (2000..3000).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&dd)
}

/// Days since 1970-01-01 of a `yyyymmdd` (Howard Hinnant's `days_from_civil`,
/// integers only), so "how many days until it expires" is a subtraction.
pub fn day_number(d: i64) -> i64 {
    let (y, m, dd) = (d / 10_000, d / 100 % 100, d % 100);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + dd - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The `yyyymmdd` of a day number ([`day_number`]'s inverse).
pub fn day_of_number(z: i64) -> i64 {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    y * 10_000 + m * 100 + d
}

/// The `yyyymmdd` of a LOCAL instant (ms already shifted into the venue's
/// zone, `tz::local_ms`).
pub fn day_of_local_ms(local_ms: i64) -> i64 {
    day_of_number(local_ms.div_euclid(86_400_000))
}

/// `"2026-10-03"` -> 20261003; anything else `None`.
pub fn parse_day(s: &str) -> Option<i64> {
    let b = s.trim().as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let n = |r: std::ops::Range<usize>| std::str::from_utf8(&b[r]).ok()?.parse::<i64>().ok();
    let d = n(0..4)? * 10_000 + n(5..7)? * 100 + n(8..10)?;
    (valid_day(d) && day_of_number(day_number(d)) == d).then_some(d)
}

/// 20261003 -> `"2026-10-03"`.
pub fn show_day(d: i64) -> String {
    format!("{:04}-{:02}-{:02}", d / 10_000, d / 100 % 100, d % 100)
}

impl StockLog {
    /// THE REQUEST CLOCK. Once set, every record this log writes carries
    /// `"at"` -- the order lifecycle's reservations as well as an owner's
    /// delivery -- without a single `StockEvent` changing shape.
    pub fn set_clock(&mut self, now_ms: i64) {
        self.clock = Some(now_ms);
    }

    /// The meta a write stamps when the caller gave none: the clock.
    pub(super) fn stamped(&self, meta: &Meta) -> Meta {
        let mut m = meta.clone();
        if m.at.is_none() {
            m.at = self.clock;
        }
        m
    }

    /// Append one event with what it says besides its movement, AFTER the
    /// ledger agreed -- the same gate and signer rule as [`StockLog::append`].
    pub fn append_with(&mut self, ev: &StockEvent, meta: &Meta) -> Result<(), StockError> {
        self.append_all_with(&[(ev.clone(), meta.clone())])
    }

    /// Several as ONE decision, each with its own meta (a stocktake session:
    /// one `expected` per line). All or nothing, as [`StockLog::append_all`].
    pub fn append_all_with(&mut self, evs: &[(StockEvent, Meta)]) -> Result<(), StockError> {
        for (ev, _) in evs {
            super::signed(ev)?;
        }
        let mut trial = self.ledger()?;
        for (ev, _) in evs {
            trial.apply(ev)?;
        }
        for (ev, meta) in evs {
            let m = self.stamped(meta);
            self.write_payload(with_meta(&encode(ev), &m).into_bytes())?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "meta/tests.rs"]
mod tests;
