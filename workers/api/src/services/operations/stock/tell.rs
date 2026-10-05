//! WHAT A STOCK MOVEMENT TELLS THE GROUPS (W0a), pure: the plan that was
//! applied, the shelf before and after, the lots after, in; the events and
//! their data (`notify::route::render::event`'s shapes) out.
//!
//!   stock.received     {name, qty, unit, lot?, expiry?}
//!   stock.wasted       {name, qty, unit, reason}
//!   stocktake.variance {items: [{name, expected, observed, unit}]}  (drift only)
//!   stock.low          {items: [{name, on_hand, low_at, unit}]}      (crossings)
//!   stock.expiring     {items: [{item, name, qty, unit, expiry, surplus?}],
//!                       forecast?}                                   (once a day)
//!
//! `telegram.stock_expiring.v2` (W-PREP, P7): each row names its `item`, and
//! `surplus` is what the kitchen's forecast will NOT use before the date
//! ("won't be used in time: N g -> use first / special"); `forecast` says why
//! there is no surplus at all (still learning, or the history unreadable).
//!
//! None of them names a customer.

use dowiz_hub::stock::lots::Lot;
use dowiz_hub::stock::meta::{day_number, show_day};
use dowiz_hub::stock::StockEvent;
use serde_json::{json, Value};

use super::moves::Plan;
use super::view::EXPIRY_WARN_DAYS;
use crate::notify::route::produce::{self, Shelf, Supply};

fn named(supply: &dyn Fn(&str) -> Option<Supply>, id: &str) -> (String, String) {
    supply(id).map_or_else(|| (id.to_string(), String::new()), |s| (s.name, s.unit))
}

/// The events of one applied plan. `shown` is `Plan::apply`'s answer (the
/// counted lines carry `expected`). `expiring` is `Some(lots)` on the first
/// movement of the venue's day, else `None`.
pub fn events(
    plan: &Plan,
    shown: &Value,
    before: &dyn Fn(&str) -> Shelf,
    after: &dyn Fn(&str) -> Shelf,
    supply: &dyn Fn(&str) -> Option<Supply>,
    expiring: Option<(&[&Lot], i64)>,
) -> Vec<(&'static str, Value)> {
    let mut out: Vec<(&'static str, Value)> = Vec::new();
    for (ev, meta) in &plan.lines {
        match ev {
            StockEvent::Received { item, qty } => {
                let (name, unit) = named(supply, item);
                let mut d = json!({ "name": name, "qty": qty, "unit": unit });
                if let Some(l) = &meta.lot {
                    d["lot"] = json!(l);
                }
                if let Some(e) = meta.expiry {
                    d["expiry"] = json!(show_day(e));
                }
                out.push(("stock.received", d));
            }
            StockEvent::Wasted { item, qty, reason, .. } => {
                let (name, unit) = named(supply, item);
                out.push(("stock.wasted", json!({ "name": name, "qty": qty, "unit": unit, "reason": reason.as_str() })));
            }
            _ => {}
        }
    }
    let drift: Vec<Value> = shown
        .get("lines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|l| l.get("drift").and_then(Value::as_i64).is_some_and(|d| d != 0))
        .map(|l| {
            let (name, unit) = named(supply, l["item"].as_str().unwrap_or(""));
            json!({ "name": name, "expected": l["expected"], "observed": l["observed"], "unit": unit })
        })
        .collect();
    if !drift.is_empty() {
        out.push(("stocktake.variance", json!({ "items": drift })));
    }
    if let Some(d) = produce::low(&plan.items(), before, after, supply) {
        out.push(("stock.low", d));
    }
    if let Some((lots, today)) = expiring {
        if let Some(d) = soon(lots, today, supply) {
            out.push(("stock.expiring", d));
        }
    }
    out
}

/// The event's name.
pub const EXPIRING: &str = "stock.expiring";

/// The lots still on the shelf whose date is within the warning window
/// (or past it), soonest first. `None` when there are none.
pub fn soon(lots: &[&Lot], today: i64, supply: &dyn Fn(&str) -> Option<Supply>) -> Option<Value> {
    soon_with(lots, today, supply, None)
}

/// `soon`; with `surplus` (P7, v2) each row also names its `item` and, when
/// there is more than nothing, its `surplus`. Without it the rows are v1's.
pub fn soon_with(lots: &[&Lot], today: i64, supply: &dyn Fn(&str) -> Option<Supply>, surplus: Option<&dyn Fn(&Lot) -> Option<i64>>) -> Option<Value> {
    let mut near: Vec<&&Lot> =
        lots.iter().filter(|l| l.left > 0 && l.expiry.is_some_and(|e| day_number(e) - day_number(today) <= EXPIRY_WARN_DAYS)).collect();
    near.sort_by_key(|l| l.expiry);
    let rows: Vec<Value> = near
        .iter()
        .map(|l| {
            let (name, unit) = named(supply, &l.item);
            let mut row = json!({ "name": name, "qty": l.left, "unit": unit, "expiry": l.expiry.map(show_day) });
            if let Some(f) = surplus {
                row["item"] = json!(l.item);
                if let Some(n) = f(l).filter(|n| *n > 0) {
                    row["surplus"] = json!(n);
                }
            }
            row
        })
        .collect();
    (!rows.is_empty()).then(|| json!({ "items": rows }))
}

/// P7, per lot: what is left of it after the forecast's use from today
/// through its date, the item's lots taken first-expiry-first (an earlier
/// lot absorbs the use before a later one). `None`: no date, already past,
/// or an item the forecast says nothing about. Aligned with `lots`.
pub fn surpluses(lots: &[&Lot], today: i64, use_until: &dyn Fn(&str, i64) -> Option<i64>) -> Vec<Option<i64>> {
    let mut order: Vec<usize> = (0..lots.len()).collect();
    order.sort_by_key(|i| (lots[*i].item.as_str(), lots[*i].expiry.is_none(), lots[*i].expiry.unwrap_or(0), lots[*i].seq));
    let mut given: std::collections::BTreeMap<&str, i64> = std::collections::BTreeMap::new();
    let mut out = vec![None; lots.len()];
    for i in order {
        let l = lots[i];
        let Some(e) = l.expiry.filter(|e| day_number(*e) >= day_number(today)) else { continue };
        let Some(u) = use_until(&l.item, e) else { continue };
        let before = given.entry(l.item.as_str()).or_insert(0);
        let used = (u - *before).max(0).min(l.left.max(0));
        *before += used;
        out[i] = Some(l.left - used);
    }
    out
}

/// P7: the day's `stock.expiring`, rewritten with each lot's surplus.
/// `use_until` is `None` while the forecast cannot say (`why`: learning, or
/// the history unreadable) -- then the event carries `forecast: why` and no
/// surplus, rather than a guess.
pub fn with_surplus(
    told: &mut [(&'static str, Value)],
    lots: &[&Lot],
    today: i64,
    supply: &dyn Fn(&str) -> Option<Supply>,
    use_until: Option<&dyn Fn(&str, i64) -> Option<i64>>,
    why: Option<String>,
) {
    for (_, d) in told.iter_mut().filter(|(e, _)| *e == EXPIRING) {
        let Some(f) = use_until else {
            d["forecast"] = json!(why.clone().unwrap_or_else(|| "learning".into()));
            continue;
        };
        let s = surpluses(lots, today, f);
        let of = |l: &Lot| lots.iter().position(|x| std::ptr::eq(*x, l)).and_then(|i| s[i]);
        if let Some(nd) = soon_with(lots, today, supply, Some(&of)) {
            *d = nd;
        }
        d["forecast"] = json!(why.clone().unwrap_or_else(|| "ok".into()));
    }
}

/// The words of a surplus, in the group's language.
pub fn surplus_words(lang: &str) -> (&'static str, &'static str) {
    match lang {
        "sq" => ("nuk do të përdoret në kohë", "përdoreni të parin / ofertë speciale"),
        "uk" => ("не встигнуть використати", "пустити першим / спецпропозиція"),
        "ru" => ("не успеют использовать", "пустить первым / спецпредложение"),
        "en" => ("won't be used in time", "use first / special"),
        _ => ("won't be used in time", "use first / special"),
    }
}

/// A row's surplus as the message says it, or nothing.
pub fn surplus_line(row: &Value, lang: &str) -> String {
    let Some(n) = row.get("surplus").and_then(Value::as_i64).filter(|n| *n > 0) else { return String::new() };
    let (head, act) = surplus_words(lang);
    format!(" · ⚠ {head}: {n} {} → {act}", row.get("unit").and_then(Value::as_str).unwrap_or(""))
}

#[cfg(test)]
#[path = "tell/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tell/surplus_tests.rs"]
mod surplus_tests;
