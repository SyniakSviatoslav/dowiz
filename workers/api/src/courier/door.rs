//! REFUSED AT THE DOOR: the courier's tap (BLUEPRINT-OPERATIONAL-BLIND-SPOTS
//! §2.4, P1-4). `POST /api/courier/orders/:id/refused`.
//!
//! THE SAME REFUND, UNDER THE COURIER'S PRINCIPAL. Nothing about the refund is
//! decided here: the object runs `command::refund::decide` in one turn with
//! `at_door`, which pins the order to IN_DELIVERY and the reason to
//! `refused_at_door`, records `cash_collected: 0` and the courier as `by`.
//!
//! ONE IMAGE. This handler writes nothing itself; the object writes the log
//! (and the shelf, only for food the kitchen never took). The assignment in the
//! ops image is NOT stamped: every reader of "the run in hand" asks the order
//! (`run::run_over`), so a stamp would be a second write saying the same thing.
//!
//! THE FOOD THAT CAME BACK is the owner's call (§2.4: `Wasted{Returned}` or
//! resellable `Received`, "never inferred"), so this tap writes no stock event
//! for food the kitchen cooked.

use serde_json::Value;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::{courier_at, ops, replay, run, K_ASG};
use crate::command::refund::{RefundIn, RefundOut};

pub async fn refused(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let raw = req.text().await.unwrap_or_default();
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let (courier_id, loc) = match courier_at(&req, &ctx).await {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    let now = ctx.data.now_ms;
    // A tap replayed from the phone's outbox gets the first call's answer.
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &courier_id,
        "courier.refused",
        &format!("{id}:{raw}"),
        now,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };
    // G1 / D1: every exit below is an ANSWER, recorded (or, for a 5xx or an
    // internal error, released) by `answered` -- never a claim left standing.
    let res: Result<Response> = async {
        // THE TAP ALREADY RAN (W-O2): the object wrote the refund and marked
        // this key in that turn. The retry is given that refund, not a 409.
        if let Some(first) = replay::committed::<RefundOut>(idem.committed()) {
            return Response::from_json(&replay::refused_answer(&first));
        }
        // ONLY THE COURIER CARRYING IT. The same answer `deliver` gives.
        if !run::may_refuse(&ops(&place).await?.all(K_ASG), &courier_id, &id) {
            return Response::error("not your delivery", 403);
        }
        let input = RefundIn {
            order_id: id,
            location_id: loc,
            by: courier_id,
            reason: "refused_at_door".into(),
            complete: false,
            now_ms: now,
            at_door: true,
            // `{note?}`: what the courier saw. An unreadable body is no note.
            note: serde_json::from_str::<Value>(&raw).ok().and_then(|b| b.get("note").and_then(Value::as_str).map(String::from)),
        };
        let claim = idem.claim();
        let claimed = crate::idempotency::commit::Claimed { input: &input, idem: claim };
        let out: RefundOut = match crate::command::send(&place, "refund", &claimed).await {
            Ok(v) => v,
            Err((status, said)) => return Response::error(said, status),
        };
        Response::from_json(&replay::refused_answer(&out))
    }
    .await;
    idem.answered(&place, res).await
}
