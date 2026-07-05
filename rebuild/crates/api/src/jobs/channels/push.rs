//! VAPID web-push adapter (Q3 🔴) — `docs/design/rebuild-jobs-s8-council/proposal.md` §4.1.
//!
//! ## Dependency choice
//! `web-push` (`--no-default-features`) builds the signed, RFC 8291-encrypted request; the actual
//! POST goes through the SAME `reqwest` client already used by `crate::storage::R2Storage` — no
//! second HTTP-client stack (`isahc`/`hyper`) enters the dependency graph (see `Cargo.toml`'s
//! comment on the dependency).
//!
//! ## VAPID key handling (🔴 red line)
//! The private key is a raw base64url (URL-safe, no padding) P-256 scalar — the same convention
//! Node's `web-push` generator / most VAPID tooling uses (`VapidSignatureBuilder::from_base64`,
//! NOT a PEM). It is held as `config::Secret` end to end (never a bare `String` field on this
//! struct — `Debug`-deriving this struct therefore cannot leak it, see `config.rs`'s `Secret`
//! doc) and is read ONLY at sign time, once per send, never logged, never placed in a DLQ row
//! (a failed send's `SendOutcome` carries no key material — see the variants in
//! `super::SendOutcome`).
//!
//! ## Consent is the CALLER's job, not this adapter's
//! `crate::jobs::consent::customer_push_allowed` must be checked BEFORE calling [`send`] — this
//! module has no opinion on consent, it only sends what it's told to (§4.1: "prefs are re-checked
//! AT DISPATCH TIME", i.e. by the orchestrator, not buried inside the transport adapter).

use std::time::Duration;

use web_push::{
    ContentEncoding, SubscriptionInfo, SubscriptionKeys, VapidSignatureBuilder,
    WebPushMessageBuilder,
};

use crate::config::Secret;
use crate::jobs::channels::SendOutcome;

/// Mirrors the browser `pushSubscription` JSON shape (`customer_devices.push_subscription` /
/// `owner_notification_targets.address`, §4.1).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PushSubscription {
    pub endpoint: String,
    #[serde(rename = "p256dh")]
    pub p256dh: String,
    pub auth: String,
}

/// Every call to the browser-vendor push service (FCM/Mozilla autopush) is bounded by this —
/// threat S8-T11: no external call may pin a held DB connection or hang a worker slot forever.
/// Matches the Telegram/email adapters' own 5s bound (`Q-TG-CIRCUIT`, `Q-EMAIL-DIRECT`).
const SEND_TIMEOUT: Duration = Duration::from_secs(5);

pub struct VapidPushSender {
    client: reqwest::Client,
    private_key: Secret,
    subject: String,
}

impl VapidPushSender {
    /// Registers ONLY when both VAPID keys are present (`NotificationsConfig::vapid_ready`,
    /// carrying `bootstrap/notifications.ts:52`'s gate verbatim) — absent either key, the caller
    /// must not construct this (soft-disable, not a boot failure — same posture as
    /// `MediaConfig`'s optional fields).
    pub fn new(private_key: Secret, subject: String) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder().timeout(SEND_TIMEOUT).build()?;
        Ok(VapidPushSender {
            client,
            private_key,
            subject,
        })
    }

    /// Signs (VAPID) + RFC 8291-encrypts `payload_json`, then POSTs to the subscription's own
    /// endpoint. `payload_json` must already be the FINAL no-PII body (§5: "the customer-status
    /// push body is money-only + a short order id") — this function does not itself apply any
    /// PII policy, it sends exactly the bytes it's given.
    pub async fn send(
        &self,
        subscription: &PushSubscription,
        payload_json: &str,
    ) -> Result<SendOutcome, SendError> {
        let info = SubscriptionInfo {
            endpoint: subscription.endpoint.clone(),
            keys: SubscriptionKeys {
                p256dh: subscription.p256dh.clone(),
                auth: subscription.auth.clone(),
            },
        };

        let mut sig_builder = VapidSignatureBuilder::from_base64(self.private_key.expose(), &info)
            .map_err(|e| SendError::Build(e.to_string()))?;
        sig_builder.add_claim("sub", self.subject.clone());
        let signature = sig_builder
            .build()
            .map_err(|e| SendError::Build(e.to_string()))?;

        let mut builder = WebPushMessageBuilder::new(&info);
        builder.set_payload(ContentEncoding::Aes128Gcm, payload_json.as_bytes());
        builder.set_vapid_signature(signature);
        let message = builder
            .build()
            .map_err(|e| SendError::Build(e.to_string()))?;

        // NOT `web_push::request_builder::build_request` — that helper returns an
        // `http::Request` from the `http` crate's 0.2.x line (web-push's own dependency), while
        // `reqwest` 0.12 is built on `http` 1.x. Two same-named-but-different-major-version
        // crates are two distinct Rust types with no `TryFrom` between them, so this build's
        // request is assembled directly from `WebPushMessage`'s own (public) fields instead —
        // the encryption/signing above is unaffected, only the "turn the already-built message
        // into an HTTP request" step changes.
        let mut request_builder = self
            .client
            .post(message.endpoint.to_string())
            .header("TTL", message.ttl);
        if let Some(urgency) = message.urgency {
            request_builder = request_builder.header("Urgency", urgency.to_string());
        }
        if let Some(topic) = &message.topic {
            request_builder = request_builder.header("Topic", topic.clone());
        }
        if let Some(payload) = message.payload {
            request_builder = request_builder
                .header("Content-Encoding", payload.content_encoding.to_str())
                .header("Content-Type", "application/octet-stream");
            for (name, value) in payload.crypto_headers {
                request_builder = request_builder.header(name, value);
            }
            request_builder = request_builder.body(payload.content);
        }
        let request = request_builder
            .build()
            .map_err(|e| SendError::Build(e.to_string()))?;

        let response = match self.client.execute(request).await {
            Ok(r) => r,
            Err(e) if e.is_timeout() => return Ok(SendOutcome::TimedOut),
            Err(e) => {
                return Ok(SendOutcome::NetworkError {
                    message: e.to_string(),
                });
            }
        };

        let status = response.status();
        if status.is_success() {
            return Ok(SendOutcome::Delivered);
        }

        // 429 is handled BEFORE falling into web_push's own `parse_response` — that function
        // only extracts `retry_after` for 5xx (`WebPushError::ServerError`), not 429, since it
        // was written for a library caller that doesn't itself need queue-level backoff. This
        // adapter DOES (Q-TG-CIRCUIT parity: "429 honors retry-after").
        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(5));
            return Ok(SendOutcome::RateLimited { retry_after });
        }

        // 404/410 (Gone/NotFound) — the subscription is permanently stale (Q-PUSH-PRUNE: "prune
        // subscription on 410/404"). 401/403 are folded into the same bucket — a push service
        // rejecting our VAPID auth for THIS subscription is not something a retry fixes either.
        if matches!(status.as_u16(), 401 | 403 | 404 | 410) {
            return Ok(SendOutcome::PermanentlyRejected {
                reason: status.to_string(),
            });
        }

        Ok(SendOutcome::NetworkError {
            message: format!("unexpected status {status}"),
        })
    }
}

impl std::fmt::Debug for VapidPushSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `private_key` is a `Secret` (redacted `Debug`, config.rs) — spelled out explicitly here
        // anyway so a reviewer never has to trust that transitively.
        f.debug_struct("VapidPushSender")
            .field("private_key", &self.private_key)
            .field("subject", &self.subject)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SendError {
    #[error("failed to build the push request: {0}")]
    Build(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vapid_push_sender_debug_never_prints_the_private_key() {
        let secret_value = "IQ9Ur0ykXoHS9gzfYX0aBjy9lvdrjx_PFUXmie9YRcY";
        let sender = VapidPushSender::new(
            Secret::new(secret_value),
            "mailto:test@example.com".to_string(),
        )
        .expect("client build must succeed");
        let rendered = format!("{sender:?}");
        assert!(!rendered.contains(secret_value));
    }

    #[test]
    fn push_subscription_deserializes_the_browser_pushsubscription_shape() {
        let json = serde_json::json!({
            "endpoint": "https://fcm.googleapis.com/fcm/send/abc",
            "p256dh": "BGa4N1PI79lboMR",
            "auth": "EvcWjEgzr4rb",
        });
        let sub: PushSubscription = serde_json::from_value(json).unwrap();
        assert_eq!(sub.endpoint, "https://fcm.googleapis.com/fcm/send/abc");
        assert_eq!(sub.p256dh, "BGa4N1PI79lboMR");
    }

    // ── live-network proof (requires an actual push subscription endpoint; not run in this
    // sandbox — no network egress assumed available for anything beyond crates.io resolution) ──

    #[tokio::test]
    #[ignore = "requires a real browser push subscription endpoint — run manually against staging"]
    async fn send_delivers_to_a_real_subscription() {
        let sender = VapidPushSender::new(
            Secret::new(std::env::var("VAPID_PRIVATE_KEY").expect("set VAPID_PRIVATE_KEY")),
            "mailto:admin@deliveryos.local".to_string(),
        )
        .unwrap();
        let subscription = PushSubscription {
            endpoint: std::env::var("TEST_PUSH_ENDPOINT").expect("set TEST_PUSH_ENDPOINT"),
            p256dh: std::env::var("TEST_PUSH_P256DH").expect("set TEST_PUSH_P256DH"),
            auth: std::env::var("TEST_PUSH_AUTH").expect("set TEST_PUSH_AUTH"),
        };
        let outcome = sender
            .send(&subscription, r#"{"orderId":"ABCD","status":"delivered"}"#)
            .await
            .expect("send must not error");
        assert_eq!(outcome, SendOutcome::Delivered);
    }
}
