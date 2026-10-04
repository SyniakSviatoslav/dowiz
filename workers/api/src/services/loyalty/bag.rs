//! PURE. THE BAG INSERT (W-QR): the link printed in a delivery bag, where an
//! order says it came from, and what the owner's card counts.
//!
//! WHY (operator 2026-10-04): a marketplace brings a venue's first order and
//! keeps the guest (25-30 % commission, the platform owns the contact). The
//! card in the bag is how the SECOND order comes to the venue's own storefront.
//!
//! FIRST-PARTY ONLY. The landing URL carries `?src=bag[&c=<campaign>]`; the
//! storefront keeps it in sessionStorage until checkout (`store/bag.js`) and
//! the order records it as `referral`. No cookie, no fingerprint, no scan
//! counter: nothing is written until a guest places an order, so "scans" are
//! NOT counted -- the card counts orders, and says so.
//!
//! AGGREGATES ONLY. The card is three numbers and a per-campaign split; no
//! guest is listed, ranked or scored (`tools/gates/no-scoring.sh`).

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::hubdo::OrderView;
use crate::services::orders::status::took_money;

/// The one source word this build knows. A closed set, like `channel`.
pub const BAG: &str = "bag";
/// The envelope key the landing is recorded under.
pub const REFERRAL: &str = "referral";
/// A campaign is a short word the owner types: `spring`, `wolt-oct`.
pub const CAMPAIGN_MAX: usize = 24;

/// The `src` member of the order body: present only when the guest landed
/// from a bag card in this browser session. STRICT: an unknown field is a 400.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SrcIn {
    pub src: String,
    #[serde(default)]
    pub c: Option<String>,
}

/// A campaign as it may be printed and stored: lowercase letters, digits and
/// `-`, at most `CAMPAIGN_MAX`. `None` for empty; `Err` for anything else.
pub fn campaign(raw: Option<&str>) -> Result<Option<String>, &'static str> {
    let Some(c) = raw.map(str::trim).filter(|c| !c.is_empty()) else { return Ok(None) };
    let ok = c.len() <= CAMPAIGN_MAX && c.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if ok { Ok(Some(c.to_string())) } else { Err("bag_campaign: up to 24 of a-z, 0-9 and -") }
}

/// The body's `src`, checked: `Ok(None)` when absent, the campaign when it is
/// a bag landing, a refusal for a word this build does not know.
pub fn source(s: Option<&SrcIn>) -> Result<Option<Option<String>>, String> {
    let Some(s) = s else { return Ok(None) };
    if s.src != BAG {
        return Err(format!("unknown src: {}", s.src));
    }
    campaign(s.c.as_deref()).map(Some).map_err(str::to_string)
}

/// THE LINK IN THE QR: exactly the venue's own host, the root path, and the
/// two parameters. Nothing else -- no id, no token, nothing per guest.
pub fn landing_url(host: &str, c: Option<&str>) -> String {
    match c {
        Some(c) => format!("https://{host}/?src={BAG}&c={c}"),
        None => format!("https://{host}/?src={BAG}"),
    }
}

/// Record the landing on the envelope (guests only; a waiter's round has none).
pub fn stamp(envelope: &mut Value, c: Option<&str>) {
    envelope[REFERRAL] = json!({ "src": BAG, "c": c });
}

fn s<'a>(o: &'a Value, k: &str) -> &'a str {
    o.get(k).and_then(Value::as_str).unwrap_or("")
}

fn is_bag(o: &Value) -> bool {
    o.pointer("/referral/src").and_then(Value::as_str) == Some(BAG)
}

/// The owner's card. Integer money in the venue's minor units.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    /// Orders that arrived from a bag card and took money.
    pub orders: i64,
    /// Distinct phones among them (a bag order with no phone is counted above, not here).
    pub guests: i64,
    /// Later first-party orders by those guests, from any landing.
    pub repeat: i64,
    /// What those guests' first-party orders (the bag ones and the later ones) took.
    pub direct_total: i64,
    /// `direct_total` x the percent the OWNER typed; `None` when they typed none.
    pub saved: Option<i64>,
    /// Bag orders per campaign (`""` = printed without one).
    pub by_campaign: BTreeMap<String, i64>,
}

/// THE FOLD. `key_of` turns a typed phone into the venue's customer key (one
/// person, spelled two ways, is one guest); `pct` is the owner's commission.
pub fn stats(listed: &[OrderView], location_id: &str, key_of: impl Fn(&str) -> String, pct: Option<i64>) -> Stats {
    let mut out = Stats::default();
    let mut orders: Vec<(i64, Option<String>, bool, i64)> = Vec::new();
    for v in listed {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        let first_party = crate::services::ordering::channel::of(&o)
            .ok()
            .and_then(crate::services::ordering::channel::profile)
            .is_some_and(|p| p.first_party);
        if s(&o, "location_id") != location_id || !took_money(s(&o, "status")) || !first_party {
            continue;
        }
        let key = crate::services::loyalty::stamps::phone_of(&o).map(&key_of);
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        let total = o.get("total").and_then(Value::as_i64).unwrap_or(0);
        if is_bag(&o) {
            out.orders += 1;
            let c = o.pointer("/referral/c").and_then(Value::as_str).unwrap_or("").to_string();
            *out.by_campaign.entry(c).or_insert(0) += 1;
        }
        orders.push((at, key, is_bag(&o), total));
    }
    orders.sort_by_key(|x| x.0);
    let mut first: BTreeMap<String, i64> = BTreeMap::new();
    for (at, key, bag, _) in &orders {
        if let (Some(k), true) = (key, bag) {
            first.entry(k.clone()).or_insert(*at);
        }
    }
    let mut seen = BTreeSet::new();
    for (at, key, _, total) in &orders {
        let Some(k) = key else { continue };
        let Some(since) = first.get(k) else { continue };
        if at < since {
            continue;
        }
        out.direct_total += total;
        if !seen.insert(k.clone()) {
            out.repeat += 1;
        }
    }
    out.guests = first.len() as i64;
    out.saved = pct.map(|p| (i128::from(out.direct_total) * i128::from(p) / 100) as i64);
    out
}

impl Stats {
    pub fn to_json(&self) -> Value {
        json!({
            "orders": self.orders, "guests": self.guests, "repeat": self.repeat,
            "direct_total": self.direct_total, "saved": self.saved,
            "by_campaign": self.by_campaign.iter().map(|(c, n)| json!({ "c": c, "orders": n })).collect::<Vec<_>>(),
            "scans": Value::Null,
        })
    }
}

#[cfg(test)]
mod tests;
