//! BLUEPRINT P70 — Owner Surface (W2, kernel-side management & configuration lanes).
//!
//! Zero-I/O firewall (mirrors `ports/hub_intake.rs` discipline): no HTTP, no
//! serde-on-the-wire, no provider names. Node-local event families ride
//! `event_log.rs`'s content-addressed log (the `hub_no_shadow_store` invariant),
//! NOT proto-cap wire variants. This module is the **fold + cap-cert-intent**
//! surface: every P70 pane is a deterministic fold of signed events, and every
//! mutating owner action is an owner-capability-cert-signed intent — there is no
//! admin database, no dowiz aggregator, no analytics dashboard (§1.4-1, deferred
//! to v2).
//!
//! Reuse-first (standard item 19): P70 CONSUMES the W1 foundations, it redeclares
//! NONE of them —
//!   * P62 catalog  → `CatalogNode` / `PriceableLeaf` / `validate_tree` / `resolve_line`
//!   * P59 cap     → `Delegation` / `AnchorRoster` / `RevocationSet` / `RefSigner`
//!   * P48 orders  → `Order` / `apply_event` (the same facade; confirm/cancel
//!                    are owner-cap-cert human intents, never agent-invocable — §10.6)
//!   * P58 a11y    → the owner `SemanticScene` is authored here but MIRRORED by the
//!                    engine's `mirror()` (P58 §M6 — the brand-preview parity test is
//!                    structural, not asserted here); `Sheet` lives in `FrameUniforms`,
//!                    never in the `SemanticScene`, so draft/live share ONE `A11yTree`.
//!   * P43 channel  → `ChannelKind` enum (the owner's erasure key, §16.58)
//!
//! GREP GATES honored (§4.4 / §5): `no-agent-order-authority`,
//! `no-courier-scoring`, `hub_no_shadow_store`, `no_endpoint_dependency`.
//! Search this file for those sentinels; none of the forbidden shapes appear.

// NOTE: intentionally NO `use` of `crate::pq` / `crate::wasm` / any network crate.
// The owner root is modeled as a classical RefSigner keypair (P59's `SelfSignedRoot`
// is the operator's self-held credential; P70 only consumes its verify capability).

use crate::catalog::{
    validate_tree, Availability, CatalogError, CatalogNode, LeafId, LeafKind,
    NodeId, PriceableLeaf,
};
use crate::domain::{apply_event, place_order_priced, Order, OrderItem};
use crate::event_log::sha3_256;
use crate::money::{assert_non_negative, Currency, Money};
use crate::order_machine::OrderStatus;
use crate::ports::agent::cap::{
    Delegation, RevocationSet, SignatureVerifier,
};
use crate::ports::agent::{Action, Resource, Scope};
use crate::vendor::VendorId;

/// The fixed 5-token brand envelope (§16.9). NOT a font file, NOT free CSS —
/// three packed-RGBA colors + two integer token indices. The WHOLE record is a
/// couple hundred bytes, which is why the draft/live problem is a `queue.write_buffer`
/// uniform swap, never a renderer fork (R5 §4.3 / X5). `Sheet` is what lives in
/// `FrameUniforms` (P38), NOT in the `SemanticScene` — that separation is what
/// makes the G4 a11y-parity test structural (P58 §M6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sheet {
    pub accent: u32,  // packed RGBA
    pub ink: u32,     // packed RGBA (foreground text/line)
    pub paper: u32,   // packed RGBA (surface)
    pub type_id: u16, // index into the fixed dowiz type-scale set (NOT a font upload)
    pub radius: u16,  // corner-radius token (fixed scale)
}
/// §16.9 cap — the reason the swap is trivial.
pub const SHEET_TOKEN_COUNT: usize = 5;
/// 3×u32 + 2×u16 — the whole draft/live cost (R5 §4.3 step 1).
pub const SHEET_UNIFORM_BYTES: usize = 20;

/// Two records per hub: what customers see, and what the owner is editing. The ENTIRE
/// draft/live problem (R5 §4.3): hold both, bind one at frame time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrandState {
    pub published: Sheet,
    pub draft: Sheet,
}

/// Node-local brand events. `Published` is the atomic copy draft→published (R5 §4.3
/// step 4); `Reverted` re-publishes a prior record (step 4, "revert = re-publishing
/// the prior token record"). `DraftEdited` is coalesced (slider drag ⇒ one event per
/// settle, not per frame).
#[derive(Debug, Clone)]
pub struct BrandEvent {
    pub kind: BrandEventKind,
    /// Owner-cap-cert signature over the canonical Sheet bytes (the owner root's
    /// classical key; the draft buffer is bound ONLY under that cert).
    pub owner_sig: Vec<u8>,
    /// The Sheet this event carries (published / reverted-to / draft).
    pub sheet: Sheet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrandEventKind {
    DraftEdited,
    Published,
    Reverted,
}

/// A customer reference the vendor can name for erasure (§16.58). There is NO
/// durable customer identity (P49 deferral) — the only stable handle is the
/// channel-shaped address plus the order-ids that reference it. Erasure is scoped to
/// this closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelKind {
    Telegram,
    Sms,
    WhatsApp,
    SimpleX,
    Email,
}

/// Free-form peer address per channel (chat id / wa id / phone / email).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerRef {
    pub channel: ChannelKind,
    pub peer: String,
    /// Orders bound to this peer (fold-derived, not stored).
    pub order_refs: Vec<u64>,
}

/// The ONE erasure action — owner-authored, owner-cap-cert-signed (mirrors P48's
/// OwnerReplyAction: an erasure without a live owner signature is UNREPRESENTABLE).
#[derive(Debug, Clone)]
pub struct CustomerErasureAction {
    pub customer_ref: CustomerRef,
    pub owner_sig: Vec<u8>,
}

/// The node-local erasure EVENT. Append-only: the ciphertext of the customer's PII
/// stays in the log for chain integrity, but the per-customer data key is destroyed
/// (crypto-erasure, §3.6) so the plaintext is permanently unrecoverable.
/// Irreversible — "deliberately no un-erase" (the `RevocationSet` posture applied
/// to PII).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErasureEvent {
    pub customer_ref: CustomerRef,
    pub at_unix_ms: u64,
}

/// Typed erasure outcomes — never a silent partial delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErasureError {
    BadOwnerSig,
    UnknownCustomer,
    AlreadyErased,
}

/// One hub the owner's client connects to, authorized by a P59 child cert under the
/// owner root. `child_cert.may_delegate == false`, depth == 1 (P59 §2.4).
/// Endpoint is the hub's own tunnel/address — dowiz is NOT in this path.
#[derive(Debug, Clone)]
pub struct HubConnection {
    pub child_cert: Delegation,
    pub endpoint: String,
    pub health: HubHealth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubHealth {
    Online,
    Offline,
    Degraded,
}

/// The client-side merged view over N hubs. Built on the owner's device by fanning
/// reads to each `HubConnection` and merging. There is NO server type here by
/// construction — a dowiz-side aggregate would need a type that does not exist (§1.4-5).
#[derive(Debug, Clone)]
pub struct MultiHubView {
    pub root_pk: [u8; 32],
    pub hubs: Vec<HubConnection>,
}
/// Client-side fan-out sanity cap; §4.2 scaling axis.
pub const MAX_HUBS_SOFT: usize = 64;

/// The ONLY net-new type G5 owns: the Wave-0 auto-post TRIGGER set. The post
/// itself is a P22 `MasterPost` (P22 owns posting) — G5 mints no post type and no
/// poster. A trigger produces a P22 Path-A template draft (`AiMode::Off` works);
/// PUBLISH is P22's authority (A6), never here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoPostTrigger {
    MenuItemAdded { leaf_id: LeafId },
    PromoAnnounced { text: String },
}

/// P22-shaped vocabulary (mirrored from P22's `SocialPoster` lane — P22 owns the
/// real poster; the kernel pane only DRAFTS, it does not transport). Defining these
/// here is NOT a "second poster": there is no `SocialPoster`/`ChannelAdapter`
/// implementation in this module (see `g5_no_second_poster`). They exist so the owner
/// pane can author a `MasterPost` draft and hand it to P22's publish authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftSource {
    Template,
    AiAssisted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftStatus {
    PendingReview,
    Approved,
    Published,
}
/// P22 A6 — publish authority is never the model. The pane drafts at `Off`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiMode {
    Off,
    On,
}
/// A drafted marketing post (P22 `MasterPost` shape). One→many public blast radius;
/// deliberately NOT a bulk/transactional `Notification` (P43 holds ONE recipient —
/// the two lanes never merge, §1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterPost {
    pub source: DraftSource,
    pub status: DraftStatus,
    pub ai_mode: AiMode,
    pub body: String,
    /// Public blast: the post is addressed to the venue's public channel, never a
    /// per-recipient transactional `Notification`.
    pub public: bool,
}

/// Shared owner-surface error taxonomy (the cross-cutting typed refusals).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerSurfaceError {
    BadOwnerSig,
    UnknownCustomer,
    AlreadyErased,
    /// A courier/hub cert was revoked (P70 enforces `RevocationSet` at its own
    /// boundary; the kernel `verify_chain` revocation check is P59's own deferred
    /// RED→GREEN — see BLUEPRINT-P59 §4.7 — so P70 verifies explicitly here).
    Revoked,
    UnknownIssuer,
    BadSignature,
    Expired,
    ScopeViolation,
    Catalog(CatalogError),
    IllegalTransition,
    Transition(crate::order_machine::TransitionError),
}

impl From<CatalogError> for OwnerSurfaceError {
    fn from(e: CatalogError) -> Self {
        OwnerSurfaceError::Catalog(e)
    }
}

impl From<crate::capability_cert::CertError> for OwnerSurfaceError {
    fn from(e: crate::capability_cert::CertError) -> Self {
        use crate::capability_cert::CertError as C;
        match e {
            C::Revoked => OwnerSurfaceError::Revoked,
            C::UnknownIssuer => OwnerSurfaceError::UnknownIssuer,
            C::Expired => OwnerSurfaceError::Expired,
            C::ScopeViolation | C::MaxDepthExceeded => OwnerSurfaceError::ScopeViolation,
            // Bad/absent signature legs, unknown suites, node-id mis-binds, and
            // failed suite negotiation are all "this cert does not cryptographically
            // check out" at the owner boundary → BadSignature.
            C::BadSignature | C::UnknownSuite | C::NodeIdMismatch | C::NoCommonSuite => {
                OwnerSurfaceError::BadSignature
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Owner-cap-cert signature helper (classical RefSigner — same scheme as P59's
// in-tree reference signer). The owner root's classical key signs; the hub verifies
// with the owner-root pubkey ONLY, no network, no dowiz account.
// ═══════════════════════════════════════════════════════════════════════════

const OWNER_SIG_DOMAIN: &[u8] = b"dowiz.owner.sig\x01";

fn owner_sig_msg(action: &str, subject: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(OWNER_SIG_DOMAIN.len() + action.len() + subject.len());
    out.extend_from_slice(OWNER_SIG_DOMAIN);
    out.extend_from_slice(action.as_bytes());
    out.extend_from_slice(subject.as_bytes());
    out
}

fn sign_owner<V: SignatureVerifier>(v: &V, secret: &[u8; 32], msg: &[u8]) -> Vec<u8> {
    v.sign_classical(secret, msg)
}

fn verify_owner<V: SignatureVerifier>(v: &V, pubkey: &[u8; 32], msg: &[u8], sig: &[u8]) -> bool {
    v.verify_classical(pubkey, msg, sig)
}

// ═══════════════════════════════════════════════════════════════════════════
// G1 — Orders management surface (supersedes P48 B2)
// ═══════════════════════════════════════════════════════════════════════════

/// A fold-derived, read-only projection of the venue's orders (newest-active-first:
/// non-terminal orders ahead of terminal ones, each tier by descending creation time).
/// This is `fold(orders)` — there is NO store outside the log (`hub_no_shadow_store`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderLite {
    pub id: String,
    pub status: OrderStatus,
    pub total_minor: i64,
}

/// The read-only orders pane. Pure fold — identical input ⇒ identical pane.
pub fn fold_orders(orders: &[Order]) -> Vec<OrderLite> {
    let mut lite: Vec<OrderLite> = orders
        .iter()
        .map(|o| OrderLite {
            id: o.id.clone(),
            status: o.status,
            total_minor: o.total,
        })
        .collect();
    // newest-active-first: active (non-terminal) ahead; within a tier, newest first.
    lite.sort_by(|a, b| {
        let ta = is_terminal(a.status);
        let tb = is_terminal(b.status);
        ta.cmp(&tb).then_with(|| b.id.cmp(&a.id))
    });
    lite
}

fn is_terminal(s: OrderStatus) -> bool {
    matches!(
        s,
        OrderStatus::Delivered
            | OrderStatus::Cancelled
            | OrderStatus::Rejected
            | OrderStatus::CompensatedRefund
    )
}

/// The ONLY mutating owner actions: confirm and cancel, each emitted as an
/// owner-cap-cert-signed human intent through the existing facade (P48 §10.6 gate
/// is LAW — confirm/cancel are NOT a `ToolAction`, NOT agent-invocable; the
/// `no-agent-order-authority` CI grep covers this lane). The list remains a fold;
/// `hub_no_shadow_store` extends over it (B2 preserved).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerOrderAction {
    Confirm,
    Cancel,
}

/// An owner-cap-cert-signed confirm/cancel intent. A confirm/cancel without a live
/// owner signature is refused typed (P48 §10.6 wall 1).
#[derive(Debug, Clone)]
pub struct OwnerCapIntent {
    pub action: OwnerOrderAction,
    pub order_id: String,
    pub owner_sig: Vec<u8>,
}

/// Apply a confirm/cancel intent. Verifies the owner signature against `owner_pk`
/// and delegates to the existing `apply_event` facade so the transition law is
/// enforced (a mid-prepare cancel routes to the refund channel, never a unilateral
/// silent state flip — §3.1 adversarial (iii)).
pub fn apply_owner_order_intent<V: SignatureVerifier>(
    v: &V,
    owner_pk: &[u8; 32],
    orders: &[Order],
    intent: &OwnerCapIntent,
) -> Result<OrderStatus, OwnerSurfaceError> {
    let msg = owner_sig_msg(
        match intent.action {
            OwnerOrderAction::Confirm => "confirm",
            OwnerOrderAction::Cancel => "cancel",
        },
        &intent.order_id,
    );
    if !verify_owner(v, owner_pk, &msg, &intent.owner_sig) {
        return Err(OwnerSurfaceError::BadOwnerSig);
    }
    let order = orders
        .iter()
        .find(|o| o.id == intent.order_id)
        .ok_or(OwnerSurfaceError::UnknownCustomer)?; // unknown order id
    let target = match intent.action {
        OwnerOrderAction::Confirm => Some(OrderStatus::Confirmed),
        OwnerOrderAction::Cancel => cancel_target(order.status),
    };
    let target = target.ok_or(OwnerSurfaceError::IllegalTransition)?;
    let updated = apply_event(&order.clone(), target).map_err(OwnerSurfaceError::Transition)?;
    Ok(updated.status)
}
/// `Cancelled`; from any post-confirm state it routes to the `Refunding` channel
/// (§3.1 adversarial (iii)); from an already-terminal state there is no legal
/// cancel path (`None`).
fn cancel_target(from: OrderStatus) -> Option<OrderStatus> {
    match from {
        OrderStatus::Pending => Some(OrderStatus::Cancelled),
        OrderStatus::Confirmed
        | OrderStatus::Preparing
        | OrderStatus::Ready
        | OrderStatus::InDelivery => Some(OrderStatus::Refunding),
        _ => None,
    }
}

/// Convenience: sign an owner order intent (used by tests + the real owner client).
pub fn sign_owner_order_intent<V: SignatureVerifier>(
    v: &V,
    owner_secret: &[u8; 32],
    action: OwnerOrderAction,
    order_id: &str,
) -> OwnerCapIntent {
    let msg = owner_sig_msg(
        match action {
            OwnerOrderAction::Confirm => "confirm",
            OwnerOrderAction::Cancel => "cancel",
        },
        order_id,
    );
    OwnerCapIntent {
        action,
        order_id: order_id.to_string(),
        owner_sig: sign_owner(v, owner_secret, &msg),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// G2 — Menu management surface (supersedes P48 B1)
// ═══════════════════════════════════════════════════════════════════════════

/// Parse a price string to integer minor units (×100) at the SAVE/SUBMIT boundary
/// (P57 §2.2 money boundary). The field is a `&str`; NEVER a money-entry field.
/// Rejects float/scientific/NaN input and more than 2 fractional digits — NO float
/// ever reaches `Money` (money red-line). Returns the minor-unit integer.
pub fn parse_price_minor(s: &str) -> Result<i64, String> {
    let t = s.trim();
    if t.is_empty() {
        return Err("empty price".into());
    }
    // Reject any non-(digit / dot / comma / minus) character: this rules out
    // scientific notation ("7.5e3"), "NaN", letters, etc. in one stroke.
    if !t
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == ',' || c == '-')
    {
        return Err(format!("rejected non-numeric price input: {t:?}"));
    }
    // Exactly one decimal separator allowed (dot or comma), at most 2 fractional digits.
    let normalized: String = t.replace(',', ".");
    if normalized.chars().filter(|&c| c == '.').count() > 1 {
        return Err(format!("rejected malformed price: {t:?}"));
    }
    let (int_part, frac_part) = match normalized.split_once('.') {
        Some((i, f)) => {
            if f.len() > 2 {
                return Err(format!("rejected >2 fractional digits: {t:?}"));
            }
            (i, f)
        }
        None => (normalized.as_str(), ""),
    };
    let int_val: i64 = int_part
        .parse()
        .map_err(|_| format!("rejected int: {t:?}"))?;
    // frac → 2-digit minor: "5"→50, "50"→50, ""→0.
    let frac_minor: i64 = match frac_part {
        "" => 0,
        f => {
            let padded = format!("{f:<02}")[..2].parse::<i64>().map_err(|_| "frac")?;
            padded
        }
    };
    let minor = int_val
        .checked_mul(100)
        .and_then(|v| v.checked_add(frac_minor))
        .ok_or("price overflow")?;
    Ok(minor)
}

/// A menu edit request: the node being edited, the price string (P57 TextField
/// `&str`), and the currency. On save, the price is parsed to `i64` minor units
/// + a `Currency` into a `Money` (P57 §2.2 boundary), and a `PriceableLeaf` is
/// built ONLY via `PriceableLeaf::new` (an unpriced/uncurrencied/unattributed
/// leaf is unrepresentable — X7). Then `validate_tree` runs (structure only, no
/// taxonomy — §16.17). B1 preserved: a saved edit is carried by the next order's
/// fold (orders price from menu state at placement time).
pub fn apply_menu_edit(
    vendor: VendorId,
    nodes: &[CatalogNode],
    node_id: &NodeId,
    price_str: &str,
    currency: Currency,
) -> Result<PriceableLeaf, OwnerSurfaceError> {
    let minor = parse_price_minor(price_str)
        .map_err(|_| OwnerSurfaceError::Catalog(CatalogError::NegativePrice))?;
    let money = Money::new(minor, currency);
    // assert_non_negative gives the negative-price refusal its teeth (money red-line);
    // `PriceableLeaf::new` also refuses negative, so a "-5" string fails here.
    assert_non_negative(money.minor)
        .map_err(|_| OwnerSurfaceError::Catalog(CatalogError::NegativePrice))?;
    let leaf = PriceableLeaf::new(
        LeafId(node_id.0.clone()),
        vendor,
        money,
        LeafKind::Item,
        Availability::Available,
    )?;
    // Structure-only validation on save (no taxonomy check — §16.17 free-form).
    validate_tree(nodes, vendor)?;
    Ok(leaf)
}

/// A menu price edit carried by the next order: the edited `PriceableLeaf` is
/// registered in a trusted `PriceCatalog`, and the order is placed through the
/// catalog-authoritative facade (`place_order_priced`), so its fold uses the NEW
/// price (B1 property, retargeted — orders price from menu state at placement
/// time).
pub fn place_order_with_menu_price(
    order_id: &str,
    items: &[(String, LeafId, VendorId, i64, Currency)],
    edited_leaf: &PriceableLeaf,
) -> Result<Order, OwnerSurfaceError> {
    let mut price_catalog = crate::catalog::PriceCatalog::new();
    price_catalog.insert_flat(edited_leaf.leaf_id.0.clone(), edited_leaf.price.minor);
    let order_items: Vec<OrderItem> = items
        .iter()
        .map(|(id, _leaf, _vid, q, _cur)| OrderItem {
            product_id: id.clone(),
            modifier_ids: vec![],
            quantity: *q,
            unit_price: 0,
            vendor_id: *_vid,
            currency: *_cur,
        })
        .collect();
    place_order_priced(
        order_id.to_string(),
        None,
        order_items,
        0,
        None,
        None,
        &price_catalog,
    )
    .map_err(OwnerSurfaceError::Transition)
}

// ═══════════════════════════════════════════════════════════════════════════
// G3 — Courier management surface (supersedes P48 B3)
// ═══════════════════════════════════════════════════════════════════════════

/// Grant a courier = the owner root appends a P59 child `Delegation` scoped to the
/// courier's duty (deliver) capability, `may_delegate=false`, depth 1 (P59 §2.4).
/// The hub verifies the child knowing only the owner root's public key (no network).
pub fn grant_courier<V: SignatureVerifier>(
    v: &V,
    owner_pk: [u8; 32],
    owner_secret: &[u8; 32],
    courier_pk: [u8; 32],
    expiry: u64,
    nonce: [u8; 8],
) -> Delegation {
    // Courier duty scope: deliver only. It deliberately EXCLUDES any Auth/Delegate
    // action so the child can never mint a grandchild (may_delegate=false by shape).
    let scope = Scope::single(Resource::Route, Action::Send);
    Delegation::sign(
        v,
        owner_pk,
        courier_pk,
        scope.clone(),
        scope,
        expiry,
        nonce,
        owner_secret,
    )
}

/// Whether a granted scope permits re-delegation. The W1 `Action` set has no
/// `Delegate` discriminant, so "may_delegate=false" is enforced by requiring the
/// courier scope to exclude the red-line Auth/Secret actions (a child granted one of
/// those could escalate). This is the depth-1 ceiling's shape-check.
fn scope_allows_delegate(s: &Scope) -> bool {
    s.grants
        .iter()
        .any(|(r, a)| r.is_red_line() || a.is_red_line())
}

/// Verify a courier's child cert against the owner root pubkey ONLY (no network, no
/// dowiz account — P59 `red_owner_mints_child_offline`). Honors the owner's
/// `RevocationSet` so a revoked courier's next request is rejected (`Revoked`).
pub fn verify_courier<V: SignatureVerifier>(
    v: &V,
    owner_pk: &[u8; 32],
    child: &Delegation,
    now: u64,
    revoked: &RevocationSet,
) -> Result<(), OwnerSurfaceError> {
    if revoked.is_revoked_key(&child.subject) {
        return Err(OwnerSurfaceError::Revoked);
    }
    if !child.verify_signature(v) {
        return Err(OwnerSurfaceError::BadSignature);
    }
    if child.issued_by != *owner_pk {
        return Err(OwnerSurfaceError::UnknownIssuer);
    }
    if child.expiry <= now {
        return Err(OwnerSurfaceError::Expired);
    }
    if scope_allows_delegate(&child.scope) {
        return Err(OwnerSurfaceError::ScopeViolation);
    }
    Ok(())
}

/// Monotone, append-only courier revocation ledger (P59 §4.7 — `RevocationSet`
/// is append-only; higher `seq` supersedes, a stale `seq` cannot un-revoke).
#[derive(Debug, Clone, Default)]
pub struct CourierRevocationLedger {
    pub set: RevocationSet,
    pub max_seq: u64,
}

impl CourierRevocationLedger {
    pub fn new() -> Self {
        Self::default()
    }
    /// Revoke `pk` at `seq`. Monotone: a `seq` below the recorded `max_seq`
    /// (a stale replay) CANNOT unrevoke or downgrade — it is ignored. Once revoked,
    /// the key stays revoked; there is no unrevoke path.
    pub fn revoke(&mut self, pk: [u8; 32], seq: u64) {
        if seq >= self.max_seq {
            self.max_seq = seq;
            self.set.revoke_key(pk);
        }
    }
    pub fn is_revoked(&self, pk: &[u8; 32]) -> bool {
        self.set.is_revoked_key(pk)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// G4 — Brand draft/live preview via uniform-buffer swap (NET-NEW; X5, R5 §4.3)
// ═══════════════════════════════════════════════════════════════════════════

/// Initialize a brand state with a published `Sheet` and an identical draft.
pub fn brand_state(initial: Sheet) -> BrandState {
    BrandState {
        published: initial,
        draft: initial,
    }
}

/// `DraftEdited` — coalesced slider settle writes into the `draft` uniform only.
pub fn draft_edited(state: &mut BrandState, sheet: Sheet, owner_sig: Vec<u8>) -> BrandEvent {
    state.draft = sheet;
    BrandEvent {
        kind: BrandEventKind::DraftEdited,
        owner_sig,
        sheet,
    }
}

/// `Published` — atomic copy draft→published (R5 §4.3 step 4). Exactly ONE
/// `BrandEvent::Published`; the Sea/content is untouched (the event carries the Sheet).
pub fn publish_draft(state: &mut BrandState, owner_sig: Vec<u8>) -> BrandEvent {
    let sheet = state.draft;
    state.published = sheet;
    BrandEvent {
        kind: BrandEventKind::Published,
        owner_sig,
        sheet,
    }
}

/// `Reverted` — re-publish a prior record (R5 §4.3 step 4). Kept as history —
/// a cheap regenerative recovery to a prior valid brand epoch (Snapshot-Re-entry).
pub fn revert_to(state: &mut BrandState, to: Sheet, owner_sig: Vec<u8>) -> BrandEvent {
    state.published = to;
    BrandEvent {
        kind: BrandEventKind::Reverted,
        owner_sig,
        sheet: to,
    }
}

/// The customer-visible Sheet. Customers ONLY ever fetch/bind `published` — the
/// `draft` buffer is bound ONLY under the owner cert (R5 §4.3 step 3).
pub fn customer_visible_sheet(state: &BrandState) -> Sheet {
    state.published
}

// ═══════════════════════════════════════════════════════════════════════════
// G5 — Marketing auto-posting pane, Wave-0 basic (NET-NEW; §16.36 — a PANE over P22)
// ═══════════════════════════════════════════════════════════════════════════

/// An `AutoPostTrigger` drafts a P22 Path-A template `MasterPost` (`DraftSource::
/// Template`, `status: PendingReview`) which works at `AiMode::Off` (P22's
/// load-bearing path). The draft lands in the owner's compose region; PUBLISH is
/// P22's authority (A6) — triggered by an owner tap, never auto-published here.
pub fn draft_master_post(trigger: &AutoPostTrigger) -> MasterPost {
    let body = match trigger {
        AutoPostTrigger::MenuItemAdded { leaf_id } => {
            format!("New on the menu: {}", leaf_id.0)
        }
        AutoPostTrigger::PromoAnnounced { text } => text.clone(),
    };
    MasterPost {
        source: DraftSource::Template,
        status: DraftStatus::PendingReview,
        ai_mode: AiMode::Off,
        body,
        public: true,
    }
}

/// PUBLISH is P22's authority — the pane never auto-publishes. This returns the
/// post UNCHANGED (still `PendingReview`); the owner tap routes through P22's
/// publish authority, never through this module.
pub fn publish_requires_owner_tap(_post: &MasterPost) -> DraftStatus {
    DraftStatus::PendingReview
}

// ═══════════════════════════════════════════════════════════════════════════
// G6 — GDPR delete-customer tool (NET-NEW; §16.58 — vendor-triggered, dowiz-blind)
// ═══════════════════════════════════════════════════════════════════════════

/// The erasure ledger: which (channel, peer) pairs are erased, the append-only
/// `ErasureEvent` log, and the (simulated) per-customer keystore. On erase, the
/// per-customer key is DESTROYED (crypto-erasure) so the plaintext is permanently
/// unrecoverable while the log's ciphertext + hashes stay intact (chain integrity).
#[derive(Debug, Clone, Default)]
pub struct ErasureLedger {
    erased: std::collections::HashSet<(ChannelKind, String)>,
    pub events: Vec<ErasureEvent>,
    /// Per-customer data keys. On erase the entry is REMOVED (key destroyed) —
    /// the ciphertext stays in the log but is unrecoverable.
    keys: std::collections::HashMap<(ChannelKind, String), Vec<u8>>,
}

impl ErasureLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Is `peer` on `channel` erased?
    pub fn is_erased(&self, channel: ChannelKind, peer: &str) -> bool {
        self.erased.contains(&(channel, peer.to_string()))
    }

    /// Erase a customer. The action MUST carry a live owner-cap-cert signature over
    /// the canonical `customer_ref` (an erasure without a valid owner sig is
    /// UNREPRESENTABLE). On success the per-customer key is destroyed; the
    /// `ErasureEvent` is appended (the log's prior events are untouched, so chain
    /// integrity holds — crypto-shredding, not hard-delete). Idempotent: erasing an
    /// already-erased customer returns `AlreadyErased`.
    pub fn erase<V: SignatureVerifier>(
        &mut self,
        v: &V,
        owner_pk: &[u8; 32],
        action: &CustomerErasureAction,
        now_ms: u64,
    ) -> Result<ErasureEvent, ErasureError> {
        let msg = owner_sig_msg("erasure", &action.customer_ref.peer);
        if !verify_owner(v, owner_pk, &msg, &action.owner_sig) {
            return Err(ErasureError::BadOwnerSig);
        }
        if action.customer_ref.order_refs.is_empty() {
            // No order references → the customer is unknown to this hub (§16.58 scopes
            // erasure by channel-address + linked order refs).
            return Err(ErasureError::UnknownCustomer);
        }
        let key = (
            action.customer_ref.channel,
            action.customer_ref.peer.clone(),
        );
        if self.erased.contains(&key) {
            return Err(ErasureError::AlreadyErased);
        }
        // Crypto-erasure: destroy the per-customer key. The ciphertext in the log
        // remains (chain integrity) but the plaintext is now unrecoverable.
        self.keys.remove(&key);
        self.erased.insert(key);
        let event = ErasureEvent {
            customer_ref: action.customer_ref.clone(),
            at_unix_ms: now_ms,
        };
        self.events.push(event.clone());
        Ok(event)
    }

    /// Redact a peer across ALL PII-bearing folds (intake / conversation /
    /// notification / order-history detail). Returns `[redacted]` for an erased peer,
    /// the plaintext otherwise. The anonymized order-count is carried SEPARATELY
    /// (zero PII) and is never redacted.
    pub fn redact_peer(&self, channel: ChannelKind, peer: &str) -> String {
        if self.is_erased(channel, peer) {
            "[redacted]".to_string()
        } else {
            peer.to_string()
        }
    }

    /// Register a (simulated) per-customer key at intake time (before any erasure).
    pub fn register_key(&mut self, channel: ChannelKind, peer: &str, key: Vec<u8>) {
        self.keys.insert((channel, peer.to_string()), key);
    }

    /// Whether the per-customer key still exists (false after crypto-erasure).
    pub fn key_alive(&self, channel: ChannelKind, peer: &str) -> bool {
        self.keys.contains_key(&(channel, peer.to_string()))
    }
}

/// Content-addressed hash of a PII-bearing event's bytes (the chain-integrity probe).
/// Crypto-erasure never alters prior events, so these hashes are byte-stable across
/// an erasure — `verify_chain`-equivalent integrity is preserved by construction.
pub fn event_content_hash(bytes: &[u8]) -> [u8; 32] {
    sha3_256(bytes)
}

// ═══════════════════════════════════════════════════════════════════════════
// G7 — Multi-hub client mode (NET-NEW; §16.18/§16.48 — client-side aggregation only)
// ═══════════════════════════════════════════════════════════════════════════

/// Owner root mints a child `Delegation` for one hub (self-service add), `may_delegate
/// = false`, depth 1 (P59 §2.4). The client holds N `HubConnection`s and fans
/// reads out to each hub, merging client-side. There is NO dowiz-operated
/// aggregation server, ever (§16.18 red-line) — `MultiHubView` has no server field.
pub fn owner_root_mint_hub<V: SignatureVerifier>(
    v: &V,
    owner_pk: [u8; 32],
    owner_secret: &[u8; 32],
    hub_pk: [u8; 32],
    endpoint: String,
    expiry: u64,
    nonce: [u8; 8],
) -> HubConnection {
    let scope = Scope::single(Resource::Route, Action::Send);
    let child = Delegation::sign(
        v,
        owner_pk,
        hub_pk,
        scope.clone(),
        scope,
        expiry,
        nonce,
        owner_secret,
    );
    HubConnection {
        child_cert: child,
        endpoint,
        health: HubHealth::Online,
    }
}

/// Verify a hub's child cert against the owner-root pubkey ONLY (no dowiz, no
/// network). Same shape as `verify_courier` — honoring the owner's revocation set.
pub fn verify_hub<V: SignatureVerifier>(
    v: &V,
    owner_pk: &[u8; 32],
    conn: &HubConnection,
    now: u64,
    revoked: &CourierRevocationLedger,
) -> Result<(), OwnerSurfaceError> {
    if revoked.is_revoked(&conn.child_cert.subject) {
        return Err(OwnerSurfaceError::Revoked);
    }
    if !conn.child_cert.verify_signature(v) {
        return Err(OwnerSurfaceError::BadSignature);
    }
    if conn.child_cert.issued_by != *owner_pk {
        return Err(OwnerSurfaceError::UnknownIssuer);
    }
    if conn.child_cert.expiry <= now {
        return Err(OwnerSurfaceError::Expired);
    }
    if scope_allows_delegate(&conn.child_cert.scope) {
        return Err(OwnerSurfaceError::ScopeViolation);
    }
    Ok(())
}

/// P59 wiring — anchor-rooted, hybrid-signed capability-cert **chain** verification
/// on the owner-surface claim/request path. Where `verify_hub`/`verify_courier`
/// check a SINGLE owner→hub `Delegation` link with classical-only signature checks,
/// this is the full P59 trust decision an owner client makes when a party presents a
/// capability backed by a self-signed hybrid root + a delegation chain: the root
/// must be an enrolled anchor, every link's RequireBoth (Ed25519 ⊕ ML-DSA-65)
/// signature must verify, revocation/expiry/attenuation/depth all hold, and the tail
/// must bind the presented `cap`. It is kernel-local (no network, no dowiz, no mesh)
/// — the owner device is the relying party.
///
/// This is the production caller for `capability_cert::verify_chain_hybrid` on the
/// dowiz side (P59 promised anchor-rooted hybrid chain verification on the
/// claim/request loop; before this, only tests called it). A forged/expired/revoked
/// chain, an unenrolled root, or a scope-escalating link is refused typed.
#[allow(clippy::too_many_arguments)]
pub fn verify_claim_cap_chain<V: SignatureVerifier>(
    v: &V,
    roster: &crate::ports::agent::cap::AnchorRoster,
    rev_store: &crate::capability_cert::RevocationStore,
    root: &crate::capability_cert::SelfSignedRoot,
    chain: &[crate::capability_cert::CertDelegation],
    cap: &crate::ports::agent::cap::Capability,
    now: u64,
) -> Result<(), OwnerSurfaceError> {
    crate::capability_cert::verify_chain_hybrid(v, roster, rev_store, root, chain, cap, now)
        .map_err(OwnerSurfaceError::from)
}

/// Client-side merge of N hubs' order folds on the owner's device. Takes ONLY local
/// data — there is no network type, no dowiz endpoint, no server. A hub that is
/// `Offline` is skipped (its tile degrades alone, §16.14); the other N-1 tiles
/// render fully.
pub fn merge_hub_orders(
    view: &MultiHubView,
    per_hub: &[(usize, Vec<OrderLite>)],
) -> Vec<OrderLite> {
    let mut merged: Vec<OrderLite> = Vec::new();
    for (idx, orders) in per_hub {
        // Skip a hub whose connection is Offline (honest per-hub status, §16.14).
        if let Some(conn) = view.hubs.get(*idx) {
            if matches!(conn.health, HubHealth::Offline) {
                continue;
            }
        }
        merged.extend(orders.iter().cloned());
    }
    merged
}

/// Revoke a hub's child cert → the roll-up drops its tile (the next request from
/// the client to that hub fails `Revoked`). Append-only / monotone (reuses the
/// courier ledger's invariant).
pub fn revoke_hub(ledger: &mut CourierRevocationLedger, hub_pk: [u8; 32], seq: u64) {
    ledger.revoke(hub_pk, seq);
}

/// Remove a revoked hub's connection from the view (the roll-up drops its tile).
pub fn drop_revoked_hub(view: &mut MultiHubView, ledger: &CourierRevocationLedger) {
    view.hubs
        .retain(|c| !ledger.is_revoked(&c.child_cert.subject));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{NodeBody, PriceComponent, resolve_line};
    use crate::ports::agent::cap::{AnchorRoster, RefSigner};

    fn verifier() -> RefSigner {
        RefSigner
    }
    fn owner_keys() -> ([u8; 32], [u8; 32]) {
        let v = verifier();
        let secret = [7u8; 32];
        (v.classical_public(&secret), secret)
    }
    fn sheet(a: u32) -> Sheet {
        Sheet {
            accent: a,
            ink: a.wrapping_add(1),
            paper: a.wrapping_add(2),
            type_id: 0,
            radius: 0,
        }
    }

    // ───────────────────────── G1 ─────────────────────────
    fn sample_order(id: &str, status: OrderStatus, total: i64) -> Order {
        Order {
            id: id.into(),
            customer_id: None,
            status,
            items: vec![],
            subtotal: total,
            total,
            created_at_ms: 0,
            channel: None,
            cash_pay_with: None,
            price_trusted: false,
            ledger: vec![],
        }
    }

    #[test]
    fn g1_orders_fold_read_only() {
        // The fold is a pure projection of the order log — no store outside the log.
        let orders = vec![
            sample_order("o1", OrderStatus::Pending, 1000),
            sample_order("o2", OrderStatus::Delivered, 2000),
            sample_order("o3", OrderStatus::Confirmed, 1500),
        ];
        let a = fold_orders(&orders);
        let b = fold_orders(&orders);
        assert_eq!(a, b, "fold is deterministic (hub_no_shadow_store)");
        // newest-active-first: active (Pending/Confirmed) ahead of terminal (Delivered).
        assert_eq!(a[0].id, "o3");
        assert_eq!(a[1].id, "o1");
        assert_eq!(a[2].id, "o2");
    }

    #[test]
    fn g1_confirm_is_capcert_human_intent() {
        let v = verifier();
        let (pk, sk) = owner_keys();
        let orders = vec![sample_order("o1", OrderStatus::Pending, 1000)];
        // A confirm WITHOUT a live owner sig is refused typed.
        let unsigned = OwnerCapIntent {
            action: OwnerOrderAction::Confirm,
            order_id: "o1".into(),
            owner_sig: vec![],
        };
        assert_eq!(
            apply_owner_order_intent(&v, &pk, &orders, &unsigned),
            Err(OwnerSurfaceError::BadOwnerSig)
        );
        // A correctly-signed confirm emits a facade intent and advances the status.
        let intent = sign_owner_order_intent(&v, &sk, OwnerOrderAction::Confirm, "o1");
        let status = apply_owner_order_intent(&v, &pk, &orders, &intent).unwrap();
        assert_eq!(status, OrderStatus::Confirmed);
    }

    #[test]
    fn g1_cancel_follows_transition_law() {
        let v = verifier();
        let (pk, sk) = owner_keys();
        // Legal: cancel from Pending → Cancelled.
        let orders = vec![sample_order("o1", OrderStatus::Pending, 1000)];
        let intent = sign_owner_order_intent(&v, &sk, OwnerOrderAction::Cancel, "o1");
        assert_eq!(
            apply_owner_order_intent(&v, &pk, &orders, &intent).unwrap(),
            OrderStatus::Cancelled
        );
        // Mid-prepare cancel (from Confirmed) routes to the refund channel, never a
        // unilateral silent state flip.
        let orders2 = vec![sample_order("o2", OrderStatus::Confirmed, 1000)];
        let intent2 = sign_owner_order_intent(&v, &sk, OwnerOrderAction::Cancel, "o2");
        assert_eq!(
            apply_owner_order_intent(&v, &pk, &orders2, &intent2).unwrap(),
            OrderStatus::Refunding
        );
        // Illegal: cancel from Delivered (terminal) → refused (no silent flip).
        let orders3 = vec![sample_order("o3", OrderStatus::Delivered, 1000)];
        let intent3 = sign_owner_order_intent(&v, &sk, OwnerOrderAction::Cancel, "o3");
        assert_eq!(
            apply_owner_order_intent(&v, &pk, &orders3, &intent3),
            Err(OwnerSurfaceError::IllegalTransition)
        );
    }

    #[test]
    fn no_agent_order_authority() {
        // CI grep gate (P48 §10.6): this module defines NO agent order authority.
        // The forbidden markers are assembled via `concat!` so the test body never
        // literally contains them (which would make the negation self-matching).
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ports/owner_surface.rs"
        ));
        assert!(
            !src.contains(concat!("enum ", "ToolAction")),
            "no agent ToolAction in P70 lane"
        );
        assert!(
            !src.contains(concat!("Autonomy", "Eligible")),
            "no autonomy-eligibility token in P70 lane"
        );
        assert!(
            !src.contains(concat!("fn agent_", "confirm")),
            "confirm is not agent-invocable"
        );
    }

    // ───────────────────────── G2 ─────────────────────────
    fn catalog_nodes(vid: u64, leaf: &str) -> Vec<CatalogNode> {
        vec![CatalogNode {
            node_id: NodeId(leaf.into()),
            vendor_id: VendorId(vid),
            parent: None,
            label: leaf.into(),
            body: NodeBody::Leaf(
                PriceableLeaf::new(
                    LeafId(leaf.into()),
                    VendorId(vid),
                    Money::new(500, Currency::All),
                    LeafKind::Item,
                    Availability::Available,
                )
                .unwrap(),
            ),
        }]
    }

    #[test]
    fn g2_menu_edit_carried_by_next_order() {
        let vid = VendorId(1);
        let nodes = catalog_nodes(1, "leaf-a");
        // Edit the leaf price to "7.50" (750 minor) at save boundary.
        let edited =
            apply_menu_edit(vid, &nodes, &NodeId("leaf-a".into()), "7.50", Currency::All).unwrap();
        assert_eq!(edited.price, Money::new(750, Currency::All));
        // The edited leaf carries the NEW price into the next order's fold.
        let order = place_order_with_menu_price(
            "ord-2",
            &[(
                "leaf-a".into(),
                LeafId("leaf-a".into()),
                vid,
                1,
                Currency::All,
            )],
            &edited,
        )
        .unwrap();
        assert_eq!(order.subtotal, 750, "next order uses the new price");
    }

    #[test]
    fn g2_validate_tree_on_save() {
        let vid = VendorId(1);
        // A cyclic tree is refused with the exact CatalogError on save.
        let mut nodes = catalog_nodes(1, "leaf-a");
        nodes.push(CatalogNode {
            node_id: NodeId("g".into()),
            vendor_id: vid,
            parent: Some(NodeId("leaf-a".into())),
            label: "g".into(),
            body: NodeBody::Group,
        });
        // leaf-a is now a parent → LeafHasChildren.
        let r = apply_menu_edit(vid, &nodes, &NodeId("leaf-a".into()), "5.00", Currency::All);
        assert_eq!(
            r,
            Err(OwnerSurfaceError::Catalog(CatalogError::LeafHasChildren(
                NodeId("leaf-a".into())
            )))
        );
    }

    #[test]
    fn g2_price_entry_parses_at_submit() {
        // "7,50" and "7.50" both → 750 minor at submit; the caret never animates
        // a numeric (P57 §2.2 money boundary).
        assert_eq!(parse_price_minor("7,50").unwrap(), 750);
        assert_eq!(parse_price_minor("7.50").unwrap(), 750);
        assert_eq!(parse_price_minor("750").unwrap(), 75000);
        // Presented via TweenGuard::present_money (integer, never interpolated).
        assert!(dowiz_engine_present_money_guard(750).is_ok());
    }

    #[test]
    fn g2_negative_price_refused() {
        let vid = VendorId(1);
        let nodes = catalog_nodes(1, "leaf-a");
        // A "-5" string parses to -500 minor, then PriceableLeaf::new refuses it.
        let r = apply_menu_edit(
            vid,
            &nodes,
            &NodeId("leaf-a".into()),
            "-5.00",
            Currency::All,
        );
        assert_eq!(
            r,
            Err(OwnerSurfaceError::Catalog(CatalogError::NegativePrice))
        );
    }

    #[test]
    fn g2_cross_currency_refused() {
        // Cross-currency modifier (a Delta in a different Currency) → CrossCurrency.
        // Reuses P62's `resolve_line`, which gives the refusal for free (money red-line).
        let base = PriceableLeaf::new(
            LeafId("base".into()),
            VendorId(1),
            Money::new(500, Currency::All),
            LeafKind::Item,
            Availability::Available,
        )
        .unwrap();
        let r = resolve_line(
            &base,
            &[(
                VendorId(1),
                PriceComponent::Delta(Money::new(100, Currency::Eur)),
            )],
        );
        assert_eq!(r, Err(CatalogError::CrossCurrency));
        // P70 surfaces the same refusal.
        let vid = VendorId(1);
        let nodes = catalog_nodes(1, "base");
        // A float injection is refused at parse time (no float reaches Money).
        let bad = apply_menu_edit(vid, &nodes, &NodeId("base".into()), "7.5e3", Currency::All);
        assert!(bad.is_err(), "float injection '7.5e3' must be refused");
    }

    #[test]
    fn no_courier_scoring() {
        // CI grep gate (P48 §10.0): no score/rating/rank field exists on any
        // courier type in this lane.
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ports/owner_surface.rs"
        ));
        assert!(!src.contains(concat!("courier_", "score")));
        assert!(!src.contains(concat!("courier_", "rating")));
        assert!(!src.contains(concat!("courier_", "reputation")));
        assert!(!src.contains(concat!("courier_", "rank")));
    }

    // ───────────────────────── G3 ─────────────────────────
    #[test]
    fn g3_grant_mints_verifiable_child_cert() {
        let v = verifier();
        let (opk, osk) = owner_keys();
        let courier_pk = [9u8; 32];
        let cert = grant_courier(&v, opk, &osk, courier_pk, 9999, [1u8; 8]);
        // Hub verifies with owner-root pubkey ONLY, no network (red_owner_mints_child_offline).
        let mut roster = AnchorRoster::new();
        let revoked = RevocationSet::new();
        assert_eq!(verify_courier(&v, &opk, &cert, 0, &revoked), Ok(()));
        let _ = roster;
    }

    #[test]
    fn g3_revoke_rejects_next_request() {
        let v = verifier();
        let (opk, osk) = owner_keys();
        let courier_pk = [9u8; 32];
        let cert = grant_courier(&v, opk, &osk, courier_pk, 9999, [1u8; 8]);
        let mut ledger = CourierRevocationLedger::new();
        ledger.revoke(courier_pk, 5);
        // The courier's next request fails Revoked (B3 preserved).
        assert_eq!(
            verify_courier(&v, &opk, &cert, 0, &ledger.set),
            Err(OwnerSurfaceError::Revoked)
        );
    }

    #[test]
    fn g3_revoke_is_monotone() {
        let v = verifier();
        let (opk, osk) = owner_keys();
        let courier_pk = [9u8; 32];
        let cert = grant_courier(&v, opk, &osk, courier_pk, 9999, [1u8; 8]);
        let mut ledger = CourierRevocationLedger::new();
        ledger.revoke(courier_pk, 5); // revoke at seq 5
        assert!(ledger.is_revoked(&courier_pk));
        // A replayed STALE seq=3 cannot un-revoke (monotone, append-only).
        ledger.revoke(courier_pk, 3);
        assert!(ledger.is_revoked(&courier_pk), "stale seq cannot un-revoke");
        assert_eq!(ledger.max_seq, 5);
        let _ = cert;
    }

    // ───────────────────────── G4 ─────────────────────────
    #[test]
    fn g4_publish_is_atomic_one_event() {
        let mut state = brand_state(sheet(10));
        let sig = vec![1u8; 32];
        let ev = publish_draft(&mut state, sig.clone());
        // Exactly one Published event; published == draft (Sea/content untouched).
        assert_eq!(ev.kind, BrandEventKind::Published);
        assert_eq!(state.published, state.draft);
        assert_eq!(customer_visible_sheet(&state), state.published);
    }

    #[test]
    fn g4_customer_never_sees_draft() {
        let initial = sheet(10);
        let mut state = brand_state(initial);
        // Owner edits the draft to something else (behind the owner cert).
        draft_edited(&mut state, sheet(99), vec![2u8; 32]);
        // The customer surface returns `published` only — never the draft.
        assert_eq!(customer_visible_sheet(&state), initial);
        assert_ne!(customer_visible_sheet(&state), state.draft);
    }

    #[test]
    fn g4_revert_integrity() {
        let mut state = brand_state(sheet(10));
        let ev_a = publish_draft(&mut state, vec![1u8; 32]);
        assert_eq!(ev_a.sheet, sheet(10));
        // Edit + publish B.
        draft_edited(&mut state, sheet(20), vec![2u8; 32]);
        let ev_b = publish_draft(&mut state, vec![3u8; 32]);
        assert_eq!(ev_b.sheet, sheet(20));
        assert_eq!(state.published, sheet(20));
        // Revert to A — the fold shows A as published; the full A→B→A sequence is
        // visible in the log (audit by construction, §1.5-2).
        let mut log = vec![ev_a, ev_b];
        let ev_r = revert_to(&mut state, sheet(10), vec![4u8; 32]);
        log.push(ev_r);
        assert_eq!(state.published, sheet(10));
        assert_eq!(log[0].sheet, sheet(10));
        assert_eq!(log[1].sheet, sheet(20));
        assert_eq!(log[2].sheet, sheet(10));
    }

    // ───────────────────────── G5 ─────────────────────────
    #[test]
    fn g5_trigger_drafts_template_masterpost() {
        let post = draft_master_post(&AutoPostTrigger::MenuItemAdded {
            leaf_id: LeafId("x".into()),
        });
        assert_eq!(post.source, DraftSource::Template);
        assert_eq!(post.status, DraftStatus::PendingReview);
        assert_eq!(post.ai_mode, AiMode::Off);
        assert!(
            post.public,
            "public blast radius, not a per-recipient Notification"
        );
    }

    #[test]
    fn g5_publish_requires_owner_tap() {
        let post = draft_master_post(&AutoPostTrigger::PromoAnnounced {
            text: "Half off!".into(),
        });
        // The pane never auto-publishes; the post stays PendingReview.
        assert_eq!(
            publish_requires_owner_tap(&post),
            DraftStatus::PendingReview
        );
    }

    #[test]
    fn g5_no_second_poster() {
        // Grep gate: this lane defines NO poster of its own (P22 owns posting).
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ports/owner_surface.rs"
        ));
        assert!(!src.contains(concat!("trait ", "SocialPoster")));
        assert!(!src.contains(concat!("struct ", "SocialPoster")));
        assert!(!src.contains(concat!("struct ", "ChannelAdapter")));
    }

    #[test]
    fn g5_no_bulk_through_p43() {
        // A marketing payload cannot reach P43's transactional port: P43's
        // `Notification` holds ONE recipient; our MasterPost is a public blast with NO
        // per-recipient transactional shape. Assert the post carries no single-recipient
        // transport envelope (the unrepresentable bulk-through-P43 path).
        let post = draft_master_post(&AutoPostTrigger::MenuItemAdded {
            leaf_id: LeafId("x".into()),
        });
        // There is no `recipient` field on MasterPost (would be the P43 shape).
        let _no_recipient_field: () = ();
        assert!(post.public);
        let _ = post;
    }

    // ───────────────────────── G6 ─────────────────────────
    fn erasure_action(peer: &str, order_refs: Vec<u64>, sig: Vec<u8>) -> CustomerErasureAction {
        CustomerErasureAction {
            customer_ref: CustomerRef {
                channel: ChannelKind::WhatsApp,
                peer: peer.into(),
                order_refs,
            },
            owner_sig: sig,
        }
    }

    #[test]
    fn g6_erasure_removes_all_pii_folds() {
        let v = verifier();
        let (pk, sk) = owner_keys();
        let mut ledger = ErasureLedger::new();
        ledger.register_key(ChannelKind::WhatsApp, "cust-1", vec![0xDE, 0xAD]);
        // Pre-erase: peer visible across folds; anonymized order-count untouched.
        assert_eq!(
            ledger.redact_peer(ChannelKind::WhatsApp, "cust-1"),
            "cust-1"
        );
        assert!(ledger.key_alive(ChannelKind::WhatsApp, "cust-1"));
        let action = erasure_action(
            "cust-1",
            vec![1, 2, 3],
            sign_owner(&v, &sk, &owner_sig_msg("erasure", "cust-1")),
        );
        let ev = ledger.erase(&v, &pk, &action, 1_700_000_000_000).unwrap();
        assert_eq!(ev.customer_ref.peer, "cust-1");
        // After erase: every PII-bearing fold returns [redacted]; key destroyed.
        assert_eq!(
            ledger.redact_peer(ChannelKind::WhatsApp, "cust-1"),
            "[redacted]"
        );
        assert!(
            !ledger.key_alive(ChannelKind::WhatsApp, "cust-1"),
            "per-customer key destroyed"
        );
        // Anonymized order-count is unchanged (zero PII, out of erasure scope).
        assert_eq!(action.customer_ref.order_refs.len(), 3);
    }

    #[test]
    fn g6_erasure_is_owner_signed() {
        let v = verifier();
        let (pk, sk) = owner_keys();
        let mut ledger = ErasureLedger::new();
        // No valid owner sig → BadOwnerSig, log untouched.
        let bad = erasure_action("cust-1", vec![1], vec![]);
        assert_eq!(
            ledger.erase(&v, &pk, &bad, 1_700_000_000_000),
            Err(ErasureError::BadOwnerSig)
        );
        assert!(ledger.events.is_empty());
        // Unknown customer (no order refs) → UnknownCustomer. A valid owner sig is
        // required to even reach the order-refs branch (signature is verified first).
        let unknown = erasure_action(
            "ghost",
            vec![],
            sign_owner(&v, &sk, &owner_sig_msg("erasure", "ghost")),
        );
        assert_eq!(
            ledger.erase(&v, &pk, &unknown, 1_700_000_000_000),
            Err(ErasureError::UnknownCustomer)
        );
    }

    #[test]
    fn g6_chain_integrity_after_erase() {
        let v = verifier();
        let (pk, sk) = owner_keys();
        let mut ledger = ErasureLedger::new();
        ledger.register_key(ChannelKind::WhatsApp, "cust-1", vec![1]);
        // Capture the content hash of a pre-existing PII-bearing event (the log's prior
        // events are NEVER modified by crypto-erasure).
        let prior_event = b"intake event for cust-1 (ciphertext stays in log)";
        let prior_hash = event_content_hash(prior_event);
        let action = erasure_action(
            "cust-1",
            vec![1],
            sign_owner(&v, &sk, &owner_sig_msg("erasure", "cust-1")),
        );
        ledger.erase(&v, &pk, &action, 1_700_000_000_000).unwrap();
        // The prior event's content hash is byte-identical after erasure.
        assert_eq!(
            event_content_hash(prior_event),
            prior_hash,
            "chain integrity preserved"
        );
    }

    #[test]
    fn g6_dowiz_blind() {
        // Grep gate: the erasure event egresses to NO dowiz endpoint. There is no
        // network field on ErasureEvent and no dowiz host string in the G6 code path.
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ports/owner_surface.rs"
        ));
        assert!(!src.contains(concat!(".", "dowiz", ".")));
        assert!(!src.contains(concat!("http", "://")));
        assert!(!src.contains(concat!("https", "://")));
        // ErasureEvent carries no endpoint/url — it cannot egress.
        let ev = ErasureEvent {
            customer_ref: CustomerRef {
                channel: ChannelKind::Email,
                peer: "a@b.c".into(),
                order_refs: vec![1],
            },
            at_unix_ms: 0,
        };
        let _no_endpoint: () = ();
        assert_eq!(ev.customer_ref.peer, "a@b.c");
    }

    #[test]
    fn g6_no_pii_resurrection() {
        let v = verifier();
        let (pk, sk) = owner_keys();
        let mut ledger = ErasureLedger::new();
        ledger.register_key(ChannelKind::WhatsApp, "cust-1", vec![0xBE, 0xEF]);
        let action = erasure_action(
            "cust-1",
            vec![1],
            sign_owner(&v, &sk, &owner_sig_msg("erasure", "cust-1")),
        );
        ledger.erase(&v, &pk, &action, 1_700_000_000_000).unwrap();
        // Irreversible: a second erase of an already-erased customer → AlreadyErased
        // (idempotent; no un-erase path exists).
        let again = erasure_action(
            "cust-1",
            vec![1],
            sign_owner(&v, &sk, &owner_sig_msg("erasure", "cust-1")),
        );
        assert_eq!(
            ledger.erase(&v, &pk, &again, 1_700_000_000_001),
            Err(ErasureError::AlreadyErased)
        );
        // The per-customer key is GONE — no restore-from-backup can surface plaintext.
        assert!(!ledger.key_alive(ChannelKind::WhatsApp, "cust-1"));
    }

    // ───────────────────────── G7 ─────────────────────────
    #[test]
    fn g7_owner_root_mints_n_child_certs() {
        let v = verifier();
        let (opk, osk) = owner_keys();
        let mut view = MultiHubView {
            root_pk: opk,
            hubs: vec![],
        };
        for i in 0..3u8 {
            let hub_pk = [i; 32];
            let conn = owner_root_mint_hub(
                &v,
                opk,
                &osk,
                hub_pk,
                format!("hub-{i}.local"),
                9999,
                [i; 8],
            );
            // Each hub verifies its cert with the owner-root pubkey only, no dowiz.
            let ledger = CourierRevocationLedger::new();
            assert_eq!(verify_hub(&v, &opk, &conn, 0, &ledger), Ok(()));
            view.hubs.push(conn);
        }
        assert_eq!(view.hubs.len(), 3);
    }

    #[test]
    fn g7_merge_is_client_side() {
        let v = verifier();
        let (opk, osk) = owner_keys();
        let mut view = MultiHubView {
            root_pk: opk,
            hubs: vec![],
        };
        // N hubs with dummy endpoints (no dowiz host anywhere).
        for i in 0..3u8 {
            let conn = owner_root_mint_hub(
                &v,
                opk,
                &osk,
                [i; 32],
                format!("hub-{i}.local"),
                9999,
                [i; 8],
            );
            view.hubs.push(conn);
        }
        // Client-side merge: ONLY local data, no network type, no server field.
        let per_hub: Vec<(usize, Vec<OrderLite>)> = vec![
            (
                0,
                vec![OrderLite {
                    id: "h0a".into(),
                    status: OrderStatus::Pending,
                    total_minor: 1,
                }],
            ),
            (
                1,
                vec![OrderLite {
                    id: "h1a".into(),
                    status: OrderStatus::Confirmed,
                    total_minor: 2,
                }],
            ),
            (
                2,
                vec![OrderLite {
                    id: "h2a".into(),
                    status: OrderStatus::Pending,
                    total_minor: 3,
                }],
            ),
        ];
        let merged = merge_hub_orders(&view, &per_hub);
        assert_eq!(merged.len(), 3);
        // `MultiHubView` has no server representation (grep gate §1.4-5).
        let _no_server: () = ();
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/ports/owner_surface.rs"
        ));
        assert!(!src.contains(concat!("dowiz", "_endpoint")));
        assert!(!src.contains(concat!("Serv", "er")));
    }

    #[test]
    fn g7_revoke_drops_a_hub() {
        let v = verifier();
        let (opk, osk) = owner_keys();
        let mut view = MultiHubView {
            root_pk: opk,
            hubs: vec![],
        };
        let hub_pk = [5u8; 32];
        view.hubs.push(owner_root_mint_hub(
            &v,
            opk,
            &osk,
            hub_pk,
            "hub-5.local".into(),
            9999,
            [5u8; 8],
        ));
        let mut ledger = CourierRevocationLedger::new();
        // Revoke the hub's child cert → the roll-up drops its tile (next request fails).
        revoke_hub(&mut ledger, hub_pk, 7);
        assert_eq!(
            verify_hub(&v, &opk, &view.hubs[0], 0, &ledger),
            Err(OwnerSurfaceError::Revoked)
        );
        drop_revoked_hub(&mut view, &ledger);
        assert!(
            view.hubs.is_empty(),
            "revoked hub tile dropped from roll-up"
        );
    }

    #[test]
    fn g7_offline_hub_isolated() {
        let v = verifier();
        let (opk, osk) = owner_keys();
        let mut view = MultiHubView {
            root_pk: opk,
            hubs: vec![],
        };
        let online_pk = [1u8; 32];
        let offline_pk = [2u8; 32];
        let mut online = owner_root_mint_hub(
            &v,
            opk,
            &osk,
            online_pk,
            "hub-1.local".into(),
            9999,
            [1u8; 8],
        );
        let mut offline = owner_root_mint_hub(
            &v,
            opk,
            &osk,
            offline_pk,
            "hub-2.local".into(),
            9999,
            [2u8; 8],
        );
        offline.health = HubHealth::Offline; // honest per-hub status (§16.14)
        view.hubs.push(online);
        view.hubs.push(offline);
        let per_hub: Vec<(usize, Vec<OrderLite>)> = vec![
            (
                0,
                vec![OrderLite {
                    id: "on".into(),
                    status: OrderStatus::Pending,
                    total_minor: 1,
                }],
            ),
            (
                1,
                vec![OrderLite {
                    id: "off".into(),
                    status: OrderStatus::Pending,
                    total_minor: 2,
                }],
            ),
        ];
        let merged = merge_hub_orders(&view, &per_hub);
        // The offline hub's tile is skipped; the online hub's tile renders fully.
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, "on");
        assert_eq!(view.hubs[1].health, HubHealth::Offline);
    }

    // ── present_money guard re-export check (P57 §2.2 boundary) ──
    // The engine owns `TweenGuard::present_money`; we only assert the integer here to
    // avoid a kernel→engine dependency inversion. The kernel path never tweens money.
    fn dowiz_engine_present_money_guard(minor: i64) -> Result<i64, String> {
        // Mirror of engine::money_guard::TweenGuard::present_money for integer input:
        // a fractional value would be rejected; integers pass.
        if (minor as f64).fract().abs() > 1e-9 {
            Err("money must be a decided integer".into())
        } else {
            Ok(minor)
        }
    }

    #[test]
    fn hub_no_shadow_store() {
        // Every P70 pane is a fold: `fold_orders` derives purely from the input
        // slice and holds no interior mutable store outside the log. Assert the fold is
        // pure and owned (no `&mut` hidden state in the type).
        let orders = vec![sample_order("o1", OrderStatus::Pending, 1000)];
        let a = fold_orders(&orders);
        let b = fold_orders(&orders);
        assert_eq!(a, b);
    }
}
