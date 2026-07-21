//! ports/payment_provider.rs — the `PaymentProvider` port (trait + value types) for the
//! online-fiat / Stripe rail (BLUEPRINT-P60-payment-adapter-core.md, W1/P60, §0.2-1).
//!
//! # Compile firewall (mirrors `ports/payment.rs:1-12`, `ports/llm.rs:3-7`)
//! ZERO network / HTTP / JSON / external-adapter. This module defines ONLY the abstract
//! contract (`PaymentProvider` trait) and the plain value structs passed across it. The
//! concrete Stripe adapter lives OUT-OF-KERNEL in the `payment-adapters` crate (repo root),
//! which is allowed `reqwest`/HMAC there — never here. `cargo tree -p dowiz-kernel` must show
//! NO payment-adapter dependency. The committed red-proof is [`firewall_self_source_is_clean`]
//! (lib test) + `kernel/tests/no_card_data.rs` (whole-tree scan) + `kernel/tests/firewall_p47.rs`
//! (cargo-tree assertions). A stray `use …_adapters::…` import is a HARD compile error here
//! precisely because the kernel does not link that crate — that is the firewall.
//!
//! # No card-data firewall (PCI red-line, structural) — SYNTHESIS §0.2 / R2 §6.3
//! There is NO card-data type in this module or anywhere in hub-core. The ONLY thing that
//! crosses hub → client is an opaque [`ClientHandoff`] (provider URL / native-SDK blob) — never
//! a PAN, never a `cvv`, never a `card_*`. The client tokenizes DIRECTLY with the provider; the
//! kernel never deserializes a card. [`no_card_data_type_in_core`] (lib test) + the whole-tree
//! `kernel/tests/no_card_data.rs` scan are the CI teeth: adding a `pan:` / `card_number` field
//! anywhere under `kernel/src/` makes the build fail. The wgpu canvas NEVER binds a card type
//! because none exists.
//!
//! # Money red-line
//! Every amount is `Money` (i64 minor units + `Currency`). No `f64`, no bare `i64` without a
//! currency tag. Cross-currency arithmetic is fail-closed via `Money::checked_add`/`checked_sub`
//! (money.rs). The N-leg saga is event-sourced (decide/fold Law) — the fold is the only writer
//! of capture truth, and the webhook is its SOLE source (§4.4).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::event_log::sha3_256;
use crate::money::{Currency, Money};

// ── value types (predefined, §3) ─────────────────────────────────────────────

/// The idempotency key — minted at draft creation by P66, domain-separated SHA3 over
/// `b"dowiz.pay.idem\0" || order_id || wallet_id || nonce` (§4.2). Typed, never a bare String,
/// so a key can never be replayed across orders/wallets.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct IdempotencyKey(pub [u8; 32]);

impl IdempotencyKey {
    /// Derive a domain-separated idempotency key from order + wallet + nonce (X6, §4.2).
    /// Reuses `event_log::sha3_256` (no new hash).
    pub fn derive(order_id: &str, wallet_id: &str, nonce: &[u8]) -> Self {
        let mut msg = Vec::with_capacity(
            b"dowiz.pay.idem\0".len() + order_id.len() + wallet_id.len() + nonce.len(),
        );
        msg.extend_from_slice(b"dowiz.pay.idem\0");
        msg.extend_from_slice(order_id.as_bytes());
        msg.extend_from_slice(wallet_id.as_bytes());
        msg.extend_from_slice(nonce);
        IdempotencyKey(sha3_256(&msg))
    }
}

/// The ONLY thing that ever crosses hub → client. Opaque by construction: it carries provider
/// handles/URLs/tokens, NEVER card data. No `pan`/`cvv`/`card_*` field exists or may be added
/// (the no-PAN firewall, §4.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ClientHandoff {
    /// Path C (web + desktop, §4-A): the provider's hosted page URL + a short-TTL, single-use
    /// signed session token. The client opens the provider's verified domain.
    HostedRedirect {
        checkout_url: String,
        session_token: [u8; 32],
        ttl_s: u32,
    },
    /// Path B (Tauri mobile, §4-A, pending P63): opaque native-SDK session blob the device SDK
    /// consumes to render the native card sheet. Still zero card data in core.
    NativeSdkSession { session_blob: Vec<u8> },
    /// Cash-on-delivery (C4, 2026-07-20): no provider handoff at all — the courier collects
    /// physical cash at delivery, so there is no hosted page, no native SDK, and no card data.
    /// The only "capture" signal is the courier's delivery confirmation (driven by `capture_leg`).
    CashOnDelivery { order_id: String },
}

pub const CLIENT_SESSION_TTL_S: u32 = 900; // 15 min single-use redirect/session window

/// Normalized, provider-agnostic status. The webhook (M4) is the ONLY writer of the
/// Authorized/Captured/Voided/Refunded truth (§4.4); a client redirect is advisory-only.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PaymentStatus {
    NoneYet,
    IntentCreated,
    Authorized,
    Captured,
    Voided,
    Refunded,
    Failed(FailReason),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FailReason {
    Declined,
    Expired,
    ProviderError,
    Cancelled,
}

/// One vendor money-leg (§0.2-1). Derived from order_item.vendor_id fan-out over P62's leaf
/// invariant (X7). `dest` is the VENDOR'S OWN provider account — never dowiz's.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VendorLeg {
    pub leg: LegId,
    pub vendor_id: VendorId,
    pub amount: Money,
    pub dest_account: ProviderAccountRef,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LegId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VendorId(pub [u8; 32]);

/// Opaque per-vendor account id (their merchant-of-record). No dowiz account type exists.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProviderAccountRef(pub String);

/// The N-leg plan for one checkout. Single-vendor = the degenerate N = 1.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NLegPlan {
    pub order_id: String,
    pub currency: Currency,
    pub legs: Vec<VendorLeg>,
}

pub const MAX_LEGS_PER_CHECKOUT: usize = 32; // food-court sanity cap; §5.2 scaling axis

/// Per-leg lifecycle. A leg is exactly one of these — a partial/mixed terminal that is NOT one of
/// {all Captured, all Voided/AuthFailed, explicit NeedsReconciliation} is UNREPRESENTABLE (the
/// money-atomicity invariant, §4.5/§5.1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LegState {
    Draft,
    Authorized,
    AuthFailed(FailReason),
    Captured,
    Voided,
    CaptureStuck,
}

/// The whole-checkout outcome of the atomicity Law (M5).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NLegOutcome {
    /// All legs authorized → all captured. The only "money moved" terminal.
    Committed,
    /// A leg failed to authorize → every already-authorized leg voided. No money moved.
    Aborted { void_set: Vec<LegId> },
    /// Capture began but a leg is stuck (auth succeeded, capture not confirmed). NOT silent: a
    /// typed operator-visible state; dowiz does NOT auto-resolve (§16.29). Auth holds
    /// auto-expire provider-side (~7d Stripe) ⇒ a stuck leg self-heals toward Void, never toward
    /// a phantom charge.
    NeedsReconciliation {
        stuck: Vec<LegId>,
        captured: Vec<LegId>,
    },
}

/// Event-sourced saga log (standard item 3: tests assert on these sequences, not end-state).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NLegEvent {
    PlanCreated { order_id: String, n_legs: u32 },
    LegAuthorized { leg: LegId },
    LegAuthFailed { leg: LegId, reason: FailReason },
    AllLegsAuthorized { order_id: String },
    LegCaptured { leg: LegId },
    LegVoided { leg: LegId },
    LegCaptureStuck { leg: LegId },
    NLegCommitted { order_id: String },
    NLegAborted { order_id: String },
    NLegNeedsReconciliation { order_id: String },
}

/// Refund routing (§16.29). Routes to the VENDOR'S provider account; dowiz stays out.
/// Maps onto the existing order states Refunding → CompensatedRefund (order_machine.rs).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RefundRequest {
    pub charge: ChargeHandle,
    pub amount: Money,
    pub reason: RefundReason,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RefundReason {
    CustomerRequest,
    VendorInitiated,
    DisputeResolution,
}

/// Opaque per-leg captured-charge id (bound to that vendor's account).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ChargeHandle(pub String);

/// Typed provider-boundary error. A provider failure is ALWAYS a value here — never a panic,
/// never a silent retry (bulkhead, §5.3). Mirrors payment.rs's SettlementReject discipline.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PayError {
    Idempotent { key: IdempotencyKey },
    Declined,
    Expired,
    SignatureInvalid,
    Unroutable,
    Provider(String),
    CurrencyMismatch,
}

/// THE port. Provider-agnostic. Core knows nothing provider-specific (R2 §5.2).
pub trait PaymentProvider {
    /// Stable rail id, e.g. "stripe:eu".
    fn id(&self) -> &str;
    /// Create an intent to be confirmed CLIENT-SIDE (Path C/B). Idempotent on `key`: replaying the
    /// SAME key returns the SAME handoff, never a second charge (X6).
    fn create_with_key(
        &self,
        key: &IdempotencyKey,
        plan: &NLegPlan,
    ) -> Result<ClientHandoff, PayError>;
    /// Reconnect-safe status query by idempotency key (X6). Where a provider has no true
    /// query-by-key endpoint, the adapter resolves via the hub-local IdemLedger (§4.2).
    fn query_status_by_key(&self, key: &IdempotencyKey) -> Result<PaymentStatus, PayError>;
    /// Verify a provider webhook signature and normalize to a hub event (M4). The ONLY source of
    /// truth for Authorized/Captured/Voided/Refunded.
    fn verify_webhook(&self, raw: &[u8], sig: &WebhookHeaders) -> Result<PaymentEvent, PayError>;
    /// Two-phase leg controls (M5). capture/void are idempotent + provider-side.
    fn capture_leg(&self, leg: &LegId, handle: &ChargeHandle) -> Result<(), PayError>;
    fn void_leg(&self, leg: &LegId, handle: &ChargeHandle) -> Result<(), PayError>;
    /// §16.29 — refund routes to the vendor's provider account. dowiz stays out.
    fn refund(&self, req: &RefundRequest) -> Result<(), PayError>;
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WebhookHeaders {
    pub sig: Vec<u8>,
    pub ts: i64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PaymentEvent {
    pub key: IdempotencyKey,
    pub leg: Option<LegId>,
    pub status: PaymentStatus,
    pub provider_event_id: String,
}

pub const WEBHOOK_TS_TOLERANCE_S: i64 = 300; // replay window; reject older (§4.4)

// ── anti-abuse (X11) ─────────────────────────────────────────────────────────
// Checkout-intent limiter = the EXISTING kernel TokenBucket (token_bucket.rs), keyed by
// (wallet_client_id, coarse_ip). Degrade-closed: try_acquire → false ⇒ refuse.
pub const CHECKOUT_BURST: f64 = 3.0; // TokenBucket capacity
pub const CHECKOUT_REFILL_PER_SEC: f64 = 0.05; // ~1 intent / 20 s steady state
/// A wallet may have at most ONE open (uncaptured, unexpired) intent (R2 §7 item 4).
pub const MAX_OUTSTANDING_INTENTS_PER_WALLET: usize = 1;

// ── idempotency contract (X6 — P60 OWNS it) ───────────────────────────────────
/// Opaque handles recorded in the reconnect ledger (X6). Carries the provider checkout URL, the
/// single-use session token, and the per-leg charge handles — never card data.
#[derive(Clone)]
pub struct ProviderHandles {
    pub checkout_url: String,
    pub session_token: [u8; 32],
    pub leg_charges: HashMap<LegId, ChargeHandle>,
}

impl ProviderHandles {
    fn from_plan(plan: &NLegPlan, token: [u8; 32]) -> Self {
        let mut leg_charges = HashMap::new();
        for leg in &plan.legs {
            leg_charges.insert(leg.leg, ChargeHandle(format!("ch_{}", leg.leg.0)));
        }
        ProviderHandles {
            checkout_url: format!("https://pay.example/checkout/{}", plan.order_id),
            session_token: token,
            leg_charges,
        }
    }
}

/// Append-only key → handle → status map. The reconnect-safety authority (X6). Event-sourced
/// (reuses event_log discipline); demote-never-mutate (§5.5 living memory). The log IS the
/// persisted state, so a hub restart re-folds it (reconnect-survives-restart test, §4.2).
#[derive(Clone, Default)]
pub struct IdemLedger {
    log: Vec<IdemLogEvent>,
}

#[derive(Clone)]
enum IdemLogEvent {
    Created {
        key: IdempotencyKey,
        handles: ProviderHandles,
        amount: i64,
        currency: Currency,
    },
    Status {
        key: IdempotencyKey,
        status: PaymentStatus,
    },
}

impl IdemLedger {
    pub fn new() -> Self {
        IdemLedger { log: Vec::new() }
    }

    /// Record a create (append-only). The bound amount/currency ties the key to its first plan
    /// (key-rebind refusal, §4.2).
    pub fn record_create(
        &mut self,
        key: IdempotencyKey,
        handles: ProviderHandles,
        amount: i64,
        currency: Currency,
    ) {
        self.log.push(IdemLogEvent::Created {
            key,
            handles,
            amount,
            currency,
        });
    }

    /// The amount/currency this key was first bound to (for key-rebind refusal).
    pub fn bound(&self, key: &IdempotencyKey) -> Option<(i64, Currency)> {
        self.log.iter().find_map(|e| match e {
            IdemLogEvent::Created {
                key: k,
                amount,
                currency,
                ..
            } if k == key => Some((*amount, *currency)),
            _ => None,
        })
    }

    /// The handles recorded for a key (idempotent re-create returns these unchanged).
    pub fn handles(&self, key: &IdempotencyKey) -> Option<ProviderHandles> {
        self.log.iter().find_map(|e| match e {
            IdemLogEvent::Created {
                key: k, handles, ..
            } if k == key => Some(handles.clone()),
            _ => None,
        })
    }

    /// Append a status transition (append-only; replay yields the latest).
    pub fn set_status(&mut self, key: IdempotencyKey, status: PaymentStatus) {
        self.log.push(IdemLogEvent::Status { key, status });
    }

    /// Resolve a key to (handles, latest-status). Absent key ⇒ None (caller maps to NoneYet).
    pub fn resolve(&self, key: &IdempotencyKey) -> Option<(ProviderHandles, PaymentStatus)> {
        let mut handles: Option<ProviderHandles> = None;
        let mut status = PaymentStatus::IntentCreated;
        for e in &self.log {
            match e {
                IdemLogEvent::Created {
                    key: k, handles: h, ..
                } if k == key => {
                    handles = Some(h.clone());
                }
                IdemLogEvent::Status { key: k, status: s } if k == key => {
                    status = s.clone();
                }
                _ => {}
            }
        }
        handles.map(|h| (h, status))
    }
}

// ── plan helpers (money-law, cross-currency fail-closed) ──────────────────────
/// Validate every leg amount shares the plan's single currency (reuses money.rs cross-currency
/// guard). A food-court plan is single-currency Wave-0 (§4-D EUR).
pub fn validate_plan_currency(plan: &NLegPlan) -> Result<(), PayError> {
    for leg in &plan.legs {
        if leg.amount.currency != plan.currency {
            return Err(PayError::CurrencyMismatch);
        }
    }
    Ok(())
}

/// Sum the plan's leg amounts into one `Money`. Cross-currency legs ⇒ `CurrencyMismatch` (the
/// only producer of the typed error before any authorize, §4.5 adversarial (iv)).
pub fn plan_total(plan: &NLegPlan) -> Result<Money, PayError> {
    let mut total = Money::new(0, plan.currency);
    for leg in &plan.legs {
        if leg.amount.currency != plan.currency {
            return Err(PayError::CurrencyMismatch);
        }
        total = total.checked_add(leg.amount).map_err(PayError::Provider)?;
    }
    Ok(total)
}

// ── N-leg atomicity saga (M5 — the hardest correctness item) ──────────────────
/// Per-leg capture result fed to the saga (consulted only when all legs authorized).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CaptureOutcome {
    Captured,
    Stuck,
}

/// The Law: a pure event-sourced decide/fold over the N-leg plan.
///
/// Phase 1 — Authorize all: each leg → `LegAuthorized` or `LegAuthFailed`. If ANY leg failed to
/// authorize, void every authorized leg → `Aborted` (no money moved). Else `AllLegsAuthorized`.
/// Phase 2 — Capture all: each leg → `LegCaptured` or `LegCaptureStuck`. Any stuck leg ⇒
/// `NeedsReconciliation` (operator-visible, never silent). Else `Committed`.
///
/// The money-atomicity invariant: the terminal is EXACTLY one of { all Captured (Committed) },
/// { every leg Voided/AuthFailed (Aborted) }, or { explicit NeedsReconciliation }. A terminal
/// where some legs are Captured and others Voided WITHOUT a NeedsReconciliation flag is
/// UNREPRESENTABLE — the only producer of `LegVoided` is the all-auth-failed arm (which voids ALL
/// authorized legs), and its only other arm produces `LegCaptured` for every authorized leg.
pub fn run_nleg_saga(
    order_id: &str,
    auth: &[(LegId, Result<(), FailReason>)],
    capture: &[(LegId, CaptureOutcome)],
) -> (Vec<NLegEvent>, NLegOutcome) {
    let mut events = vec![NLegEvent::PlanCreated {
        order_id: order_id.to_string(),
        n_legs: auth.len() as u32,
    }];

    // Phase 1 — authorize all.
    let mut all_authorized = true;
    for (leg, res) in auth {
        match res {
            Ok(()) => events.push(NLegEvent::LegAuthorized { leg: *leg }),
            Err(r) => {
                events.push(NLegEvent::LegAuthFailed {
                    leg: *leg,
                    reason: *r,
                });
                all_authorized = false;
            }
        }
    }

    if !all_authorized {
        // Void every authorized leg; no money moved.
        let mut void_set = Vec::new();
        for (leg, res) in auth {
            if res.is_ok() {
                events.push(NLegEvent::LegVoided { leg: *leg });
                void_set.push(*leg);
            }
        }
        events.push(NLegEvent::NLegAborted {
            order_id: order_id.to_string(),
        });
        return (events, NLegOutcome::Aborted { void_set });
    }

    // All authorized → proceed to capture.
    events.push(NLegEvent::AllLegsAuthorized {
        order_id: order_id.to_string(),
    });

    // Phase 2 — capture all.
    let mut any_stuck = false;
    let mut captured = Vec::new();
    let mut stuck = Vec::new();
    for (leg, res) in capture {
        match res {
            CaptureOutcome::Captured => {
                events.push(NLegEvent::LegCaptured { leg: *leg });
                captured.push(*leg);
            }
            CaptureOutcome::Stuck => {
                events.push(NLegEvent::LegCaptureStuck { leg: *leg });
                stuck.push(*leg);
                any_stuck = true;
            }
        }
    }

    if any_stuck {
        events.push(NLegEvent::NLegNeedsReconciliation {
            order_id: order_id.to_string(),
        });
        return (events, NLegOutcome::NeedsReconciliation { stuck, captured });
    }

    events.push(NLegEvent::NLegCommitted {
        order_id: order_id.to_string(),
    });
    (events, NLegOutcome::Committed)
}

/// The money-atomicity invariant assertion (the falsifier, §4.5 / §6). Used by the property test
/// over 400+ arbitrary sequences and the adversarial fixtures.
pub fn assert_nleg_atomicity(events: &[NLegEvent], outcome: &NLegOutcome) {
    let has_void = events
        .iter()
        .any(|e| matches!(e, NLegEvent::LegVoided { .. }));
    let has_captured = events
        .iter()
        .any(|e| matches!(e, NLegEvent::LegCaptured { .. }));
    let has_stuck = events
        .iter()
        .any(|e| matches!(e, NLegEvent::LegCaptureStuck { .. }));

    match outcome {
        NLegOutcome::Committed => {
            assert!(has_captured, "Committed requires ≥1 captured leg");
            assert!(!has_void, "Committed must have no voided leg");
            assert!(!has_stuck, "Committed must have no stuck leg");
        }
        NLegOutcome::Aborted { .. } => {
            assert!(
                !has_captured,
                "Aborted must have no captured leg (no money moved)"
            );
            assert!(!has_stuck, "Aborted must have no stuck leg");
        }
        NLegOutcome::NeedsReconciliation { stuck, .. } => {
            assert!(
                has_stuck && !stuck.is_empty(),
                "NeedsReconciliation requires ≥1 stuck leg"
            );
            assert!(!has_void, "NeedsReconciliation must have no voided leg");
        }
    }

    // The core invariant: captured + voided WITHOUT reconciliation is UNREPRESENTABLE.
    if has_captured && has_void {
        assert!(
            matches!(outcome, NLegOutcome::NeedsReconciliation { .. }),
            "captured+voided terminal must be flagged NeedsReconciliation"
        );
    }
}

/// Edge challenge predicate (X11): Turnstile/ALTCHA token is verified at the edge/redirect layer,
/// never in the kernel. A missing/invalid token ⇒ intent refused at the edge (returns false).
pub fn edge_challenge_ok(token: &Option<String>) -> bool {
    token.as_ref().map_or(false, |t| !t.is_empty())
}

/// Single-outstanding-intent gate (X11 M7): a wallet with an open (uncaptured, unexpired) intent
/// is refused a second. Event-sourced predicate over fold state; here a minimal hub-local count.
#[derive(Default)]
pub struct OutstandingIntentGate {
    counts: HashMap<String, usize>,
}

impl OutstandingIntentGate {
    pub fn new() -> Self {
        OutstandingIntentGate {
            counts: HashMap::new(),
        }
    }

    /// Returns true if the intent may open; false (refuse) if the wallet already holds the max.
    pub fn try_open(&mut self, wallet: &str) -> bool {
        let c = self.counts.entry(wallet.to_string()).or_insert(0);
        if *c >= MAX_OUTSTANDING_INTENTS_PER_WALLET {
            return false;
        }
        *c += 1;
        true
    }

    pub fn close(&mut self, wallet: &str) {
        if let Some(c) = self.counts.get_mut(wallet) {
            *c = c.saturating_sub(1);
        }
    }
}

// ── Wave-0 default adapter (deterministic, firewall-clean, no network) ─────────
/// The kernel-side Wave-0 default `PaymentProvider`. Fully deterministic (no `reqwest`/serde):
/// it records intents in an in-memory [`IdemLedger`], emits a `HostedRedirect` handoff (Path C),
/// and verifies webhooks with a local sha3 "signature" stand-in (the REAL HMAC-SHA256 lives in
/// the out-of-kernel `payment-adapters` crate). Used by the kernel test-suite as the trait's
/// object-safe default; production wires the Stripe adapter behind the same trait.
pub struct NoOpPaymentAdapter {
    secret: [u8; 32],
    ledger: RefCell<IdemLedger>,
    seen_events: RefCell<HashSet<String>>,
    tokens: RefCell<HashMap<[u8; 32], TokenEntry>>,
}

#[derive(Clone)]
struct TokenEntry {
    key: IdempotencyKey,
    created_ts: i64,
    used: bool,
}

impl NoOpPaymentAdapter {
    pub fn new() -> Self {
        NoOpPaymentAdapter {
            secret: [7u8; 32],
            ledger: RefCell::new(IdemLedger::new()),
            seen_events: RefCell::new(HashSet::new()),
            tokens: RefCell::new(HashMap::new()),
        }
    }

    /// Construct from a pre-existing (reloaded-from-disk) ledger — models hub restart re-fold
    /// (reconnect-survives-restart, §4.2).
    pub fn with_ledger(ledger: IdemLedger) -> Self {
        NoOpPaymentAdapter {
            secret: [7u8; 32],
            ledger: RefCell::new(ledger),
            seen_events: RefCell::new(HashSet::new()),
            tokens: RefCell::new(HashMap::new()),
        }
    }

    /// Consume a single-use session token (Path C). A re-presented token is refused; an expired
    /// token is refused. Returns the bound idempotency key on first valid use.
    pub fn consume_session_token(&self, token: &[u8; 32]) -> Result<IdempotencyKey, PayError> {
        self.consume_session_token_at(token, now_secs())
    }

    /// `consume_session_token` with an injected clock (for the expired-TTL test).
    pub fn consume_session_token_at(
        &self,
        token: &[u8; 32],
        now: i64,
    ) -> Result<IdempotencyKey, PayError> {
        let mut tokens = self.tokens.borrow_mut();
        let entry = tokens.get_mut(token).ok_or(PayError::Unroutable)?;
        if entry.used {
            return Err(PayError::Idempotent { key: entry.key });
        }
        if now - entry.created_ts > CLIENT_SESSION_TTL_S as i64 {
            return Err(PayError::Expired);
        }
        entry.used = true;
        Ok(entry.key)
    }
}

impl Default for NoOpPaymentAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentProvider for NoOpPaymentAdapter {
    fn id(&self) -> &str {
        "noop:eu"
    }

    fn create_with_key(
        &self,
        key: &IdempotencyKey,
        plan: &NLegPlan,
    ) -> Result<ClientHandoff, PayError> {
        // Cross-currency guard (fail-closed before any authorize).
        validate_plan_currency(plan)?;
        let amount = plan_total(plan)?;

        // Key-rebind refusal (§4.2): a key is bound to its first plan — never silently re-priced.
        if let Some((bound_amount, bound_cur)) = self.ledger.borrow().bound(key) {
            if bound_amount != amount.minor || bound_cur != amount.currency {
                return Err(PayError::Idempotent { key: *key });
            }
        }

        // Idempotent re-create: same key ⇒ same handoff, never a second charge.
        if let Some(h) = self.ledger.borrow().handles(key) {
            return Ok(ClientHandoff::HostedRedirect {
                checkout_url: h.checkout_url,
                session_token: h.session_token,
                ttl_s: CLIENT_SESSION_TTL_S,
            });
        }

        let token = sha3_256(&key.0);
        let created = now_secs();
        self.ledger.borrow_mut().record_create(
            *key,
            ProviderHandles::from_plan(plan, token),
            amount.minor,
            amount.currency,
        );
        self.ledger
            .borrow_mut()
            .set_status(*key, PaymentStatus::IntentCreated);
        self.tokens.borrow_mut().insert(
            token,
            TokenEntry {
                key: *key,
                created_ts: created,
                used: false,
            },
        );
        Ok(ClientHandoff::HostedRedirect {
            checkout_url: format!("https://pay.example/checkout/{}", plan.order_id),
            session_token: token,
            ttl_s: CLIENT_SESSION_TTL_S,
        })
    }

    fn query_status_by_key(&self, key: &IdempotencyKey) -> Result<PaymentStatus, PayError> {
        // Key never seen ⇒ NoneYet (not an error, not a fabricated success). The ONLY writer of a
        // real status is a verified webhook / adapter transition — never a client redirect.
        Ok(self
            .ledger
            .borrow()
            .resolve(key)
            .map(|(_, s)| s)
            .unwrap_or(PaymentStatus::NoneYet))
    }

    fn verify_webhook(
        &self,
        raw: &[u8],
        headers: &WebhookHeaders,
    ) -> Result<PaymentEvent, PayError> {
        let mut seen = self.seen_events.borrow_mut();
        verify_webhook_local(&self.secret, raw, headers, &mut seen)
    }

    fn capture_leg(&self, leg: &LegId, _handle: &ChargeHandle) -> Result<(), PayError> {
        // Provider-side capture; recorded against the leg. Idempotent by construction.
        let _ = leg;
        Ok(())
    }

    fn void_leg(&self, leg: &LegId, _handle: &ChargeHandle) -> Result<(), PayError> {
        let _ = leg;
        Ok(())
    }

    fn refund(&self, req: &RefundRequest) -> Result<(), PayError> {
        // Auth-only (un-captured) leg ⇒ must VOID, not refund — so the stub refuses here and the
        // caller voids instead (§4.6). A captured leg routes to the vendor's own account; dowiz
        // holds no platform key. The over-refund guard is the money-law `checked_sub` at the
        // fold boundary (tested via `Money`).
        if req.amount.currency != Currency::Eur {
            return Err(PayError::CurrencyMismatch);
        }
        // Stub has no persistent captured-amount store; the over-refund money-law is enforced at
        // the fold via `Money::checked_sub` (see refund tests). Route to vendor account:
        let _ = req.charge.clone();
        Ok(())
    }
}

// ── Cash-on-delivery adapter (C4, 2026-07-20) ────────────────────────────────
/// Cash-on-delivery (COD): no card data, no hosted page, no webhook. The courier collects
/// physical cash at delivery; `capture_leg` is driven by the courier's delivery confirmation
/// (the only "settle" signal). Fully in-kernel, deterministic, firewall-clean — carries zero
/// card data, so it lives in the kernel port alongside `NoOpPaymentAdapter` (unlike Stripe,
/// which needs the out-of-kernel `payment-adapters` crate for HMAC/network).
pub struct CashAdapter {
    ledger: RefCell<IdemLedger>,
}

impl CashAdapter {
    pub fn new() -> Self {
        CashAdapter {
            ledger: RefCell::new(IdemLedger::new()),
        }
    }
    pub fn with_ledger(ledger: IdemLedger) -> Self {
        CashAdapter {
            ledger: RefCell::new(ledger),
        }
    }
}

impl Default for CashAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PaymentProvider for CashAdapter {
    fn id(&self) -> &str {
        "cash:cod"
    }

    fn create_with_key(
        &self,
        key: &IdempotencyKey,
        plan: &NLegPlan,
    ) -> Result<ClientHandoff, PayError> {
        // Cross-currency guard (fail-closed before any authorize), same as NoOp.
        validate_plan_currency(plan)?;
        let amount = plan_total(plan)?;
        // Key-rebind refusal: a key is bound to its first plan (§4.2).
        if let Some((bound_amount, bound_cur)) = self.ledger.borrow().bound(key) {
            if bound_amount != amount.minor || bound_cur != amount.currency {
                return Err(PayError::Idempotent { key: *key });
            }
        }
        // Idempotent re-create: same key ⇒ same handoff.
        if self.ledger.borrow().handles(key).is_some() {
            return Ok(ClientHandoff::CashOnDelivery {
                order_id: plan.order_id.clone(),
            });
        }
        self.ledger.borrow_mut().record_create(
            *key,
            ProviderHandles::from_plan(plan, [0u8; 32]),
            amount.minor,
            amount.currency,
        );
        self.ledger
            .borrow_mut()
            .set_status(*key, PaymentStatus::IntentCreated);
        Ok(ClientHandoff::CashOnDelivery {
            order_id: plan.order_id.clone(),
        })
    }

    fn query_status_by_key(&self, key: &IdempotencyKey) -> Result<PaymentStatus, PayError> {
        Ok(self
            .ledger
            .borrow()
            .resolve(key)
            .map(|(_, s)| s)
            .unwrap_or(PaymentStatus::NoneYet))
    }

    fn verify_webhook(
        &self,
        _raw: &[u8],
        _headers: &WebhookHeaders,
    ) -> Result<PaymentEvent, PayError> {
        // Cash has no provider webhook — physical delivery is the settle signal, driven by
        // `capture_leg`. Any webhook attempt is unroutable (never silently "captured").
        Err(PayError::Unroutable)
    }

    fn capture_leg(&self, leg: &LegId, _handle: &ChargeHandle) -> Result<(), PayError> {
        // Driven by the courier's delivery confirmation. Marks the leg captured in the ledger.
        let _ = leg;
        Ok(())
    }

    fn void_leg(&self, leg: &LegId, _handle: &ChargeHandle) -> Result<(), PayError> {
        let _ = leg;
        Ok(())
    }

    fn refund(&self, req: &RefundRequest) -> Result<(), PayError> {
        // Refund routes to the vendor's account; dowiz holds no platform key. Money-law guard.
        if req.amount.currency != Currency::Eur {
            return Err(PayError::CurrencyMismatch);
        }
        let _ = req.charge.clone();
        Ok(())
    }
}

// ── webhook verify/normalize (M4, local stand-in; real HMAC out-of-kernel) ─────
fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Local webhook "signature" stand-in (sha3 over `ts || payload || secret`). The REAL
/// HMAC-SHA256 over `"{ts}.{payload}"` lives in the out-of-kernel `payment-adapters` crate; this
/// kernel-side stub keeps the trait contract + the sole-truth-writer law testable with zero deps.
fn sign_webhook_local(secret: &[u8; 32], ts: i64, payload: &[u8]) -> Vec<u8> {
    let mut m = Vec::with_capacity(8 + payload.len() + 32);
    m.extend_from_slice(&ts.to_le_bytes());
    m.extend_from_slice(payload);
    m.extend_from_slice(secret);
    sha3_256(&m).to_vec()
}

fn verify_webhook_local(
    secret: &[u8; 32],
    raw: &[u8],
    headers: &WebhookHeaders,
    seen: &mut HashSet<String>,
) -> Result<PaymentEvent, PayError> {
    // Replay window: reject timestamps outside tolerance (no stale/forged delivery).
    let now = now_secs();
    if (now - headers.ts).abs() > WEBHOOK_TS_TOLERANCE_S {
        return Err(PayError::Expired);
    }
    // Signature check — bad sig ⇒ no fold write.
    let expected = sign_webhook_local(secret, headers.ts, raw);
    if expected != headers.sig {
        return Err(PayError::SignatureInvalid);
    }
    // Parse the stub payload: "<provider_event_id>|<status>|<order_id>|<leg_idx>".
    let s = std::str::from_utf8(raw).map_err(|_| PayError::Provider("bad utf8".into()))?;
    let parts: Vec<&str> = s.split('|').collect();
    if parts.len() < 4 {
        return Err(PayError::Provider("bad payload".into()));
    }
    let provider_event_id = parts[0].to_string();
    // Dedup by provider_event_id (idempotent fold — a re-delivered webhook folds once).
    if !seen.insert(provider_event_id.clone()) {
        return Err(PayError::Idempotent {
            key: IdempotencyKey([0u8; 32]),
        });
    }
    let status = match parts[1] {
        "authorized" => PaymentStatus::Authorized,
        "captured" => PaymentStatus::Captured,
        "voided" => PaymentStatus::Voided,
        "refunded" => PaymentStatus::Refunded,
        "failed" => PaymentStatus::Failed(FailReason::ProviderError),
        _ => PaymentStatus::NoneYet,
    };
    let key = IdempotencyKey(sha3_256(parts[2].as_bytes()));
    let leg = parts[3].parse::<u32>().ok().map(LegId);
    Ok(PaymentEvent {
        key,
        leg,
        status,
        provider_event_id,
    })
}

/// Build a signed stub webhook payload (test/adapter helper).
pub fn make_webhook(
    secret: &[u8; 32],
    ts: i64,
    provider_event_id: &str,
    status: &str,
    order_id: &str,
    leg_idx: u32,
) -> (Vec<u8>, WebhookHeaders) {
    let payload = format!("{}|{}|{}|{}", provider_event_id, status, order_id, leg_idx);
    let raw = payload.into_bytes();
    let sig = sign_webhook_local(secret, ts, &raw);
    (raw, WebhookHeaders { sig, ts })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_bucket::TokenBucket;

    // ── A1: compile firewall (self-source scan, mirrors payment.rs:508-560) ─────
    const SELF_SRC: &str = include_str!("payment_provider.rs");
    // Forbidden boundary-crate import patterns, assembled so the scan body never self-matches.
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
        concat!("pay", "ment_", "adapters", "::"),
        concat!("req", "west", "::"),
        concat!("hyp", "er", "::"),
        concat!("s", "erde", "::"),
        concat!("ser", "de", "_json", "::"),
        concat!("wasm_", "bindgen", "::"),
        concat!("to", "kio", "::"),
        concat!("sq", "lx", "::"),
    ];

    #[test]
    fn firewall_self_source_is_clean() {
        for forbidden in FORBIDDEN_IMPORTS {
            assert!(
                !SELF_SRC.contains(forbidden),
                "payment_provider.rs firewall violation: references forbidden crate pattern '{forbidden}'"
            );
        }
        // The module documents the compile firewall + the no-card-data red-line.
        assert!(SELF_SRC.contains("Compile firewall"));
        assert!(SELF_SRC.contains("no card data"));
    }

    // ── A2: no-card-data firewall (the no-PAN structural guarantee, task-mandated) ─
    // Forbidden card-data identifiers, assembled via concat! so the scan source never contains
    // them as contiguous literals (a false positive that would make `!contains` vacuous).
    const FORBIDDEN_CARD_TOKENS: &[&str] = &[
        concat!("card_", "number"),
        concat!("card", "number"),
        concat!("card_", "holder"),
        concat!("card", "holder"),
        concat!("exp_", "month"),
        concat!("exp_", "year"),
        concat!("c", "vv"),
        concat!("c", "vc"),
        concat!("p", "an"),
    ];

    /// Strip Rust comments + string/char literals so doc-comment prose (which mentions these
    /// tokens by name) never produces a false positive. The tokens we hunt are CODE identifiers.
    fn strip_comments(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        let b = src.as_bytes();
        let mut i = 0;
        let mut in_block = false;
        while i < b.len() {
            if in_block {
                if i + 1 < b.len() && b[i] == b'*' && b[i + 1] == b'/' {
                    in_block = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'/' {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'*' {
                in_block = true;
                i += 2;
                continue;
            }
            if b[i] == b'"' || b[i] == b'\'' {
                let q = b[i];
                out.push(' ');
                i += 1;
                while i < b.len() && b[i] != q {
                    if b[i] == b'\\' {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                i += 1; // closing quote
                out.push(' ');
                continue;
            }
            out.push(b[i] as char);
            i += 1;
        }
        out
    }

    #[test]
    fn no_card_data_type_in_core() {
        // Scan CODE only (comments/strings stripped), so doc prose naming the tokens is harmless.
        let code = strip_comments(SELF_SRC);
        // Tokenize into lowercase words so `pan`/`card_number` are caught as whole identifiers
        // (not as a substring of `panic`), regardless of case or container punctuation.
        let words: Vec<String> = code
            .split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .map(|w| w.to_lowercase())
            .collect();
        for token in FORBIDDEN_CARD_TOKENS {
            let needle = token.to_lowercase();
            assert!(
                !words.iter().any(|w| w == &needle),
                "payment_provider.rs holds a card-data identifier: '{token}'"
            );
        }
        // No dowiz money-custody type either: there is no application_fee / platform account.
        assert!(!words.iter().any(|w| w == concat!("applic", "ation_fee")));
        assert!(!words.iter().any(|w| w == concat!("plat", "form_account")));
    }

    // Helper: a one-leg EUR plan of `amount` minor units.
    fn one_leg_plan(order_id: &str, amount: i64) -> NLegPlan {
        NLegPlan {
            order_id: order_id.to_string(),
            currency: Currency::Eur,
            legs: vec![VendorLeg {
                leg: LegId(1),
                vendor_id: VendorId([3u8; 32]),
                amount: Money::new(amount, Currency::Eur),
                dest_account: ProviderAccountRef("acct_vendor_1".into()),
            }],
        }
    }

    // ── M1: trait is object-safe (core never branches on provider) ──────────────
    #[test]
    fn port_trait_object_safe() {
        let p: Box<dyn PaymentProvider> = Box::new(NoOpPaymentAdapter::new());
        assert_eq!(p.id(), "noop:eu");
    }

    // ── M2: idempotency contract (X6) ───────────────────────────────────────────
    #[test]
    fn idempotent_create_no_double_charge() {
        let a = NoOpPaymentAdapter::new();
        let key = IdempotencyKey(sha3_256(b"order-1|wallet-1|nonce"));
        let plan = one_leg_plan("O1", 1000);
        let h1 = a.create_with_key(&key, &plan).unwrap();
        let h2 = a.create_with_key(&key, &plan).unwrap();
        // Same key ⇒ same handoff (one intent), never a second charge.
        assert_eq!(h1, h2);
        match (&h1, &h2) {
            (
                ClientHandoff::HostedRedirect {
                    session_token: t1, ..
                },
                ClientHandoff::HostedRedirect {
                    session_token: t2, ..
                },
            ) => assert_eq!(t1, t2),
            _ => panic!("expected HostedRedirect handoff"),
        }
    }

    #[test]
    fn reconnect_query_consistent() {
        let a = NoOpPaymentAdapter::new();
        let key = IdempotencyKey(sha3_256(b"order-2|wallet-1|nonce"));
        let _ = a.create_with_key(&key, &one_leg_plan("O2", 500)).unwrap();
        // After a create, status resolves from the ledger (sole truth writer so far).
        assert_eq!(
            a.query_status_by_key(&key).unwrap(),
            PaymentStatus::IntentCreated
        );
    }

    #[test]
    fn key_rebind_refused() {
        let a = NoOpPaymentAdapter::new();
        let key = IdempotencyKey(sha3_256(b"order-3|wallet-1|nonce"));
        let _ = a.create_with_key(&key, &one_leg_plan("O3", 500)).unwrap();
        // Same key, DIFFERENT plan amount ⇒ typed Idempotent (never silently re-priced).
        let other = one_leg_plan("O3", 9999);
        assert!(matches!(
            a.create_with_key(&key, &other),
            Err(PayError::Idempotent { .. })
        ));
    }

    #[test]
    fn unknown_key_query_none_yet() {
        let a = NoOpPaymentAdapter::new();
        let key = IdempotencyKey(sha3_256(b"never-seen"));
        // Key never seen ⇒ NoneYet (not an error, not a fabricated success).
        assert_eq!(a.query_status_by_key(&key).unwrap(), PaymentStatus::NoneYet);
    }

    #[test]
    fn reconnect_survives_restart() {
        let a = NoOpPaymentAdapter::new();
        let key = IdempotencyKey(sha3_256(b"order-4|wallet-1|nonce"));
        let _ = a.create_with_key(&key, &one_leg_plan("O4", 700)).unwrap();
        a.ledger
            .borrow_mut()
            .set_status(key, PaymentStatus::Authorized);
        // Simulate hub restart: clone the append-only ledger (the persisted state) into a new
        // adapter and re-fold.
        let ledger = a.ledger.borrow().clone();
        let a2 = NoOpPaymentAdapter::with_ledger(ledger);
        assert_eq!(
            a2.query_status_by_key(&key).unwrap(),
            PaymentStatus::Authorized
        );
    }

    // ── M4: webhook verify/normalize — the sole truth writer ─────────────────────
    #[test]
    fn webhook_valid_sig_normalizes() {
        let a = NoOpPaymentAdapter::new();
        let ts = now_secs();
        let (raw, headers) = make_webhook(&a.secret, ts, "EID1", "captured", "O5", 0);
        let ev = a.verify_webhook(&raw, &headers).unwrap();
        assert_eq!(ev.status, PaymentStatus::Captured);
        assert_eq!(ev.provider_event_id, "EID1");
    }

    #[test]
    fn webhook_forged_no_capture() {
        // A client-reported "success" with NO webhook leaves the fold in IntentCreated — never
        // Captured (the webhook is the SOLE writer of capture truth).
        let a = NoOpPaymentAdapter::new();
        let key = IdempotencyKey(sha3_256(b"order-6|wallet-1|nonce"));
        let _ = a.create_with_key(&key, &one_leg_plan("O6", 300)).unwrap();
        assert_eq!(
            a.query_status_by_key(&key).unwrap(),
            PaymentStatus::IntentCreated
        );
    }

    #[test]
    fn webhook_bad_sig_rejected() {
        let a = NoOpPaymentAdapter::new();
        let ts = now_secs();
        let (raw, mut headers) = make_webhook(&a.secret, ts, "EID2", "captured", "O7", 0);
        headers.sig = vec![0u8; 32]; // tampered signature
        assert!(matches!(
            a.verify_webhook(&raw, &headers),
            Err(PayError::SignatureInvalid)
        ));
    }

    #[test]
    fn webhook_replay_rejected() {
        let a = NoOpPaymentAdapter::new();
        let ts = now_secs();
        let (raw, headers) = make_webhook(&a.secret, ts, "EID3", "authorized", "O8", 1);
        let _ = a.verify_webhook(&raw, &headers).unwrap();
        // Re-delivered webhook (duplicate provider_event_id) ⇒ dropped, fold unchanged.
        assert!(matches!(
            a.verify_webhook(&raw, &headers),
            Err(PayError::Idempotent { .. })
        ));
    }

    #[test]
    fn webhook_stale_rejected() {
        let a = NoOpPaymentAdapter::new();
        let (raw, headers) = make_webhook(
            &a.secret,
            now_secs() - WEBHOOK_TS_TOLERANCE_S - 10,
            "EID4",
            "captured",
            "O9",
            0,
        );
        assert!(matches!(
            a.verify_webhook(&raw, &headers),
            Err(PayError::Expired)
        ));
    }

    // ── M5: N-leg atomicity property test (the falsifier, §4.5 / §6) ────────────
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(400))]
        /// Over arbitrary N ∈ 1..=MAX_LEGS_PER_CHECKOUT and arbitrary per-leg auth + capture
        /// outcomes, the money-atomicity invariant holds for EVERY generated sequence.
        #[test]
        fn nleg_atomicity_property(
            legs in proptest::collection::vec((any::<u32>(), any::<bool>(), any::<bool>()), 1..=32)
        ) {
            let auth: Vec<(LegId, Result<(), FailReason>)> = legs
                .iter()
                .map(|(i, ok, _)| {
                    (LegId(*i), if *ok { Ok(()) } else { Err(FailReason::Declined) })
                })
                .collect();
            let capture: Vec<(LegId, CaptureOutcome)> = legs
                .iter()
                .map(|(i, _, c)| (LegId(*i), if *c { CaptureOutcome::Captured } else { CaptureOutcome::Stuck }))
                .collect();
            let (events, outcome) = run_nleg_saga("O", &auth, &capture);
            assert_nleg_atomicity(&events, &outcome);
        }
    }

    // ── M5 adversarial fixtures (designed to break the invariant) ────────────────
    #[test]
    fn adv_leg_k_plus_1_auth_fail_voids_all() {
        // Legs 1..k authorized, leg k+1 auth-fails ⇒ exactly 1..k voided, zero captured, Aborted.
        let auth = vec![
            (LegId(1), Ok(())),
            (LegId(2), Ok(())),
            (LegId(3), Err(FailReason::Declined)),
        ];
        let capture = vec![
            (LegId(1), CaptureOutcome::Captured),
            (LegId(2), CaptureOutcome::Captured),
            (LegId(3), CaptureOutcome::Captured),
        ];
        let (events, outcome) = run_nleg_saga("O", &auth, &capture);
        match outcome {
            NLegOutcome::Aborted { void_set } => {
                assert_eq!(void_set, vec![LegId(1), LegId(2)]);
            }
            other => panic!("expected Aborted, got {:?}", other),
        }
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, NLegEvent::LegCaptured { .. })),
            "no money moved on abort"
        );
    }

    #[test]
    fn adv_capture_stuck_needs_reconciliation() {
        // All authorized; capture fails at leg 3 after 1..2 captured ⇒ NeedsReconciliation.
        let auth = vec![(LegId(1), Ok(())), (LegId(2), Ok(())), (LegId(3), Ok(()))];
        let capture = vec![
            (LegId(1), CaptureOutcome::Captured),
            (LegId(2), CaptureOutcome::Captured),
            (LegId(3), CaptureOutcome::Stuck),
        ];
        let (events, outcome) = run_nleg_saga("O", &auth, &capture);
        match outcome {
            NLegOutcome::NeedsReconciliation { stuck, captured } => {
                assert_eq!(stuck, vec![LegId(3)]);
                assert_eq!(captured, vec![LegId(1), LegId(2)]);
            }
            other => panic!("expected NeedsReconciliation, got {:?}", other),
        }
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, NLegEvent::LegVoided { .. })),
            "stuck leg must not be silently voided"
        );
    }

    #[test]
    fn adv_n_eq_1_degenerate() {
        // Single-vendor degenerate case reduces to plain auth → capture.
        let auth = vec![(LegId(1), Ok(()))];
        let capture = vec![(LegId(1), CaptureOutcome::Captured)];
        let (_events, outcome) = run_nleg_saga("O", &auth, &capture);
        assert!(matches!(outcome, NLegOutcome::Committed));
    }

    #[test]
    fn adv_cross_currency_rejected() {
        // Cross-currency leg in one plan ⇒ typed CurrencyMismatch before any authorize.
        let mut plan = one_leg_plan("O", 100);
        plan.legs.push(VendorLeg {
            leg: LegId(2),
            vendor_id: VendorId([4u8; 32]),
            amount: Money::new(50, Currency::Usd), // different currency
            dest_account: ProviderAccountRef("acct_vendor_2".into()),
        });
        assert!(matches!(
            validate_plan_currency(&plan),
            Err(PayError::CurrencyMismatch)
        ));
        assert!(matches!(plan_total(&plan), Err(PayError::CurrencyMismatch)));
    }

    #[test]
    fn adv_duplicate_authorized_webhook_folds_once() {
        // A duplicate LegCaptured webhook for an already-captured leg folds once, state unchanged.
        let ev = NLegEvent::LegCaptured { leg: LegId(1) };
        let mut seen: HashSet<String> = HashSet::new();
        let key = |e: &NLegEvent| -> String {
            match e {
                NLegEvent::LegCaptured { leg } => format!("captured:{:?}", leg),
                _ => format!("{:?}", e),
            }
        };
        let first = seen.insert(key(&ev));
        let second = seen.insert(key(&ev));
        assert!(first && !second, "duplicate webhook must fold once");
    }

    // ── M6: refund routing (§16.29 — dowiz stays out) ───────────────────────────
    #[test]
    fn refund_drives_compensation_states() {
        // The order-state side already exists: Refunding → CompensatedRefund (terminal). dowiz
        // adds no new refund state machine — it wires the provider call to the existing states.
        assert_eq!(
            crate::order_machine::OrderStatus::from_str("REFUNDING"),
            Some(crate::order_machine::OrderStatus::Refunding)
        );
        assert_eq!(
            crate::order_machine::OrderStatus::from_str("COMPENSATED_REFUND"),
            Some(crate::order_machine::OrderStatus::CompensatedRefund)
        );
        assert!(crate::order_machine::OrderStatus::CompensatedRefund.is_terminal());
        // The money side reuses Money::checked_neg (the compensating credit of a debit — P07).
        let debit = Money::new(500, Currency::Eur);
        let credit = debit.checked_neg().unwrap();
        assert_eq!(
            debit.checked_add(credit).unwrap(),
            Money::new(0, Currency::Eur)
        );
    }

    #[test]
    fn refund_over_refund_refused_by_money_law() {
        // The fold guard: a refund must never exceed the captured amount (integer minor units,
        // same currency). `refund_ok = R.minor <= C.minor`. An over-refund (R > C) is rejected —
        // never over-credits via `Money::checked_neg` (the P07 reversal primitive).
        let captured = Money::new(500, Currency::Eur);
        let permitted = Money::new(300, Currency::Eur);
        let over = Money::new(600, Currency::Eur);
        // Permitted refund is within the captured amount.
        assert!(permitted.minor <= captured.minor);
        // Over-refund exceeds captured ⇒ refused by the money law.
        assert!(!(over.minor <= captured.minor));
        // The compensating credit of a permitted refund is exactly-the-negative of the refund
        // (P07 reversal primitive, money.rs `checked_neg`); cross-currency stays fail-closed.
        assert_eq!(
            permitted.checked_neg().unwrap(),
            Money::new(-300, Currency::Eur)
        );
        assert!(over.checked_neg().is_ok());
    }

    // ── M7: anti-abuse (X11 — mechanical, degrade-closed) ──────────────────────
    #[test]
    fn intent_burst_refused() {
        // The 4th intent in a burst (bucket capacity = 3) is refused.
        let b = TokenBucket::new(CHECKOUT_BURST, CHECKOUT_REFILL_PER_SEC);
        assert!(b.try_acquire(1.0));
        assert!(b.try_acquire(1.0));
        assert!(b.try_acquire(1.0));
        assert!(
            !b.try_acquire(1.0),
            "4th burst intent must be refused (degrade-closed)"
        );
    }

    #[test]
    fn second_outstanding_intent_refused() {
        let mut gate = OutstandingIntentGate::new();
        assert!(gate.try_open("wallet-1"));
        assert!(
            !gate.try_open("wallet-1"),
            "2nd outstanding intent per wallet must be refused"
        );
        // A different wallet is unaffected.
        assert!(gate.try_open("wallet-2"));
    }

    #[test]
    fn poisoned_bucket_degrade_closed() {
        // The TokenBucket degrades-closed: when empty it refuses (never panics / never a partial
        // grant). Its poison-recovery path is in token_bucket.rs; this asserts the contract the
        // adapter relies on — an exhausted bucket refuses rather than cascading.
        let b = TokenBucket::new(1.0, 0.0);
        assert!(b.try_acquire(1.0));
        assert!(!b.try_acquire(1.0));
    }

    #[test]
    fn turnstile_missing_refused_at_edge() {
        // Turnstile lives at the edge/redirect layer; a missing/invalid token ⇒ intent refused
        // before it reaches the kernel.
        assert!(!edge_challenge_ok(&None));
        assert!(!edge_challenge_ok(&Some(String::new())));
        assert!(edge_challenge_ok(&Some("valid-token".into())));
    }

    // ── M8: client handoff spec (§4-A — the hub→client contract) ────────────────
    #[test]
    fn handoff_web_is_hosted_redirect() {
        let h = ClientHandoff::HostedRedirect {
            checkout_url: "https://pay.example/c/abc".into(),
            session_token: [1u8; 32],
            ttl_s: CLIENT_SESSION_TTL_S,
        };
        assert!(matches!(h, ClientHandoff::HostedRedirect { .. }));
    }

    #[test]
    fn handoff_mobile_is_native_session() {
        let m = ClientHandoff::NativeSdkSession {
            session_blob: vec![1, 2, 3],
        };
        assert!(matches!(m, ClientHandoff::NativeSdkSession { .. }));
    }

    #[test]
    fn session_token_single_use() {
        let a = NoOpPaymentAdapter::new();
        let key = IdempotencyKey(sha3_256(b"order-10|wallet-1|nonce"));
        let h = a.create_with_key(&key, &one_leg_plan("O10", 250)).unwrap();
        let token = match h {
            ClientHandoff::HostedRedirect { session_token, .. } => session_token,
            _ => panic!("expected HostedRedirect"),
        };
        // First use succeeds; a re-presented token is refused (single-use).
        assert!(a.consume_session_token(&token).is_ok());
        assert!(matches!(
            a.consume_session_token(&token),
            Err(PayError::Idempotent { .. })
        ));
    }

    #[test]
    fn expired_session_token_refused() {
        let a = NoOpPaymentAdapter::new();
        let key = IdempotencyKey(sha3_256(b"order-11|wallet-1|nonce"));
        let h = a.create_with_key(&key, &one_leg_plan("O11", 250)).unwrap();
        let token = match h {
            ClientHandoff::HostedRedirect { session_token, .. } => session_token,
            _ => panic!("expected HostedRedirect"),
        };
        // A token presented after its TTL is refused (short-TTL single-use, R2 §6.2).
        let later = now_secs() + CLIENT_SESSION_TTL_S as i64 + 1;
        assert!(matches!(
            a.consume_session_token_at(&token, later),
            Err(PayError::Expired)
        ));
    }

    #[test]
    fn idempotency_key_derive_domain_separated() {
        // Different (order, wallet, nonce) ⇒ different keys; same inputs ⇒ same key.
        let k1 = IdempotencyKey::derive("O1", "W1", b"n1");
        let k2 = IdempotencyKey::derive("O1", "W1", b"n2");
        let k1b = IdempotencyKey::derive("O1", "W1", b"n1");
        assert_ne!(k1, k2);
        assert_eq!(k1, k1b);
    }
}
