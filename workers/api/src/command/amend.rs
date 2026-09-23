//! AMEND A ROUND: lines added, removed, re-counted or comped, and the table it
//! stands at — as one signed `Amended` event, in one object turn.
//!
//! THE INTENT FORM (BLUEPRINT-POS-THE-ROOM §4.4, §7 item 2). The tablet sends
//! what the waiter DID — add this, remove line 2, make line 0 three — not the
//! line set it ended up with. Two adds from two tablets therefore commute: the
//! object applies each to the round as it is NOW. What does not commute is an
//! edit to a line somebody else may have changed, so `Remove`, `SetQty` and
//! `Comp` carry `base_seq`, the version the tablet was looking at, and a stale
//! one is refused: "this order changed while you were editing it". That is
//! Square's `order.version`, and the authoritative server's "your input was for
//! tick 100 and we are at 104".
//!
//! THE PRICE IS NOT DECIDED HERE. An added line arrives priced by the Worker's
//! one pricer (`services::ordering::pricing::price_basket`); this command only
//! sums priced lines, which is the kernel's own subtotal rule.
//!
//! THE RECORD OUTRANKS THE PERMISSION (§2.6). Every amendment names its signer
//! and is appended under the order it changed, in the chained log, so
//! `Hub::history(order_id)` is the exception report and law 8 proves the
//! served bill equals the fold of it. A change with no signer is refused.

use super::room_rules::{self as rr, Stage, VoidReason};
use super::Refused;
use crate::hubdo::OrderView;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// One thing a waiter did to a round. `line` is an index into the round's
/// `items` AS THE TABLET SAW THEM at `base_seq`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// A priced line, as `pricing::price_basket` produced it.
    Add { line: Value },
    Remove { line: usize },
    SetQty { line: usize, qty: i64 },
    /// The line stays on the round and is not charged: `discount` grows by its
    /// amount and the comp is written into `adjustments`.
    Comp { line: usize },
    /// The round now stands at another table.
    Table { table: String },
}

impl Op {
    /// Does this op depend on the line set the tablet was looking at?
    fn needs_version(&self) -> bool {
        matches!(self, Op::Remove { .. } | Op::SetQty { .. } | Op::Comp { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmendIn {
    pub order_id: String,
    /// The venue that was AUTHORISED; checked against the order's own.
    pub location_id: String,
    pub base_seq: u64,
    pub ops: Vec<Op>,
    /// The signer. Mandatory, and never waived.
    pub by: String,
    /// A `VoidReason` word. Mandatory for `Remove` and `Comp`.
    pub reason: Option<String>,
    /// The signer holds `void`: a line may leave a round the kitchen has.
    pub may_void: bool,
    /// `(product_id, catalogue record)` for every product the Worker saw, so
    /// the shelf can follow the lines. Empty at a venue with no recipes.
    pub boms: Vec<(String, String)>,
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmendOut {
    /// The round as it now is.
    pub merged: String,
    pub seq: u64,
    pub generation: i64,
}

/// The seq the next event on an order is written at: the clock, but never at
/// or before the version it replaces, so `base_seq` always moves.
pub fn next_seq(prev: u64, now_ms: i64) -> u64 {
    (now_ms.max(0) as u64).max(prev + 1)
}

/// Apply the ops to a round, in memory. Returns the new round and the delta
/// body to append. Nothing is written by this function.
pub fn apply(current: &OrderView, input: &AmendIn) -> Result<(Value, bool), Refused> {
    let old: Value = serde_json::from_str(&current.order_json)
        .map_err(|e| Refused::Append(format!("order json unreadable: {e}")))?;
    let stage = rr::changeable(&old, &input.location_id, &input.by)?;
    if input.ops.is_empty() {
        return Err(Refused::Invalid("an amendment with nothing in it".into()));
    }
    if input.ops.iter().any(Op::needs_version) && input.base_seq != current.seq {
        return Err(Refused::Conflict("this order changed while you were editing it".into()));
    }
    let reason = input.reason.as_deref().map(|r| {
        VoidReason::parse(r).ok_or_else(|| Refused::Invalid(format!("{r:?} is not a void reason")))
    });
    let reason = reason.transpose()?;
    let takes_off = input.ops.iter().any(|o| matches!(o, Op::Remove { .. } | Op::Comp { .. }));
    if takes_off && reason.is_none() {
        return Err(Refused::Invalid("a line taken off a round says why".into()));
    }
    // WHAT THE KITCHEN HAS IS WHAT WAS COOKED. Nothing is added to it or
    // re-counted — that is a new round — and a line leaves it only with `void`.
    // A comp is the Counter-Manager's word at any stage: money not taken for
    // food that was served is the same act before and after the pass.
    if stage == Stage::InKitchen
        && input.ops.iter().any(|o| matches!(o, Op::Add { .. } | Op::SetQty { .. }))
    {
        return Err(Refused::Conflict("the kitchen has this round: order a new one instead".into()));
    }
    let voids = input.ops.iter().any(|o| match o {
        Op::Comp { .. } => true,
        Op::Remove { .. } => stage == Stage::InKitchen,
        _ => false,
    });
    if voids && !input.may_void {
        return Err(Refused::Conflict("this needs the void capability".into()));
    }

    let mut items: Vec<Value> = old.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
    let n = items.len();
    let mut gone = vec![false; n];
    let mut touched = vec![false; n];
    let mut discount = old.get("discount").and_then(Value::as_i64).unwrap_or(0);
    let mut round = old.clone();
    let mut restock = false;
    for op in &input.ops {
        let at = match op {
            Op::Remove { line } | Op::SetQty { line, .. } | Op::Comp { line } => Some(*line),
            _ => None,
        };
        if let Some(i) = at {
            if i >= n || touched[i] {
                return Err(Refused::Invalid(format!("line {i} is not one line of this round, once")));
            }
            touched[i] = true;
            if items[i].get("comped").and_then(Value::as_bool) == Some(true) {
                return Err(Refused::Conflict(format!("line {i} was comped; it stays as it is")));
            }
        }
        match op {
            Op::Add { line } => {
                let q = line.get("quantity").and_then(Value::as_i64).unwrap_or(0);
                if q < 1 || line.get("product_id").and_then(Value::as_str).is_none() {
                    return Err(Refused::Invalid("an added line names a dish and a quantity".into()));
                }
                items.push(line.clone());
                restock = true;
            }
            Op::Remove { line } => {
                gone[*line] = true;
                restock = true;
            }
            Op::SetQty { line, qty } => {
                if *qty < 1 {
                    return Err(Refused::Invalid("a quantity is at least one; remove the line instead".into()));
                }
                items[*line]["quantity"] = json!(qty);
                restock = true;
            }
            Op::Comp { line } => {
                let amount = rr::line_amount(&items[*line])?;
                discount = discount.checked_add(amount).ok_or_else(|| Refused::Invalid("overflow".into()))?;
                items[*line]["comped"] = json!(true);
                rr::push(&mut round, "adjustments", json!({
                    "kind": "comp", "line": line, "product_id": items[*line].get("product_id"),
                    "amount": amount, "reason": reason.as_ref().map(VoidReason::word),
                    "by": input.by, "at": input.now_ms,
                }));
            }
            Op::Table { table } => {
                let t = table.trim();
                if t.is_empty() {
                    return Err(Refused::Invalid("a table has a name".into()));
                }
                if !round.get("fulfilment").is_some_and(Value::is_object) {
                    round["fulfilment"] = json!({});
                }
                round["fulfilment"]["table"] = json!(t);
            }
        }
    }
    let kept: Vec<Value> = items
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i >= n || !gone[*i])
        .map(|(_, l)| l)
        .collect();
    rr::reprice(&mut round, kept, discount)?;
    rr::push(&mut round, "amended", json!({
        "by": input.by, "at": input.now_ms,
        "reason": reason.as_ref().map(VoidReason::word),
        "ops": serde_json::to_value(&input.ops).unwrap_or(Value::Null),
    }));
    Ok((round, restock && stage == Stage::BeforeKitchen))
}

/// THE WHOLE AMENDMENT, over two images already in memory. Both are mutated
/// in place and written by the caller only if this returns `Ok`.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    stock: &mut dowiz_hub::stock::StockLog,
    current: Option<&OrderView>,
    input: &AmendIn,
) -> Result<(Value, String, u64), Refused> {
    let Some(current) = current else { return Err(Refused::NotFound) };
    let (round, restock) = apply(current, input)?;
    if restock {
        let items = round.get("items").and_then(Value::as_array).cloned().unwrap_or_default();
        rr::restock(stock, &input.order_id, &items, &input.boms)?;
    }
    let old: Value = serde_json::from_str(&current.order_json).unwrap_or(json!({}));
    let body = crate::fold::delta(&old, &round).to_string();
    let seq = next_seq(current.seq, input.now_ms);
    hub.append(dowiz_hub::EventKind::Amended, &input.order_id, &body, seq, [0u8; 32])
        .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    Ok((round, body, seq))
}

#[cfg(test)]
mod tests;
