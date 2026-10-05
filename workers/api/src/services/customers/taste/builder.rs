//! PURE. THE OWNER'S SEGMENT BUILDER over the guests' taste profiles (W-SENSE row 7).
//!
//! "Loves smoky, not seen 4+ days, orders in the evening": one key of the closed sensory
//! vocabulary at a strength, crossed with recency, frequency, the guest's usual band and a
//! weather they ordered in. Answers a COUNT, a six-month TREND from the monthly snapshots and
//! WHERE IN THE WEEK the segment orders -- never a list of people (the owner reaches people only
//! through a campaign, which needs each person's own marketing consent on its channel).
//!
//! CLOSED: the key is a vocabulary key (`dowiz_hub::sense::key_ok`), so no allergen and no health
//! word can be targeted; the bands and weathers are the context's own words. A profile the guest
//! objected to does not exist (it was deleted), so it is never counted.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::senses::{self, month_of};
use super::{expired, Profile};

/// The default strength: the key is at least half of the guest's strongest.
pub const MIN_DEFAULT: i64 = 500;
pub const TREND_MONTHS: usize = 6;
use crate::services::venue::context::{BANDS, WEATHERS};

fn min_default() -> i64 {
    MIN_DEFAULT
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Filter {
    /// `t:spicy`, `x:crispy`, `a:smoky`.
    pub key: String,
    /// Per mille of the guest's strongest key, 1..=1000.
    #[serde(default = "min_default")]
    pub min: i64,
    /// The last order is at least this many days old, 1..=365.
    #[serde(default)]
    pub not_seen_days: Option<i64>,
    /// At least this many orders, 1..=1000.
    #[serde(default)]
    pub min_orders: Option<i64>,
    /// The band the guest orders in most (`venue::context::BANDS`).
    #[serde(default)]
    pub band: Option<String>,
    /// A weather the guest ordered in at least once (`venue::context::WEATHERS`).
    #[serde(default)]
    pub weather: Option<String>,
}

impl Filter {
    pub fn check(&self) -> Result<(), String> {
        if !dowiz_hub::sense::key_ok(&self.key) {
            return Err(format!("{:?} is not a taste, texture or aroma of the vocabulary", self.key));
        }
        if !(1..=1000).contains(&self.min) {
            return Err("min is 1 to 1000 (per mille of the guest's strongest)".into());
        }
        if self.not_seen_days.is_some_and(|d| !(1..=365).contains(&d)) {
            return Err("not_seen_days is 1 to 365".into());
        }
        if self.min_orders.is_some_and(|n| !(1..=1000).contains(&n)) {
            return Err("min_orders is 1 to 1000".into());
        }
        if self.band.as_deref().is_some_and(|b| !BANDS.contains(&b)) {
            return Err(format!("band is one of {BANDS:?}"));
        }
        if self.weather.as_deref().is_some_and(|w| !WEATHERS.contains(&w)) {
            return Err(format!("weather is one of {WEATHERS:?}"));
        }
        Ok(())
    }
}

/// Does this guest's profile fall in the segment today?
pub fn matches(p: &Profile, f: &Filter, today: i64) -> bool {
    if expired(p, today) || senses::share(p, &f.key) < f.min {
        return false;
    }
    if f.not_seen_days.is_some_and(|d| today - p.last_day < d) {
        return false;
    }
    if f.min_orders.is_some_and(|n| p.orders < n) {
        return false;
    }
    if f.band.as_deref().is_some_and(|b| senses::main_band(p).as_deref() != Some(b)) {
        return false;
    }
    if let Some(w) = &f.weather {
        if !p.ctx.contains_key(&format!("wx:{w}")) {
            return false;
        }
    }
    true
}

/// The month before "YYYY-MM".
fn prev_month(m: &str) -> String {
    let (y, mo) = m.split_once('-').and_then(|(y, mo)| Some((y.parse::<i64>().ok()?, mo.parse::<i64>().ok()?))).unwrap_or((1970, 1));
    if mo <= 1 { format!("{:04}-12", y - 1) } else { format!("{y:04}-{:02}", mo - 1) }
}

/// The count, the trend and where in the week the segment orders.
pub fn report(profiles: &[Profile], f: &Filter, today: i64) -> Value {
    let live: Vec<&Profile> = profiles.iter().filter(|p| !expired(p, today)).collect();
    let hit: Vec<&Profile> = live.iter().copied().filter(|p| matches(p, f, today)).collect();
    let mut months = vec![month_of(today)];
    while months.len() < TREND_MONTHS {
        let m = prev_month(months.last().map(String::as_str).unwrap_or(""));
        months.push(m);
    }
    months.reverse();
    let trend: Vec<Value> = months
        .iter()
        .map(|m| {
            let n = live.iter().filter(|p| p.months.get(m).and_then(|s| s.get(&f.key)).is_some_and(|w| *w >= f.min)).count();
            json!({ "month": m, "count": n })
        })
        .collect();
    let mut when: BTreeMap<String, i64> = BTreeMap::new();
    for p in &hit {
        for (slot, n) in &p.when {
            *when.entry(slot.clone()).or_default() += n;
        }
    }
    let mut slots: Vec<(String, i64)> = when.into_iter().collect();
    slots.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    json!({
        "contract": "customers.taste-builder.v1",
        "filter": f,
        "count": hit.len(),
        "of": live.len(),
        "trend": trend,
        "when": slots.iter().take(3).map(|(s, n)| json!({ "slot": s, "orders": n })).collect::<Vec<_>>(),
        "busiest": slots.first().map(|(s, _)| s.clone()),
    })
}
