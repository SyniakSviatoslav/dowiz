//! MOVE A WHOLE SITTING TO ANOTHER TABLE (BLUEPRINT-POS-THE-ROOM §2.9):
//! "`Amended { fulfilment.table }` on every non-terminal round of the sitting,
//! one command, one turn. The `sitting_id` does not change; the bill does not
//! change."
//!
//! EACH ROUND GETS ITS OWN SIGNED DELTA, built by `amend::apply` with one
//! `Op::Table` — the same rule set a single round's table change goes through,
//! so there is one definition of "who may change a round, and when".
//!
//! WHICH ROUNDS. Every round still in the room: PENDING to READY. A round the
//! kitchen has IS moved — its food has to reach the guests where they now sit,
//! and a table is neither money nor shelf. A round that is over (served,
//! cancelled, refunded) is history and keeps the table it was served at.
//!
//! A PAID ROUND STILL IN THE ROOM REFUSES THE WHOLE MOVE. Every `Amended` is
//! refused on a round whose bill is settled (`room_rules::changeable`, the
//! confirmed frame, §4.5), and the alternative — moving the others and leaving
//! it — sends a paid dish to an empty table without a word. The waiter is told
//! which round, and moves the sitting once it is served.
//!
//! THE BILL IS CHECKED, NOT ASSUMED: every moved round's total must be what it
//! was. A table change that re-summed a round into a different number would be
//! a price change nobody signed.

use super::super::amend::{self, AmendIn, Op};
use super::super::sitting;
use super::super::Refused;
use crate::hubdo::OrderView;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveIn {
    pub sitting_id: String,
    pub location_id: String,
    pub table: String,
    pub by: String,
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovedRound {
    pub order_id: String,
    pub merged: String,
    pub body: String,
    pub seq: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveOut {
    pub sitting_id: String,
    pub table: String,
    /// `(order_id, seq)` of every round that moved, oldest first.
    pub moved: Vec<(String, u64)>,
    pub generation: i64,
}

/// Every round's new state and delta, in memory. Nothing is written.
pub fn apply(listed: &[OrderView], input: &MoveIn) -> Result<Vec<MovedRound>, Refused> {
    let table = input.table.trim();
    if table.is_empty() {
        return Err(Refused::Invalid("a table has a name".into()));
    }
    if input.by.trim().is_empty() {
        return Err(Refused::Invalid("a change to a round names who made it".into()));
    }
    let rounds: Vec<_> = sitting::rounds(listed, &input.sitting_id)
        .into_iter()
        .filter(|r| r.order.get("location_id").and_then(Value::as_str) == Some(input.location_id.as_str()))
        .collect();
    if rounds.is_empty() {
        return Err(Refused::NotFound);
    }
    let live: Vec<_> = rounds
        .iter()
        .filter(|r| super::super::room_rules::stage(r.status()) != super::super::room_rules::Stage::Over)
        .collect();
    if let Some(p) = live.iter().find(|r| r.order.get("payment_status").and_then(Value::as_str) == Some("paid")) {
        return Err(Refused::Conflict(format!(
            "round {} is paid and not yet served; move the table once it is",
            p.view.order_id
        )));
    }
    let mut out = Vec::new();
    for r in live {
        if r.order.pointer("/fulfilment/table").and_then(Value::as_str) == Some(table) {
            continue;
        }
        let one = AmendIn {
            order_id: r.view.order_id.clone(),
            location_id: input.location_id.clone(),
            base_seq: r.view.seq,
            ops: vec![Op::Table { table: table.to_string() }],
            by: input.by.clone(),
            reason: None,
            may_void: false,
            boms: Vec::new(),
            now_ms: input.now_ms,
        };
        let (round, restock) = amend::apply(r.view, &one)?;
        if restock || round.get("total") != r.order.get("total") {
            return Err(Refused::Conflict(format!(
                "round {} does not add up to the total it carries; it was not moved",
                r.view.order_id
            )));
        }
        let body = crate::fold::delta(&r.order, &round).to_string();
        out.push(MovedRound {
            order_id: r.view.order_id.clone(),
            merged: round.to_string(),
            body,
            seq: amend::next_seq(r.view.seq, input.now_ms),
        });
    }
    if out.is_empty() {
        return Err(Refused::Invalid(format!("no open round of this sitting is anywhere but table {table}")));
    }
    Ok(out)
}

/// THE WHOLE MOVE over the log in memory: one `Amended` per round, all or
/// none. The shelf is not an argument — a table is not stock.
pub fn decide(hub: &mut dowiz_hub::Hub, listed: &[OrderView], input: &MoveIn) -> Result<Vec<MovedRound>, Refused> {
    let moved = apply(listed, input)?;
    for m in &moved {
        hub.append(dowiz_hub::EventKind::Amended, &m.order_id, &m.body, m.seq, [0u8; 32])
            .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    }
    Ok(moved)
}

#[cfg(test)]
mod tests;
