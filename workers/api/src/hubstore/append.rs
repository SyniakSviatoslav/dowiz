//! APPEND ONE EVENT DECIDED AGAINST THE ORDER AS IT STANDS.
//!
//! Moved out of `hubstore.rs` (W-O2, 2026-09-28) because that file is the
//! file-size ratchet's worst and the append grew a claim; `hubstore`
//! re-exports `append_for`, so every caller is unchanged.
//!
//! THE CLAIM (W-O2, `idempotency::commit::Marked`). A courier's tap whose
//! decisive write IS this append hands its idempotency claim beside the event,
//! with the answer it decided (`out`), and the object marks the claim
//! committed in the turn the event lands. A reply lost after that is a retry
//! answered from `out`, never a second event.

use super::Place;
use crate::idempotency::commit::{Claim, Marked};
use worker::wasm_bindgen::JsValue;
use worker::*;

/// Append one event, retrying if another writer moved the log first.
///
/// `read` is given the CURRENT state of the order (`None` when the hub has
/// never seen it) and returns the event to write: its kind and its payload.
/// Returning `Ok(None)` means "nothing to record", which is not a failure --
/// the Stripe webhook replaying a payment already recorded takes that branch.
///
/// THE RETRY IS THE SAME CONTRACT `with_hub` had. A Durable Object serialises
/// its own requests, so the guard fires only if a second Worker appended
/// between this reader's question and this writer's answer; then the decision
/// is made again against the newer state, which is correct for a log whose
/// events are deltas.
pub async fn append_for<F>(
    place: &Place,
    order_id: &str,
    now_ms: i64,
    decide: F,
) -> Result<Option<serde_json::Value>>
where
    F: FnMut(Option<String>) -> Result<Option<(dowiz_hub::EventKind, String, serde_json::Value)>>,
{
    append_claimed(place, order_id, now_ms, None, decide).await
}

/// `append_for`, carrying `claim`: the object marks it committed with the
/// decided `out` (as JSON) in the turn the event lands. `None` is `append_for`.
pub async fn append_claimed<F>(
    place: &Place,
    order_id: &str,
    now_ms: i64,
    claim: Option<&Claim>,
    mut decide: F,
) -> Result<Option<serde_json::Value>>
where
    F: FnMut(Option<String>) -> Result<Option<(dowiz_hub::EventKind, String, serde_json::Value)>>,
{
    for _ in 0..5 {
        let stub = place.stub()?;
        let req = Request::new(
            &format!("https://hub/fold/order?id={}", crate::mcp::enc(order_id)),
            Method::Get,
        )?;
        let mut res = stub.fetch_with_request(req).await?;
        let generation: i64 =
            res.headers().get("x-generation").ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0);
        let current = match res.status_code() {
            200 => Some(res.json::<crate::hubdo::OrderView>().await?.order_json),
            404 => None,
            other => {
                return Err(Error::RustError(format!("hub object refused an order: {other}")))
            }
        };
        let Some((kind, payload, out)) = decide(current)? else { return Ok(None) };
        let mut body = serde_json::json!({
            "kind": kind as u8,
            "order_id": order_id,
            "payload": payload,
            "clock": now_ms,
        });
        if let Some(c) = claim {
            let mark = Marked { claim: c.clone(), output: out.to_string() };
            body["idem"] = serde_json::to_value(&mark).map_err(|e| Error::RustError(format!("idem mark: {e}")))?;
        }
        let mut write = Request::new_with_init(
            "https://hub/fold/append",
            RequestInit::new()
                .with_method(Method::Post)
                .with_body(Some(JsValue::from_str(&body.to_string()))),
        )?;
        write.headers_mut()?.set("x-generation", &generation.to_string())?;
        write.headers_mut()?.set("content-type", "application/json")?;
        let res = stub.fetch_with_request(write).await?;
        match res.status_code() {
            200 => return Ok(Some(out)),
            // Someone else appended first: ask again and decide again.
            409 => continue,
            other => {
                return Err(Error::RustError(format!("hub object refused an append: {other}")))
            }
        }
    }
    Err(Error::RustError("hub log is contended; five attempts lost the generation guard".into()))
}
