//! THE CARD REFUND, SENT AND RECORDED (W-REFUND): the outbox drain's arm for
//! `stripe::refund::KIND`, and the one writer of Stripe's word onto an order
//! (used by the drain and by the webhook alike).
//!
//! THE RETRY IS THE OUTBOX'S (`outbox::after_attempt`, six tries over about
//! half an hour), not a second loop. Every try carries the job's key, so
//! Stripe answers a repeat with the refund it already made. A refusal Stripe
//! will repeat (a 4xx) is final at once. Either way the failure is written on
//! the order (`failed: <reason>`, the console's "try again") and said in the
//! venue's audit (`loud!`), which `/api/owner/health` lists.

use serde_json::Value;
use worker::*;
#[allow(unused_imports)] use crate::{edge::{Ctx as RouteContext, Date, Env, ObjectNamespace, Stub}, wire::{Call as Request, Fields as Headers, Reply as Response, RequestInit}};

use super::refund::{classify, form, Job, Sent, API_REFUNDS, STRIPE_VERSION};
use crate::command::refund::card::{apply, close, Update};
use crate::outbox::{after_attempt, Entry, Verdict};

/// `POST /v1/refunds` for `job`, with its Idempotency-Key and the pinned version.
pub async fn post(sk: &str, job: &Job) -> Sent {
    let body = match form(job) {
        Ok(b) => b,
        Err(e) => return Sent::Final(e),
    };
    let mut headers = Headers::new();
    for (k, v) in [
        ("authorization", format!("Bearer {sk}")),
        ("content-type", "application/x-www-form-urlencoded".to_string()),
        ("idempotency-key", job.key.clone()),
        ("stripe-version", STRIPE_VERSION.to_string()),
    ] {
        if let Err(e) = headers.set(k, &v) {
            return Sent::Final(e.to_string());
        }
    }
    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_headers(headers).with_body(Some(body.into()));
    let req = match Request::new_with_init(API_REFUNDS, &init) {
        Ok(r) => r,
        Err(e) => return Sent::Final(e.to_string()),
    };
    match crate::edge::fetch(req).await {
        Ok(mut res) => {
            let code = res.status_code();
            let text = res.text().await.unwrap_or_default();
            classify(code, &text)
        }
        Err(e) => Sent::Transient(e.to_string()),
    }
}

/// Write `up` onto the order, then end the order if Stripe's word covers the
/// card part. `Ok(true)`: something was written. A refusal (`another venue`,
/// `another payment`) comes back as `Err("refused: ...")`, which no retry fixes.
pub async fn record(place: &crate::hubstore::Place, order_id: &str, up: Update, now_ms: i64) -> Result<bool> {
    let venue = place.venue.clone();
    let wrote = crate::hubstore::append_for(place, order_id, now_ms, move |current| {
        let current = current.ok_or_else(|| Error::RustError("order not found".into()))?;
        let old: Value = serde_json::from_str(&current).map_err(|e| Error::RustError(format!("order json unreadable: {e}")))?;
        match apply(&old, &venue, &up) {
            Ok(Some(new)) => Ok(Some((dowiz_hub::EventKind::Noted, crate::fold::delta(&old, &new).to_string(), Value::Bool(true)))),
            Ok(None) => Ok(None),
            Err(crate::command::Refused::NotFound) => Err(Error::RustError("order not found".into())),
            Err(r) => Err(Error::RustError(format!("refused: {}", r.message()))),
        }
    })
    .await?
    .is_some();
    // THE SECOND EDGE, read off the order as it now is: a retry of a lost
    // close finds it still REFUNDING and covered, and closes it then.
    crate::hubstore::append_for(place, order_id, now_ms, move |current| {
        let Some(current) = current else { return Ok(None) };
        let old: Value = serde_json::from_str(&current).unwrap_or_default();
        match close(&old, now_ms) {
            Ok(Some(delta)) => Ok(Some((dowiz_hub::EventKind::Advanced, delta, Value::Bool(true)))),
            Ok(None) => Ok(None),
            Err(r) => Err(Error::RustError(format!("refused: {}", r.message()))),
        }
    })
    .await?;
    Ok(wrote)
}

fn failed(job: &Job, why: String, now_ms: i64) -> Update {
    Update { key: Some(job.key.clone()), pi: Some(job.pi.clone()), venue: Some(job.venue.clone()), status: "failed".into(), failure: Some(why), at: now_ms, ..Update::default() }
}

/// The drain's arm. `Some(ok)`: the drain applies `after_attempt` itself.
/// `None`: the verdict is already in `verdicts`, or the entry waits (no key).
pub async fn send(env: &Env, place: &crate::hubstore::Place, e: &Entry, now_ms: i64, verdicts: &mut Vec<(String, Verdict)>) -> Option<bool> {
    let Ok(job) = serde_json::from_str::<Job>(&e.text) else {
        crate::loud!(&place.ns, Some(&place.venue), "stripe.refund", "{}: unreadable card refund entry", e.id);
        verdicts.push((e.id.clone(), Verdict::Abandon { after: 0 }));
        return None;
    };
    // A RAIL THAT IS NOT CONFIGURED HAS NOT FAILED: the entry waits, visible
    // as outbox depth on the health pane, until the key is back.
    let Some(sk) = super::key(env).ok() else { return None };
    match post(&sk, &job).await {
        Sent::Made { id, status, failure } => {
            let up = Update { key: Some(job.key.clone()), refund_id: Some(id), pi: Some(job.pi.clone()), venue: Some(job.venue.clone()), status, failure, at: now_ms, ..Update::default() };
            // Stripe HAS the refund. A lost record is repaired by its webhook.
            if let Err(err) = record(place, &job.order_id, up, now_ms).await {
                crate::loud!(&place.ns, Some(&place.venue), "stripe.refund", "{}: refunded at Stripe, not recorded on the order: {err}", job.order_id);
            }
            Some(true)
        }
        Sent::Final(why) => {
            let _ = record(place, &job.order_id, failed(&job, why.clone(), now_ms), now_ms).await;
            crate::loud!(&place.ns, Some(&place.venue), "stripe.refund", "{}: Stripe refused the card refund: {why}", job.order_id);
            verdicts.push((e.id.clone(), Verdict::Abandon { after: 0 }));
            None
        }
        Sent::Transient(why) => {
            let v = after_attempt(e, false, now_ms);
            if let Verdict::Abandon { after } = v {
                let said = format!("gave up after {after} tries: {why}");
                let _ = record(place, &job.order_id, failed(&job, said.clone(), now_ms), now_ms).await;
                crate::loud!(&place.ns, Some(&place.venue), "stripe.refund", "{}: {said}", job.order_id);
            }
            verdicts.push((e.id.clone(), v));
            None
        }
    }
}
