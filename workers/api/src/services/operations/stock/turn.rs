//! ONE STOCK MOVEMENT AS AN OBJECT TURN (W0a), pure: the request as the
//! Worker authorised it in, the log in memory changed, the screen's answer and
//! the groups' events out. The venue's object (`hubdo/stock_turn.rs`) writes
//! the stock image and, in the same turn, the outbox entries -- so the Worker
//! handler writes no image at all and `one-image` stays at its baseline.

use std::collections::BTreeMap;

use dowiz_hub::stock::StockLog;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{moves, tell};
use crate::notify::route::produce::{Shelf, Supply};

/// One supply as the turn needs it: its words and its thresholds.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupplyIn {
    pub name: String,
    pub unit: String,
    #[serde(default)]
    pub low_at: i64,
    #[serde(default)]
    pub shelf_days: Option<i64>,
}

/// What the Worker decided and the object executes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockTurnIn {
    pub kind: String,
    /// The request body, as sent (`moves::StockMoveIn`).
    pub body: Value,
    /// The AUTHENTICATED signer.
    pub by: String,
    pub now_ms: i64,
    /// The venue's local day, `yyyymmdd`.
    pub today: i64,
    /// Every supply of the catalogue, retired ones too (history keeps them).
    pub supplies: BTreeMap<String, SupplyIn>,
}

/// The supplies of a catalogue, as the turn reads them. PURE.
pub fn supplies_of(list: Vec<(String, String)>) -> BTreeMap<String, SupplyIn> {
    list.into_iter()
        .map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).unwrap_or(Value::Null);
            let s = SupplyIn {
                name: v.get("name").and_then(Value::as_str).unwrap_or(&id).to_string(),
                unit: v.get("unit").and_then(Value::as_str).unwrap_or("g").to_string(),
                low_at: v.get("lowAt").and_then(Value::as_i64).unwrap_or(0),
                shelf_days: v.get("shelfDays").and_then(Value::as_i64),
            };
            (id, s)
        })
        .collect()
}

pub type Told = Vec<(&'static str, Value)>;

/// Apply the movement to `log` and say what it tells. `expiring_due`: this is
/// the venue's first movement of `today` that could tell the expiring lots.
/// A refusal is `(status, words)` and leaves nothing written -- the caller
/// drops the log.
pub fn run(log: &mut StockLog, input: &StockTurnIn, expiring_due: bool) -> Result<(Value, Told), (u16, String)> {
    let body: moves::StockMoveIn = serde_json::from_value(input.body.clone()).map_err(|e| (400, format!("bad request body: {e}")))?;
    let shelf = |id: &str| input.supplies.get(id).and_then(|s| s.shelf_days);
    let plan = moves::plan(&input.kind, body, &input.by, input.now_ms, input.today, shelf)?;
    if let Some(unknown) = plan.items().into_iter().find(|i| !input.supplies.contains_key(i)) {
        return Err((404, format!("not found: {unknown}")));
    }
    log.set_clock(input.now_ms);
    let refused = |e: dowiz_hub::stock::StockError| (409, e.to_string());
    let before = log.ledger().map_err(refused)?;
    let shown = plan.apply(log).map_err(refused)?;
    let after = log.journal().map_err(refused)?;
    let supply = |id: &str| input.supplies.get(id).map(|s| Supply { name: s.name.clone(), unit: s.unit.clone(), low_at: s.low_at });
    let was = |id: &str| Shelf { free: before.available(id), counted: before.is_counted(id) };
    let now = |id: &str| Shelf { free: after.ledger.available(id), counted: after.ledger.is_counted(id) };
    let open = after.lots.open();
    let expiring = expiring_due.then_some((open.as_slice(), input.today));
    let told = tell::events(&plan, &shown, &was, &now, &supply, expiring);
    Ok((shown, told))
}

#[cfg(test)]
#[path = "turn/tests.rs"]
mod tests;
