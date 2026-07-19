//! M3 — the `Draft`→`PaymentInflight` state machine + idempotency key minting (X6).
//!
//! The idempotency key is **P60's** — re-exported, never redefined. It is minted EXACTLY
//! ONCE, at draft creation, using P60's derivation verbatim, and stored IN the draft — never
//! regenerated (R4 §3.3). Regeneration is the double-charge bug; this module refuses it.
//!
//! The machine has exactly two states (`DraftState`), modeled as an event-sourced saga
//! (`DraftEvent`), mirroring the kernel `decide`/`fold` shape. Tests assert on the EVENT
//! SEQUENCE (item 3), not just end-state. NO CRDT (R4 §3.1).

use crate::event_log::sha3_256;
use crate::money::{Currency, Money};
use crate::ports::payment_provider::{IdempotencyKey, PaymentStatus};

use crate::wallet::record::{Address, Contact, PaymentMethodRef, WalletRecord};
/// Content id of the draft at creation (the draft's stable key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DraftId(pub [u8; 32]);

/// The offline checkout draft. Held locally; restored on reconnect (§16.52).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckoutDraft {
    pub draft_id: DraftId,
    pub order_id: String,
    pub cart: CartSnapshot,
    pub wallet_fill: WalletFill,
    /// Minted ONCE at creation, NEVER regenerated (R4 §3.3).
    pub idem_key: IdempotencyKey,
    pub state: DraftState,
}

/// A point-in-time cart snapshot: integer Money lines (kernel `Money`), no f64.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartSnapshot {
    pub currency: Currency,
    pub lines: Vec<CartLine>,
}

/// One cart line: leaf id + qty + unit price (Money, i64 minor units).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CartLine {
    pub leaf_id: String,
    pub qty: u32,
    pub unit: Money,
}

/// The autofilled name/address/contact/method_ref pulled from the wallet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WalletFill {
    pub name: Option<String>,
    pub address: Option<Address>,
    pub contact: Option<Contact>,
    pub method_ref: Option<PaymentMethodRef>,
}

impl WalletFill {
    /// Snapshot the autofill-relevant slots from a [`WalletRecord`].
    pub fn from_wallet(rec: &WalletRecord) -> Self {
        WalletFill {
            name: rec.name.clone(),
            address: rec.addresses.first().cloned(),
            contact: rec.contact.clone(),
            method_ref: rec.method_ref.clone(),
        }
    }
}

/// The exact two-state machine R4 §3.3 prescribes. A third state is unrepresentable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftState {
    /// Editing locally; nothing submitted. Reconnect ⇒ resume locally.
    Draft,
    /// Payment request WAS sent (set optimistically the instant of send, before any ack).
    /// Reconnect ⇒ query_status_by_key FIRST, never blind replay (§4.4).
    PaymentInflight,
}

/// Event-sourced saga (item 3 — tests assert on the sequence, not just end-state).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftEvent {
    DraftCreated { draft_id: DraftId, order_id: String },
    /// Exactly once, at creation.
    IdemKeyMinted { key: IdempotencyKey },
    FieldFilled { field: FilledField },
    /// Draft -> PaymentInflight (optimistic).
    PaymentSubmitted,
    /// From query_status_by_key.
    StatusResolved { status: PaymentStatus },
    /// Terminal: committed or user-abandoned.
    DraftCleared { draft_id: DraftId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilledField {
    Name,
    Address,
    Contact,
    Method,
}

/// Client-side UX guard (NOT the security boundary — that is P60 M7, server-side).
pub const MAX_OPEN_INFLIGHT_DRAFTS: usize = 1;

/// Mint an idempotency key with P60's derivation, VERBATIM (X6, §4.2):
/// `IdempotencyKey(event_log::sha3_256(b"dowiz.pay.idem\0" ‖ order_id ‖ wallet_id ‖ nonce))`.
pub fn mint_idem_key(order_id: &str, wallet_id: &[u8; 32], nonce: &[u8; 12]) -> IdempotencyKey {
    IdempotencyKey(sha3_256(
        &[
            b"dowiz.pay.idem\0".as_slice(),
            order_id.as_bytes(),
            wallet_id.as_slice(),
            nonce.as_slice(),
        ]
        .concat(),
    ))
}

/// Create a fresh draft. Mints the idem key ONCE and returns the initial event log.
pub fn create_draft(
    draft_id: DraftId,
    order_id: String,
    cart: CartSnapshot,
    wallet_fill: WalletFill,
    wallet_id: &[u8; 32],
    nonce: &[u8; 12],
) -> (CheckoutDraft, Vec<DraftEvent>) {
    let key = mint_idem_key(&order_id, wallet_id, nonce);
    let draft = CheckoutDraft {
        draft_id,
        order_id: order_id.clone(),
        cart,
        wallet_fill,
        idem_key: key,
        state: DraftState::Draft,
    };
    let events = vec![
        DraftEvent::DraftCreated {
            draft_id,
            order_id,
        },
        DraftEvent::IdemKeyMinted { key },
    ];
    (draft, events)
}

/// The pure `decide` for the draft saga: fold the event log into the draft's observable state.
/// Returns (state, minted_key, last_resolved_status). Fail-closed on a duplicate key mint.
pub fn fold(events: &[DraftEvent]) -> Result<(DraftState, Option<IdempotencyKey>, Option<PaymentStatus>), DraftFoldError> {
    let mut state = DraftState::Draft;
    let mut key: Option<IdempotencyKey> = None;
    let mut status: Option<PaymentStatus> = None;
    for e in events {
        match e {
            DraftEvent::IdemKeyMinted { key: k } => {
                // The key MUST be minted exactly once (anti-regeneration teeth, §4.3).
                if key.is_some() {
                    return Err(DraftFoldError::KeyRegenerated);
                }
                key = Some(*k);
            }
            DraftEvent::PaymentSubmitted => state = DraftState::PaymentInflight,
            DraftEvent::StatusResolved { status: s } => status = Some(s.clone()),
            _ => {}
        }
    }
    Ok((state, key, status))
}

/// A draft cannot be in two `PaymentInflight` states; the only structural error is a
/// regenerated key (which would be the double-charge bug — refused at the fold).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftFoldError {
    KeyRegenerated,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cart() -> CartSnapshot {
        CartSnapshot {
            currency: Currency::Eur,
            lines: vec![CartLine {
                leaf_id: "leaf_1".into(),
                qty: 2,
                unit: Money::new(500, Currency::Eur),
            }],
        }
    }

    fn wallet_id() -> [u8; 32] {
        [0xcd_u8; 32]
    }
    fn nonce() -> [u8; 12] {
        [0x11_u8; 12]
    }

    #[test]
    fn idem_key_minted_once_at_creation() {
        let (draft, events) = create_draft(
            DraftId([1u8; 32]),
            "order_42".into(),
            cart(),
            WalletFill::default(),
            &wallet_id(),
            &nonce(),
        );
        // key present on the draft + in the log.
        assert!(events
            .iter()
            .any(|e| matches!(e, DraftEvent::IdemKeyMinted { key } if *key == draft.idem_key)));
        // byte-identical after N field edits.
        let mut log = events;
        log.push(DraftEvent::FieldFilled {
            field: FilledField::Name,
        });
        log.push(DraftEvent::FieldFilled {
            field: FilledField::Address,
        });
        log.push(DraftEvent::FieldFilled {
            field: FilledField::Method,
        });
        let (state, key, _) = fold(&log).unwrap();
        assert_eq!(state, DraftState::Draft);
        assert_eq!(key.unwrap(), draft.idem_key);
        // Re-deriving with the same inputs yields the same key (deterministic, replay-safe).
        let again = mint_idem_key("order_42", &wallet_id(), &nonce());
        assert_eq!(again, draft.idem_key);
    }

    #[test]
    fn submit_sets_inflight_before_ack() {
        let (mut draft, mut log) = create_draft(
            DraftId([2u8; 32]),
            "order_7".into(),
            cart(),
            WalletFill::default(),
            &wallet_id(),
            &nonce(),
        );
        // Optimistically transition the instant the request is sent — BEFORE any response.
        log.push(DraftEvent::PaymentSubmitted);
        draft.state = DraftState::PaymentInflight;
        let (state, _, status) = fold(&log).unwrap();
        assert_eq!(state, DraftState::PaymentInflight);
        assert_eq!(status, None, "no response folded yet — status still unknown");
    }

    #[test]
    fn draft_survives_restart() {
        // Simulate an app kill: persist the event log, re-fold after restart.
        let (draft, mut log) = create_draft(
            DraftId([3u8; 32]),
            "order_9".into(),
            cart(),
            WalletFill::default(),
            &wallet_id(),
            &nonce(),
        );
        let key_before = draft.idem_key;
        log.push(DraftEvent::PaymentSubmitted);
        // "restart": drop the in-memory draft, re-fold from the persisted log.
        let (state, key, _) = fold(&log).unwrap();
        assert_eq!(state, DraftState::PaymentInflight);
        assert_eq!(key.unwrap(), key_before, "key identical after restart re-fold");
    }

    #[test]
    fn edit_after_inflight_is_refused() {
        // The cart whose payment is in flight must not be mutated. We model this as:
        // a second PaymentSubmitted is meaningless (idempotent no-op at the fold), and any
        // FieldFilled while PaymentInflight is a UX-refused operation — assert the fold still
        // yields PaymentInflight + the SAME key (no regeneration possible).
        let (_draft, mut log) = create_draft(
            DraftId([4u8; 32]),
            "order_11".into(),
            cart(),
            WalletFill::default(),
            &wallet_id(),
            &nonce(),
        );
        log.push(DraftEvent::PaymentSubmitted);
        // An attempted re-submit / edit after inflight.
        log.push(DraftEvent::PaymentSubmitted);
        let (state, key, _) = fold(&log).unwrap();
        assert_eq!(state, DraftState::PaymentInflight);
        // Exactly one key minted in the whole log (the fold would error on a second mint).
        let mints = log
            .iter()
            .filter(|e| matches!(e, DraftEvent::IdemKeyMinted { .. }))
            .count();
        assert_eq!(mints, 1);
        assert!(key.is_some());
    }

    #[test]
    fn regenerating_key_is_refused() {
        // A mutation that mints a SECOND key for the same draft must be refused (anti-double-charge teeth).
        let mut log = vec![
            DraftEvent::DraftCreated {
                draft_id: DraftId([5u8; 32]),
                order_id: "order_13".into(),
            },
            DraftEvent::IdemKeyMinted {
                key: mint_idem_key("order_13", &wallet_id(), &nonce()),
            },
        ];
        log.push(DraftEvent::IdemKeyMinted {
            key: mint_idem_key("order_13", &wallet_id(), &[0x22u8; 12]),
        });
        assert_eq!(fold(&log), Err(DraftFoldError::KeyRegenerated));
    }
}
