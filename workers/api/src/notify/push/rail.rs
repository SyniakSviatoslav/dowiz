//! THE SEND: one outbox entry of kind `push` to one push service.
//!
//! Runs in the venue's drain (`outbox/rails.rs`), inside the alarm of a
//! Durable Object -- never on a customer's request -- so the two P-256
//! operations (the ECDH of RFC 8291 and the ES256 signature of RFC 8292) are
//! not charged against a request's 10 ms of CPU.
//!
//! THE PUSH SERVICE'S ANSWER (RFC 8030 §5, §7.3; RFC 8292 §4):
//!   2xx      -> delivered to the push service: removed;
//!   404, 410 -> the subscription is gone (uninstalled, permission revoked,
//!               expired): the entry is dropped AND the device's record removed;
//!   400, 413 -> this message can never be accepted: dropped, said out loud;
//!   anything else (401/403 a VAPID refusal, 429, 5xx, no network) -> an
//!               ordinary failed attempt, retried on the outbox's backoff.
//! A drain with no `VAPID_PRIVATE_KEY` (or the wrong one) sends nothing and
//! leaves every push entry waiting, visible on the health pane, as a missing
//! Telegram token does.

use serde::Deserialize;
use serde_json::Value;
use worker::Method;

use super::{ece, subs, vapid};
use crate::outbox::Entry;

/// How long a push service keeps an undelivered message (RFC 8030 §5.2).
/// Half an hour: a "your order is on the way" read the next morning is noise.
pub const TTL_SECS: u32 = 1800;

/// What an entry's text carries (written by `push::plan`).
#[derive(Debug, Deserialize)]
pub struct Payload {
    pub k: String,
    pub id: String,
    pub p256dh: String,
    pub auth: String,
    pub msg: Value,
}

/// The outcome the drain applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Push {
    /// Not configured: the entry waits, no try spent.
    Wait,
    /// Attempted: `true` = delivered, `false` = an ordinary failure (backoff).
    Done(bool),
    /// Never deliverable: remove the entry now.
    Drop,
}

/// Classify a push service's status. PURE.
pub fn classify(status: u16) -> (Push, bool) {
    match status {
        200..=299 => (Push::Done(true), false),
        404 | 410 => (Push::Drop, true),
        400 | 413 => (Push::Drop, false),
        _ => (Push::Done(false), false),
    }
}

/// The `Topic` header (RFC 8030 §5.4): one per order and audience, so a phone
/// that was offline gets the order's LATEST state, not five stale ones.
/// At most 32 characters of the base64url alphabet.
pub fn topic(entry_id: &str) -> String {
    let (order, rest) = entry_id.split_once("/push/").unwrap_or((entry_id, ""));
    let who = rest.split('/').nth(1).unwrap_or("");
    format!("{order}{who}").chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(32).collect()
}

/// One drain's push state: the signer (made once, on the first push entry)
/// and the devices found gone.
#[derive(Default)]
pub struct Rail {
    signer: Option<Result<vapid::Signer, String>>,
    pub gone: Vec<(String, String)>,
    pub said: Vec<String>,
}

impl Rail {
    /// Attempt one entry.
    pub async fn send(&mut self, env: &crate::edge::Env, e: &Entry, now_ms: i64) -> Push {
        if self.signer.is_none() {
            let made = env
                .secret(vapid::SECRET_NAME)
                .map_err(|_| format!("push: the Worker secret {} is not set; push messages wait", vapid::SECRET_NAME))
                .and_then(|s| vapid::Signer::new(&s, &vapid::public_key(env)).map_err(|e| format!("push: {} is not the pair of the public key browsers hold ({e:?}); push messages wait", vapid::SECRET_NAME)));
            if let Err(why) = &made {
                self.said.push(why.clone());
            }
            self.signer = Some(made);
        }
        let Some(Ok(signer)) = &self.signer else { return Push::Wait };
        let Ok(p) = serde_json::from_str::<Payload>(&e.text) else {
            self.said.push(format!("{}: unreadable push entry", e.id));
            return Push::Drop;
        };
        let (push, gone) = deliver(signer, e, &p, now_ms).await;
        if gone {
            self.gone.push((p.k.clone(), p.id.clone()));
        } else if push == Push::Drop {
            self.said.push(format!("{}: the push service refused this message for good", e.id));
        }
        push
    }

    /// After the drain's write: remove the gone devices' records. What must
    /// be said out loud is in `said`, which the drain takes BEFORE its write
    /// (a drain that changes nothing returns early and must still say it).
    pub async fn finish(self, place: &crate::hubstore::Place) -> Vec<String> {
        let mut said = Vec::new();
        if !self.gone.is_empty() {
            let gone = self.gone.clone();
            let n = gone.len();
            let r = crate::hubstore::with_table(place, subs::IMAGE_PUSH, subs::PUSH_BYTES, move |t| {
                for (k, id) in &gone {
                    t.remove(k, id);
                }
                Ok(())
            })
            .await;
            if let Err(e) = r {
                said.push(format!("push: {n} gone device(s) could not be removed: {e}"));
            }
        }
        said
    }
}

/// Seal, sign and post one message. `(outcome, device gone)`.
pub async fn deliver(signer: &vapid::Signer, e: &Entry, p: &Payload, now_ms: i64) -> (Push, bool) {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
    use base64::Engine;
    let (Ok(ua), Ok(auth)) = (B64.decode(&p.p256dh), B64.decode(&p.auth)) else { return (Push::Drop, true) };
    let Ok(body) = ece::encrypt(p.msg.to_string().as_bytes(), &ua, &auth) else { return (Push::Drop, true) };
    let Ok(authz) = signer.authorization(&e.to, now_ms) else { return (Push::Drop, true) };
    let mut h = crate::wire::Fields::new();
    for (k, v) in [
        ("content-encoding", "aes128gcm"),
        ("content-type", "application/octet-stream"),
        ("ttl", &TTL_SECS.to_string()),
        ("urgency", "high"),
        ("topic", &topic(&e.id)),
        ("authorization", &authz),
    ] {
        if h.set(k, v).is_err() {
            return (Push::Done(false), false);
        }
    }
    let init = crate::wire::RequestInit::new().with_method(Method::Post).with_headers(h).with_body(Some(body)).clone();
    let Ok(req) = crate::wire::Call::new_with_init(&e.to, &init) else { return (Push::Drop, false) };
    match crate::edge::fetch(req).await {
        Ok(res) => classify(res.status_code()),
        Err(_) => (Push::Done(false), false),
    }
}

#[cfg(test)]
mod tests;
