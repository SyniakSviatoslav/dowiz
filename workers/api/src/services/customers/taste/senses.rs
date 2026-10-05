//! PURE. THE GUEST ON THE DISH'S AXES: taste, texture and aroma (W-SENSE rows 3, 4 and 6).
//!
//! The same fold as the tags (`taste.rs`), over the dish's declared sensory vector
//! (`dowiz_hub::sense::vector`, per mille of each scale): an order adds `qty x UNIT x share`, old
//! weights fade with the 60-day half-life. Three more things are kept in the same record, so the
//! objection, the export and the erasure cover them with no new path:
//!   * `ctx`: the same weights per CONTEXT the order was placed in (`band:evening`, `day:weekend`,
//!     `wx:rain`), so the storefront can rank by the moment;
//!   * `when`: how many orders fell in each weekday x band ("thu:evening"), for the owner's
//!     planning hint;
//!   * `months`: one snapshot of the vector per calendar month (the newest `MONTHS_KEEP`), the
//!     guest's "taste over the year" and the owner's trend.
//! Keys outside the vocabulary are never written (`dowiz_hub::sense::key_ok`).

use std::collections::BTreeMap;

use super::{fade, Line, Profile, UNIT};

/// Months of snapshots kept.
pub const MONTHS_KEEP: usize = 12;
/// Keys in one snapshot (the strongest).
pub const SNAP_TOP: usize = 12;
/// Weekday x band slots kept (7 x 5 is all of them).
pub const WHEN_KEEP: usize = 35;
/// Snapshot weights are per mille of the month's strongest key.
pub const SNAP_SCALE: i64 = 1000;

/// The context an order was placed in, as the profile files it (`venue::context::Bucket`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct At {
    /// `band:evening`, `day:weekday`, `wx:rain` ...
    pub keys: Vec<String>,
    /// `thu:evening`.
    pub when: String,
}

/// "2026-10" for a day number (days since 1970-01-01).
pub fn month_of(day: i64) -> String {
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}", yoe + era * 400 + i64::from(m <= 2), m)
}

fn fade_map(m: &mut BTreeMap<String, i64>, gap: i64) {
    for w in m.values_mut() {
        *w = fade(*w, gap);
    }
    m.retain(|_, w| *w > 0);
}

/// The strongest `n` of a map, scaled to per mille of its top, zeros dropped.
pub fn snapshot(m: &BTreeMap<String, i64>, n: usize) -> BTreeMap<String, i64> {
    let mut v: Vec<(&String, &i64)> = m.iter().filter(|(_, w)| **w > 0).collect();
    v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    let top = v.first().map_or(1, |(_, w)| (**w).max(1));
    v.into_iter().take(n).map(|(k, w)| (k.clone(), (w * SNAP_SCALE / top).max(1))).collect()
}

/// One order folded in: fade by `gap` days, add each line's vector, file the context, and
/// re-take this month's snapshot. Called by `taste::apply_order_at` after the tags.
pub fn fold(p: &mut Profile, lines: &[Line], gap: i64, day: i64, at: Option<&At>) {
    fade_map(&mut p.sense, gap);
    for m in p.ctx.values_mut() {
        fade_map(m, gap);
    }
    p.ctx.retain(|_, m| !m.is_empty());
    let mut add: BTreeMap<String, i64> = BTreeMap::new();
    for l in lines.iter().filter(|l| l.qty > 0) {
        for (k, share) in l.sense.iter().filter(|(k, _)| dowiz_hub::sense::key_ok(k)) {
            *add.entry(k.clone()).or_default() += l.qty * UNIT * (*share).clamp(0, 1000) / 1000;
        }
    }
    for (k, w) in &add {
        *p.sense.entry(k.clone()).or_default() += w;
    }
    if let Some(at) = at {
        for c in &at.keys {
            let m = p.ctx.entry(c.clone()).or_default();
            for (k, w) in &add {
                *m.entry(k.clone()).or_default() += w;
            }
        }
        if !at.when.is_empty() && (p.when.contains_key(&at.when) || p.when.len() < WHEN_KEEP) {
            *p.when.entry(at.when.clone()).or_default() += 1;
        }
    }
    if !p.sense.is_empty() {
        p.months.insert(month_of(day), snapshot(&p.sense, SNAP_TOP));
        while p.months.len() > MONTHS_KEEP {
            let first = p.months.keys().next().cloned().unwrap_or_default();
            p.months.remove(&first);
        }
    }
}

/// The guest's strength on one key, per mille of their strongest key (0..=1000). The server's own
/// fold first; a guest with none yet is read from the device's last vector.
pub fn share(p: &Profile, key: &str) -> i64 {
    let own = snapshot(&p.sense, usize::MAX);
    if !own.is_empty() {
        return own.get(key).copied().unwrap_or(0);
    }
    p.device.as_ref().map(|d| snapshot(&d.sense, usize::MAX).get(key).copied().unwrap_or(0)).unwrap_or(0)
}

/// "smoky + crispy": the two strongest texture or aroma keys (taste when there are none), as ids.
pub fn because(p: &Profile) -> Vec<String> {
    let top = snapshot(&p.sense, usize::MAX);
    let mut v: Vec<(&String, &i64)> = top.iter().filter(|(k, _)| !k.starts_with("t:")).collect();
    if v.is_empty() {
        v = top.iter().collect();
    }
    v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    v.into_iter().take(2).map(|(k, _)| k.clone()).collect()
}

/// The guest's most frequent band, by `when` ("evening"), if any order was filed with one.
pub fn main_band(p: &Profile) -> Option<String> {
    let mut by: BTreeMap<String, i64> = BTreeMap::new();
    for (k, n) in &p.when {
        if let Some((_, band)) = k.split_once(':') {
            *by.entry(band.to_string()).or_default() += n;
        }
    }
    by.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0))).map(|(b, _)| b)
}
