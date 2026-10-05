//! Why an order was refunded (moved out of `refund.rs` by W-REFUND so the card
//! half could land without crossing the 300-line ratchet).

use crate::command::room_rules::OTHER_MAX_CHARS;

/// Why an order was refunded: a word the owner can count, as `VoidReason`.
/// `refused_at_door` is the one the blueprint names (P1-4); the rest are the
/// memo's abandoned orders (customer rang off, venue closed, no courier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefundReason {
    RefusedAtDoor,
    VenueCancelled,
    CustomerRequest,
    PaymentError,
    Other(String),
}

impl RefundReason {
    pub fn parse(s: &str) -> Option<RefundReason> {
        match s.trim() {
            "refused_at_door" => Some(RefundReason::RefusedAtDoor),
            "venue_cancelled" => Some(RefundReason::VenueCancelled),
            "customer_request" => Some(RefundReason::CustomerRequest),
            "payment_error" => Some(RefundReason::PaymentError),
            other => {
                let text = other.strip_prefix("other:")?.trim();
                (!text.is_empty() && text.chars().count() <= OTHER_MAX_CHARS)
                    .then(|| RefundReason::Other(text.to_string()))
            }
        }
    }

    pub fn word(&self) -> String {
        match self {
            RefundReason::RefusedAtDoor => "refused_at_door".into(),
            RefundReason::VenueCancelled => "venue_cancelled".into(),
            RefundReason::CustomerRequest => "customer_request".into(),
            RefundReason::PaymentError => "payment_error".into(),
            RefundReason::Other(t) => format!("other:{t}"),
        }
    }
}
