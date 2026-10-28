//! SHEAF S2b, IN THE VENUE'S OBJECT, IN ONE TURN: a kitchen tablet's offline taps rebased onto
//! the order log (`dowiz_hub::glue::orders::rebase`) by the object's OWN deciders -- an offline
//! "seen" is `command::kitchen_ack::decide`, an offline bump is `command::advance::decide` (the
//! kernel's FSM, the shelf settled in the same turn) -- then the log and the shelf written, each
//! written event broadcast, and the customer told as an online bump tells them. Held taps become
//! decisions on the kitchen's strip (`orders::hold_notes`). The contract is
//! `services/operations/orders/glue.rs`.
//!
//! A refused tail writes nothing. The law `sent == merged + held` is checked BEFORE anything is
//! persisted; a turn that would lose a tap is refused whole.

use super::super::HubImages;
use crate::command::advance::AdvanceIn;
use crate::command::kitchen_ack::KitchenAckIn;
use crate::services::operations::orders::glue::GlueOrdersIn;
use dowiz_hub::glue::orders::{hold_notes, read_tail, rebase, Kind, Op, Outcome, MAX_OFFLINE_MS};
use dowiz_hub::room::view::current;
use dowiz_hub::stock::StockLog;
use dowiz_hub::{EventKind, Hub};
use serde_json::{json, Value};
use worker::*;
use crate::wire::Reply as Response;

/// One event this turn wrote: what the sockets are sent, and the status a bump reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Wrote {
    pub kind: u8,
    pub order: String,
    pub body: String,
    pub next: Option<String>,
}

/// THE PURE HALF: the rebase over images in memory, each op decided by the object's own decider.
pub(crate) fn rebase_turn(hub: &mut Hub, stock: &mut StockLog, input: &GlueOrdersIn, ops: &[Op]) -> (Outcome, Vec<Wrote>) {
    let mut wrote = Vec::new();
    let now = input.now_ms;
    let out = rebase(hub, &input.location_id, ops, |hub, op| match &op.kind {
        Kind::Ack => {
            let cur = current(hub, &op.order);
            let ack = KitchenAckIn { order_id: op.order.clone(), location_id: input.location_id.clone(), by: input.by.clone(), now_ms: op.at.clamp(now - MAX_OFFLINE_MS, now) };
            match crate::command::kitchen_ack::decide(hub, cur.as_ref(), &ack) {
                Ok((_, Some((body, _)))) => {
                    wrote.push(Wrote { kind: EventKind::Noted as u8, order: op.order.clone(), body, next: None });
                    Ok(())
                }
                Ok((_, None)) => Ok(()),
                Err(r) => Err(r.message().to_string()),
            }
        }
        Kind::Advance { next, .. } => {
            let old = current(hub, &op.order).map(|v| v.order_json);
            let adv = AdvanceIn { order_id: op.order.clone(), location_id: input.location_id.clone(), next: next.clone(), reason: None, now_ms: now };
            match crate::command::advance::decide(hub, stock, old.as_deref(), &adv) {
                Ok(merged) => {
                    let was: Value = serde_json::from_str(old.as_deref().unwrap_or("{}")).unwrap_or_default();
                    let body = crate::fold::delta(&was, &merged).to_string();
                    wrote.push(Wrote { kind: EventKind::Advanced as u8, order: op.order.clone(), body, next: Some(next.clone()) });
                    Ok(())
                }
                Err(r) => Err(r.message().to_string()),
            }
        }
    });
    (out, wrote)
}

/// The answer on the wire.
pub(crate) fn reply_json(sent: usize, o: &Outcome, notes: usize) -> Value {
    json!({
        "sent": sent,
        "merged": o.merged,
        "applied": o.applied,
        "held": o.held.iter().map(|(pos, why)| json!({"pos": pos, "why": why})).collect::<Vec<_>>(),
        "notes": notes,
    })
}

impl HubImages {
    pub(in crate::hubdo) async fn glue_orders_route(&self, input: GlueOrdersIn) -> Result<Response> {
        if !crate::services::operations::stock::glue::ctx_ok(&input.ctx) {
            return Response::error("ctx: a tablet's name", 400);
        }
        let ops = match read_tail(&input.tail) {
            Ok(o) => o,
            Err(b) => return Response::error(format!("order glue refused (kitchen side): {b:?}"), 422),
        };
        let (log_gen, mut hub) = self.log_hub().await?;
        let (stock_gen, mut stock) = self.stock_log_at(input.now_ms).await?;
        let shelf = stock.len();
        let (out, wrote) = rebase_turn(&mut hub, &mut stock, &input, &ops);
        if !out.law(ops.len()) {
            return Response::error("the order glue lost a tap: merged + held != sent; nothing written", 500);
        }
        let notes = match hold_notes(&mut stock, &input.ctx, &ops, &out.held) {
            Ok(n) => n,
            Err(e) => return Response::error(format!("the held taps could not be noted: {e}; nothing written"), 500),
        };
        let moved = (stock.len() != shelf).then_some((stock_gen, &stock));
        if !wrote.is_empty() {
            let next = match self.write_both("a kitchen tablet's sync", log_gen, &hub, moved).await? {
                Ok(n) => n,
                Err(r) => return Response::error(r.message(), r.status()),
            };
            for w in &wrote {
                self.broadcast(w.kind, &w.order, &w.body, next);
            }
            // THE CUSTOMER IS TOLD as an online bump tells them (`hubdo/orders.rs` `advance`).
            let (_, listed) = self.orders_view().await?;
            let pairs: Vec<(String, String)> = listed.into_iter().map(|o| (o.order_id, o.order_json)).collect();
            for w in wrote.iter().filter(|w| w.next.is_some()) {
                self.tell_orders(Some((&w.order, w.next.as_deref().unwrap_or_default())), &pairs, input.now_ms).await;
            }
        } else if let Some((gen, s)) = moved {
            if self.put_image(crate::hubstore::IMAGE_STOCK, gen, &s.to_bytes_trimmed()).await?.is_none() {
                return Response::error("the stock generation moved during the order glue", 409);
            }
        }
        Response::from_json(&reply_json(ops.len(), &out, notes))
    }
}

#[cfg(test)]
mod tests;
