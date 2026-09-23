//! TRANSFER LINES BETWEEN ROUNDS (BLUEPRINT-POS-THE-ROOM §2.9, G4): a guest
//! joins another table, or "split these into their own check". One command,
//! one object turn, two signed `Amended` deltas — the source loses the lines
//! and their money, the destination gains exactly the same.
//!
//! CONSERVATION (G4). Σ line totals over `{from, to}` is the same before and
//! after, and so is Σ `total`: a comped line takes its comp with it, so the
//! discount moves by the same amount the lines do. Each round still obeys
//! law 3 on its own — `total = lines + fee + tip − discount` — because each is
//! re-summed by `room_rules::reprice`, the one rule `amend` uses.
//!
//! ONLY BEFORE THE KITCHEN, ON BOTH SIDES (§2.9: "refuses the whole thing if
//! either order is past CONFIRMED"). This is stricter than `amend`, where the
//! `void` capability lets a line leave a round the kitchen has: a transfer
//! would also ADD that line to the other round, and nothing is added to what
//! the kitchen has. So there is no capability that makes it legal, and the
//! shelf never moves after the kitchen because this never reaches it.
//!
//! STOCK FOLLOWS THE LINES (§2.9). Reservations are per order id, so the
//! source's are released and re-reserved for what it keeps, and the
//! destination's for what it now holds, in the same turn: I3 holds on both.
//!
//! THE CONFIRMED FRAME. A paid round is neither source nor destination
//! (`room_rules::changeable`), and each side names the version its tablet saw:
//! a stale `base_seq` on either is a Conflict, because moving "line 1" of a
//! round somebody else just changed moves the wrong dish.

use super::amend::next_seq;
use super::room_rules::{self as rr, Stage};
use super::Refused;
use crate::hubdo::OrderView;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub mod sitting;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferIn {
    pub from_order_id: String,
    pub to_order_id: String,
    /// The venue that was AUTHORISED; checked against both orders' own.
    pub location_id: String,
    /// The versions the tablet was looking at. Both are checked.
    pub from_base_seq: u64,
    pub to_base_seq: u64,
    /// Indices into the source round's `items` as the tablet saw them.
    pub lines: Vec<usize>,
    /// The signer. Mandatory, and never waived.
    pub by: String,
    /// `(product_id, catalogue record)` for every product the Worker saw.
    pub boms: Vec<(String, String)>,
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferOut {
    pub from_merged: String,
    pub from_seq: u64,
    pub to_merged: String,
    pub to_seq: u64,
    pub generation: i64,
}

/// Both rounds after the move, in memory, and what moved. Nothing is written.
pub struct Moved {
    pub from: Value,
    pub to: Value,
    /// Σ line amounts that moved (gross, before any comp).
    pub amount: i64,
    /// The part of `amount` that was comped, and moved its discount with it.
    pub comp: i64,
}

fn parse(v: &OrderView) -> Result<Value, Refused> {
    serde_json::from_str(&v.order_json).map_err(|e| Refused::Append(format!("order json unreadable: {e}")))
}

fn items(o: &Value) -> Vec<Value> {
    o.get("items").and_then(Value::as_array).cloned().unwrap_or_default()
}

/// Apply a transfer to two rounds, in memory.
pub fn apply(from: &OrderView, to: &OrderView, input: &TransferIn) -> Result<Moved, Refused> {
    if input.from_order_id == input.to_order_id {
        return Err(Refused::Invalid("a round cannot take lines from itself".into()));
    }
    let (mut src, mut dst) = (parse(from)?, parse(to)?);
    for (o, stage_of) in [(&src, "source"), (&dst, "destination")] {
        if rr::changeable(o, &input.location_id, &input.by)? != Stage::BeforeKitchen {
            return Err(Refused::Conflict(format!("the kitchen has the {stage_of} round: nothing moves between rounds after that")));
        }
    }
    if input.from_base_seq != from.seq || input.to_base_seq != to.seq {
        return Err(Refused::Conflict("this order changed while you were editing it".into()));
    }
    let old = items(&src);
    if input.lines.is_empty() {
        return Err(Refused::Invalid("a transfer moves at least one line".into()));
    }
    let mut picked = vec![false; old.len()];
    for &i in &input.lines {
        if i >= old.len() || picked[i] {
            return Err(Refused::Invalid(format!("line {i} is not one line of the source round, once")));
        }
        picked[i] = true;
    }
    let (mut kept, mut moving) = (Vec::new(), Vec::new());
    for (i, l) in old.into_iter().enumerate() {
        if picked[i] { moving.push(l) } else { kept.push(l) }
    }
    let amount = rr::subtotal_of(&moving)?;
    let comped: Vec<&Value> = moving.iter().filter(|l| l.get("comped").and_then(Value::as_bool) == Some(true)).collect();
    let comp = rr::subtotal_of(&comped.iter().map(|l| (*l).clone()).collect::<Vec<_>>())?;
    let src_disc = src.get("discount").and_then(Value::as_i64).unwrap_or(0);
    let dst_disc = dst.get("discount").and_then(Value::as_i64).unwrap_or(0);
    if comp > src_disc {
        return Err(Refused::Conflict("the source's discount does not hold the comps of the lines it would give".into()));
    }
    let dst_disc = dst_disc.checked_add(comp).ok_or_else(|| Refused::Invalid("overflow".into()))?;
    let mut grown = items(&dst);
    grown.extend(moving.iter().cloned());
    rr::reprice(&mut src, kept, src_disc - comp)?;
    rr::reprice(&mut dst, grown, dst_disc)?;

    let id = format!("{}>{}@{}", input.from_order_id, input.to_order_id, input.now_ms);
    for l in &comped {
        let a = rr::line_amount(l)?;
        let pid = l.get("product_id").cloned().unwrap_or(Value::Null);
        rr::push(&mut src, "adjustments", json!({
            "kind": "comp", "product_id": pid, "amount": -a, "moved_to": input.to_order_id,
            "transfer": id, "by": input.by, "at": input.now_ms,
        }));
        rr::push(&mut dst, "adjustments", json!({
            "kind": "comp", "product_id": pid, "amount": a, "moved_from": input.from_order_id,
            "transfer": id, "by": input.by, "at": input.now_ms,
        }));
    }
    let n = input.lines.len();
    rr::push(&mut src, "amended", json!({
        "by": input.by, "at": input.now_ms,
        "transfer": { "id": id, "dir": "out", "other": input.to_order_id, "lines": n, "amount": amount, "comp": comp },
    }));
    rr::push(&mut dst, "amended", json!({
        "by": input.by, "at": input.now_ms,
        "transfer": { "id": id, "dir": "in", "other": input.from_order_id, "lines": n, "amount": amount, "comp": comp },
    }));
    Ok(Moved { from: src, to: dst, amount, comp })
}

/// What `decide` hands the object: both rounds, both delta bodies, both seqs.
pub struct Decided {
    pub moved: Moved,
    pub from_body: String,
    pub to_body: String,
    pub from_seq: u64,
    pub to_seq: u64,
}

/// THE WHOLE TRANSFER, over two images in memory. Both are mutated in place
/// and written by the caller only if this returns `Ok`.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    stock: &mut dowiz_hub::stock::StockLog,
    from: Option<&OrderView>,
    to: Option<&OrderView>,
    input: &TransferIn,
) -> Result<Decided, Refused> {
    let (Some(from), Some(to)) = (from, to) else { return Err(Refused::NotFound) };
    let moved = apply(from, to, input)?;
    // Both sides are before the kitchen (apply refused otherwise), so both
    // still hold reservations, and both are re-reserved for their new lines.
    rr::restock(stock, &input.from_order_id, &items(&moved.from), &input.boms)?;
    rr::restock(stock, &input.to_order_id, &items(&moved.to), &input.boms)?;
    let from_body = crate::fold::delta(&parse(from)?, &moved.from).to_string();
    let to_body = crate::fold::delta(&parse(to)?, &moved.to).to_string();
    let (from_seq, to_seq) = (next_seq(from.seq, input.now_ms), next_seq(to.seq, input.now_ms));
    hub.append(dowiz_hub::EventKind::Amended, &input.from_order_id, &from_body, from_seq, [0u8; 32])
        .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    hub.append(dowiz_hub::EventKind::Amended, &input.to_order_id, &to_body, to_seq, [0u8; 32])
        .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    Ok(Decided { moved, from_body, to_body, from_seq, to_seq })
}

#[cfg(test)]
mod tests;
