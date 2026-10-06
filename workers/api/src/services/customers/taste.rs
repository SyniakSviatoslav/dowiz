//! PURE. THE GUEST'S TASTE, SCORED BY THE VENUE, UNLESS THEY OBJECTED (W-MR0 row MR8).
//!
//! OPERATOR 2026-10-04, verbatim: "смак і поведінку гостя треба оцінювати ... на пристрої і сервері".
//! DECISIONS.md D0 is amended for exactly this, and `tools/gates/no-scoring.sh` names THIS file as
//! the one server module where a guest's taste may be computed. Its rules:
//!   * THE OBJECTION FIRST (operator ruling 2026-10-04: automatic, legitimate interest, no box).
//!     Nothing here runs for a guest who objected (`dowiz_hub::consent::objected`, one tap on the
//!     storefront or the order page); `taste_routes.rs` asks the log before it calls anything below.
//!   * INPUTS: the dishes of the guest's own orders at this venue (tags and category, decayed with a
//!     60-day half-life, the device's rule) and, when the phone sent it, the device's aggregated
//!     vector (`SyncIn`: integer weights per tag and category, nothing else).
//!   * OUTPUT: a taste vector, a recency/frequency summary, the device's last vector, and a SEGMENT
//!     among four with the rule that put the guest there. Shown to the owner and to the guest.
//!   * NEVER MONEY. Nothing here is read by an order's total, a code, a fee or a decision to serve:
//!     personalised pricing would have to be disclosed (EU Omnibus 2019/2161), and a decision from a
//!     score is Art. 22 GDPR. The gate refuses this file the day it names one.
//!   * RETENTION: 12 months after the guest's last order (GUESS, 2026-10-04): an older profile reads
//!     as absent and is removed on the next write (`expired`). No nightly sweep yet.
//! The record lives in the venue's `taste` image, kind `KIND`, under the guest's `customer_key`,
//! in its own image `IMAGE_TASTE`; the erasure (`hubdo/forget.rs` step 1b) removes it with the card, and an
//! objection removes it alone.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The venue's taste image: its OWN image, not the `people` one, because it is its own store with
/// its own purpose, basis and retention (privacy registry: one row per image).
pub const IMAGE_TASTE: &str = "taste";
pub const TASTE_BYTES: usize = dowiz_hub::CEILING_BYTES;
/// The record kind in it.
pub const KIND: &str = "taste";
pub const VERSION: u8 = 1;
/// A signal counts half after this many days (the device's half-life, store/taste.js).
pub const HALF_LIFE_DAYS: i64 = dowiz_hub::rank::HALF_LIFE_DAYS;
/// One portion, as an integer weight.
pub const UNIT: i64 = 1000;
/// Tags and categories kept per profile, strongest first; the rest fade out.
pub const KEEP: usize = 24;
/// What the device may send: at most this many keys per map, each 0..=SYNC_SCALE.
pub const SYNC_MAX_KEYS: usize = 12;
pub const SYNC_SCALE: i64 = 1000;
pub const SYNC_KEY_MAX_CHARS: usize = 32;
/// Days without an order before a guest reads as at risk, then lapsed.
pub const AT_RISK_DAYS: i64 = 30;
pub const LAPSED_DAYS: i64 = 60;
/// Days after the last order the profile is kept (GUESS 2026-10-04, see the module header).
pub const KEEP_DAYS: i64 = 365;
pub const KEEP_MS: i64 = KEEP_DAYS * 86_400_000;

/// The device's vector, as the order body carries it (`taste_sync`). CLOSED and small.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct SyncIn {
    pub v: u8,
    #[serde(default)]
    pub tags: BTreeMap<String, i64>,
    #[serde(default)]
    pub cats: BTreeMap<String, i64>,
    /// W-SENSE: the device's taste/texture/aroma vector, keys of the closed vocabulary only.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sense: BTreeMap<String, i64>,
}

impl SyncIn {
    /// The shape `store/taste.js syncVector` makes, or the reason it is not.
    pub fn validate(&self) -> Result<(), String> {
        if self.v != VERSION {
            return Err(format!("taste_sync version {} is not {VERSION}", self.v));
        }
        for (name, m) in [("tags", &self.tags), ("cats", &self.cats)] {
            if m.len() > SYNC_MAX_KEYS {
                return Err(format!("taste_sync.{name} holds {} keys; at most {SYNC_MAX_KEYS}", m.len()));
            }
            for (k, w) in m {
                if k.is_empty() || k.chars().count() > SYNC_KEY_MAX_CHARS {
                    return Err(format!("taste_sync.{name}: a key of 1..={SYNC_KEY_MAX_CHARS} characters"));
                }
                if !(0..=SYNC_SCALE).contains(w) {
                    return Err(format!("taste_sync.{name}.{k} = {w}; 0..={SYNC_SCALE}"));
                }
            }
        }
        for (k, w) in &self.sense {
            if !dowiz_hub::sense::key_ok(k) {
                return Err(format!("taste_sync.sense: {k:?} is not a key of the vocabulary"));
            }
            if !(0..=SYNC_SCALE).contains(w) {
                return Err(format!("taste_sync.sense.{k} = {w}; 0..={SYNC_SCALE}"));
            }
        }
        Ok(())
    }
}

/// One guest's profile at this venue.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Profile {
    pub v: u8,
    /// Day numbers (UTC days since 1970-01-01: a day, not an instant, is all that is kept).
    pub first_day: i64,
    pub last_day: i64,
    pub orders: i64,
    pub portions: i64,
    /// Decayed weights as of `last_day`, `UNIT` per portion.
    pub tags: BTreeMap<String, i64>,
    pub cats: BTreeMap<String, i64>,
    /// The device's last vector and the day it came, when the guest sent one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<SyncIn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_day: Option<i64>,
    /// W-SENSE (`taste/senses.rs`): the taste/texture/aroma weights, the same per context, the
    /// weekday x band counts and the monthly snapshots. Absent in a record written before.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sense: BTreeMap<String, i64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ctx: BTreeMap<String, BTreeMap<String, i64>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub when: BTreeMap<String, i64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub months: BTreeMap<String, BTreeMap<String, i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub snn_held: Option<dowiz_hub::snn::quality::Held>, // W-SNN: the two top-3s, until the next order
}

/// One line of an order, as this module needs it: the dish's tags, its category, how many, and
/// its declared sensory vector (`dowiz_hub::sense::vector`; empty when the dish declares none).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Line {
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub qty: i64,
    pub sense: BTreeMap<String, i64>,
    pub id: String, // W-SNN: the product id (`line_of`), for the quality check at the next order
}

/// W-TASTE: the ONE integer half-life the phone uses too (`dowiz_hub::rank::fade`, a Q16 table;
/// it was `f64::powf().round()`, which no second machine was bound to reproduce).
fn fade(w: i64, days: i64) -> i64 {
    dowiz_hub::rank::fade(w, days)
}

/// The strongest `KEEP` entries, faded entries at zero dropped.
fn trim(m: BTreeMap<String, i64>) -> BTreeMap<String, i64> {
    let mut v: Vec<(String, i64)> = m.into_iter().filter(|(_, w)| *w > 0).collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.into_iter().take(KEEP).collect()
}

/// The profile after one order on `day`: the old weights faded to today, this order's dishes added,
/// the device's vector kept when it came with the order. `prev` past `KEEP_DAYS` starts afresh.
#[cfg_attr(not(test), allow(dead_code))] // W-SENSE: the request paths call the `_where`/`_at` form; tests keep this one
pub fn apply_order(prev: Option<Profile>, lines: &[Line], sync: Option<&SyncIn>, day: i64) -> Profile {
    apply_order_at(prev, lines, sync, day, None)
}

/// [`apply_order`] with the context the order was placed in (`senses::At`).
pub fn apply_order_at(prev: Option<Profile>, lines: &[Line], sync: Option<&SyncIn>, day: i64, at: Option<&senses::At>) -> Profile {
    let mut p = match prev.filter(|p| p.v == VERSION && !expired(p, day)) {
        Some(p) => p,
        None => Profile { v: VERSION, first_day: day, last_day: day, ..Profile::default() },
    };
    let gap = day - p.last_day;
    p.tags = p.tags.into_iter().map(|(k, w)| (k, fade(w, gap))).collect();
    p.cats = p.cats.into_iter().map(|(k, w)| (k, fade(w, gap))).collect();
    for l in lines.iter().filter(|l| l.qty > 0) {
        for t in &l.tags {
            *p.tags.entry(t.clone()).or_default() += l.qty * UNIT;
        }
        if let Some(c) = &l.category {
            *p.cats.entry(c.clone()).or_default() += l.qty * UNIT;
        }
        p.portions += l.qty;
    }
    p.tags = trim(p.tags);
    p.cats = trim(p.cats);
    senses::fold(&mut p, lines, gap, day, at);
    p.orders += 1;
    p.last_day = p.last_day.max(day);
    if let Some(s) = sync {
        p.device = Some(s.clone());
        p.device_day = Some(day);
    }
    p
}

/// Past its retention: read as absent, removed on the next write.
pub fn expired(p: &Profile, today: i64) -> bool {
    today - p.last_day > KEEP_DAYS
}

/// Four segments, by recency and frequency only. A SEGMENT IS NOT A TIER: it says when the guest
/// last came, never what they are worth, and nothing reads it to set a sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    New,
    Regular,
    AtRisk,
    Lapsed,
}

impl Segment {
    pub fn as_str(self) -> &'static str {
        match self {
            Segment::New => "new",
            Segment::Regular => "regular",
            Segment::AtRisk => "at_risk",
            Segment::Lapsed => "lapsed",
        }
    }
    pub const ALL: [Segment; 4] = [Segment::New, Segment::Regular, Segment::AtRisk, Segment::Lapsed];
}

/// The segment and the rule that decided it, as words a guest can check against their own orders.
pub fn segment(p: &Profile, today: i64) -> (Segment, String) {
    let since = (today - p.last_day).max(0);
    if since > LAPSED_DAYS {
        return (Segment::Lapsed, format!("last order {since} days ago (more than {LAPSED_DAYS})"));
    }
    if since > AT_RISK_DAYS {
        return (Segment::AtRisk, format!("last order {since} days ago (more than {AT_RISK_DAYS})"));
    }
    if p.orders <= 1 {
        return (Segment::New, format!("one order, {since} days ago"));
    }
    (Segment::Regular, format!("{} orders, the last {since} days ago (at most {AT_RISK_DAYS})", p.orders))
}

/// The strongest `n` of a map, as `[{key, w}]`.
fn top(m: &BTreeMap<String, i64>, n: usize) -> Vec<serde_json::Value> {
    let mut v: Vec<(&String, &i64)> = m.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    v.into_iter().take(n).map(|(k, w)| serde_json::json!({ "key": k, "w": w })).collect()
}

/// What the owner's card and the guest's own page show: everything held, and why the segment.
pub fn view(p: &Profile, today: i64) -> serde_json::Value {
    let (seg, why) = segment(p, today);
    serde_json::json!({
        "contract": "customers.taste.v1",
        "segment": seg.as_str(), "why": why,
        "orders": p.orders, "portions": p.portions, "firstDay": p.first_day, "lastDay": p.last_day,
        "daysSince": (today - p.last_day).max(0),
        "tags": top(&p.tags, 8), "cats": top(&p.cats, 8),
        "device": p.device.as_ref().map(|d| serde_json::json!({ "tags": d.tags, "cats": d.cats, "day": p.device_day })),
        "keptUntilDay": p.last_day + KEEP_DAYS,
        "sense": top(&senses::snapshot(&p.sense, usize::MAX), 12),
        "because": senses::because(p),
        "history": p.months,
        "contexts": p.ctx.iter().map(|(c, m)| (c.clone(), senses::snapshot(m, 6))).collect::<BTreeMap<_, _>>(),
        "when": p.when,
    })
}

/// The venue's counts per segment, over every stored profile (expired ones not counted).
pub fn segment_counts<'a>(profiles: impl Iterator<Item = &'a Profile>, today: i64) -> BTreeMap<&'static str, i64> {
    let mut out: BTreeMap<&'static str, i64> = Segment::ALL.iter().map(|s| (s.as_str(), 0)).collect();
    for p in profiles.filter(|p| !expired(p, today)) {
        *out.entry(segment(p, today).0.as_str()).or_default() += 1;
    }
    out
}

/// PURE. Erasure: remove the profile of every key in the person's circle. Returns how many went.
pub fn forget(t: &mut dowiz_hub::table::Table, keys: &[String]) -> usize {
    keys.iter().filter(|id| t.remove(KIND, id)).count()
}

/// A stored record, read; anything unreadable is absent (it is rebuilt by the next order).
pub fn parse(json: &str) -> Option<Profile> {
    serde_json::from_str::<Profile>(json).ok().filter(|p| p.v == VERSION)
}

/// The dish's tags and category from its catalogue JSON (`taste/line.rs`).
#[path = "taste/line.rs"]
mod line;
pub use line::line_of;

/// W-SENSE: the guest on the dish's axes, per context, per month.
#[path = "taste/senses.rs"]
pub mod senses;
/// W-SENSE row 7: the owner's segment builder over the profiles.
#[path = "taste/builder.rs"]
pub mod builder;
/// W-TASTE2 row 2 ("For you" on the order page) and W-SNN (the sheaf network beside the ranker, in shadow).
#[path = "taste/foryou.rs"]
pub mod foryou;
#[path = "taste/agreement.rs"]
pub mod agreement;
#[path = "taste/snn.rs"]
pub mod snn;

#[cfg(test)]
#[path = "taste/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "taste/senses_tests.rs"]
mod senses_tests;
