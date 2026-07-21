//! P48-INTAKE — channel-agnostic inbound order-intake adapters.
//!
//! Compile firewall: this crate produces [`InboundMessage`] values and hands them
//! to the intake service. It structurally cannot call `place_order` — it does not
//! import kernel mutation symbols. Provider payload types (Telegram `Update`, Meta
//! `WebhookEntry`, etc.) die here and never reach the kernel.
//!
//! Each channel module is feature-gated off-by-default:
//! - `telegram` — Telegram Bot API webhook receive + `getUpdates` long-poll
//!
//! The canonical normalization template: `fn verify_and_normalize(&self, raw, hdrs)
//! -> Result<Vec<InboundMessage>, IntakeError>` — one shape per channel, matching
//! the `payment_provider::verify_webhook` pattern (fact 0.6 in the blueprint).

pub mod telegram;

/// Re-export the kernel's channel-agnostic vocabulary.
pub use dowiz_kernel::ports::hub_intake::{
    channel_str, Ambiguity, IntentOutcome, IntentParser, KeywordParser, OrderIntent, OrderLine,
};

/// External webhook headers, channel-agnostic. The concrete adapter extracts
/// the provider-specific header it needs from this bag.
#[derive(Debug, Clone, Default)]
pub struct IntakeWebhookHeaders {
    /// Telegram: `X-Telegram-Bot-Api-Secret-Token`
    pub telegram_secret: Option<String>,
    /// Meta: `X-Hub-Signature-256` (WhatsApp, Instagram, Facebook)
    pub meta_hub_signature: Option<String>,
}

/// Errors from the intake adapter pipeline — typed, never opaque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntakeError {
    /// Signature / secret-token verification failed — reject at the edge,
    /// zero kernel calls. The body is never deserialized.
    SignatureMismatch,
    /// The payload failed to parse as a valid provider update.
    MalformedPayload(String),
    /// Idempotent dedup: this `provider_msg_id` was already processed.
    Duplicate(String),
    /// The webhook's timestamp is outside the tolerance window.
    StaleTimestamp,
}

impl std::fmt::Display for IntakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntakeError::SignatureMismatch => write!(f, "signature mismatch"),
            IntakeError::MalformedPayload(s) => write!(f, "malformed payload: {s}"),
            IntakeError::Duplicate(id) => write!(f, "duplicate message: {id}"),
            IntakeError::StaleTimestamp => write!(f, "stale timestamp outside tolerance"),
        }
    }
}

impl std::error::Error for IntakeError {}

/// Constant-time comparison of two byte slices. Prevents timing side-channel
/// attacks on secret-token or HMAC verification. Returns `true` iff the slices
/// are equal in length AND every byte matches.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_equal() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn constant_time_eq_different_length() {
        assert!(!constant_time_eq(b"hello", b"hell"));
    }

    #[test]
    fn constant_time_eq_different_content() {
        assert!(!constant_time_eq(b"hello", b"world"));
    }
}
