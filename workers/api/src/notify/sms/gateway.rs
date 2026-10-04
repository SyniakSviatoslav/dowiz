//! THE THREE PROVIDERS: the request each wants, and its answer, classified.
//! The request and the classification are PURE; `post` is the one fetch.
//!
//!   smsgate  POST {url} (default https://api.sms-gate.app/3rdparty/v1/messages)
//!            Basic user:password (shown in the SMSGate app, Cloud server)
//!            {"id", "textMessage": {"text"}, "phoneNumbers": [to], "ttl"}
//!            202 queued for the phone; 409 = this id is already queued (a
//!            retried drain), which is delivery, not failure; 503 = queue
//!            full or the phone has not been seen (OpenAPI v1.49.0,
//!            `api.sms-gate.app/docs/doc.json`, read 2026-10-03).
//!   textbee  POST {url} (default https://api.textbee.dev/api/v1/gateway/send-sms)
//!            x-api-key; {"recipients": [to], "message"} (textbee README, 2026-10-04)
//!   twilio   POST https://api.twilio.com/2010-04-01/Accounts/{SID}/Messages.json
//!            Basic SID:token; form To, From, Body; 201 created.
//!
//! THE ANSWER:
//!   2xx (and smsgate's 409)  -> sent
//!   400, 404, 413, 422       -> this message can never be sent (a number the
//!                               provider refuses): dropped, said out loud
//!   401, 403                 -> the venue's credentials are wrong: an ordinary
//!                               failed try, so six of them end loud, and the
//!                               health record names the cause
//!   429, 5xx, no network     -> retried on the outbox's backoff

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde_json::json;

use super::config::{Cfg, Provider};

/// How long SMSGate keeps an unsent message for an offline phone: an hour.
/// "Your order is on the way" read the next morning is noise.
pub const TTL_SECS: u32 = 3600;

/// One request, as plain data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Req {
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    pub body: String,
}

/// SMSGate's message id: at most 36 characters, stable per outbox entry, so a
/// drain that retries after a lost answer hears 409, not a second SMS.
pub fn message_id(entry_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let h = Sha256::digest(entry_id.as_bytes());
    h[..16].iter().map(|b| format!("{b:02x}")).collect()
}

fn form(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", v.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>()))
        .collect::<Vec<_>>()
        .join("&")
}

/// The request for one text. PURE.
pub fn request(c: &Cfg, to: &str, text: &str, entry_id: &str) -> Req {
    let basic = |u: &str, p: &str| format!("Basic {}", B64.encode(format!("{u}:{p}")));
    match c.provider {
        Provider::SmsGate => Req {
            url: c.url.clone(),
            headers: vec![("content-type", "application/json".into()), ("authorization", basic(&c.user, &c.secret))],
            body: json!({ "id": message_id(entry_id), "textMessage": { "text": text }, "phoneNumbers": [to], "ttl": TTL_SECS }).to_string(),
        },
        Provider::TextBee => Req {
            url: c.url.clone(),
            headers: vec![("content-type", "application/json".into()), ("x-api-key", c.secret.clone())],
            body: json!({ "recipients": [to], "message": text }).to_string(),
        },
        Provider::Twilio => Req {
            url: format!("{}/{}/Messages.json", super::config::TWILIO_BASE, c.user),
            headers: vec![("content-type", "application/x-www-form-urlencoded".into()), ("authorization", basic(&c.user, &c.secret))],
            body: form(&[("To", to), ("From", &c.from), ("Body", text)]),
        },
    }
}

/// What the drain does with an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Sent,
    /// Never sendable: drop it now.
    Refused,
    /// An ordinary failed try (backoff).
    Failed,
}

/// Classify a provider's status. PURE.
pub fn classify(p: Provider, status: u16) -> Outcome {
    match status {
        200..=299 => Outcome::Sent,
        409 if p == Provider::SmsGate => Outcome::Sent,
        400 | 404 | 413 | 422 => Outcome::Refused,
        _ => Outcome::Failed,
    }
}

/// The console key for a failure, so the owner reads the cause in their language.
pub fn why(status: u16) -> &'static str {
    match status {
        0 => "sms_why_network",
        401 | 403 => "sms_why_auth",
        400 | 404 | 413 | 422 => "sms_why_refused",
        429 => "sms_why_busy",
        503 => "sms_why_offline",
        _ => "sms_why_provider",
    }
}

/// Post one request. `(status, provider's words)`; status 0 = no answer.
pub async fn post(r: &Req) -> (u16, String) {
    let mut h = crate::wire::Fields::new();
    for (k, v) in &r.headers {
        if h.set(k, v).is_err() {
            return (0, "header refused".into());
        }
    }
    let init = crate::wire::RequestInit::new().with_method(worker::Method::Post).with_headers(h).with_body(Some(r.body.clone().into_bytes())).clone();
    let Ok(req) = crate::wire::Call::new_with_init(&r.url, &init) else { return (0, "bad gateway address".into()) };
    match crate::edge::fetch(req).await {
        Ok(mut res) => {
            let code = res.status_code();
            let words: String = res.text().await.unwrap_or_default().chars().take(200).collect();
            (code, words)
        }
        Err(e) => (0, e.to_string().chars().take(200).collect()),
    }
}

/// THE CUSTOMER DOOR (`tools/gates/consent.sh` counts it): one order-status
/// text, which cannot be called without the fold's witness that this
/// customer ticked the SMS box and has not said STOP since.
pub async fn sms_text(c: &Cfg, who: &dowiz_hub::consent::Consented, to: &str, text: &str, entry_id: &str) -> (Outcome, u16) {
    debug_assert_eq!(who.channel(), dowiz_hub::consent::CHANNEL_SMS);
    let (code, _) = post(&request(c, to, text, entry_id)).await;
    (if code == 0 { Outcome::Failed } else { classify(c.provider, code) }, code)
}

#[cfg(test)]
mod tests;
