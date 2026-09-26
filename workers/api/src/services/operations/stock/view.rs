//! WHAT THE STOCK SCREEN SHOWS, pure: the catalogue's supplies beside ONE
//! fold of the log (`StockLog::journal`). Levels, the average cost a priced
//! delivery set, the lots on hand with their dates, the price history, the
//! measured yields and the last movements -- every number the screen draws is
//! a row here, and every row is a record in the log.

use dowiz_hub::stock::journal::{Entry, Journal};
use dowiz_hub::stock::meta::{day_number, show_day};
use dowiz_hub::stock::{signer, StockEvent};
use serde_json::{json, Value};

/// A lot this close to its date is flagged; past it, it is expired.
pub const EXPIRY_WARN_DAYS: i64 = 2;
/// How many rows of a supply's history the card shows.
const HISTORY: usize = 8;
/// How many movements the "last movements" list shows.
const RECENT: usize = 40;

/// Minor units per `basis` base units for a receipt priced per `per`.
fn per_basis(unit_cost: i64, per: i64, basis: i64) -> Option<i64> {
    dowiz_hub::stock::journal::priced(basis, unit_cost, per)
}

/// The movement's word, as the screen names it.
pub fn kind_of(ev: &StockEvent) -> &'static str {
    match ev {
        StockEvent::Received { .. } => "received",
        StockEvent::Reserved { .. } => "reserved",
        StockEvent::Consumed { .. } => "consumed",
        StockEvent::Released { .. } => "released",
        StockEvent::Wasted { .. } => "wasted",
        StockEvent::Stocktake { .. } => "stocktake",
        StockEvent::Served { .. } => "served",
        StockEvent::Returned { .. } => "returned",
        StockEvent::Unserved { .. } => "unserved",
        StockEvent::Produced { .. } => "produced",
    }
}

/// One movement as a row. The quantity is the one the event moves; a count
/// shows what was counted and its drift.
pub fn movement_row(e: &Entry) -> Value {
    let (qty, extra) = match &e.ev {
        StockEvent::Stocktake { observed, .. } => (*observed, json!({ "expected": e.expected(), "drift": observed - e.expected() })),
        StockEvent::Wasted { qty, reason, .. } => (*qty, json!({ "reason": reason.as_str() })),
        StockEvent::Produced { qty, out, stage, into, .. } => {
            (*qty, json!({ "out": out, "stage": stage.as_str(), "into": into, "yieldPm": if *qty > 0 { out * 1000 / qty } else { 0 } }))
        }
        StockEvent::Received { qty, .. } => (*qty, json!({ "supplier": e.meta.supplier, "doc": e.meta.doc })),
        other => (other_qty(other), json!({ "order": other.order_id() })),
    };
    let mut row = json!({
        "seq": e.seq, "at": e.meta.at, "kind": kind_of(&e.ev), "item": e.ev.item(), "qty": qty,
        "value": e.value, "lot": e.meta.lot, "by": signer(&e.ev).or(e.meta.by.as_deref()),
    });
    if let (Some(m), Some(x)) = (row.as_object_mut(), extra.as_object()) {
        for (k, v) in x {
            m.insert(k.clone(), v.clone());
        }
    }
    row
}

fn other_qty(ev: &StockEvent) -> i64 {
    match ev {
        StockEvent::Reserved { qty, .. }
        | StockEvent::Consumed { qty, .. }
        | StockEvent::Released { qty, .. }
        | StockEvent::Served { qty, .. }
        | StockEvent::Returned { qty, .. }
        | StockEvent::Unserved { qty, .. } => *qty,
        _ => 0,
    }
}

/// What the Stock screen adds to a supply's row, beside its level.
pub fn extras(id: &str, basis: i64, j: &Journal, today: i64) -> Value {
    let lots: Vec<Value> = j
        .lots
        .of(id)
        .iter()
        .map(|l| {
            let days_left = l.expiry.map(|d| day_number(d) - day_number(today));
            json!({ "code": l.code, "left": l.left, "received": l.received, "expiry": l.expiry.map(show_day),
                    "daysLeft": days_left, "supplier": l.supplier, "doc": l.doc, "at": l.at })
        })
        .collect();
    let left = |max: i64| lots.iter().filter(|l| l["daysLeft"].as_i64().is_some_and(|d| d <= max)).count();
    let mine = || j.entries.iter().filter(move |e| e.ev.item() == id);
    let prices: Vec<Value> = mine()
        .filter_map(|e| match (&e.ev, e.meta.unit_cost, e.meta.per) {
            (StockEvent::Received { qty, .. }, Some(c), Some(p)) => Some(json!({
                "at": e.meta.at, "perBasis": per_basis(c, p, basis), "qty": qty, "value": e.value,
                "supplier": e.meta.supplier, "doc": e.meta.doc, "lot": e.meta.lot,
            })),
            _ => None,
        })
        .collect();
    let yields: Vec<Value> = mine()
        .filter_map(|e| match &e.ev {
            StockEvent::Produced { qty, out, stage, into, .. } if *qty > 0 => Some(json!({
                "at": e.meta.at, "stage": stage.as_str(), "qty": qty, "out": out, "pm": out * 1000 / qty, "into": into,
            })),
            _ => None,
        })
        .collect();
    let last_pm = |stage: &str| yields.iter().rev().find(|y| y["stage"] == stage).and_then(|y| y["pm"].as_i64());
    let last_count = mine().filter(|e| matches!(e.ev, StockEvent::Stocktake { .. })).last().map(movement_row);
    let moves: Vec<Value> =
        mine().filter(|e| !matches!(e.ev, StockEvent::Reserved { .. } | StockEvent::Released { .. })).map(movement_row).collect();
    json!({
        "wac": j.book.wac(id, basis),
        "lots": lots,
        "expiring": left(EXPIRY_WARN_DAYS),
        "expired": left(-1),
        "prices": tail(prices, HISTORY),
        "yields": tail(yields.clone(), HISTORY),
        "measuredCleanPm": last_pm("clean"),
        "measuredCookPm": last_pm("cook"),
        "lastCount": last_count,
        "moves": tail(moves, HISTORY),
    })
}

fn tail(v: Vec<Value>, n: usize) -> Vec<Value> {
    let skip = v.len().saturating_sub(n);
    v.into_iter().skip(skip).collect()
}

/// The last movements, newest first, and every supplier any receipt named
/// (for the delivery form's list), sorted.
pub fn recent_and_suppliers(j: &Journal) -> (Vec<Value>, Vec<String>) {
    let recent = j
        .entries
        .iter()
        .rev()
        .filter(|e| !matches!(e.ev, StockEvent::Reserved { .. } | StockEvent::Released { .. }))
        .take(RECENT)
        .map(movement_row)
        .collect();
    let mut suppliers: Vec<String> = j.entries.iter().filter_map(|e| e.meta.supplier.clone()).collect();
    suppliers.sort();
    suppliers.dedup();
    (recent, suppliers)
}

/// The last count SESSIONS, newest first: lines, and the drift's value.
pub fn sessions(j: &Journal, n: usize) -> Vec<Value> {
    let mut out: Vec<(String, Option<i64>, i64, i64)> = Vec::new();
    for e in &j.entries {
        let StockEvent::Stocktake { stocktake_id, .. } = &e.ev else { continue };
        let id = e.meta.session.clone().unwrap_or_else(|| stocktake_id.clone());
        match out.iter_mut().find(|s| s.0 == id) {
            Some(s) => {
                s.2 += 1;
                s.3 += e.value.unwrap_or(0);
            }
            None => out.push((id, e.meta.at, 1, e.value.unwrap_or(0))),
        }
    }
    out.iter().rev().take(n).map(|(id, at, lines, value)| json!({ "session": id, "at": at, "lines": lines, "value": value })).collect()
}

#[cfg(test)]
mod tests;
