//! M4 — reconnect outbox: query-before-replay (the double-charge-prevention core, §16.52).
//!
//! On the `online` transition (`Net::is_online` true), for every persisted draft, branch on
//! state via [`decide_reconnect`] (pure):
//!   * `Draft` ⇒ `ResumeLocalEditing` — restore the cart + wallet-fill; nothing was sent.
//!   * `PaymentInflight` ⇒ `QueryThenDecide` — call `query_status_by_key(idem_key)` FIRST,
//!     never a blind resubmit. Then [`decide_post_query`]:
//!       - `Captured`/`Authorized`/`IntentCreated` ⇒ `ShowSuccessClearDraft` (intent already
//          lives on the hub; do NOT resubmit - this is what prevents a double charge).
//!       - `NoneYet`/`Failed` ⇒ `ResubmitSameKey` — safe (P60's `create_with_key` treats the
//         same key idempotently; even the resubmit path cannot double-charge).
//
// The falsifiable double-charge-prevention test lives here.

use crate::ports::payment_provider::{IdempotencyKey, PaymentProvider, PaymentStatus};

use crate::wallet::draft::{DraftState, MAX_OPEN_INFLIGHT_DRAFTS};

// On reconnect, the branch decision (R4 §3.3). Pure; no I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectAction {
    // state == Draft
    ResumeLocalEditing,
    // state == PaymentInflight -> query_status_by_key FIRST
    QueryThenDecide,
}

// Pure branch on the persisted draft state.
pub fn decide_reconnect(state: &DraftState) -> ReconnectAction {
    match state {
        DraftState::Draft => ReconnectAction::ResumeLocalEditing,
        DraftState::PaymentInflight => ReconnectAction::QueryThenDecide,
    }
}

// After the query resolves, what the client does — NEVER a blind resubmit on a live intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostQueryAction {
    // Captured/Authorized/IntentCreated: the intent LIVES — do not resubmit.
    ShowSuccessClearDraft,
    // NoneYet/Failed: safe to (re)submit with the SAME idem_key.
    ResubmitSameKey,
}

// Pure decision on the hub-returned status (P60's — never self-certified).
pub fn decide_post_query(status: &PaymentStatus) -> PostQueryAction {
    match status {
        PaymentStatus::Captured | PaymentStatus::Authorized | PaymentStatus::IntentCreated => {
            PostQueryAction::ShowSuccessClearDraft
        }
        PaymentStatus::NoneYet | PaymentStatus::Failed(_) => PostQueryAction::ResubmitSameKey,
        // A voided/refunded intent is terminal: show that outcome and clear — do not resubmit.
        PaymentStatus::Voided | PaymentStatus::Refunded => PostQueryAction::ShowSuccessClearDraft,
    }
}

// Connectivity port (web: `online` event; Tauri: connectivity loop) — R4 §3.2.
pub trait Net {
    fn is_online(&self) -> bool;
}

// The double-charge-prevention driver. Drives ONE draft's reconnect against a `PaymentProvider`.
//
// Returns whether a (re)submit to `create_with_key` happened — the caller asserts this is ZERO
// when the hub already captured (the money-sharp safety property). `query_status_by_key` is
// ALWAYS called first when the draft is `PaymentInflight`; `create_with_key` is only ever called
// AFTER the query resolves to `NoneYet`/`Failed`. The same `idem_key` is reused — never regenerated.
pub fn reconnect_draft<P: PaymentProvider>(
    draft_state: &DraftState,
    idem_key: &IdempotencyKey,
    provider: &P,
    plan: &crate::ports::payment_provider::NLegPlan,
) -> ReconnectOutcome {
    match decide_reconnect(draft_state) {
        ReconnectAction::ResumeLocalEditing => ReconnectOutcome::ResumedLocal,
        ReconnectAction::QueryThenDecide => {
            // ALWAYS query first — never blind replay.
            let status = provider
                .query_status_by_key(idem_key)
                .unwrap_or(PaymentStatus::NoneYet); // fail-closed: unknown ⇒ treat as "maybe charged"
            match decide_post_query(&status) {
                PostQueryAction::ShowSuccessClearDraft => ReconnectOutcome::AlreadyLive { status },
                PostQueryAction::ResubmitSameKey => {
                    // Resubmit with the SAME key (P60 idempotency makes a replay a no-op if it
                    // did in fact go through). This is the ONLY path that calls create_with_key.
                    let _ = provider.create_with_key(idem_key, plan);
                    ReconnectOutcome::ResubmittedSameKey
                }
            }
        }
    }
}

// What [`reconnect_draft`] did — the observable safety result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconnectOutcome {
    // Draft state — no network, local resume.
    ResumedLocal,
    // PaymentInflight + hub already had the intent live (Captured/Authorized/IntentCreated/Voided/Refunded).
    // `create_with_key` was NOT called (no second charge).
    AlreadyLive { status: PaymentStatus },
    // PaymentInflight + hub returned NoneYet/Failed ⇒ resubmitted with the SAME key.
    ResubmittedSameKey,
}

// One-open-intent guard (UX, NOT the security boundary — that is P60 M7, server-side).
pub fn single_open_guard(open: usize) -> bool {
    open < MAX_OPEN_INFLIGHT_DRAFTS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Currency;
    use crate::ports::payment_provider::{
        FailReason, IdemLedger, LegId, NLegPlan, ProviderHandles, VendorId, VendorLeg,
    };

    fn plan() -> NLegPlan {
        NLegPlan {
            order_id: "order_42".into(),
            currency: Currency::Eur,
            legs: vec![VendorLeg {
                leg: LegId(1),
                vendor_id: VendorId([9u8; 32]),
                amount: crate::money::Money::new(1000, Currency::Eur),
                dest_account: crate::ports::payment_provider::ProviderAccountRef("acct".into()),
            }],
        }
    }

    // A mock provider that decides status by `query_status_by_key` and RECORDS how many times
    // `create_with_key` was invoked (the double-charge probe).
    struct MockProvider {
        ledger: std::cell::RefCell<IdemLedger>,
        captured: bool,
        creates: std::cell::RefCell<usize>,
    }
    impl MockProvider {
        fn new(captured: bool) -> Self {
            MockProvider {
                ledger: std::cell::RefCell::new(IdemLedger::new()),
                captured,
                creates: std::cell::RefCell::new(0),
            }
        }
    }
    impl PaymentProvider for MockProvider {
        fn id(&self) -> &str {
            "mock"
        }
        fn create_with_key(
            &self,
            key: &IdempotencyKey,
            p: &NLegPlan,
        ) -> Result<
            crate::ports::payment_provider::ClientHandoff,
            crate::ports::payment_provider::PayError,
        > {
            *self.creates.borrow_mut() += 1;
            if self.captured {
                // Model: the hub captured between send and ack (the dangerous case).
                self.ledger
                    .borrow_mut()
                    .set_status(*key, PaymentStatus::Captured);
            }
            let _ = p;
            Ok(
                crate::ports::payment_provider::ClientHandoff::HostedRedirect {
                    checkout_url: "https://pay.example/checkout/order_42".into(),
                    session_token: [0u8; 32],
                    ttl_s: 900,
                },
            )
        }
        fn query_status_by_key(
            &self,
            key: &IdempotencyKey,
        ) -> Result<PaymentStatus, crate::ports::payment_provider::PayError> {
            if self.captured {
                if let Some((h, _)) = self.ledger.borrow().resolve(key) {
                    let _: ProviderHandles = h;
                }
                Ok(PaymentStatus::Captured)
            } else {
                Ok(PaymentStatus::NoneYet)
            }
        }
        fn verify_webhook(
            &self,
            _raw: &[u8],
            _headers: &crate::ports::payment_provider::WebhookHeaders,
        ) -> Result<
            crate::ports::payment_provider::PaymentEvent,
            crate::ports::payment_provider::PayError,
        > {
            unimplemented!()
        }
        fn capture_leg(
            &self,
            _leg: &LegId,
            _handle: &crate::ports::payment_provider::ChargeHandle,
        ) -> Result<(), crate::ports::payment_provider::PayError> {
            Ok(())
        }
        fn void_leg(
            &self,
            _leg: &LegId,
            _handle: &crate::ports::payment_provider::ChargeHandle,
        ) -> Result<(), crate::ports::payment_provider::PayError> {
            Ok(())
        }
        fn refund(
            &self,
            _req: &crate::ports::payment_provider::RefundRequest,
        ) -> Result<(), crate::ports::payment_provider::PayError> {
            Ok(())
        }
    }

    #[test]
    fn double_charge_prevention_draft_inflight_captured() {
        // Scenario: draft created, key minted, PaymentSubmitted -> PaymentInflight, socket
        // dropped AFTER the hub captured but BEFORE the client saw the ack.
        // On reconnect: query FIRST, Captured => zero resubmit.
        let key = IdempotencyKey([0x42u8; 32]);
        let provider = MockProvider::new(true); // hub already captured
        let outcome = reconnect_draft(&DraftState::PaymentInflight, &key, &provider, &plan());
        assert_eq!(
            outcome,
            ReconnectOutcome::AlreadyLive {
                status: PaymentStatus::Captured
            }
        );
        // THE TEETH: create_with_key called ZERO times — no second charge.
        assert_eq!(*provider.creates.borrow(), 0);
    }

    #[test]
    fn draft_state_resumes_locally_no_query() {
        // Nothing was sent; reconnect just resumes. No query, no create.
        let key = IdempotencyKey([0x43u8; 32]);
        let provider = MockProvider::new(false);
        let outcome = reconnect_draft(&DraftState::Draft, &key, &provider, &plan());
        assert_eq!(outcome, ReconnectOutcome::ResumedLocal);
        assert_eq!(*provider.creates.borrow(), 0);
    }

    #[test]
    fn query_noneyet_resubmits_same_key_once() {
        // Hub never saw the intent => safe to resubmit with the SAME key (idempotent replay).
        let key = IdempotencyKey([0x44u8; 32]);
        let provider = MockProvider::new(false);
        let outcome = reconnect_draft(&DraftState::PaymentInflight, &key, &provider, &plan());
        assert_eq!(outcome, ReconnectOutcome::ResubmittedSameKey);
        assert_eq!(
            *provider.creates.borrow(),
            1,
            "exactly one resubmit with the same key"
        );
    }

    #[test]
    fn query_failed_resubmits_same_key() {
        // Model a failed intent (declined) => resubmit same key.
        struct FailedProvider;
        impl PaymentProvider for FailedProvider {
            fn id(&self) -> &str {
                "failed"
            }
            fn create_with_key(
                &self,
                _k: &IdempotencyKey,
                _p: &NLegPlan,
            ) -> Result<
                crate::ports::payment_provider::ClientHandoff,
                crate::ports::payment_provider::PayError,
            > {
                Ok(
                    crate::ports::payment_provider::ClientHandoff::HostedRedirect {
                        checkout_url: "x".into(),
                        session_token: [0; 32],
                        ttl_s: 1,
                    },
                )
            }
            fn query_status_by_key(
                &self,
                _k: &IdempotencyKey,
            ) -> Result<PaymentStatus, crate::ports::payment_provider::PayError> {
                Ok(PaymentStatus::Failed(FailReason::Declined))
            }
            fn verify_webhook(
                &self,
                _r: &[u8],
                _h: &crate::ports::payment_provider::WebhookHeaders,
            ) -> Result<
                crate::ports::payment_provider::PaymentEvent,
                crate::ports::payment_provider::PayError,
            > {
                unimplemented!()
            }
            fn capture_leg(
                &self,
                _l: &LegId,
                _h: &crate::ports::payment_provider::ChargeHandle,
            ) -> Result<(), crate::ports::payment_provider::PayError> {
                Ok(())
            }
            fn void_leg(
                &self,
                _l: &LegId,
                _h: &crate::ports::payment_provider::ChargeHandle,
            ) -> Result<(), crate::ports::payment_provider::PayError> {
                Ok(())
            }
            fn refund(
                &self,
                _r: &crate::ports::payment_provider::RefundRequest,
            ) -> Result<(), crate::ports::payment_provider::PayError> {
                Ok(())
            }
        }
        let key = IdempotencyKey([0x45u8; 32]);
        let outcome = reconnect_draft(&DraftState::PaymentInflight, &key, &FailedProvider, &plan());
        assert_eq!(outcome, ReconnectOutcome::ResubmittedSameKey);
    }

    #[test]
    fn single_open_guard_boundary() {
        assert!(single_open_guard(0));
        assert!(!single_open_guard(MAX_OPEN_INFLIGHT_DRAFTS));
    }

    #[test]
    fn decide_post_query_terminal_no_resubmit() {
        // Voided/Refunded => do NOT resubmit (show outcome, clear).
        assert_eq!(
            decide_post_query(&PaymentStatus::Voided),
            PostQueryAction::ShowSuccessClearDraft
        );
        assert_eq!(
            decide_post_query(&PaymentStatus::Refunded),
            PostQueryAction::ShowSuccessClearDraft
        );
        assert_eq!(
            decide_post_query(&PaymentStatus::Captured),
            PostQueryAction::ShowSuccessClearDraft
        );
        assert_eq!(
            decide_post_query(&PaymentStatus::Authorized),
            PostQueryAction::ShowSuccessClearDraft
        );
        assert_eq!(
            decide_post_query(&PaymentStatus::IntentCreated),
            PostQueryAction::ShowSuccessClearDraft
        );
        assert_eq!(
            decide_post_query(&PaymentStatus::NoneYet),
            PostQueryAction::ResubmitSameKey
        );
    }
}
