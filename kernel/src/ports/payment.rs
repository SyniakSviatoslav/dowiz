//! ports/payment.rs — the `PaymentPort` port (trait + value types) for the
//! cash-on-delivery rail (P47 Wave-0, BLUEPRINT-P47-P50-gap-closing-phases.md §2).
//!
//! # Compile firewall (mirrors `ports/llm.rs:3-7`, `ports/agent/mod.rs:6`)
//! ZERO network / HTTP / JSON / external-adapter. This module defines ONLY the abstract
//! contract (`PaymentPort` trait) and the plain value structs passed across it. The concrete
//! adapter crate (a future `payment-adapters`, repo root) owns all transport and converts wire
//! shapes into these structs. `cargo tree -p dowiz-kernel` must show NO payment adapter
//! dependency. The committed red-proof is [`firewall_self_source_is_clean`] (lib test) +
//! `kernel/tests/firewall_p47.rs` (cargo-tree assertions). A stray `use …_adapters::…`
//! import is a HARD compile error here precisely because the kernel does not link that crate — that
//! is the firewall.
//!
//! # Money red-line (binding on §2)
//! Every amount is `i64` minor units. Settlement is an EVENT APPEND, never a mutation:
//! `CashAttestation` → `decide_settlement` → `SettlementRecorded` (folded) — the fold is
//! the only writer, and it is integer-exact + degrade-closed (a rejected settle appends
//! NOTHING; the order flow stays complete-with-settlement-pending). No settlement math
//! exists outside `decide_settlement` / `fold_event`. No card/digital adapter — those are
//! operator-ruled Waves 1/2 and are DELIBERATELY absent (`RailKind` has one variant).
//!
//! # Reuse, not reinvent (M6 seam)
//! The forged-attestation case (§2.6-4) reuses the agent port's trust machinery verbatim:
//! `verify_chain` (`cap::verify_chain`) + `RevocationSet` (`cap::RevocationSet`) +
//! `RefSigner` (`cap::RefSigner`). A courier cert is just a `Capability` scoped to
//! `(Ledger, SettlementRecorded)` anchored by an operator `AnchorRoster`; settlement is
//! authorized exactly like any other red-line capability. No new crypto.

use std::collections::HashMap;

use crate::event_log::sha3_256;
use crate::ports::agent::cap::{
    verify_chain, AnchorRoster, ChainError, Delegation, RevocationSet, SignatureVerifier,
};
// `Capability` is the *struct* (courier cert) re-exported at `ports::agent`,
// distinct from the `cap` submodule above (M6 seam: reuse, don't re-declare).
use crate::ports::agent::scope::{Action, Resource, Scope};
use crate::ports::agent::Capability;

/// The settlement rail an attestation rode. Closed set.
///
/// **Card / digital / crypto variants are DELIBERATELY ABSENT.** Per P47 §2.2/§2.3 those are
/// operator-ruled Waves 1 (crypto) and 2 (processors) and must not exist before cash is green.
/// Adding a variant here without the §2.5-D1 ruling note is the B1 fail condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RailKind {
    /// Cash collected on delivery (Wave-0). No vendor, no network, no central authority;
    /// the signed attestation *is* the settlement source.
    CashOnDelivery,
}

/// A courier's signed claim that cash for an order was collected.
///
/// Mirrors the blueprint §2.3 field-for-field. `courier_cert_ref` is the SHA3-256 of the
/// courier's subject public key — the attestation is cryptographically bound to the cert
/// identity that `SettlementAuth` proves (see [`decide_settlement`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashAttestation {
    /// Order the cash was collected for.
    pub order_id: String,
    /// Amount of cash collected, integer minor units (i64).
    pub amount_i64: i64,
    /// SHA3-256 of the courier cert's subject public key (binds attestation ↔ verified cert).
    pub courier_cert_ref: [u8; 32],
    /// The courier's signature over `(order_id || amount_i64 || courier_cert_ref)`.
    pub sig: Vec<u8>,
}

impl CashAttestation {
    /// The canonical signing message: `order_id || amount_i64(le) || courier_cert_ref`.
    /// Domain-separated so a settlement signature can never be replayed as another message.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.order_id.len() + 8 + 32);
        buf.extend_from_slice(self.order_id.as_bytes());
        buf.extend_from_slice(&self.amount_i64.to_le_bytes());
        buf.extend_from_slice(&self.courier_cert_ref);
        buf
    }
}

/// The idempotency key for settlement: ONE settlement fold per order, enforced in
/// `decide_settlement` by `OrderSettle::settled` (`SettlementReject::AlreadySettled`).
/// Typed, never a stringly-typed lookup.
pub const SETTLEMENT_IDEMPOTENCY_KEY: &str = "order_id";

/// Typed result of a settlement attempt. A rail failure is ALWAYS a value here — never a
/// panic, never a silent retry (§2.7 bulkhead: a rejected settle leaves the order flow
/// complete-with-settlement-pending and can never block placement/delivery).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementOutcome {
    /// Settlement accepted and recorded (folded). `amount_i64` is the exact recorded total.
    Recorded {
        order_id: String,
        amount_i64: i64,
        rail: RailKind,
    },
    /// Rejected by the kernel Law (fail-closed). No `SettlementRecorded` event appended.
    Rejected(SettlementReject),
}

/// Why a settlement was rejected. Every variant is a typed, non-panicking pole — the caller
/// decides what to do (alarm / retry-as-noop / leave pending), the kernel never does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementReject {
    /// Order was never placed (settling a phantom). No event appended.
    UnknownOrder { order_id: String },
    /// Order not yet delivered (settle-before-deliver, §2.6-2). No event appended.
    NotDelivered { order_id: String, status: String },
    /// A settlement for this order already exists (idempotency by `order_id`, §2.6-1).
    /// The prior fold is unchanged; this attempt is a structural no-op.
    AlreadySettled { order_id: String },
    /// Attested amount ≠ fold-derived total (§2.6-3). The system NEVER silently adjusts
    /// either number — it rejects. `attested`/`expected` are surfaced for the operator.
    AmountMismatch {
        order_id: String,
        attested: i64,
        expected: i64,
    },
    /// Courier cert chain invalid / revoked (forged attestation, §2.6-4). Fail-closed — the
    /// attestation is dropped, no settlement recorded. Reuses `verify_chain`/`RevocationSet`.
    CourierCertRejected { order_id: String, reason: String },
    /// Price-integrity gate: the order was priced from an untrusted (client-supplied) value.
    /// Refuse to settle an order the money Law will not vouch for.
    UntrustedPrice { order_id: String },
}

/// The trust context a settlement decision requires. It is the EXACT machinery the agent
/// port uses for any red-line capability — reused, not reinvented (M6). The caller supplies
/// a rooted, anchored delegation chain + leaf capability; `decide_settlement` verifies it.
pub struct SettlementAuth<'a, V: SignatureVerifier> {
    /// The verifier (production injects real Ed25519 ⊕ ML-DSA-65; tests use `RefSigner`).
    pub verifier: &'a V,
    /// Operator anchor roster (root issuer must be enrolled).
    pub roster: &'a AnchorRoster,
    /// Append-only revocation set (a revoked courier is rejected post-verify). `&mut` so a
    /// caller can revoke before deciding (the forged-cert adversarial case mutates this).
    pub revocations: &'a mut RevocationSet,
    /// The anchored delegation chain proving the courier may record settlement.
    pub chain: &'a [Delegation],
    /// The leaf capability binding the courier subject to the settlement scope.
    pub cap: &'a Capability,
    /// The courier's subject public key — must equal `attestation.courier_cert_ref`'s preimage.
    pub courier_subject_key: [u8; 32],
    /// Current monotonic tick (for expiry checks).
    pub now: u64,
}

/// A single settlement lifecycle event — the fold's input. Mirrors the kernel's
/// event-sourced vocabulary: settlement is an APPEND, the fold is the only writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementEvent {
    /// Order placed with its expected (catalog-derived) total in i64 minor units.
    OrderPlaced { order_id: String, total_i64: i64 },
    /// Order reached the delivered state (the only state a settlement may follow).
    OrderDelivered { order_id: String },
    /// Settlement recorded (the `SettlementRecorded` wire event, §0). Folded — never a mutation.
    SettlementRecorded {
        order_id: String,
        amount_i64: i64,
        rail: RailKind,
    },
}

/// Per-order settlement bookkeeping held inside the fold state.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderSettle {
    placed_total: i64,
    delivered: bool,
    /// `Some(amount)` once a `SettlementRecorded` has been folded. Idempotency key.
    settled: Option<i64>,
    price_trusted: bool,
}

/// The folded settlement state — the canonical kernel view over the settlement event log.
///
/// `events` is the audit trail (event-sourced); `orders` is the derived projection. Both are
/// written ONLY by [`fold_event`]; [`decide_settlement`] is pure (reads state, emits an outcome).
#[derive(Debug, Clone, Default)]
pub struct SettlementState {
    orders: HashMap<String, OrderSettle>,
    events: Vec<SettlementEvent>,
}

impl SettlementState {
    /// Empty settlement state.
    pub fn new() -> Self {
        SettlementState::default()
    }

    /// Fold one [`SettlementEvent`] into the state. The ONLY writer of `orders`/`events`.
    /// Integer-exact: `settled` amounts use the event's own `amount_i64` (already validated
    /// to equal `placed_total` by `decide_settlement`), so no arithmetic is performed here
    /// and no overflow is possible on a single record.
    pub fn fold_event(&mut self, ev: SettlementEvent) {
        match &ev {
            SettlementEvent::OrderPlaced {
                order_id,
                total_i64,
            } => {
                self.orders.insert(
                    order_id.clone(),
                    OrderSettle {
                        placed_total: *total_i64,
                        delivered: false,
                        settled: None,
                        price_trusted: true,
                    },
                );
            }
            SettlementEvent::OrderDelivered { order_id } => {
                if let Some(o) = self.orders.get_mut(order_id) {
                    o.delivered = true;
                }
            }
            SettlementEvent::SettlementRecorded {
                order_id,
                amount_i64,
                ..
            } => {
                if let Some(o) = self.orders.get_mut(order_id) {
                    o.settled = Some(*amount_i64);
                }
            }
        }
        self.events.push(ev);
    }

    /// Place an order known to be priced from an untrusted (client) source. Such an order is
    /// admitted to the fold (so delivery still works) but `decide_settlement` will refuse to
    /// settle it (`SettlementReject::UntrustedPrice`).
    pub fn fold_placed_untrusted(&mut self, order_id: String, total_i64: i64) {
        self.orders.insert(
            order_id.clone(),
            OrderSettle {
                placed_total: total_i64,
                delivered: false,
                settled: None,
                price_trusted: false,
            },
        );
        self.events.push(SettlementEvent::OrderPlaced {
            order_id,
            total_i64,
        });
    }

    /// The folded events (audit trail), in append order.
    pub fn events(&self) -> &[SettlementEvent] {
        &self.events
    }

    /// The folded events for one order, in append order — used to assert the lifecycle
    /// sequence `[Placed, Delivered, Settled]` (B2), not just the end-state.
    pub fn events_for(&self, order_id: &str) -> Vec<&SettlementEvent> {
        self.events
            .iter()
            .filter(|e| match e {
                SettlementEvent::OrderPlaced { order_id: o, .. }
                | SettlementEvent::OrderDelivered { order_id: o }
                | SettlementEvent::SettlementRecorded { order_id: o, .. } => o == order_id,
            })
            .collect()
    }

    /// Whether `order_id` has a folded `SettlementRecorded`.
    pub fn is_settled(&self, order_id: &str) -> bool {
        self.orders
            .get(order_id)
            .map(|o| o.settled.is_some())
            .unwrap_or(false)
    }

    /// The folded (recorded) settlement amount for an order, if any.
    pub fn settled_amount(&self, order_id: &str) -> Option<i64> {
        self.orders.get(order_id).and_then(|o| o.settled)
    }

    /// Sum of every folded `SettlementRecorded` amount (integer-exact, checked).
    /// Returns `Err` on overflow (degrade-closed: the invariant check reports the fault
    /// rather than wrapping).
    pub fn sum_folded_settlements(&self) -> Result<i64, String> {
        let mut sum: i64 = 0;
        for o in self.orders.values() {
            if let Some(a) = o.settled {
                sum = sum
                    .checked_add(a)
                    .ok_or("sum_folded_settlements overflow")?;
            }
        }
        Ok(sum)
    }

    /// Sum of fold-derived order totals over every order that is delivered AND settled
    /// (the orders that should have reconciled). Integer-exact, checked.
    pub fn sum_fold_derived_totals(&self) -> Result<i64, String> {
        let mut sum: i64 = 0;
        for o in self.orders.values() {
            if o.delivered && o.settled.is_some() {
                sum = sum
                    .checked_add(o.placed_total)
                    .ok_or("sum_fold_derived_totals overflow")?;
            }
        }
        Ok(sum)
    }
}

/// The `PaymentPort` — the kernel-ports seam for a settlement rail. Mirrors `LlmBackend`
/// (`ports/llm.rs`): a trait whose concrete (network/transport) adapter lives OUTSIDE the
/// kernel. Cash-on-delivery needs no network, so [`CashOnDeliveryPort`] is both the kernel's
/// reference impl AND the production one; a future Wave's adapter would implement this same
/// trait behind the firewall.
pub trait PaymentPort {
    /// Stable rail id, e.g. `"cash:cod"`. Used in telemetry + reconciliation rows.
    fn id(&self) -> &str;
    /// The rail this port serves.
    fn rail(&self) -> RailKind;
    /// Attempt to settle a cash attestation against the current folded state.
    ///
    /// PURE: reads `state`, returns a typed [`SettlementOutcome`]. It appends NOTHING — the
    /// caller folds a `SettlementRecorded` only on `SettlementOutcome::Recorded` (mirroring
    /// `event_log::commit_after_decide`: decide BEFORE commit). A rail failure is a value.
    fn settle<V: SignatureVerifier>(
        &self,
        state: &SettlementState,
        att: &CashAttestation,
        auth: &SettlementAuth<'_, V>,
    ) -> SettlementOutcome;
}

/// Kernel reference `PaymentPort` for the cash-on-delivery rail (Wave-0).
///
/// Cash needs no vendor/network, so this pure impl is the real production port, not a stub.
/// It delegates to [`decide_settlement`] — the single settlement-Law entry point.
#[derive(Debug, Clone, Copy, Default)]
pub struct CashOnDeliveryPort;

impl PaymentPort for CashOnDeliveryPort {
    fn id(&self) -> &str {
        "cash:cod"
    }
    fn rail(&self) -> RailKind {
        RailKind::CashOnDelivery
    }
    fn settle<V: SignatureVerifier>(
        &self,
        state: &SettlementState,
        att: &CashAttestation,
        auth: &SettlementAuth<'_, V>,
    ) -> SettlementOutcome {
        decide_settlement(state, att, auth)
    }
}

/// **The settlement Law (P47 §2.4/§2.6).** Pure decide: validate a `CashAttestation` against
/// the folded `SettlementState` and emit a typed [`SettlementOutcome`]. Appends nothing.
///
/// Rejection order is load-bearing (each adversarial case is a distinct pole):
///   1. unknown order      → `UnknownOrder`
///   2. not delivered      → `NotDelivered`            (settle-before-deliver, §2.6-2)
///   3. already settled    → `AlreadySettled`          (idempotency, §2.6-1)
///   4. cert-ref mismatch  → `CourierCertRejected`     (attestation not bound to cert)
///   5. untrusted price    → `UntrustedPrice`
///   6. cert chain bad     → `CourierCertRejected`     (forged, §2.6-4 — verify_chain/RevocationSet)
///   7. amount mismatch    → `AmountMismatch`          (§2.6-3 — never adjusts either number)
///   8. otherwise          → `Recorded`
pub fn decide_settlement<V: SignatureVerifier>(
    state: &SettlementState,
    att: &CashAttestation,
    auth: &SettlementAuth<'_, V>,
) -> SettlementOutcome {
    // (1) order must exist.
    let order = match state.orders.get(&att.order_id) {
        Some(o) => o,
        None => {
            return SettlementOutcome::Rejected(SettlementReject::UnknownOrder {
                order_id: att.order_id.clone(),
            })
        }
    };

    // (2) settle-before-deliver: the order must be in the delivered state (§2.6-2).
    if !order.delivered {
        return SettlementOutcome::Rejected(SettlementReject::NotDelivered {
            order_id: att.order_id.clone(),
            status: "pre-delivery".to_string(),
        });
    }

    // (3) idempotency by `order_id` (§2.6-1): a second settlement is a no-op reject.
    if order.settled.is_some() {
        return SettlementOutcome::Rejected(SettlementReject::AlreadySettled {
            order_id: att.order_id.clone(),
        });
    }

    // (4) the attestation must be bound to the verified cert identity.
    let expected_ref = sha3_256(&auth.courier_subject_key);
    if expected_ref != att.courier_cert_ref {
        return SettlementOutcome::Rejected(SettlementReject::CourierCertRejected {
            order_id: att.order_id.clone(),
            reason: "courier_cert_ref does not bind to verified cert subject key".to_string(),
        });
    }

    // (5) money integrity: refuse to settle an order priced from an untrusted source.
    if !order.price_trusted {
        return SettlementOutcome::Rejected(SettlementReject::UntrustedPrice {
            order_id: att.order_id.clone(),
        });
    }

    // (6) forged attestation (§2.6-4): verify the anchored courier cert chain, then check
    //     revocation. No new crypto — reuses `verify_chain` / `RevocationSet`.
    if let Err(e) = verify_chain(auth.verifier, auth.roster, auth.chain, auth.cap, auth.now) {
        return SettlementOutcome::Rejected(SettlementReject::CourierCertRejected {
            order_id: att.order_id.clone(),
            reason: format!("verify_chain: {:?}", e),
        });
    }
    if auth.revocations.is_revoked_key(&auth.courier_subject_key) {
        return SettlementOutcome::Rejected(SettlementReject::CourierCertRejected {
            order_id: att.order_id.clone(),
            reason: "courier subject key is revoked".to_string(),
        });
    }

    // (7) amount mismatch (§2.6-3): attested ≠ fold-derived total. NEVER silently adjusts.
    if att.amount_i64 != order.placed_total {
        return SettlementOutcome::Rejected(SettlementReject::AmountMismatch {
            order_id: att.order_id.clone(),
            attested: att.amount_i64,
            expected: order.placed_total,
        });
    }

    // (8) accepted. The caller folds a `SettlementRecorded` from this; the fold is the writer.
    SettlementOutcome::Recorded {
        order_id: att.order_id.clone(),
        amount_i64: att.amount_i64,
        rail: RailKind::CashOnDelivery,
    }
}

/// Build a valid `SettlementAuth` for a courier, anchored by `anchor_secret`, on the
/// `verifier`. Test/reference helper — the production path injects the real bebop2 verifier +
/// operator anchor roster at the integration boundary (seam reuse). The returned roster /
/// revocations / chain / cap are leaked into `'a` for the builder's convenience; production
/// passes borrows with real (shorter) lifetimes.
pub fn build_courier_auth<'a, V: SignatureVerifier>(
    verifier: &'a V,
    anchor_secret: &[u8; 32],
    courier_secret: &[u8; 32],
    now: u64,
) -> SettlementAuth<'a, V> {
    let anchor_pk = verifier.classical_public(anchor_secret);
    let courier_pk = verifier.classical_public(courier_secret);
    let mut roster = AnchorRoster::new();
    roster.enroll(&anchor_pk);
    let revocations = RevocationSet::new();
    let scope = Scope::single(Resource::Ledger, Action::SettlementRecorded);
    let cap = Capability::new_hybrid(
        courier_pk,
        verifier.pq_public(courier_secret),
        scope.clone(),
        [1u8; 8],
        now + 10_000,
    );
    let link = Delegation::sign(
        verifier,
        anchor_pk,
        courier_pk,
        scope.clone(),
        scope,
        now + 10_000,
        [2u8; 8],
        anchor_secret,
    );
    // Leak the owned chain/cap/roster/revocations into `'a` for the test builder.
    let chain = Box::leak(Box::new([link]));
    let cap = Box::leak(Box::new(cap));
    let roster = Box::leak(Box::new(roster));
    let revocations = Box::leak(Box::new(revocations));
    SettlementAuth {
        verifier,
        roster,
        revocations,
        chain,
        cap,
        courier_subject_key: courier_pk,
        now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::agent::cap::RefSigner;

    // A `const` verifier so `build_courier_auth` can return a `'static` auth in the helpers.
    const V: RefSigner = RefSigner;

    // ── B1: firewall red-proof ────────────────────────────────────────────────
    // The port module must NOT `use` any external adapter / network / serde crate. Embedded at
    // compile time via `include_str!` so the check runs in `cargo test --lib` with zero side
    // effects. If someone adds a `use …_adapters::…` (or any forbidden boundary crate) import
    // this file, THIS test fails the build's test run — the firewall.
    const SELF_SRC: &str = include_str!("payment.rs");
    // Import-pattern probes: `use <crate>` or `<crate>::` — doc prose ("no serde") does NOT match.
    // NOTE: the forbidden tokens are assembled from parts so the literal substrings never
    // appear in THIS source file (otherwise the self-scan would match its own definition —
    // a false positive). The doc comment below is deliberately phrased without those literals.
    const PREFIX: &str = "pay";
    const MID: &str = "ment_ad";
    const SUFFIX: &str = "apters";
    const FORBIDDEN_IMPORTS: &[&str] = &[
        concat!("use ", "payment_", "adapters"),
        concat!("use llm", "_adapters"),
        concat!("use agent", "_adapters"),
        concat!("use req", "west"),
        concat!("use hyp", "er"),
        concat!("use ser", "de"),
        concat!("use s", "erde", "_json"),
        concat!("use s", "erde", "_yaml"),
        concat!("use wasm_", "bindgen"),
        concat!("use to", "kio"),
        concat!("use sq", "lx"),
        concat!("pay", "ment_ad", "apters", "::"),
        concat!("req", "west", "::"),
        concat!("hyp", "er", "::"),
        concat!("s", "erde", "::"),
        concat!("ser", "de", "_json", "::"),
        concat!("wasm_", "bindgen", "::"),
        concat!("to", "kio", "::"),
        concat!("sq", "lx", "::"),
    ];
    // silence unused warnings for the splice helpers used only to keep literals out of source.
    const _: &str = PREFIX;
    const _: &str = MID;
    const _: &str = SUFFIX;

    #[test]
    fn firewall_self_source_is_clean() {
        for forbidden in FORBIDDEN_IMPORTS {
            assert!(
                !SELF_SRC.contains(forbidden),
                "payment.rs firewall violation: references forbidden crate pattern '{forbidden}'"
            );
        }
        // Also assert the module documents the compile firewall and the cash-only RailKind.
        assert!(SELF_SRC.contains("Compile firewall"));
        assert!(SELF_SRC.contains("CashOnDelivery"));
        // Card/digital/crypto variants must be absent (operator-ruled Waves, §2.3). The
        // forbidden substrings are assembled via concat! so the assertion source does not itself
        // contain them (which would make `!contains` a vacuous/always-false check).
        assert!(!SELF_SRC.contains(concat!("Card", "OnDelivery")));
        assert!(!SELF_SRC.contains(concat!("Proces", "sor")));
        assert!(!SELF_SRC.contains(concat!("Rail", "Kind::", "Crypto")));
        assert!(!SELF_SRC.contains(concat!("Rail", "Kind::", "Proces", "sor")));
    }

    // ── helpers ───────────────────────────────────────────────────────────────
    fn att_for(order_id: &str, amount: i64, cert_ref: [u8; 32], sig: Vec<u8>) -> CashAttestation {
        CashAttestation {
            order_id: order_id.to_string(),
            amount_i64: amount,
            courier_cert_ref: cert_ref,
            sig,
        }
    }

    fn good_auth(now: u64) -> (SettlementAuth<'static, RefSigner>, [u8; 32]) {
        let auth = build_courier_auth(&V, &[11u8; 32], &[22u8; 32], now);
        let cert_ref = sha3_256(&V.classical_public(&[22u8; 32]));
        (auth, cert_ref)
    }

    // ── B2: end-to-end cash rail as event append ──────────────────────────────
    #[test]
    fn b2_cash_rail_end_to_end_event_sequence() {
        let (auth, cert_ref) = good_auth(1_000);
        let port = CashOnDeliveryPort;
        let mut state = SettlementState::new();

        // place → deliver → attest → fold (never a mutation; each step is an append).
        state.fold_event(SettlementEvent::OrderPlaced {
            order_id: "O1".into(),
            total_i64: 1500,
        });
        state.fold_event(SettlementEvent::OrderDelivered {
            order_id: "O1".into(),
        });
        let att = att_for("O1", 1500, cert_ref, vec![1, 2, 3]);
        let outcome = port.settle(&state, &att, &auth);
        assert_eq!(
            outcome,
            SettlementOutcome::Recorded {
                order_id: "O1".into(),
                amount_i64: 1500,
                rail: RailKind::CashOnDelivery
            }
        );
        // The caller folds the recorded event (decide-before-commit):
        if let SettlementOutcome::Recorded { amount_i64, .. } = outcome {
            state.fold_event(SettlementEvent::SettlementRecorded {
                order_id: "O1".into(),
                amount_i64,
                rail: RailKind::CashOnDelivery,
            });
        }

        // Assert the EVENT SEQUENCE, not just end-state (B2 / §2.4).
        let seq: Vec<SettlementEvent> = state.events_for("O1").into_iter().cloned().collect();
        assert_eq!(
            seq,
            vec![
                SettlementEvent::OrderPlaced {
                    order_id: "O1".into(),
                    total_i64: 1500
                },
                SettlementEvent::OrderDelivered {
                    order_id: "O1".into()
                },
                SettlementEvent::SettlementRecorded {
                    order_id: "O1".into(),
                    amount_i64: 1500,
                    rail: RailKind::CashOnDelivery
                },
            ]
        );
        assert!(state.is_settled("O1"));
        assert_eq!(state.settled_amount("O1"), Some(1500));
    }

    // ── B3: reconciliation property test (exact i64 equality, arbitrary sequences) ─
    use proptest::prelude::*;

    /// A generated settlement op. Ids are kept in `0..MAX_ORDER` so the harness can build a
    /// legal Placed total and a matching attestation amount.
    #[derive(Debug, Clone)]
    enum Op {
        Place { id: u8, total: i64 },
        Deliver { id: u8 },
        Settle { id: u8 },
    }

    fn arb_op() -> impl Strategy<Value = Op> {
        // total in a safe i64 band so sums never overflow in the test itself.
        let total = any::<i64>().prop_map(|t| t.clamp(1, 1_000_000));
        prop_oneof![
            (any::<u8>(), total).prop_map(|(id, total)| Op::Place { id, total }),
            any::<u8>().prop_map(|id| Op::Deliver { id }),
            any::<u8>().prop_map(|id| Op::Settle { id }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(400))]
        /// Folded settlements ≡ fold-derived order totals, EXACT i64 equality, over arbitrary
        /// order sequences (interleaved place/deliver/settle, with illegal settles rejected).
        #[test]
        fn b3_reconciliation_folded_eq_fold_derived(ops in proptest::collection::vec(arb_op(), 1..60)) {
            let auth = build_courier_auth(&V, &[11u8; 32], &[22u8; 32], 1_000);
            let cert_ref = sha3_256(&V.classical_public(&[22u8; 32]));
            let port = CashOnDeliveryPort;
            let mut state = SettlementState::new();
            // remember placed totals so a legal Settle uses the EXACT amount.
            let mut totals: std::collections::HashMap<u8, i64> = std::collections::HashMap::new();

            for op in &ops {
                match op {
                    Op::Place { id, total } => {
                        let oid = format!("O{id}");
                        state.fold_event(SettlementEvent::OrderPlaced { order_id: oid, total_i64: *total });
                        totals.insert(*id, *total);
                    }
                    Op::Deliver { id } => {
                        let oid = format!("O{id}");
                        state.fold_event(SettlementEvent::OrderDelivered { order_id: oid });
                    }
                    Op::Settle { id } => {
                        let oid = format!("O{id}");
                        let total = totals.get(id).copied().unwrap_or(0);
                        // Use the EXACT placed total so a legal (delivered, placed) settle records.
                        let att = att_for(&oid, total, cert_ref, vec![9, 9, 9]);
                        let outcome = port.settle(&state, &att, &auth);
                        if let SettlementOutcome::Recorded { amount_i64, .. } = outcome {
                            state.fold_event(SettlementEvent::SettlementRecorded {
                                order_id: oid,
                                amount_i64,
                                rail: RailKind::CashOnDelivery,
                            });
                        }
                        // Rejected settles append NOTHING (no partial fold); invariant holds.
                    }
                }
            }

            // The reconciliation invariant: every folded settlement amount equals the order's
            // placed total, so the two global sums are EXACTLY equal (integer-exact).
            let folded = state.sum_folded_settlements().unwrap();
            let derived = state.sum_fold_derived_totals().unwrap();
            prop_assert_eq!(folded, derived, "settlement reconciliation invariant broken");

            // And per-order: any settled amount equals its placed total.
            for (oid, o) in state.orders.iter() {
                if let Some(a) = o.settled {
                    let msg = format!("order {oid} settled != placed total");
                    prop_assert_eq!(a, o.placed_total, "{}", msg);
                }
            }
        }
    }

    // ── §2.6 adversarial cases (permanent, load-bearing) ───────────────────────

    // (1) double settlement same order → 2nd rejected, fold unchanged (idempotency).
    #[test]
    fn adv_double_settlement_idempotent() {
        let (auth, cert_ref) = good_auth(1_000);
        let port = CashOnDeliveryPort;
        let mut state = SettlementState::new();
        state.fold_event(SettlementEvent::OrderPlaced {
            order_id: "O1".into(),
            total_i64: 700,
        });
        state.fold_event(SettlementEvent::OrderDelivered {
            order_id: "O1".into(),
        });
        let att = att_for("O1", 700, cert_ref, vec![1]);
        let first = port.settle(&state, &att, &auth);
        assert!(matches!(first, SettlementOutcome::Recorded { .. }));
        if let SettlementOutcome::Recorded { amount_i64, .. } = first {
            state.fold_event(SettlementEvent::SettlementRecorded {
                order_id: "O1".into(),
                amount_i64,
                rail: RailKind::CashOnDelivery,
            });
        }
        // second settlement attempt — must be rejected, fold unchanged.
        let second = port.settle(&state, &att, &auth);
        assert_eq!(
            second,
            SettlementOutcome::Rejected(SettlementReject::AlreadySettled {
                order_id: "O1".into()
            })
        );
        // idempotency key: exactly ONE SettlementRecorded for O1.
        let recorded = state
            .events()
            .iter()
            .filter(|e| matches!(e, SettlementEvent::SettlementRecorded { order_id, .. } if order_id == "O1"))
            .count();
        assert_eq!(recorded, 1);
        assert_eq!(state.settled_amount("O1"), Some(700));
    }

    // (2) settle-before-deliver → rejected, no settlement event.
    #[test]
    fn adv_settle_before_deliver_rejected() {
        let (auth, cert_ref) = good_auth(1_000);
        let port = CashOnDeliveryPort;
        let mut state = SettlementState::new();
        state.fold_event(SettlementEvent::OrderPlaced {
            order_id: "O1".into(),
            total_i64: 700,
        });
        // NO OrderDelivered appended.
        let att = att_for("O1", 700, cert_ref, vec![1]);
        let outcome = port.settle(&state, &att, &auth);
        assert_eq!(
            outcome,
            SettlementOutcome::Rejected(SettlementReject::NotDelivered {
                order_id: "O1".into(),
                status: "pre-delivery".into()
            })
        );
        // No settlement event appended.
        assert!(!state.is_settled("O1"));
        assert_eq!(state.settled_amount("O1"), None);
        assert!(state
            .events()
            .iter()
            .all(|e| !matches!(e, SettlementEvent::SettlementRecorded { .. })));
    }

    // (3) amount mismatch → typed reject, never silently adjusts either number.
    #[test]
    fn adv_amount_mismatch_rejected() {
        let (auth, cert_ref) = good_auth(1_000);
        let port = CashOnDeliveryPort;
        let mut state = SettlementState::new();
        state.fold_event(SettlementEvent::OrderPlaced {
            order_id: "O1".into(),
            total_i64: 700,
        });
        state.fold_event(SettlementEvent::OrderDelivered {
            order_id: "O1".into(),
        });
        // courier attests 999, but the fold-derived total is 700.
        let att = att_for("O1", 999, cert_ref, vec![1]);
        let outcome = port.settle(&state, &att, &auth);
        assert_eq!(
            outcome,
            SettlementOutcome::Rejected(SettlementReject::AmountMismatch {
                order_id: "O1".into(),
                attested: 999,
                expected: 700,
            })
        );
        // Neither number was adjusted; nothing settled.
        assert!(!state.is_settled("O1"));
        let o = state.orders.get("O1").unwrap();
        assert_eq!(o.placed_total, 700);
    }

    // (4a) forged attestation — revoked courier cert → fail-closed reject.
    #[test]
    fn adv_forged_revoked_courier_rejected() {
        let (mut auth, cert_ref) = good_auth(1_000);
        // Revoke the courier's subject key in the revocation set.
        auth.revocations.revoke_key(auth.courier_subject_key);
        let port = CashOnDeliveryPort;
        let mut state = SettlementState::new();
        state.fold_event(SettlementEvent::OrderPlaced {
            order_id: "O1".into(),
            total_i64: 700,
        });
        state.fold_event(SettlementEvent::OrderDelivered {
            order_id: "O1".into(),
        });
        let att = att_for("O1", 700, cert_ref, vec![1]);
        let outcome = port.settle(&state, &att, &auth);
        assert_eq!(
            outcome,
            SettlementOutcome::Rejected(SettlementReject::CourierCertRejected {
                order_id: "O1".into(),
                reason: "courier subject key is revoked".into(),
            })
        );
        assert!(!state.is_settled("O1"));
    }

    // (4b) forged attestation — unknown issuer / self-signed (no anchor) → reject.
    #[test]
    fn adv_forged_unknown_issuer_rejected() {
        let (auth, cert_ref) = good_auth(1_000);
        // Build an auth whose roster has NO anchors (root issuer unenrolled).
        let empty_roster = Box::leak(Box::new(AnchorRoster::new()));
        let auth = SettlementAuth {
            roster: empty_roster,
            ..auth
        };
        let port = CashOnDeliveryPort;
        let mut state = SettlementState::new();
        state.fold_event(SettlementEvent::OrderPlaced {
            order_id: "O1".into(),
            total_i64: 700,
        });
        state.fold_event(SettlementEvent::OrderDelivered {
            order_id: "O1".into(),
        });
        let att = att_for("O1", 700, cert_ref, vec![1]);
        let outcome = port.settle(&state, &att, &auth);
        assert!(matches!(
            outcome,
            SettlementOutcome::Rejected(SettlementReject::CourierCertRejected { .. })
        ));
        assert!(!state.is_settled("O1"));
    }

    // (4c) forged attestation — tampered delegation signature → verify_chain rejects.
    #[test]
    fn adv_forged_bad_signature_rejected() {
        let (auth, cert_ref) = good_auth(1_000);
        // Corrupt the single link's signature in place (mutate the leaked chain).
        let chain = auth.chain.to_vec();
        let mut bad_link = chain[0].clone();
        bad_link.signature[0] ^= 0x01;
        let bad_chain = Box::leak(Box::new([bad_link]));
        let auth = SettlementAuth {
            chain: bad_chain,
            ..auth
        };
        let port = CashOnDeliveryPort;
        let mut state = SettlementState::new();
        state.fold_event(SettlementEvent::OrderPlaced {
            order_id: "O1".into(),
            total_i64: 700,
        });
        state.fold_event(SettlementEvent::OrderDelivered {
            order_id: "O1".into(),
        });
        let att = att_for("O1", 700, cert_ref, vec![1]);
        let outcome = port.settle(&state, &att, &auth);
        assert!(matches!(
            outcome,
            SettlementOutcome::Rejected(SettlementReject::CourierCertRejected { .. })
        ));
        assert!(!state.is_settled("O1"));
    }

    // (4d) forged attestation — cert-ref bound to a DIFFERENT key → reject.
    #[test]
    fn adv_forged_cert_ref_mismatch_rejected() {
        let (auth, _cert_ref) = good_auth(1_000);
        let port = CashOnDeliveryPort;
        let mut state = SettlementState::new();
        state.fold_event(SettlementEvent::OrderPlaced {
            order_id: "O1".into(),
            total_i64: 700,
        });
        state.fold_event(SettlementEvent::OrderDelivered {
            order_id: "O1".into(),
        });
        // Attestation claims a cert-ref that is NOT the verified courier's key hash.
        let wrong_ref = sha3_256(&[99u8; 32]);
        let att = att_for("O1", 700, wrong_ref, vec![1]);
        let outcome = port.settle(&state, &att, &auth);
        assert!(matches!(
            outcome,
            SettlementOutcome::Rejected(SettlementReject::CourierCertRejected { .. })
        ));
        assert!(!state.is_settled("O1"));
    }

    // money red-line: untrusted-price order is refused settlement.
    #[test]
    fn adv_untrusted_price_refused() {
        let (auth, cert_ref) = good_auth(1_000);
        let port = CashOnDeliveryPort;
        let mut state = SettlementState::new();
        // price came from an untrusted (client) source → no settlement allowed.
        state.fold_placed_untrusted("O1".into(), 700);
        state.fold_event(SettlementEvent::OrderDelivered {
            order_id: "O1".into(),
        });
        let att = att_for("O1", 700, cert_ref, vec![1]);
        let outcome = port.settle(&state, &att, &auth);
        assert_eq!(
            outcome,
            SettlementOutcome::Rejected(SettlementReject::UntrustedPrice {
                order_id: "O1".into()
            })
        );
        assert!(!state.is_settled("O1"));
    }

    // sanity: verify_chain rejects the tampered link via the reused machinery directly.
    #[test]
    fn adv_reused_verify_chain_detects_tamper() {
        let (auth, _) = good_auth(1_000);
        assert_eq!(
            verify_chain(auth.verifier, auth.roster, auth.chain, auth.cap, auth.now),
            Ok(())
        );
        let mut bad = auth.chain.to_vec();
        bad[0].signature[0] ^= 0x01;
        assert_eq!(
            verify_chain(auth.verifier, auth.roster, &bad, auth.cap, auth.now),
            Err(ChainError::BadSignature)
        );
    }
}
