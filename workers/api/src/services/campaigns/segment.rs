//! WHO A CAMPAIGN IS FOR — a closed list of predicates, never a stored list.
//!
//! §2.5 of BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22: a segment is a pure filter
//! over the customer fold ⋈ the consent fold ⋈ the card's tags, evaluated at
//! SEND time. A stored recipient list is PII that goes stale the moment
//! someone withdraws, and its existence is the thing an erasure would have to
//! find; so there is no list, only this function.
//!
//! CLOSED. Four variants, chosen from a menu; an owner cannot type a query,
//! because a query language over people is a profiling tool with a text box.
//!
//! EVERY VARIANT IS A SUBSET OF "CONSENTED". `matches` answers `false` for a
//! person with no marketing consent whatever the variant says -- and the send
//! additionally needs the `Consented` witness itself (`send::entry`), so this
//! check is the second lock, not the only one.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::services::customers::roll::Row;

pub const DAY_MS: i64 = 24 * 60 * 60 * 1000;
/// "Not seen for more than a year" is not a campaign, it is a mailing list.
pub const MAX_DAYS: i64 = 365;
/// A week: today and the six days after it, in the venue's own calendar.
pub const WEEK_DAYS: i64 = 7;

/// The closed list (§3.6). Serialised as `{"kind": "...", ...}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Segment {
    /// Everyone holding a marketing consent.
    EveryoneConsented,
    /// Their last order is at least `days` days old.
    NotSeenSince { days: i64 },
    /// Their card carries this tag (from `record::TAGS`, the closed list).
    Tag { tag: String },
    /// Their card's MM-DD falls in the next seven days, today included.
    BirthdayThisWeek,
}

impl Segment {
    /// Refused shapes, by name. The type already closes the KIND; this closes
    /// the values inside it.
    pub fn check(&self) -> Result<(), String> {
        match self {
            Segment::NotSeenSince { days } if !(1..=MAX_DAYS).contains(days) => {
                Err(format!("\"not seen since\" is 1 to {MAX_DAYS} days"))
            }
            Segment::Tag { tag } if !crate::services::customers::record::TAGS.contains(&tag.as_str()) => {
                Err(format!("tag {tag:?} is not in the closed list"))
            }
            _ => Ok(()),
        }
    }
}

/// What the CARD contributes: its tags and its birthday. A linked person
/// (§3.4) has several cards; their tags are the union and the first
/// well-formed birthday is theirs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    pub tags: Vec<String>,
    pub birthday_md: Option<String>,
    pub lang: Option<String>,
}

impl Record {
    pub fn of_cards<'a>(cards: impl IntoIterator<Item = &'a str>) -> Record {
        let mut r = Record::default();
        for c in cards {
            let Ok(v) = serde_json::from_str::<Value>(c) else { continue };
            for t in v.get("tags").and_then(Value::as_array).into_iter().flatten() {
                if let Some(t) = t.as_str().filter(|t| !r.tags.iter().any(|x| x == t)) {
                    r.tags.push(t.to_string());
                }
            }
            if r.birthday_md.is_none() {
                r.birthday_md =
                    v.get("birthday_md").and_then(Value::as_str).filter(|b| md(b).is_some()).map(str::to_string);
            }
            if r.lang.is_none() {
                r.lang = v.get("lang").and_then(Value::as_str).filter(|l| !l.is_empty()).map(str::to_string);
            }
        }
        r
    }
}

/// Whether the person holds a marketing consent on the campaign's channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsentState {
    pub given: bool,
}

/// The request's instant, twice: UTC for "how long ago", the venue's wall
/// clock for "which day is it" (`tz::local_ms`). Taken from `ctx.data.now_ms`,
/// never read here (`tools/gates/clock.sh`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now {
    pub utc_ms: i64,
    pub local_ms: i64,
}

/// THE PREDICATE. Pure; no clock, no I/O.
pub fn matches(seg: &Segment, row: &Row, rec: &Record, consent: &ConsentState, now: Now) -> bool {
    if !consent.given {
        return false;
    }
    match seg {
        Segment::EveryoneConsented => true,
        Segment::NotSeenSince { days } => now.utc_ms - row.last_at >= days * DAY_MS,
        Segment::Tag { tag } => rec.tags.iter().any(|t| t == tag),
        Segment::BirthdayThisWeek => {
            rec.birthday_md.as_deref().and_then(md).is_some_and(|b| birthday_within(b, now.local_ms))
        }
    }
}

/// `MM-DD` → (month, day), or nothing. The card already refuses a bad one on
/// write; a record written by an older build is read with the same rule.
fn md(s: &str) -> Option<(i64, i64)> {
    let b = s.as_bytes();
    if b.len() != 5 || b[2] != b'-' {
        return None;
    }
    let m: i64 = s[0..2].parse().ok()?;
    let d: i64 = s[3..5].parse().ok()?;
    ((1..=12).contains(&m) && (1..=31).contains(&d)).then_some((m, d))
}

/// Whether (month, day) falls in the week starting on the local day of
/// `local_ms`. A 29 February birthday is kept on 28 February in a common year
/// -- the day that year HAS, rather than a greeting that never comes.
fn birthday_within((bm, bd): (i64, i64), local_ms: i64) -> bool {
    let today = local_ms.div_euclid(DAY_MS);
    (0..WEEK_DAYS).any(|i| {
        let (y, m, d) = civil(today + i);
        (m, d) == (bm, bd) || ((bm, bd) == (2, 29) && !leap(y) && (m, d) == (2, 28))
    })
}

fn leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Days since 1970-01-01 → (year, month, day). Hinnant's `civil_from_days`,
/// exact integer arithmetic (`dowiz_hub::tz` keeps its own copy private).
pub fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests;
