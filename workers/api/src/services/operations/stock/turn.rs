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
    /// The catalogue record itself, sent only for a production act (its
    /// card and every card under it are read in the object: `cook`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<String>,
    /// A recipe names it, directly or under a semi-finished card: its loss
    /// between counts is tracked (`digest`). Filled for a count only.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub linked: bool,
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
    /// The venue's local day, `yyyymmdd`. FILLED BY THE OBJECT from the
    /// catalogue it holds (`from_catalogue`, BN1); the Worker sends 0.
    pub today: i64,
    /// Every supply of the catalogue, retired ones too (history keeps them).
    /// FILLED BY THE OBJECT (`from_catalogue`); the Worker sends none.
    pub supplies: BTreeMap<String, SupplyIn>,
    /// The venue's currency code, for the words of a production act's cost
    /// (`stock.cooked`, W-PF3 T2). Empty: none is shown. FILLED BY THE OBJECT
    /// (`from_catalogue`); the Worker sends none.
    #[serde(default)]
    pub currency: String,
}

impl StockTurnIn {
    /// The catalogue's part of the turn -- the supplies for this kind of act,
    /// the venue's day at `now_ms`, its currency -- read from the catalogue
    /// the object holds (BN1), whatever the Worker sent. PURE.
    pub fn from_catalogue(mut self, cat: &dowiz_hub::catalog::Catalog) -> Self {
        self.supplies = super::cook::supplies_for(&self.kind, cat.supplies());
        if is_count(&self.kind) {
            for id in linked_of(cat) {
                if let Some(s) = self.supplies.get_mut(&id) {
                    s.linked = true;
                }
            }
        }
        self.today = super::today_of(cat, self.now_ms);
        // W-STORE2: a freezing's start, typed in the venue's local time.
        if self.kind == super::storages::FROZEN {
            let zone = crate::hubstore::zone_of(cat.location().and_then(|j| serde_json::from_str::<Value>(&j).ok()).as_ref());
            super::storages::bind::stamp_started(&mut self.body, zone);
        }
        self.currency = crate::services::venue::currency_of(cat);
        self
    }
}

/// A count: one line or a session.
pub fn is_count(kind: &str) -> bool {
    kind == "count" || kind == "stocktake"
}

/// Every supply a dish's recipe names, directly or through a semi-finished
/// card (its raw leaves). PURE.
pub fn linked_of(cat: &dowiz_hub::catalog::Catalog) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    for (_, j) in cat.products() {
        out.extend(dowiz_hub::stock::bom_of(&j).into_iter().map(|l| l.supply));
        out.extend(crate::services::analytics::kitchen::leaves_of(cat, &j).into_iter().map(|(s, _)| s));
    }
    out
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
                record: None,
                linked: false,
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
    // A BATCH COOKED AHEAD (W-PF2 R2): its own door, the card read here.
    if input.kind == super::cook::KIND {
        return super::cook::run(log, input);
    }
    // A SUPPLIER'S CARD, OR AN ORDER SENT (W-STOCK P5): notes, never a movement.
    if input.kind == super::suppliers::CARD || input.kind == super::suppliers::ORDERED {
        return super::suppliers::run(log, input);
    }
    // WHAT A SUPPLIER'S INVOICES CALL OUR SUPPLIES (W-OCR): notes, never a movement.
    if input.kind == super::aliases::KIND {
        return super::aliases::run(log, input);
    }
    // STORAGES, TRANSFERS, FREEZING RECORDS (P12/P13, W-STORE): notes, never a movement.
    if super::storages::is_mine(&input.kind) {
        return super::storages::run(log, input);
    }
    let body: moves::StockMoveIn = serde_json::from_value(input.body.clone()).map_err(|e| (400, format!("bad request body: {e}")))?;
    let shelf = |id: &str| input.supplies.get(id).and_then(|s| s.shelf_days);
    let plan = moves::plan(&input.kind, body, &input.by, input.now_ms, input.today, shelf)?;
    // A deletion names supplies the catalogue has just let go (W-NOM).
    if let Some(unknown) = plan.items().into_iter().find(|i| input.kind != "removed" && !input.supplies.contains_key(i)) {
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
    let mut told = tell::events(&plan, &shown, &was, &now, &supply, expiring);
    // THE WEEKLY LOSS DIGEST rides on a count (W-STOCK P4, `digest`).
    if is_count(&input.kind) {
        let named = |id: &str| input.supplies.get(id).map(|s| (s.name.clone(), s.unit.clone(), s.linked));
        if let Some(d) = super::digest::owe(log, &after, input.now_ms, &named, &input.currency) {
            told.push((super::digest::EVENT, d));
        }
    }
    Ok((shown, told))
}

#[cfg(test)]
#[path = "turn/tests.rs"]
mod tests;
