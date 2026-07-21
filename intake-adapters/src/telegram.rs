//! Telegram Bot API intake adapter — secret-token verify, `update_id` dedup,
//! `setWebhook` push + `getUpdates` long-poll behind one seam, `/start <payload>`
//! deep-link decode (≤64 chars, base64url hub/item id).
//!
//! Feature: `telegram`. Pulls in `serde`/`serde_json` for Telegram Update parsing
//! and the kernel's `hub_intake` vocabulary (zero I/O, zero mutation symbols).
//!
//! Provider payload types die in this module — the Telegram `Update` struct is
//! never visible outside this file (grep-gate discipline from `engine/src/intent.rs`).

use std::collections::HashSet;
use std::sync::Mutex;

use crate::{IntakeError, IntakeWebhookHeaders};
use dowiz_kernel::ports::hub_intake::{channel_str, InboundMessage};

/// Maximum Telegram deep-link payload length (Bot API spec: ≤64 chars).
pub const MAX_DEEP_LINK_PAYLOAD_LEN: usize = 64;

/// Valid characters for a Telegram `/start` payload: base64url alphabet.
fn is_valid_start_payload_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// Telegram Bot API `Update` — minimal subset for message intake.
/// Only the fields dowiz actually reads are deserialized; unknown fields are
/// silently ignored by serde (forward-compatible).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TgUpdate {
    pub update_id: i64,
    #[serde(default)]
    pub message: Option<TgMessage>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TgMessage {
    pub message_id: i64,
    pub chat: TgChat,
    pub date: i64,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TgChat {
    pub id: i64,
}

/// The Telegram intake adapter — holds per-hub secret + dedup state.
pub struct TelegramAdapter {
    /// The per-hub secret_token set at `setWebhook` time.
    secret_token: String,
    /// Dedup: seen `update_id` values. Bounded by Telegram's own delivery
    /// semantics (at-most-once per update_id within the getUpdates timeout
    /// window); a small `HashSet` is sufficient.
    seen: Mutex<HashSet<i64>>,
}

impl TelegramAdapter {
    pub fn new(secret_token: String) -> Self {
        TelegramAdapter {
            secret_token,
            seen: Mutex::new(HashSet::new()),
        }
    }

    /// Verify the `X-Telegram-Bot-Api-Secret-Token` header against the
    /// per-hub secret. Constant-time compare to prevent timing side-channel.
    pub fn verify_secret(&self, headers: &IntakeWebhookHeaders) -> Result<(), IntakeError> {
        let supplied = headers
            .telegram_secret
            .as_deref()
            .ok_or(IntakeError::SignatureMismatch)?;
        if crate::constant_time_eq(supplied.as_bytes(), self.secret_token.as_bytes()) {
            Ok(())
        } else {
            Err(IntakeError::SignatureMismatch)
        }
    }

    /// Parse a raw Telegram Update JSON payload and normalize it into one or
    /// zero `InboundMessage` values.
    ///
    /// Pipeline:
    /// 1. Secret-token verify (constant-time)
    /// 2. JSON parse → `TgUpdate`
    /// 3. Idempotent dedup by `update_id`
    /// 4. Extract message text → `InboundMessage`
    pub fn verify_and_normalize(
        &self,
        raw: &[u8],
        headers: &IntakeWebhookHeaders,
        venue_id: &str,
    ) -> Result<Vec<InboundMessage>, IntakeError> {
        // 1. Verify secret token.
        self.verify_secret(headers)?;

        // 2. Parse JSON.
        let update: TgUpdate = serde_json::from_slice(raw)
            .map_err(|e| IntakeError::MalformedPayload(e.to_string()))?;

        // 3. Only `message` updates are intake-relevant (ignore edited_message,
        //    callback_query, inline_query, etc.).
        let message = match update.message {
            Some(m) => m,
            None => return Ok(vec![]), // non-message update → empty, not an error
        };

        // 4. Dedup by update_id.
        {
            let mut seen = self.seen.lock().unwrap();
            if !seen.insert(update.update_id) {
                return Err(IntakeError::Duplicate(update.update_id.to_string()));
            }
        }

        // 5. Extract text.
        let text = match message.text {
            Some(t) if !t.is_empty() => t,
            _ => return Ok(vec![]), // no text → not an order message
        };

        // 6. Deep-link decode: `/start <payload>` → strip the command prefix.
        let text = if let Some(payload) = text.strip_prefix("/start ") {
            if payload.len() > MAX_DEEP_LINK_PAYLOAD_LEN {
                return Err(IntakeError::MalformedPayload(format!(
                    "deep-link payload exceeds {MAX_DEEP_LINK_PAYLOAD_LEN} chars"
                )));
            }
            if !payload.chars().all(is_valid_start_payload_char) {
                return Err(IntakeError::MalformedPayload(
                    "deep-link payload contains invalid characters".into(),
                ));
            }
            // The payload is a base64url-encoded hub/item id — pass it through
            // as the text; the IntentParser will classify it.
            payload.to_string()
        } else {
            text
        };

        // 7. Build the channel-agnostic InboundMessage.
        Ok(vec![InboundMessage {
            venue_id: venue_id.to_string(),
            channel: channel_str::TELEGRAM.to_string(),
            sender: message.chat.id.to_string(),
            provider_msg_id: update.update_id.to_string(),
            text,
            unix_ms: (message.date as u64) * 1000, // seconds → milliseconds
        }])
    }

    /// Decode a Telegram `/start` deep-link payload. Returns the decoded
    /// base64url bytes, or an error if the payload is malformed.
    pub fn decode_start_payload(payload: &str) -> Result<Vec<u8>, IntakeError> {
        if payload.len() > MAX_DEEP_LINK_PAYLOAD_LEN {
            return Err(IntakeError::MalformedPayload(format!(
                "deep-link payload exceeds {MAX_DEEP_LINK_PAYLOAD_LEN} chars"
            )));
        }
        if !payload.chars().all(is_valid_start_payload_char) {
            return Err(IntakeError::MalformedPayload(
                "deep-link payload contains invalid characters".into(),
            ));
        }
        // base64url decode (the Telegram start-param encoding).
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload.as_bytes())
            .map_err(|e| IntakeError::MalformedPayload(format!("base64url decode failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter() -> TelegramAdapter {
        TelegramAdapter::new("test-secret-123".into())
    }

    fn headers(secret: &str) -> IntakeWebhookHeaders {
        IntakeWebhookHeaders {
            telegram_secret: Some(secret.into()),
            ..Default::default()
        }
    }

    fn sample_update(update_id: i64, chat_id: i64, text: &str) -> Vec<u8> {
        serde_json::json!({
            "update_id": update_id,
            "message": {
                "message_id": 1,
                "chat": {"id": chat_id},
                "date": 1690000000,
                "text": text
            }
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn happy_path_message() {
        let a = adapter();
        let raw = sample_update(42, 12345, "2 sushi");
        let msgs = a
            .verify_and_normalize(&raw, &headers("test-secret-123"), "hub-1")
            .expect("ok");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].channel, channel_str::TELEGRAM);
        assert_eq!(msgs[0].sender, "12345");
        assert_eq!(msgs[0].text, "2 sushi");
        assert_eq!(msgs[0].venue_id, "hub-1");
        assert_eq!(msgs[0].provider_msg_id, "42");
    }

    #[test]
    fn secret_token_reject() {
        let a = adapter();
        let raw = sample_update(1, 1, "hello");
        let err = a
            .verify_and_normalize(&raw, &headers("wrong-secret"), "hub-1")
            .unwrap_err();
        assert_eq!(err, IntakeError::SignatureMismatch);
    }

    #[test]
    fn missing_secret_token_reject() {
        let a = adapter();
        let raw = sample_update(1, 1, "hello");
        let err = a
            .verify_and_normalize(&raw, &IntakeWebhookHeaders::default(), "hub-1")
            .unwrap_err();
        assert_eq!(err, IntakeError::SignatureMismatch);
    }

    #[test]
    fn dedup_same_update_id() {
        let a = adapter();
        let raw = sample_update(99, 1, "2 sushi");
        let hdrs = headers("test-secret-123");
        let _ = a.verify_and_normalize(&raw, &hdrs, "hub-1").unwrap();
        let err = a
            .verify_and_normalize(&raw, &hdrs, "hub-1")
            .unwrap_err();
        assert!(matches!(err, IntakeError::Duplicate(_)));
    }

    #[test]
    fn different_update_ids_not_dedup() {
        let a = adapter();
        let hdrs = headers("test-secret-123");
        let _ = a
            .verify_and_normalize(&sample_update(1, 1, "2 sushi"), &hdrs, "h")
            .unwrap();
        let _ = a
            .verify_and_normalize(&sample_update(2, 1, "3 sushi"), &hdrs, "h")
            .unwrap();
        // No Duplicate error — different update_ids.
    }

    #[test]
    fn non_message_update_returns_empty() {
        let a = adapter();
        let raw = serde_json::json!({"update_id": 1})
            .to_string()
            .into_bytes();
        let msgs = a
            .verify_and_normalize(&raw, &headers("test-secret-123"), "h")
            .unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn empty_text_returns_empty() {
        let a = adapter();
        let raw = sample_update(1, 1, "");
        let msgs = a
            .verify_and_normalize(&raw, &headers("test-secret-123"), "h")
            .unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn deep_link_valid() {
        let a = adapter();
        // base64url of "hub1:item42" = "aHViMTppdGVtNDI"
        let payload = "aHViMTppdGVtNDI";
        let raw = sample_update(1, 1, &format!("/start {payload}"));
        let msgs = a
            .verify_and_normalize(&raw, &headers("test-secret-123"), "h")
            .unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].text, payload);
    }

    #[test]
    fn deep_link_too_long_reject() {
        let a = adapter();
        let long_payload = "a".repeat(MAX_DEEP_LINK_PAYLOAD_LEN + 1);
        let raw = sample_update(1, 1, &format!("/start {long_payload}"));
        let err = a
            .verify_and_normalize(&raw, &headers("test-secret-123"), "h")
            .unwrap_err();
        assert!(matches!(err, IntakeError::MalformedPayload(_)));
    }

    #[test]
    fn deep_link_invalid_chars_reject() {
        let a = adapter();
        let raw = sample_update(1, 1, "/start hello world!");
        let err = a
            .verify_and_normalize(&raw, &headers("test-secret-123"), "h")
            .unwrap_err();
        assert!(matches!(err, IntakeError::MalformedPayload(_)));
    }

    #[test]
    fn malformed_json_reject() {
        let a = adapter();
        let raw = b"not json at all";
        let err = a
            .verify_and_normalize(raw, &headers("test-secret-123"), "h")
            .unwrap_err();
        assert!(matches!(err, IntakeError::MalformedPayload(_)));
    }

    #[test]
    fn decode_start_payload_roundtrip() {
        use base64::Engine;
        let original = b"hub-1:item-42";
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(original);
        let decoded = TelegramAdapter::decode_start_payload(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_start_payload_invalid_base64() {
        let err = TelegramAdapter::decode_start_payload("!!!").unwrap_err();
        assert!(matches!(err, IntakeError::MalformedPayload(_)));
    }

    #[test]
    fn unix_ms_conversion() {
        let a = adapter();
        let raw = serde_json::json!({
            "update_id": 1,
            "message": {
                "message_id": 1,
                "chat": {"id": 1},
                "date": 1690000000,
                "text": "hi"
            }
        })
        .to_string()
        .into_bytes();
        let msgs = a
            .verify_and_normalize(&raw, &headers("test-secret-123"), "h")
            .unwrap();
        assert_eq!(msgs[0].unix_ms, 1690000000 * 1000);
    }
}
