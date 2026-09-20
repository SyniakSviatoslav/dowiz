//! The Stripe rail, as a Worker speaks it.
//!
//! WHY NOT THE EXISTING ADAPTER. `payment-adapters/src/stripe.rs` implements the
//! kernel's `PaymentProvider` port over `ureq` — raw sockets, which a Worker does
//! not have. The port's SHAPE is still the contract this follows (create with an
//! idempotency key, query by that key, verify a webhook, capture, void, refund);
//! only the transport differs, because on Workers the transport is `fetch`.
//!
//! NO CARD DATA CROSSES THIS BOUNDARY, by construction rather than by policy.
//! The Worker creates a PaymentIntent and hands the browser a `client_secret`;
//! the card details go from the customer's browser to Stripe directly and never
//! touch dowiz. There is no type here that could hold a PAN.
//!
//! MONEY STAYS INTEGER. Stripe's `amount` is minor units, which is the same unit
//! the kernel uses, so nothing is converted and no float appears on this path.

use serde::Deserialize;
use worker::*;

use crate::auth::sha256_hex;

const API: &str = "https://api.stripe.com/v1";
/// Stripe rejects a timestamp outside this window; so do we, before spending a
/// signature comparison on a replayed event.
const WEBHOOK_TOLERANCE_S: i64 = 300;

#[derive(Debug)]
pub enum PayError {
    /// No key configured. The rail is OFF, and that is a 503, not a silent skip.
    NotConfigured,
    Upstream(String),
    BadSignature,
    Replayed,
}

impl PayError {
    pub fn into_response(self) -> Result<Response> {
        match self {
            PayError::NotConfigured => Response::error("card payments are not configured", 503),
            PayError::Upstream(e) => Response::error(format!("payment provider: {e}"), 502),
            PayError::BadSignature => Response::error("bad webhook signature", 400),
            PayError::Replayed => Response::error("webhook timestamp outside tolerance", 400),
        }
    }
}

fn key(env: &Env) -> std::result::Result<String, PayError> {
    env.secret("STRIPE_SECRET_KEY")
        .map(|v| v.to_string())
        .map_err(|_| PayError::NotConfigured)
}

/// Create a PaymentIntent for an order.
///
/// `idempotency_key` is the ORDER ID. Stripe then guarantees that a retried
/// request — a flaky connection, a double tap, a Worker replay after a lost
/// generation guard — returns the SAME intent instead of charging twice. That is
/// the property worth more than anything else on this path.
pub async fn create_intent(
    env: &Env,
    order_id: &str,
    amount_minor: i64,
    currency: &str,
) -> std::result::Result<(String, String), PayError> {
    if amount_minor <= 0 {
        return Err(PayError::Upstream("amount must be positive".into()));
    }
    let sk = key(env)?;
    let body = format!(
        "amount={amount_minor}&currency={}&automatic_payment_methods[enabled]=true&metadata[order_id]={}",
        currency.to_lowercase(),
        urlencode(order_id)
    );

    let mut headers = Headers::new();
    headers
        .set("authorization", &format!("Bearer {sk}"))
        .map_err(|e| PayError::Upstream(e.to_string()))?;
    headers
        .set("content-type", "application/x-www-form-urlencoded")
        .map_err(|e| PayError::Upstream(e.to_string()))?;
    headers
        .set("idempotency-key", order_id)
        .map_err(|e| PayError::Upstream(e.to_string()))?;

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(body.into()));
    let req = Request::new_with_init(&format!("{API}/payment_intents"), &init)
        .map_err(|e| PayError::Upstream(e.to_string()))?;
    let mut res = Fetch::Request(req)
        .send()
        .await
        .map_err(|e| PayError::Upstream(e.to_string()))?;

    #[derive(Deserialize)]
    struct Intent {
        id: String,
        client_secret: Option<String>,
    }
    if res.status_code() >= 400 {
        let t = res.text().await.unwrap_or_default();
        return Err(PayError::Upstream(t.chars().take(300).collect()));
    }
    let intent: Intent = res
        .json()
        .await
        .map_err(|e| PayError::Upstream(e.to_string()))?;
    let secret = intent
        .client_secret
        .ok_or_else(|| PayError::Upstream("intent carried no client_secret".into()))?;
    Ok((intent.id, secret))
}

/// Verify a Stripe webhook signature: `t=<ts>,v1=<hex>` over `"<ts>.<payload>"`.
///
/// Both halves matter. Without the timestamp check a captured signature replays
/// forever; without the constant-time compare the signature leaks byte by byte
/// through timing. Stripe's own library does both, and reimplementing it in a
/// Worker means reimplementing BOTH.
pub fn verify_webhook(
    env: &Env,
    payload: &str,
    sig_header: &str,
    now_s: i64,
) -> std::result::Result<(), PayError> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let secret = env
        .secret("STRIPE_WEBHOOK_SECRET")
        .map(|v| v.to_string())
        .map_err(|_| PayError::NotConfigured)?;

    let mut ts: Option<i64> = None;
    let mut sigs: Vec<&str> = Vec::new();
    for part in sig_header.split(',') {
        match part.split_once('=') {
            Some(("t", v)) => ts = v.trim().parse().ok(),
            Some(("v1", v)) => sigs.push(v.trim()),
            _ => {}
        }
    }
    let ts = ts.ok_or(PayError::BadSignature)?;
    if (now_s - ts).abs() > WEBHOOK_TOLERANCE_S {
        return Err(PayError::Replayed);
    }
    if sigs.is_empty() {
        return Err(PayError::BadSignature);
    }

    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes())
        .map_err(|_| PayError::NotConfigured)?;
    mac.update(format!("{ts}.{payload}").as_bytes());
    let expected = mac.finalize().into_bytes();
    let expected_hex: String = expected.iter().map(|b| format!("{b:02x}")).collect();

    // Stripe may send several v1 signatures during a secret roll; any one
    // matching is valid. Every candidate is compared in constant time.
    use subtle::ConstantTimeEq;
    let ok = sigs.iter().any(|s| {
        s.len() == expected_hex.len() && bool::from(s.as_bytes().ct_eq(expected_hex.as_bytes()))
    });
    if ok {
        Ok(())
    } else {
        Err(PayError::BadSignature)
    }
}

/// A stable id for an event, so the same delivery is not applied twice. Stripe
/// retries webhooks for days; an un-deduplicated handler applies them all.
pub fn event_fingerprint(event_id: &str) -> String {
    sha256_hex(event_id)
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `POST /api/webhooks/stripe`
///
/// Two things here are easy to get wrong and both are fatal if you do. The
/// signature is computed over the RAW body, so the payload is read as text and
/// never round-tripped through a parser first. And Stripe retries a delivery for
/// days, so an un-deduplicated handler applies the same event over and over --
/// this one returns 200 for an already-applied event, because 200 is what stops
/// the retries.
pub async fn webhook(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let sig = req
        .headers()
        .get("stripe-signature")
        .ok()
        .flatten()
        .unwrap_or_default();
    let raw = req.text().await?;
    let now_s = (Date::now().as_millis() / 1000) as i64;

    if let Err(e) = verify_webhook(&ctx.env, &raw, &sig, now_s) {
        return e.into_response();
    }

    #[derive(Deserialize)]
    struct Event {
        id: String,
        #[serde(rename = "type")]
        kind: String,
        data: EventData,
    }
    #[derive(Deserialize)]
    struct EventData {
        object: serde_json::Value,
    }
    let ev: Event = match serde_json::from_str(&raw) {
        Ok(e) => e,
        Err(e) => return Response::error(format!("unreadable event: {e}"), 400),
    };

    // Anything else is acknowledged and ignored: an unhandled type is not an
    // error, and answering non-200 would make Stripe retry it forever.
    if ev.kind != "payment_intent.succeeded" {
        return Response::from_json(&serde_json::json!({ "ok": true, "ignored": ev.kind }));
    }

    let Some(order_id) = ev
        .data
        .object
        .get("metadata")
        .and_then(|m| m.get("order_id"))
        .and_then(|x| x.as_str())
        .map(str::to_string)
    else {
        return Response::from_json(&serde_json::json!({ "ok": true, "ignored": "no order_id" }));
    };
    let amount = ev
        .data
        .object
        .get("amount_received")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    let intent_id = ev
        .data
        .object
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let fingerprint = event_fingerprint(&ev.id);

    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let oid = order_id.clone();
    let applied = crate::hubstore::with_hub(&place, move |hub| {
        // Already paid? Then this is a retry of a delivery we handled.
        let already = hub
            .events()
            .into_iter()
            .any(|e| e.order_id == oid && e.kind == dowiz_hub::EventKind::Paid);
        if already {
            return Ok(false);
        }
        let current = crate::hubstore::order_state(hub, &oid)
            .ok_or_else(|| Error::RustError("order not found".into()))?;
        let old: serde_json::Value = serde_json::from_str(&current).unwrap_or_default();
        let mut v = old.clone();
        v["payment_status"] = serde_json::json!("paid");
        v["payment_intent"] = serde_json::json!(intent_id);
        // Recorded as RECEIVED, not as the order total. If they differ, the
        // difference is the thing someone will need later.
        v["amount_received"] = serde_json::json!(amount);
        v["stripe_event"] = serde_json::json!(fingerprint);
        let body = crate::fold::delta(&old, &v).to_string();
        hub.append(
            dowiz_hub::EventKind::Paid,
            &oid,
            &body,
            now_s as u64 * 1000,
            [0u8; 32],
        )
        .map_err(|e| Error::RustError(format!("hub append failed: {e:?}")))?;
        Ok(true)
    })
    .await;

    match applied {
        Ok(true) => Response::from_json(&serde_json::json!({ "ok": true, "applied": order_id })),
        Ok(false) => {
            Response::from_json(&serde_json::json!({ "ok": true, "duplicate": order_id }))
        }
        // An order we do not know is still acknowledged: retrying will not make
        // it appear, and a 500 here just fills Stripe's retry queue.
        Err(e) => Response::from_json(&serde_json::json!({
            "ok": true, "unapplied": e.to_string()
        })),
    }
}
