//! STRIPE'S WORD ABOUT A REFUND, BY WEBHOOK (W-REFUND). The same door as the
//! payment (`stripe::webhook`): the signature is verified there over the raw
//! body before anything here runs, and each event's fingerprint is kept on
//! the order's card record so a delivery Stripe repeats is applied once.
//!
//! WHICH EVENTS. A Refund object: `refund.created`, `refund.updated`,
//! `refund.failed`, `charge.refund.updated`. A Charge object:
//! `charge.refunded`, whose `refunds.data` (when the endpoint's version still
//! includes it) is read as Refund objects. Anything else stays the payment
//! webhook's business.
//!
//! WHOSE ORDER. The URL names the venue (`Place::of_any`), the refund's own
//! metadata names the order and, for a refund this platform made, the venue
//! and the attempt's key. A refund naming another venue, or another payment
//! than the order's, is REFUSED and acknowledged (200), never applied here: a
//! retry would not make it this venue's.

use serde_json::{json, Value};
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use crate::command::refund::card::Update;

/// Is this event a refund's?
pub fn is_refund_event(kind: &str) -> bool {
    matches!(kind, "refund.created" | "refund.updated" | "refund.failed" | "charge.refund.updated" | "charge.refunded")
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_string)
}

/// One Refund object → (order id, the update). `None` when it names no order.
pub fn of_refund(r: &Value, order_hint: Option<&str>, fingerprint: &str, now_ms: i64) -> Option<(String, Update)> {
    let meta = r.get("metadata");
    let m = |k: &str| meta.and_then(|m| m.get(k)).and_then(Value::as_str).map(str::to_string);
    let order_id = m("order_id").or_else(|| order_hint.map(str::to_string))?;
    let currency = s(r, "currency").unwrap_or_default();
    let amount = r.get("amount").and_then(Value::as_i64).and_then(|a| super::refund::from_stripe(a, &currency));
    Some((
        order_id,
        Update {
            venue: m("venue"),
            pi: s(r, "payment_intent"),
            refund_id: s(r, "id"),
            key: m("key"),
            status: s(r, "status").unwrap_or_default(),
            amount,
            failure: s(r, "failure_reason"),
            fingerprint: Some(fingerprint.to_string()),
            at: now_ms,
        },
    ))
}

/// Every (order, update) one event carries.
pub fn updates(kind: &str, obj: &Value, fingerprint: &str, now_ms: i64) -> Vec<(String, Update)> {
    if kind != "charge.refunded" {
        return of_refund(obj, None, fingerprint, now_ms).into_iter().collect();
    }
    let hint = obj.get("metadata").and_then(|m| m.get("order_id")).and_then(Value::as_str);
    let pi = s(obj, "payment_intent");
    obj.pointer("/refunds/data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, r)| {
            // One fingerprint per refund inside the one event.
            let (o, mut up) = of_refund(r, hint, &format!("{fingerprint}:{i}"), now_ms)?;
            up.pi = up.pi.or_else(|| pi.clone());
            Some((o, up))
        })
        .collect()
}

/// The answer whose status decides whether Stripe sends the event again:
/// only a transient failure is a 503.
pub fn ack(results: &[(String, std::result::Result<bool, String>)]) -> (u16, Value) {
    let mut out = Vec::new();
    let mut retry = false;
    for (order, r) in results {
        out.push(match r {
            Ok(true) => json!({"order": order, "applied": true}),
            Ok(false) => json!({"order": order, "duplicate": true}),
            Err(e) if e.contains("order not found") => json!({"order": order, "unapplied": e}),
            Err(e) if e.contains("refused:") => json!({"order": order, "refused": e}),
            Err(e) => {
                retry = true;
                json!({"order": order, "retry": e})
            }
        });
    }
    if out.is_empty() {
        return (200, json!({"ok": true, "ignored": "no order named"}));
    }
    if retry { (503, json!({"ok": false, "refunds": out})) } else { (200, json!({"ok": true, "refunds": out})) }
}

/// `POST /api/webhooks/stripe` for a refund event; the signature is already checked.
pub async fn handle(req: &Request, ctx: &RouteContext<crate::Req>, event_id: &str, kind: &str, obj: &Value) -> Result<Response> {
    let fingerprint = super::event_fingerprint(event_id);
    let place = match crate::hubstore::Place::of_any(req, ctx).await {
        Ok(p) => p,
        Err(e) => {
            log_line!("stripe.webhook: no venue in this URL: {e}");
            return Response::from_json(&json!({ "ok": true, "ignored": "this URL does not name a venue" }));
        }
    };
    let mut results = Vec::new();
    for (order_id, up) in updates(kind, obj, &fingerprint, ctx.data.now_ms) {
        let r = super::refund_io::record(&place, &order_id, up, ctx.data.now_ms).await.map_err(|e| e.to_string());
        results.push((order_id, r));
    }
    let (status, body) = ack(&results);
    Ok(Response::from_json(&body)?.with_status(status))
}

/// The card refund END TO END in memory: route → outbox → drain → fake Stripe → webhook.
#[cfg(test)]
mod tests;
