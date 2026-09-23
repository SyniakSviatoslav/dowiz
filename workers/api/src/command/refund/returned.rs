//! THE FOOD THAT CAME BACK: the owner's one choice on an order refused at the
//! door (BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.4, P1-4) -- resell it, or waste
//! it. "Chosen by the owner, never inferred."
//!
//! ONE IMAGE. The choice is written as `StockEvent::Returned`, one per line the
//! kitchen `Consumed` for the order, and each carries the order id -- so
//! "was it already chosen" is answered by the stock log alone and nothing is
//! written on the order. A second choice is refused.
//!
//! WHY NOT §2.4's `Wasted{Returned}` PER LINE: those ingredients left the shelf
//! at PREPARING (`Consumed`); a `Wasted` takes them off a second time, and is
//! refused outright when the rest of the shelf is promised to other orders.
//! Waste is therefore recorded without moving the shelf (`stock.rs`).

use super::super::Refused;
use crate::hubdo::OrderView;
use dowiz_hub::stock::{returned_lines, StockEvent, StockLog};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnedIn {
    pub order_id: String,
    /// The venue that was AUTHORISED; checked against the order's own.
    pub location_id: String,
    /// Who chose. Mandatory.
    pub by: String,
    /// `"resell"` or `"waste"`.
    pub choice: String,
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnedOut {
    pub choice: String,
    /// `(item, qty)` per line written, in the item's base unit.
    pub lines: Vec<(String, i64)>,
    /// The courier the food came back with, as recorded on each line.
    pub courier: String,
}

/// Decide and append. `stock` is written by the caller only on `Ok`; every
/// refusal comes before the append.
pub fn decide(stock: &mut StockLog, current: Option<&OrderView>, input: &ReturnedIn) -> Result<ReturnedOut, Refused> {
    let Some(current) = current else { return Err(Refused::NotFound) };
    let order: Value = serde_json::from_str(&current.order_json)
        .map_err(|e| Refused::Append(format!("order json unreadable: {e}")))?;
    if order.get("location_id").and_then(Value::as_str) != Some(input.location_id.as_str()) {
        return Err(Refused::NotFound);
    }
    if input.by.trim().is_empty() {
        return Err(Refused::Invalid("the choice names who made it".into()));
    }
    let resell = match input.choice.trim() {
        "resell" => true,
        "waste" => false,
        other => return Err(Refused::Invalid(format!("{other:?} is not resell or waste"))),
    };
    let status = order.get("status").and_then(Value::as_str).unwrap_or("");
    if !matches!(status, "REFUNDING" | "COMPENSATED_REFUND") {
        return Err(Refused::Conflict(format!("this order is {status}, not refunded")));
    }
    let refund = order.get("refund");
    if refund.and_then(|r| r.get("reason")).and_then(Value::as_str) != Some("refused_at_door") {
        return Err(Refused::Conflict("only food refused at the door comes back".into()));
    }
    // THE COURIER WHO CARRIED IT back; the refund's signer if none was recorded.
    let courier = order
        .get("courier_id")
        .and_then(Value::as_str)
        .filter(|c| !c.is_empty())
        .or_else(|| refund.and_then(|r| r.get("by")).and_then(Value::as_str))
        .unwrap_or("")
        .to_string();
    let (chosen, lines) = returned_lines(stock, &input.order_id);
    if chosen {
        return Err(Refused::Conflict("the food that came back was already recorded".into()));
    }
    if lines.is_empty() {
        return Err(Refused::Conflict("the kitchen took nothing for this order, so nothing came back".into()));
    }
    let evs: Vec<StockEvent> = lines
        .iter()
        .map(|(item, qty)| StockEvent::Returned {
            item: item.clone(),
            qty: *qty,
            order_id: input.order_id.clone(),
            resell,
            by: courier.clone(),
            chosen_by: input.by.clone(),
        })
        .collect();
    stock.append_all(&evs).map_err(|e| Refused::Stock(e.to_string()))?;
    Ok(ReturnedOut { choice: input.choice.trim().to_string(), lines, courier })
}

#[cfg(test)]
mod tests;
