//! THE WEEK'S PURCHASE LIST (operator 2026-11-23): what to buy for the coming
//! week from the AVERAGE WEEKLY use of the last four whole weeks (Monday to
//! Sunday, the venue's days), PURE.
//!
//!   weekly = used x 7 / days the ledger drew on, rounded UP
//!   need   = used x (7 + lead days) / days, rounded UP
//!   to buy = need - free, rounded UP to the supply's first pack, never < 0
//!
//! THE DAYS: a week before the venue's first sale draw is not a week of zero
//! use, so the average stands on the days from the first draw the log shows
//! (`dataFrom`, `dataDays`). Use is what left the shelf, by the order list's
//! one rule (`order_list::drawn`): orders, till sales (eBills draw through
//! the same ledger), batches, prep and write-offs.
//!
//! WHAT IS NOT KNOWN STAYS UNKNOWN: a stock nobody counted, or one below zero
//! (deliveries not recorded), is not subtracted -- the row says so
//! (`stockKnown: false`, `toBuy: null`) and the need stands; a row with no
//! price has `cost: null`, never 0. Sales of dishes with no recipe draw
//! nothing, so the list is a FLOOR: `coverage` says what share of revenue
//! that is and names the top dishes. Revenue keys are the ones
//! `access::numbers_for_kitchen` strips for staff; quantities and cost stay.

use super::super::sales::{self, Dish, Sales, Supply, Window};
use crate::services::operations::stock::order_list::{self, DEFAULT_LEAD_DAYS};
use crate::services::operations::stock::suppliers::{self, Card, LEAD_MAX};
use dowiz_hub::stock::journal::Journal;
use dowiz_hub::stock::meta::{day_number, day_of_local_ms, day_of_number, show_day};
use dowiz_hub::stock::StockEvent;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

/// The whole weeks the average is taken over.
pub const WEEKS: i64 = 4;
/// How many dishes without a recipe are named.
pub const TOP: usize = 10;

/// How one supply is bought: the lead of its supplier's card, its first
/// pack, and the supplier as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buy {
    pub lead_days: i64,
    pub pack: Option<i64>,
    pub supplier: String,
}

/// `used x per / days`, rounded up, in i128 and saturated; `None` when
/// nothing was used or there is no day.
fn scaled(used: i64, days: i64, per: i64) -> Option<i64> {
    (used > 0 && days > 0).then(|| {
        let (n, d) = (i128::from(used) * i128::from(per), i128::from(days));
        i64::try_from((n + d - 1) / d).unwrap_or(i64::MAX)
    })
}

/// The average week's use of `used` over `days`.
pub fn weekly(used: i64, days: i64) -> Option<i64> {
    scaled(used, days, 7)
}

/// What a week and the supplier's lead take; a lead outside 0..=LEAD_MAX is clamped.
pub fn need(used: i64, days: i64, lead: i64) -> Option<i64> {
    scaled(used, days, 7 + lead.clamp(0, LEAD_MAX))
}

/// The four whole weeks before the venue's current one, in its days.
pub fn weeks_window(zone: dowiz_hub::tz::Zone, now: i64) -> Result<Window, String> {
    let today = day_number(day_of_local_ms(dowiz_hub::tz::local_ms(zone, now)));
    let monday = today - super::super::super::history::weekday(today) as i64;
    let day = |n: i64| show_day(day_of_number(n));
    super::super::window(zone, now, Some(&day(monday - 7 * WEEKS)), Some(&day(monday - 1)), None)
}

/// Per catalogue supply that is bought (no ПФ, no retired one): its card's
/// lead (its own `supplier`, else whoever delivered it last), its first pack.
pub fn buy_of(cat: &dowiz_hub::catalog::Catalog, cards: &[Card], j: &Journal) -> HashMap<String, Buy> {
    let last = order_list::last_supplier(j);
    cat.supplies()
        .into_iter()
        .filter_map(|(id, text)| {
            let v: Value = serde_json::from_str(&text).ok()?;
            if v["kind"] == dowiz_hub::prep::KIND || v.get("active").and_then(Value::as_bool) == Some(false) {
                return None;
            }
            let pack = v["packs"].get(0).and_then(|p| p["qty"].as_i64()).filter(|q| *q > 0);
            let own = v["supplier"].as_str().unwrap_or("").trim();
            let named = if own.is_empty() { last.get(&id).map_or("", String::as_str) } else { own };
            let card = order_list::card_for(cards, named);
            let buy = Buy {
                lead_days: card.map_or(DEFAULT_LEAD_DAYS, |c| c.lead_days.clamp(0, LEAD_MAX)),
                pack,
                supplier: card.map_or_else(|| named.to_string(), |c| c.name.clone()),
            };
            Some((id, buy))
        })
        .collect()
}

/// The index of the first day the ledger drew a sale on (an order or the
/// till), or `None` when nothing drew before the window's end.
fn first_draw(j: &Journal, w: &Window) -> Option<usize> {
    let at = j
        .entries
        .iter()
        .filter(|e| matches!(e.ev, StockEvent::Consumed { .. } | StockEvent::Served { .. }))
        .filter_map(|e| e.meta.at)
        .filter(|at| *at < w.end)
        .min()?;
    Some(w.bucket(at).unwrap_or(0))
}

/// What share of the sales draws nothing: dishes with no recipe, and
/// products the catalogue no longer has.
fn coverage(s: &Sales, dishes: &HashMap<String, Dish>) -> Value {
    let revenue: i64 = s.days.iter().map(|d| d.revenue).sum();
    let mut top: Vec<(i64, &String, i64)> = s
        .dishes
        .iter()
        .filter(|(id, r)| r.sold > 0 && dishes.get(*id).is_none_or(|d| d.lines.is_empty()))
        .map(|(id, r)| (r.revenue, id, r.sold))
        .collect();
    top.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    let (rev, portions) = top.iter().fold((0i64, 0i64), |(r, p), x| (r + x.0, p + x.2));
    let named: Vec<Value> = top
        .iter()
        .take(TOP)
        .map(|(r, id, sold)| json!({ "id": id, "name": dishes.get(*id).map_or((*id).clone(), |d| d.name.clone()), "revenue": r, "sold": sold }))
        .collect();
    json!({ "revenue": revenue, "noRecipe": { "revenue": rev, "portions": portions, "pm": super::pm(rev, revenue) }, "top": named })
}

/// The block over a four-week window `w`.
pub fn block(s: &Sales, j: &Journal, dishes: &HashMap<String, Dish>, supplies: &HashMap<String, Supply>, buys: &HashMap<String, Buy>, w: &Window) -> Value {
    let n = w.starts.len();
    let start = first_draw(j, w);
    let days = start.map_or(0, |i| (n - i) as i64);
    let mut used: BTreeMap<&str, i64> = BTreeMap::new();
    if let Some(i) = start {
        for e in &j.entries {
            if e.meta.at.is_some_and(|at| at >= w.starts[i] && at < w.end) {
                let u = used.entry(e.ev.item()).or_insert(0);
                *u = u.saturating_add(order_list::drawn(&e.ev));
            }
        }
    }
    let (mut cost, mut priced, mut unpriced) = (0i64, 0i64, 0i64);
    let mut rows: Vec<(String, Value)> = Vec::new();
    for (id, u) in used {
        // Not bought (a ПФ, a retired supply) or nothing net used: not on the list.
        let Some(b) = buys.get(id) else { continue };
        let Some(nd) = need(u, days, b.lead_days) else { continue };
        let s = supplies.get(id);
        let available = j.ledger.available(id);
        let counted = j.ledger.is_counted(id);
        let known = counted && available >= 0;
        let to_buy = known.then(|| order_list::suggest(nd, available, 0, b.pack));
        // What the money is for: the list, else (stock unknown) the need in packs.
        let qty = to_buy.unwrap_or_else(|| order_list::suggest(nd, 0, 0, b.pack));
        let c = s.and_then(|s| sales::cost_of(s, &j.book, qty));
        if qty > 0 {
            match c {
                Some(v) => (cost, priced) = (cost.saturating_add(v), priced + 1),
                None => unpriced += 1,
            }
        }
        let name = s.map_or(id.to_string(), |s| s.name.clone());
        rows.push((name.clone(), json!({
            "id": id, "name": name, "unit": s.map(|s| s.unit.clone()), "supplier": b.supplier, "pack": b.pack,
            "used": u, "weekly": weekly(u, days), "need": nd, "leadDays": b.lead_days,
            "available": available, "counted": counted, "stockKnown": known, "toBuy": to_buy,
            "cost": c, "priced": c.is_some(),
        })));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1["id"].as_str().cmp(&b.1["id"].as_str())));
    json!({
        "from": show_day(w.days[0]), "to": show_day(w.days[n - 1]),
        "dataFrom": start.map(|i| show_day(w.days[i])), "dataDays": days,
        "rows": rows.into_iter().map(|(_, r)| r).collect::<Vec<_>>(),
        "totals": { "cost": (priced > 0).then_some(cost), "priced": priced, "unpriced": unpriced },
        "coverage": coverage(s, dishes),
    })
}

/// The whole answer over the venue's orders (already its own), catalogue and
/// stock log, and the cube's rows for the days the hot log no longer holds:
/// a cube that cannot be read is said (`coverage.historyError`).
pub fn answer(orders: &[Value], cat: &dowiz_hub::catalog::Catalog, stock: &dowiz_hub::stock::StockLog, now: i64, cold: super::super::ColdRows) -> Value {
    let zone = crate::hubstore::zone_of(cat.location().and_then(|j| serde_json::from_str::<Value>(&j).ok()).as_ref());
    let w = match weeks_window(zone, now) {
        Ok(w) => w,
        Err(why) => return json!({ "error": why }),
    };
    let j = match stock.journal_since(w.starts[0]) {
        Ok(j) => j,
        Err(e) => return json!({ "error": e.to_string() }),
    };
    let (dishes, supplies) = (super::super::dishes_of(cat), super::super::supplies_of(cat));
    let (rows, unread) = match cold(w.days[0], w.days[w.days.len() - 1]) {
        Ok(r) => (r, None),
        Err(why) => (BTreeMap::new(), Some(why)),
    };
    let s = sales::fold_with(orders, &rows, &dishes, &w);
    let buys = buy_of(cat, &suppliers::cards(stock), &j);
    let mut out = block(&s, &j, &dishes, &supplies, &buys, &w);
    out["coverage"]["historyError"] = json!(unread);
    out
}

#[cfg(test)]
#[path = "week/tests.rs"]
mod tests;
