//! A BATCH OF A SEMI-FINISHED PRODUCT COOKED AHEAD -- the production act
//! (акт приготування, W-PF2 R2), `POST /api/owner/stock/cooked`, pure:
//!
//!   {"item": "<ПФ id>", "qty": <what the card makes from what went in>,
//!    "out": <what came off, weighed; default qty>, "lot"?, "expiry"?}
//!
//! The raw items leave the shelf by the card, exactly (a ready ПФ inside the
//! card is taken first), the batch lands as one weighed record with what it
//! cost, and the answer shows the loss on cooking. The ledger's own door
//! (`dowiz_hub::stock::StockLog::cook`) decides; this file reads the request.
//! Same guard as a prep or a delivery (`signer_for`: SHELF, the kitchen).

use std::collections::BTreeMap;

use dowiz_hub::stock::act::Act;
use dowiz_hub::stock::meta::parse_day;
use dowiz_hub::stock::StockLog;
use serde_json::{json, Value};

use super::moves::StockMoveIn;
use super::turn::{supplies_of, StockTurnIn, SupplyIn, Told};

/// The movement's word on the wire.
pub const KIND: &str = "cooked";

/// The turn's supplies; for an act, each with its record (the cards).
pub fn supplies_for(kind: &str, list: Vec<(String, String)>) -> BTreeMap<String, SupplyIn> {
    let mut out = supplies_of(list.clone());
    if kind == KIND {
        for (id, j) in list {
            if let Some(s) = out.get_mut(&id) {
                s.record = Some(j);
            }
        }
    }
    out
}

/// Apply the act to `log` and say what it did. A refusal leaves nothing written.
pub fn run(log: &mut StockLog, input: &StockTurnIn) -> Result<(Value, Told), (u16, String)> {
    let body: StockMoveIn = serde_json::from_value(input.body.clone()).map_err(|e| (400, format!("bad request body: {e}")))?;
    let item = body.item.trim().to_string();
    let rec = input.supplies.get(&item).ok_or((404, format!("not found: {item}")))?;
    let record = rec.record.clone().unwrap_or_default();
    if !dowiz_hub::prep::is_prep(&record) {
        return Err((400, format!("{item} is not a semi-finished product: only a card can be cooked")));
    }
    let planned = body.qty.ok_or((400, "how much does the card make from what went in?".to_string()))?;
    let out = body.out.unwrap_or(planned);
    let expiry = match body.expiry.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(parse_day(s).ok_or((400, format!("{s:?} is not a date (yyyy-mm-dd)")))?),
        None => rec.shelf_days.filter(|d| *d > 0).map(|d| {
            dowiz_hub::stock::meta::day_of_number(dowiz_hub::stock::meta::day_number(input.today) + d)
        }),
    };
    let lot = body.lot.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let act = Act { prep: item.clone(), planned, out, act: format!("pa_{}", input.now_ms), by: input.by.clone(), lot, expiry };
    let supply = |id: &str| input.supplies.get(id).and_then(|s| s.record.clone());
    log.set_clock(input.now_ms);
    let done = log.cook(&act, &supply).map_err(|e| (409, e.to_string()))?;
    let lines: Vec<Value> = done
        .inputs
        .iter()
        .map(|(i, q, uq)| {
            let s = input.supplies.get(i);
            json!({ "item": i, "name": s.map_or(i.as_str(), |s| s.name.as_str()), "unit": s.map_or("g", |s| s.unit.as_str()), "qty": q, "uq": uq })
        })
        .collect();
    let g = done.gross;
    let shown = json!({
        "ok": true, "kind": KIND, "item": item, "act": act.act, "planned": planned, "out": out, "gross": g,
        "lossG": (g > 0).then(|| g - out), "yieldPm": (g > 0).then(|| out * 1000 / g), "cardPm": (g > 0).then(|| planned * 1000 / g),
        "value": done.value, "lines": lines, "expiry": expiry.map(dowiz_hub::stock::meta::show_day),
    });
    Ok((shown, Vec::new()))
}

#[cfg(test)]
#[path = "cook/tests.rs"]
mod tests;
