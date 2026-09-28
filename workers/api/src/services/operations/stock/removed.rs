//! A DELETION AS A STOCK MOVEMENT (W-NOM, 2026-09-28), pure: `kind =
//! "removed"` through the same object turn every movement takes
//! (`/fold/stock_move`), so the stock image is written by the venue's object
//! and no route or object arm is added.
//!
//! The Worker deletes the supply from the CATALOGUE first
//! (`supplies/delete.rs`), then sends this: so the ids are NOT in the turn's
//! supply list any more, and `turn::run` does not ask for them. An id the
//! ledger never knew writes nothing -- deleting twice is harmless.

use dowiz_hub::stock::meta::Meta;
use dowiz_hub::stock::{StockError, StockEvent, StockLog};
use serde_json::{json, Value};

use super::moves::{Plan, StockMoveIn};

/// The most ids one deletion names: a whole nomenclature, not an unbounded body.
pub const MAX_IDS: usize = 500;

/// The plan: one signed `Removed` per distinct id (`item` and/or `items`).
pub fn plan(body: &StockMoveIn, by: &str, now_ms: i64) -> Result<Plan, (u16, String)> {
    let mut ids: Vec<String> = Vec::new();
    for id in std::iter::once(&body.item).chain(body.items.iter().flatten()) {
        let id = id.trim();
        if !id.is_empty() && !ids.iter().any(|x| x == id) {
            ids.push(id.to_string());
        }
    }
    if ids.is_empty() {
        return Err((400, "which ingredients?".into()));
    }
    if ids.len() > MAX_IDS {
        return Err((400, format!("at most {MAX_IDS} ingredients at once")));
    }
    let lines = ids
        .into_iter()
        .map(|item| (StockEvent::Removed { item, by: by.to_string() }, Meta::at(now_ms)))
        .collect();
    Ok(Plan { kind: "removed".into(), lines })
}

/// Write the lines whose item the ledger knows, as ONE decision; the others
/// are answered as `unknown` and write nothing.
pub fn apply(plan: &Plan, log: &mut StockLog) -> Result<Value, StockError> {
    let led = log.ledger()?;
    let known: Vec<String> = led.items().into_iter().map(|(i, _)| i).collect();
    let (hit, miss): (Vec<_>, Vec<_>) = plan.lines.iter().cloned().partition(|(ev, _)| known.iter().any(|k| k == ev.item()));
    if !hit.is_empty() {
        log.append_all_with(&hit)?;
    }
    let ids = |v: &[(StockEvent, Meta)]| v.iter().map(|(e, _)| json!(e.item())).collect::<Vec<_>>();
    Ok(json!({ "ok": true, "kind": "removed", "removed": ids(&hit), "unknown": ids(&miss), "lines": [], "value": 0 }))
}

#[cfg(test)]
#[path = "removed/tests.rs"]
mod tests;
