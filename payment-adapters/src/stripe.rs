//! `stripe` — the Stripe rail adapter (out-of-kernel, P60-PAYMENT).
//!
//! Degrade-closed placeholder. The real Stripe integration (ureq + HMAC webhook
//! signature verification, `never sees card data`) is NOT yet implemented: every
//! operation returns `PayError::Provider("stripe adapter not yet implemented")`.
//!
//! This is NAMED ABSENCE, never a fabricated success — the crate compiles and
//! re-exports the port contract, but no charge / refund / capture is ever
//! fabricated. Upgrade trigger: a real Stripe account + secret wiring land here
//! (the `ureq`/`hmac`/`sha2`/`serde_json` deps in Cargo.toml already exist for it).

use dowiz_kernel::ports::payment_provider::{
    ChargeHandle, ClientHandoff, IdempotencyKey, LegId, NLegPlan, PayError, PaymentEvent,
    PaymentProvider, PaymentStatus, RefundRequest, WebhookHeaders,
};

/// Stripe rail adapter — degrade-closed stub (no real Stripe integration yet).
pub struct StripeProvider;

impl PaymentProvider for StripeProvider {
    fn id(&self) -> &str {
        "stripe:eu"
    }

    fn create_with_key(
        &self,
        _key: &IdempotencyKey,
        _plan: &NLegPlan,
    ) -> Result<ClientHandoff, PayError> {
        Err(PayError::Provider(
            "stripe adapter not yet implemented".to_string(),
        ))
    }

    fn query_status_by_key(&self, _key: &IdempotencyKey) -> Result<PaymentStatus, PayError> {
        Err(PayError::Provider(
            "stripe adapter not yet implemented".to_string(),
        ))
    }

    fn verify_webhook(&self, _raw: &[u8], _sig: &WebhookHeaders) -> Result<PaymentEvent, PayError> {
        Err(PayError::Provider(
            "stripe adapter not yet implemented".to_string(),
        ))
    }

    fn capture_leg(&self, _leg: &LegId, _handle: &ChargeHandle) -> Result<(), PayError> {
        Err(PayError::Provider(
            "stripe adapter not yet implemented".to_string(),
        ))
    }

    fn void_leg(&self, _leg: &LegId, _handle: &ChargeHandle) -> Result<(), PayError> {
        Err(PayError::Provider(
            "stripe adapter not yet implemented".to_string(),
        ))
    }

    fn refund(&self, _req: &RefundRequest) -> Result<(), PayError> {
        Err(PayError::Provider(
            "stripe adapter not yet implemented".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripe_id_is_the_eu_rail() {
        assert_eq!(StripeProvider.id(), "stripe:eu");
    }

    #[test]
    fn stripe_is_degrade_closed_not_fabricated() {
        // Never fabricates a handoff: every op names the absence. IdempotencyKey
        // is a public newtype over [u8; 32], so a zero key is constructible here.
        let key = IdempotencyKey([0u8; 32]);
        assert!(matches!(
            StripeProvider.query_status_by_key(&key),
            Err(PayError::Provider(_))
        ));
    }
}
