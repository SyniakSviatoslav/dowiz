//! THE ORDER LIST (W-STOCK P5), pure: par levels from the average daily use,
//! what is already on its way, rounded up to the packs a supplier sells, and
//! grouped by supplier -- the list an owner sends from the phone.
//!
//!   par      = average daily use x (lead days + days until the next delivery)
//!   suggest  = par - free on the shelf - already ordered, rounded UP to a pack
//!
//! THE AVERAGE DAILY USE stands in until a forecast exists (research
//! 2026-10-03 row P6): what left the shelf over the last [`ADU_DAYS`] days --
//! orders' draws, till sales net of voids, batches cooked, write-offs and prep
//! moved into another supply -- divided by those days (fewer when the supply is
//! newer), rounded UP: a kitchen runs out, it does not run over.
//!
//! An ingredient nobody counted has no known shelf, so it gets no suggestion
//! ("count it first"), never a guess. Quantities are base units; nothing here
//! is money.

use std::collections::BTreeMap;

use dowiz_hub::stock::journal::Journal;
use dowiz_hub::stock::{moved_into, StockEvent};
use serde_json::{json, Value};

use super::suppliers::{Card, OnOrder};

/// The days the average daily use is taken over.
pub const ADU_DAYS: i64 = 14;
/// Days between deliveries for a supplier whose card names no delivery days.
pub const CYCLE_DEFAULT_DAYS: i64 = 7;
/// Lead time for a supply with no supplier card.
pub const DEFAULT_LEAD_DAYS: i64 = 1;
const DAY_MS: i64 = 86_400_000;

/// Rounded-up daily use of `used` over `days`; `None` when nothing was used.
pub fn adu(used: i64, days: i64) -> Option<i64> {
    (used > 0 && days > 0).then(|| (used + days - 1) / days)
}

/// Days between deliveries for a card's weekdays (1 = Monday .. 7 = Sunday):
/// seven divided by how many, rounded up. None named: [`CYCLE_DEFAULT_DAYS`].
pub fn cycle_days(days: &[u8]) -> i64 {
    match days.len() as i64 {
        0 => CYCLE_DEFAULT_DAYS,
        n => (7 + n - 1) / n,
    }
}

/// What the shelf should hold when an order is placed.
pub fn par(adu: i64, lead_days: i64, cycle: i64) -> i64 {
    adu.max(0).saturating_mul(lead_days.max(0) + cycle.max(1))
}

/// What to order: up to `par`, less what is free and what is on its way,
/// rounded UP to whole packs of `pack` (none: the exact amount). 0 = nothing.
pub fn suggest(par: i64, available: i64, on_order: i64, pack: Option<i64>) -> i64 {
    let need = par - available.max(0) - on_order.max(0);
    if need <= 0 {
        return 0;
    }
    match pack.filter(|p| *p > 0) {
        Some(p) => (need + p - 1) / p * p,
        None => need,
    }
}

/// Per supply, what left the shelf since `since` and the first instant the
/// supply appears at all (a newer supply is averaged over its own days).
pub fn used_since(j: &Journal, since: i64) -> BTreeMap<String, (i64, i64)> {
    let mut out: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    for e in &j.entries {
        let Some(at) = e.meta.at else { continue };
        let q = match &e.ev {
            StockEvent::Consumed { qty, .. } | StockEvent::Served { qty, .. } | StockEvent::Cooked { qty, .. } | StockEvent::Wasted { qty, .. } => *qty,
            StockEvent::Unserved { qty, .. } => -qty,
            StockEvent::Produced { item, qty, into, .. } if moved_into(item, into).is_some() => *qty,
            _ => 0,
        };
        let row = out.entry(e.ev.item().to_string()).or_insert((0, at));
        row.1 = row.1.min(at);
        if at >= since {
            row.0 += q;
        }
    }
    out
}

/// One supply as the list needs it (from the Stock screen's row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub unit: String,
    /// The supply's `supplier` text, as typed.
    pub supplier: String,
    pub available: i64,
    pub counted: bool,
    /// The first pack it is bought in: (name, base units).
    pub pack: Option<(String, i64)>,
}

/// The card a supply's `supplier` text names: by id, else by name, any case.
pub fn card_for<'a>(cards: &'a [Card], supplier: &str) -> Option<&'a Card> {
    let s = supplier.trim();
    if s.is_empty() {
        return None;
    }
    let slug = dowiz_hub::import::slug(s);
    cards.iter().find(|c| c.id == slug || c.id == s || c.name.trim().eq_ignore_ascii_case(s))
}

/// The whole list, grouped by supplier (cards in their order, then the
/// supplies with no card). `now` is the request clock.
pub fn list(items: &[Item], j: &Journal, cards: &[Card], on_order: &OnOrder, now: i64) -> Value {
    let since = now - ADU_DAYS * DAY_MS;
    let used = used_since(j, since);
    let mut groups: Vec<(Option<&Card>, Vec<Value>)> = cards.iter().map(|c| (Some(c), Vec::new())).collect();
    let mut loose: Vec<Value> = Vec::new();
    for it in items {
        let (qty, first) = used.get(&it.id).copied().unwrap_or((0, now));
        let days = ((now - first.max(since)) / DAY_MS + 1).clamp(1, ADU_DAYS);
        let a = adu(qty, days);
        let ordered = on_order.get(&it.id).map_or(0, |o| o.qty);
        if a.is_none() && ordered == 0 {
            continue;
        }
        let card = card_for(cards, &it.supplier);
        let (lead, cycle) = card.map_or((DEFAULT_LEAD_DAYS, CYCLE_DEFAULT_DAYS), |c| (c.lead_days, cycle_days(&c.days)));
        let p = par(a.unwrap_or(0), lead, cycle);
        let pack = it.pack.as_ref().map(|(_, q)| *q);
        let row = json!({
            "id": it.id, "name": it.name, "unit": it.unit, "supplierText": it.supplier,
            "available": it.available, "counted": it.counted, "adu": a, "par": p,
            "onOrder": ordered, "orderedAt": on_order.get(&it.id).and_then(|o| o.since),
            "pack": it.pack.as_ref().map(|(n, q)| json!({ "name": n, "qty": q })),
            "leadDays": lead, "cycleDays": cycle,
            // Uncounted: its shelf is unknown, so nothing is suggested.
            "suggest": it.counted.then(|| suggest(p, it.available, ordered, pack)),
        });
        match card.and_then(|c| groups.iter().position(|(g, _)| g.is_some_and(|g| g.id == c.id))) {
            Some(i) => groups[i].1.push(row),
            None => loose.push(row),
        }
    }
    let mut out: Vec<Value> = groups.into_iter().map(|(c, lines)| json!({ "supplier": c, "lines": lines })).collect();
    if !loose.is_empty() {
        out.push(json!({ "supplier": Value::Null, "lines": loose }));
    }
    json!({ "aduDays": ADU_DAYS, "groups": out })
}

#[cfg(test)]
#[path = "order_list/tests.rs"]
mod tests;
