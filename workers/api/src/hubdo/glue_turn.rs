//! SHEAF S2, IN THE VENUE'S OBJECT, IN ONE TURN: a kitchen tablet's tail glued against the stock
//! log and adopted (`dowiz_hub::glue::merge`), the stock image written under its generation, and
//! the reply the tablet replays (`/fold/glue`); and the decision list a glue leaves for a person,
//! with its one action, close (`/fold/desk`). The contract is `services/operations/stock/glue.rs`.
//!
//! A refusal writes nothing: the merge runs on the loaded copy and only an `Ok` is persisted.

use super::HubImages;
use crate::services::operations::stock::glue::{DeskIn, GlueIn};
use dowiz_hub::glue::merge::{merge, Entry, Reply as Merged};
use dowiz_hub::glue::wire::Tail;
use dowiz_hub::glue::{desk, Conflict, Side, Why};
use dowiz_hub::stock::StockLog;
use serde_json::{json, Value};
use worker::*;
use crate::wire::Reply as Response;

/// 64 hex characters as a record id.
fn id_of(s: &str) -> Option<[u8; 32]> {
    let v = dowiz_hub::crypto::unhex(s)?;
    v.try_into().ok()
}

fn side(s: Side) -> &'static str {
    if s == Side::Hub { "hub" } else { "kitchen" }
}

/// A conflict as the board reads it.
pub(crate) fn conflict_json(c: &Conflict) -> Value {
    match c {
        Conflict::Oversold { item, merged, .. } => json!({"kind": "oversold", "item": item, "qty": merged.on_hand.min(merged.available())}),
        Conflict::Recount { item, absolute_in } => json!({"kind": "recount", "item": item, "side": side(*absolute_in)}),
        Conflict::DoubleTake { order_id, item, what, merged } => json!({"kind": "double", "item": item, "order": order_id, "what": what, "qty": merged}),
        Conflict::Diverged { item, .. } => json!({"kind": "diverged", "item": item}),
        Conflict::Provisional { side: s, item, order_id, qty, .. } => json!({"kind": "provisional", "item": item, "order": order_id, "qty": qty, "side": side(*s)}),
    }
}

/// The reply on the wire: an echo is its index, a hub record its text.
pub(crate) fn reply_json(r: &Merged) -> Value {
    let tail: Vec<Value> = r.entries.iter().map(|e| match e { Entry::Rec(s) => json!(s), Entry::Echo(i) => json!(i) }).collect();
    json!({
        "tip": dowiz_hub::crypto::hex(&r.tip),
        "tail": tail,
        "merged": r.merged,
        "held": r.held.iter().map(|(pos, why)| json!({"pos": pos, "why": why})).collect::<Vec<_>>(),
        "conflicts": r.conflicts.iter().map(conflict_json).collect::<Vec<_>>(),
        "grants": r.grants,
        "replay": r.replay,
        "wire_up": r.wire_up,
        "wire_down": r.wire_down(),
    })
}

/// The status a glue refusal answers: 409 when the hub no longer holds the tablet's base (it
/// starts over), 500 for the hub's own image, 422 for what the tablet sent.
fn status_of(side: Side, why: &Why) -> u16 {
    match (side, why) {
        (_, Why::NoCommonAncestor) => 409,
        (Side::Hub, _) => 500,
        (Side::Kitchen, _) => 422,
    }
}

impl HubImages {
    async fn stock_bytes(&self) -> Result<(i64, Vec<u8>)> {
        Ok(match self.image(crate::hubstore::IMAGE_STOCK).await? {
            Some((meta, bytes)) => (meta.generation, bytes),
            None => (0, StockLog::create_sized(64 * 1024).map_err(|_| Error::RustError("cannot create stock image".into()))?.to_bytes_trimmed()),
        })
    }

    pub(super) async fn glue_route(&self, input: GlueIn) -> Result<Response> {
        let (Some(base), Some(tip)) = (id_of(&input.base), id_of(&input.tip)) else {
            return Response::error("base and tip are record ids, 64 hex characters", 400);
        };
        if !crate::services::operations::stock::glue::ctx_ok(&input.ctx) {
            return Response::error("ctx: a tablet's name", 400);
        }
        let (gen, bytes) = self.stock_bytes().await?;
        let mut log = StockLog::load(&bytes).map_err(|_| Error::RustError("stock image is unreadable".into()))?;
        log.set_clock(input.now_ms);
        if let Some(seed) = crate::entropy_seed() {
            log.set_entropy(seed); // M0
        }
        let before = log.len();
        let t = Tail { ctx: input.ctx, base, tip, recs: input.tail };
        let r = match merge(&bytes, &mut log, &t) {
            Ok(r) => r,
            Err(e) => return Response::error(format!("glue refused ({} side): {:?}", side(e.side), e.why), status_of(e.side, &e.why)),
        };
        // THE MERGE LAW, checked before anything is persisted: every record the tablet sent is a
        // movement or a held decision. A merge that lost one is refused whole.
        if !r.law(t.recs.len()) {
            return Response::error("the glue lost a record: merged + held != sent; nothing written", 500);
        }
        if log.len() != before && self.put_image(crate::hubstore::IMAGE_STOCK, gen, &log.to_bytes_trimmed()).await?.is_none() {
            return Response::error("the stock generation moved during the glue", 409);
        }
        Response::from_json(&reply_json(&r))
    }

    pub(super) async fn desk_route(&self, input: DeskIn) -> Result<Response> {
        let (gen, bytes) = self.stock_bytes().await?;
        let mut log = StockLog::load(&bytes).map_err(|_| Error::RustError("stock image is unreadable".into()))?;
        if let Some(seed) = crate::entropy_seed() {
            log.set_entropy(seed); // M0
        }
        let mut closed = false;
        if let Some(did) = &input.did {
            closed = match desk::close(&mut log, did, &input.by) {
                Ok(c) => c,
                Err(e) => return Response::error(e.to_string(), 400),
            };
            if closed && self.put_image(crate::hubstore::IMAGE_STOCK, gen, &log.to_bytes_trimmed()).await?.is_none() {
                return Response::error("the stock generation moved during the close", 409);
            }
        }
        let open: Vec<Value> = desk::open(&log)
            .into_iter()
            .map(|d| json!({"did": d.did, "ctx": d.ctx, "kind": d.kind, "item": d.item, "order": d.order, "qty": d.qty, "why": d.why, "rec": d.rec, "at": d.at}))
            .collect();
        Response::from_json(&json!({"closed": closed, "open": open}))
    }
}

/// S2b: the kitchen tablet's offline order taps (`/fold/glue_orders`).
mod orders;

#[cfg(test)]
#[path = "glue_turn/tests.rs"]
mod tests;
