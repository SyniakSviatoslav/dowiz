# BLUEPRINT P70 — Owner Surface: orders/menu/courier management, brand draft-live preview, marketing auto-posting, GDPR delete-tool, multi-hub client mode (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §8 — every point
> addressed, none skipped). This is **Wave W2, blueprint P70** per
> `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5's W2 table and its X1/X5 cross-cutting findings.
> Structure/depth template: `BLUEPRINT-P48-owner-hub-surface.md` / `BLUEPRINT-P51-open-map-routing.md`
> / `BLUEPRINT-P62-catalog-multivendor-data-model.md`.
>
> ## P70 vs P48 — the supersede/extend/consume line, stated up front (SYNTHESIS §5 canon-diff row P48/P52)
>
> The SYNTHESIS canon-diff table rules: *"P48/P52 — Not revised now — consumed by P70/P71 below,
> which supersede-or-extend them explicitly in their §2 scope sections."* P70 is the **owner
> management & configuration surface**; P48 (as it evolved) is the **omnichannel intake &
> conversation surface**. Both are "the owner's hub," rendered as P38 panes; they are different
> lanes. Precisely (full detail §1.2):
>
> - **P70 SUPERSEDES** the three *thin carried stubs* P48 held only as one-liners — its
>   carried **B1** (menu edit → order fold), **B2** (live order list = read-only projection),
>   **B3** (roster grant/revoke over raw `RevocationSet`). P70 gives each its full management
>   surface, now grounded on **P62** (catalog data model) and **P59** (capability-cert chain) —
>   both of which POSTDATE P48 and did not exist when B1/B2/B3 were written. P48's *properties*
>   (a menu edit is carried by the next order's fold; the order list is a fold with no shadow
>   state; a revoked courier's next mutating request is rejected) are **preserved, not
>   re-litigated** — P70 supersedes only the *how*.
> - **P70 ADDS** four net-new surfaces absent from P48: **brand draft/live preview** (G4),
>   **marketing auto-posting pane** (G5), **GDPR delete-customer tool** (G6), **multi-hub client
>   mode** (G7).
> - **P70 CONSUMES P48 UNCHANGED** (does not duplicate): H1 intake, H2 notifications, H3 reviews,
>   H4 agentic assist, H5 unified inbox, H6/H7 reply/promo *drafting*, H8 autonomy policy + the
>   fixed order-confirm/cancel human gate. Those panes stay P48's. In particular the **§10.6
>   order-confirm/cancel human gate is LAW here too** (§4.1) — P70's orders surface emits
>   confirm/cancel as owner-cap-cert human intents, never agent-invocable.
> - **a11y supersession (P58 §M7):** *"P48/P52 supersession: their a11y is subsumed by P70/P71
>   which own the owner/courier surfaces."* P70 authors the ONE `SemanticScene` for the whole
>   owner surface (P48's panes included, rendered in it) and imports P58's shared `a11yGate`;
>   P48 writes no a11y of its own. This is the single integration point P70 owns for P48.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Verified 2026-07-18 against `/root/dowiz` `main` (clean). Two cite classes, both live-checked
this pass: **(A)** the dependency-blueprint *contracts* P70 binds to by exact section, and **(B)**
the kernel *substrate* those contracts and P70 sit on. Ground truth is non-discussible.

### 0.A Dependency-blueprint contracts (read in full this pass; cited by exact section)

| # | Contract P70 binds | Cite (this pass) | Status |
|---|---|---|---|
| 1 | **P57 `TextField`** editor over cosmic-text: `EditCmd`/`EditEvent` closed alphabet, `apply(cmd,clip)->Vec<EditEvent>`, `value()`/`set_value()`, `WidgetId` field id, `ClipboardPort`, byte-cursor; contributes `Intent::Text(EditCmd)` to P64's enum | `BLUEPRINT-P57-canvas-text-input.md` §2.1 (M2/M6/M7), §3 (types) | verified — every typed owner field (menu label, price string, promo text, customer-ref, hub-connect handle) is a `TextField` |
| 2 | **P57 money-entry boundary:** a P57 field is a `&str`; **NOT a money-entry field** — a numeric amount is parsed to `i64` minor units **at the consumer's submit boundary** and presented via `TweenGuard::present_money` (`money_guard.rs`, 🔴 RED-LINE), never typed into a tweened caret | `BLUEPRINT-P57-canvas-text-input.md` §2.2 (money bullet), §5.1 | verified — G2's menu-price entry rides exactly this boundary |
| 3 | **P58 mirror keystone:** `mirror(scene:&SemanticScene)->A11yTree` is a PURE function of the `SemanticScene` alone (no token/matrix/floor is an input — those live in P38 `FrameUniforms`); `SemanticScene`/`SemanticNode`/`Role`/`NodeState`/`EditState` are the authored a11y layer | `BLUEPRINT-P58-a11y-mirror-everywhere.md` §2 (types), §M1 | verified — G1–G7 author one `SemanticScene`; the mirror is the a11y half by construction |
| 4 | **P58 draft-parity invariant (X1/X5):** the `A11yTree` is invariant under a **brand/theme swap** because that is a `queue.write_buffer` of `theme_tokens` (pixels, not the `SemanticScene`) ⇒ *"brand-preview accessibility is correct by construction, not by separate work … P70's owner brand-preview inherits it — it writes zero a11y code of its own."* Falsifiable test `a11y_tree_invariant_under_presentation_swap` | `BLUEPRINT-P58-a11y-mirror-everywhere.md` §M6 (esp. M6-1), §M7 (P48/P52 supersession) | verified — the load-bearing basis of G4's parity DoD (§5, THE falsifiable brand-preview test) |
| 5 | **P58 surface-integration contract (§M7):** every surface (P70 named) does three things — author a `SemanticScene`, import `a11yGate(page, manifest)` with a per-screen `MIRROR_NODE_BUDGET_DEFAULT`-derived node budget, carry the FE-16 WebGL2/CPU floor line verbatim | `BLUEPRINT-P58-a11y-mirror-everywhere.md` §M7, §2 (`MIRROR_NODE_BUDGET_DEFAULT=256`) | verified — P70's every-item DoD imports this gate, defines no bespoke a11y harness |
| 6 | **P59 owner root + delegation (§16.48):** `SelfSignedRoot{classical_pub,pq_pub,node_id,alg_suite,self_sig,not_after}`; child hub cert = a `Delegation` block with `may_delegate=false`, `MAX_DELEGATION_DEPTH=1`; `OwnerRoot` mints/attenuates/revokes offline; a hub verifies a child knowing **only the owner root's public key, no network, no dowiz account**; `verify_chain_hybrid`; revoke via signed `RevocationBlob` | `BLUEPRINT-P59-capability-cert-chain.md` §3 (types), §2.4 (two-anchor `depth<=1`), §4.5 (M5) | verified — G3 courier grants and G7 multi-hub certs are P59 `Delegation`s; nothing re-invented |
| 7 | **P59 revocation is monotone:** `RevocationBlob::sign`/`apply_blob`; higher `seq` supersedes (LWW); **append-only, no unrevoke**; `red_stale_seq_cannot_unrevoke` | `BLUEPRINT-P59-capability-cert-chain.md` §4.7 (M7) | verified — G3 revoke and G6 erasure both borrow the irreversibility shape |
| 8 | **P62 catalog leaf invariant (X7):** `VendorId(u64)`; `PriceableLeaf{leaf_id,vendor_id,price:Money,kind,availability}` built ONLY via `PriceableLeaf::new` (unpriced/uncurrencied/unattributed leaf UNREPRESENTABLE); free-form `CatalogNode`/`NodeBody{Group,Leaf}`; `validate_tree`, `resolve_line`, `charge_legs`, `kitchen_tickets`, `menu_jsonld`; `CatalogError{NegativePrice,CrossCurrency,CrossVendor,…}` | `BLUEPRINT-P62-catalog-multivendor-data-model.md` §3 (types), §1.2 (leaf invariant), §4 | verified — G2 menu management reads/writes P62's tree; never forks its price authority |
| 9 | **P62 RLS shape:** outer `location_id` FORCE RLS deny-on-unset (RED-LINE, existing) + inner `app.vendor_scope` opt-in narrowing (NOT a second tenant boundary) | `BLUEPRINT-P62-catalog-multivendor-data-model.md` §3 (RLS predicate), §4.6 (M6) | verified — G2 writes ride this; P70 adds no new RLS boundary |
| 10 | **P48 order-confirm/cancel is a FIXED human gate, not a config key**, enforced by the `no-agent-order-authority` CI grep (mirrors `no-courier-scoring`), the `ToolAction` set = `{Read,Draft}` (no `Confirm/Cancel/Send`), and the `AutonomyEligible` whitelist test | `BLUEPRINT-P48-owner-hub-surface.md` §10.6 (three walls) | verified — G1's confirm/cancel actions are owner-cap-cert human intents; P70 preserves the gate, does not weaken it |
| 11 | **P22 posting/campaign lane owns ALL marketing:** `SocialPoster` (public feed/channel posting) + IP-15 `ChannelAdapter` campaign lane (consent-ledger-gated); dual-path generation `MasterPost{source:DraftSource,status:DraftStatus}`, Path A templates work at `AiMode::Off`; **publish authority is never the model** (A6 ratchet) | `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md` §11.1/§11.3/§11.5; corroborated by `BLUEPRINT-P43-external-integration-ports.md` §1.2 (P22 boundary), §1.3 (row P22) | verified — G5 gives this lane an owner PANE; it reimplements no poster (P48 §1.1 verbatim: *"the hub gives it a pane, not a second implementation"*) |
| 12 | **P43 is transactional-only** (`Channel{Telegram,Sms,WhatsApp,SimpleX,Email}`, `ChannelSend`, `NotifyKind{OrderStatus,Otp}`, `Marketing` deliberately absent, `Notification` holds ONE recipient) — *"NOT social posting, NOT campaigns, NOT marketing. P22 owns all of it"* | `BLUEPRINT-P43-external-integration-ports.md` §2, anti-scope 1 | verified — G5's public posting goes through P22, never through P43's transactional port (a bulk/marketing payload through P43 is unrepresentable) |

### 0.B Kernel substrate (live `file:line`, verified this pass in `/root/dowiz/kernel`)

| # | Claim | Cite (this pass) | Status |
|---|---|---|---|
| 13 | Content-addressed local-first event log; duplicate = structural no-op: `EventStore` trait, `EventLog<S>`, `append`, `commit_after_decide` | `kernel/src/event_log.rs:1-4,182,282,302,366` | verified — every P70 pane is a fold over this; G4/G6 append node-local events onto it |
| 14 | Money is a type, not an i64: `Currency` (`:29`) + `code()` (`:39`), `Money` (`:59`), `checked_add` (`:71`), `assert_non_negative` (`:323`) | `kernel/src/money.rs` listed lines | verified — G2 prices are `Money`; negative/cross-currency fail closed for free (P62 reuses this) |
| 15 | Capability chain substrate: `NodeId` (`:54`) + `from_keys` (`:58`), `Capability` (`:169`), `Delegation` (`:291`), `RevocationSet` (`:412`), `verify_chain` (`:486`) — the exact code P59 extends to hybrid | `kernel/src/ports/agent/cap.rs` listed lines | verified — G3/G7 sit on P59's extension of these; `RevocationSet` is append-only (no unrevoke, cap.rs:412) |
| 16 | Order domain: `OrderItem` (`:30`), `Order` (`:42`), `compute_order_total` (`:129`), `place_order` (`:156`), `place_order_priced` (`:198`) | `kernel/src/domain.rs` listed lines | verified — G1's orders surface folds `Order`s; confirm/cancel route through the existing facade, never a parallel representation |

### 0.C Operator roadmap decisions this blueprint executes (§16/§17)

`§16.9` 5-token brand model (accent/ink/paper/type/radius) · `§16.18` multi-hub owner view =
client-side aggregation, never server-side · `§16.36` admin dashboard scope = orders/menu/couriers
Wave-0, marketing Wave-0 (basic), **analytics DEFERRED to v2** · `§16.48` owner multi-hub = a
root/delegating capability-cert the owner holds (self-service add/modify/revoke child hub nodes) ·
`§16.58` live draft/staging brand preview before publish + vendor-responsible GDPR deletion with a
dowiz-provided built-in delete-tool · `§16.14` no central dowiz state (honest offline status) ·
`§16.60` in-hub assistant excluded from customer PII by default. All read this pass; carried inline.

---

## 1. Scope & role — what P70 owns, supersedes, adds, consumes, and must never touch

### 1.1 P70's single sentence

The owner surface IS the venue owner's **management & configuration** hub: orders/menu/courier
management, brand look, marketing posts, GDPR erasure, and the multi-hub roll-up — every pane a
fold-derived P38 projection of the venue's own content-addressed event log, every mutating action
an owner-capability-cert intent through the same Law as everything else, with **zero** dowiz-side
aggregation, **zero** analytics dashboards (deferred, §16.36), and one `SemanticScene` giving the
whole surface its a11y mirror for free.

### 1.2 The supersede / extend / add / consume ledger (the precise §2 scope statement the canon-diff demands)

| P48 element | P70's relationship | Grounded on (postdates P48) |
|---|---|---|
| **B1** menu edit → order fold carries change (one-liner) | **SUPERSEDED by G2:** full menu-management surface. B1's *property* preserved as G2's DoD; the *how* is now P62's `CatalogNode` tree + `validate_tree` + `PriceableLeaf::new`, entered via P57 `TextField` | P62 §3/§4; P57 §2.1 |
| **B2** live order list = read-only projection + `hub_no_shadow_store` (one-liner) | **SUPERSEDED by G1:** full orders-management surface. Still a fold with no shadow state; adds owner confirm/cancel as human-gated cap-cert intents (P48 §10.6 gate PRESERVED) | domain.rs (row 16); P48 §10.6 |
| **B3** roster grant/revoke over raw `RevocationSet` (one-liner) | **SUPERSEDED by G3:** full courier-management surface. Grant = P59 `Delegation` child cert; revoke = P59 `RevocationBlob` (was raw `RevocationSet`); no-courier-scoring red-line kept | P59 §3/§4.5/§4.7 |
| **B4** no password admin; owner auth = capability cert | **CONSUMED unchanged** — P70's every mutating action is owner-cap-cert-signed; there is no admin-password path anywhere in P70 | P59 owner root (row 6) |
| **H1–H8, §10.7** intake/notifications/reviews/agentic/inbox/reply-drafting/promo-drafting/autonomy/courier-conversation | **CONSUMED unchanged** — P70 does not duplicate these panes. It renders them inside its `SemanticScene` (a11y supersession only) and links to them; it owns none of their machinery | P48 §3/§10 |
| **a11y (any pane)** | **SUPERSEDED by P70** per P58 §M7 — P70 authors the one owner-surface `SemanticScene` and imports `a11yGate`; P48 writes no a11y | P58 §M7 |
| *(none — net-new)* **G4 brand preview · G5 marketing pane · G6 GDPR delete-tool · G7 multi-hub** | **ADDED by P70** — no P48 antecedent | §16.9/§16.58 · §16.36 · §16.58 · §16.18/§16.48 |

### 1.3 Boundary map (every neighbor's claim honored; nothing scope-duplicated)

| Neighbor | Owns (cited) | P70's side of the line |
|---|---|---|
| **P48** (intake/conversation hub) | H1–H8 intake, notifications, reviews, inbox, reply/promo *drafting*, autonomy, the order-confirm/cancel human gate | Management/config panes (G1–G7); authors the shared `SemanticScene` (a11y supersession, P58 §M7). P70 emits no channel message and drafts no reply — it manages orders/menu/couriers/brand/marketing/erasure/hubs |
| **P62** (catalog data model) | `VendorId`, `PriceableLeaf`+invariant, `CatalogNode` tree, `validate_tree`/`charge_legs`/`kitchen_tickets`/`menu_jsonld`, the RLS shape | G2 is a **UI/intent surface over P62's model** — it constructs `CatalogNode`s and calls `validate_tree`; it defines no catalog type and forks no price authority |
| **P59** (cap-cert chain) | `SelfSignedRoot`, `Delegation`, `RevocationBlob`, `verify_chain_hybrid`, owner→hub `depth<=1`/`may_delegate=false` | G3 and G7 **consume** P59's mint/attenuate/revoke/verify; P70 adds no crypto, defines no cert type |
| **P57** (canvas text input) | `TextField`, `EditCmd`/`EditEvent`, the money-entry boundary | Every typed owner field is a P57 `TextField`; G2 price entry rides P57 §2.2's parse-at-submit money boundary |
| **P58** (a11y mirror + harness) | `mirror()`, `SemanticScene`, `a11yGate`, the draft-parity invariant | P70 authors a `SemanticScene`, imports `a11yGate`, and **inherits brand-preview a11y parity by construction** (P58 §M6-1) — writes zero a11y code |
| **P22** (social posting + campaigns) | `SocialPoster` (public posts), IP-15 campaign lane (private marketing to lists), publish authority | G5 is an owner **PANE over P22's poster** + a Wave-0 basic auto-trigger; it reimplements no poster and mints no bulk-send |
| **P43** (integration ports) | Transactional `ChannelSend` (order-status/OTP), `Marketing` absent | P70 sends nothing through P43; G5's public posting is P22's lane, G1's status updates are P48/P61's lane |
| **P49** (customer identity) | Customer-side records; durable identity DEFERRED (channel-address is the only id) | G6 scopes erasure by channel-address + linked order refs (no durable identity to key on); P70 stores no customer record |
| **P64** (intent engine) | Intent-driven navigation; the raw-data read path the owner uses in lieu of analytics | The owner reads raw order data through P64's intent interface (§16.36); P70 builds **no** analytics/charts (§1.4-1) |

### 1.4 Anti-scope (each a review-rejectable smell; #1 and #6 are the load-bearing exclusions)

1. **🚫 NO ANALYTICS / REPORTS / CHART-DASHBOARDS — deferred to v2 (§16.36), named here so nobody
   half-builds it.** The owner reads raw order data directly through the P64 intent interface and
   the G1 orders fold; P70 ships **no** aggregation dashboard, no KPI screen, no charting. A
   purpose-built analytics dashboard is an explicit *future* blueprint. Building even a "small"
   metrics panel here is the smell. (Extends P48 §1.4-3 "no analytics dashboards" and makes it a
   first-class Wave-0 boundary.)
2. **No new channel-transport code** — P43's (transactional) / P22's (posting) lanes. G1 status
   updates and G5 posts both go through existing lanes.
3. **No second brand renderer / preview subdomain / staging deploy** (X5) — brand preview is a
   uniform-buffer swap on the ONE wgpu pipeline (G4). A parallel render path is the smell.
4. **No second a11y mirror** — P58's `mirror()` is the only one; a brand-preview-specific a11y
   path is *unrepresentable*, not merely discouraged (P58 §M6, §5 DoD).
5. **No dowiz-operated aggregation server, ever** — multi-hub roll-up (G7) is client-side only
   (§16.18 red-line). Any P70 component whose availability depends on a dowiz endpoint fails the
   grep gate.
6. **🔴 NO order-confirm/cancel autonomy, and no agent order authority** — carried verbatim from
   P48 §10.6: confirm/cancel are owner-cap-cert human intents, never a `ToolAction`, never an
   `AutonomyEligible` variant, never a config key. The `no-agent-order-authority` CI grep binds
   P70's lane too. Red-line-class.
7. **No password/admin-auth path, no admin framework** — carried from P48 §1.4-3; owner auth is
   the P59 owner root cert (B4).
8. **No customer CRM/profile machinery** — P49's anti-scope holds; G6 erases, it does not enrich.
9. **No dowiz visibility into GDPR erasure** — G6 is hub-local; when/why it runs never egresses to
   dowiz (§16.14/§16.36 isolation as a grep gate, §3.6).

### 1.5 Central design principle — the management surface is folds + cap-cert intents, nothing new (argued, not asserted)

The same argument P48 §1.5 makes for intake applies verbatim to management: **every management
pane is a fold over the venue's own content-addressed event log (row 13), and every owner action
is a signed intent through the same capability-cert Law (row 15) — there is no admin database, no
shadow table, no dowiz aggregator.** Four falsifiable consequences specific to P70:

1. **One Law, no second truth.** Orders, menu, roster, brand, and erasure are all events; a pane
   is `fold(events)`. `hub_no_shadow_store` (P48 §5, B2) extends over every P70 pane — any
   management store outside the log fails the review-gate. *Falsifier:* grep for a P70-lane store
   that is not a fold-derived projection.
2. **Brand/erasure are events, so audit and revert are free.** "What the brand was, and when it
   changed" and "who erased customer X, when" are append-only log facts (`verify_chain`) — G4's
   revert is *re-publishing a prior record* (§3.4) and G6's erasure is a signed event (§3.6), each
   an audit trail by construction, not a mutable admin table.
3. **Multi-hub is aggregation of N signed logs on the owner's device, never a server.** G7 holds N
   P59 child certs and fans reads out to N hubs, merging client-side (§16.18). *Falsifier:* the
   full G7 suite runs with every dowiz endpoint black-holed and still merges.
4. **The owner root is the whole authority model.** Menu edit, courier grant, brand publish, GDPR
   erasure, and each hub connection are authorized by the one P59 owner root the owner self-holds
   (§16.48) — no dowiz account participates in any of them (B4; P59 §4.5 `red_owner_mints_child_offline`).

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

P70 REUSES upstream vocabulary and adds only the four net-new surfaces' types. Reused verbatim
(never redeclared): P57 `TextField`/`EditCmd`; P58 `SemanticScene`/`mirror`/`a11yGate`; P59
`SelfSignedRoot`/`Delegation`/`RevocationBlob`/`verify_chain_hybrid`; P62 `VendorId`/`PriceableLeaf`/
`CatalogNode`/`validate_tree`; P22 `MasterPost`/`DraftSource`/`DraftStatus`; domain `Order`/`OrderItem`.

```rust
// ── kernel/src/ports/owner_surface.rs — NEW module. Zero-I/O firewall (mirrors
// ports/hub_intake.rs discipline): no HTTP, no serde-on-the-wire, no provider names.
// Node-local event families ride event_log.rs (row 13), NOT proto-cap wire variants
// (P34 anti-scope — the same move P48 §10.1's ConversationEvent makes). ─────────────

// ============ G4 — brand draft/live preview (the 5-token Sheet, §16.9) ============

/// The dowiz-owned brand type id (Sheet). NOT a font file, NOT free CSS — the fixed
/// 5-token envelope (§16.9). `type_id`/`radius` are integer token indices, colors are
/// packed RGBA; the WHOLE record is a couple hundred bytes (R5 §4.3 step 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sheet {
    pub accent: u32,   // packed RGBA
    pub ink:    u32,   // packed RGBA (foreground text/line)
    pub paper:  u32,   // packed RGBA (surface)
    pub type_id: u16,  // index into the fixed dowiz type-scale set (NOT a font upload)
    pub radius:  u16,  // corner-radius token (fixed scale)
}
pub const SHEET_TOKEN_COUNT: usize = 5;              // §16.9 cap — the reason the swap is trivial
pub const SHEET_UNIFORM_BYTES: usize = 20;           // 3×u32 + 2×u16 — the whole draft/live cost

/// Two records per hub: what customers see, and what the owner is editing. The ENTIRE
/// draft/live problem (R5 §4.3): hold both, bind one at frame time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrandState { pub published: Sheet, pub draft: Sheet }

/// Node-local brand events. `Published` is the atomic copy draft→published (R5 §4.3 step 4);
/// `Reverted` re-publishes a prior record (step 4, "revert = re-publishing the prior token
/// record"). `DraftEdited` is coalesced (slider drag ⇒ one event per settle, not per frame).
#[derive(Debug, Clone)]
pub enum BrandEvent {
    DraftEdited { sheet: Sheet, owner_sig: Vec<u8> },      // owner-cap-cert over canonical Sheet bytes
    Published   { sheet: Sheet, owner_sig: Vec<u8> },      // atomic; prior published kept as history
    Reverted    { to: Sheet,    owner_sig: Vec<u8> },      // re-publish a prior record
}

// ============ G6 — GDPR delete-customer tool (§16.58) ============

/// A customer reference the vendor can name for erasure. There is NO durable customer
/// identity (P49 deferral) — the only stable handle is the channel-shaped address plus
/// the order-ids that reference it. Erasure is scoped to this closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerRef {
    pub channel: Channel,             // P43's enum (row 12)
    pub peer: String,                 // channel-shaped address (chat id / wa id / phone / email)
    pub order_refs: Vec<u64>,         // orders bound to this peer (fold-derived, not stored)
}

/// The ONE erasure action — owner-authored, owner-cap-cert-signed (mirrors P48's
/// OwnerReplyAction: an erasure without a live owner signature is UNREPRESENTABLE).
#[derive(Debug, Clone)]
pub struct CustomerErasureAction {
    pub customer_ref: CustomerRef,
    pub owner_sig: Vec<u8>,           // owner-cert sig over canonical(customer_ref)
}
/// The node-local erasure EVENT. Append-only (row 13): the ciphertext of the customer's
/// PII stays in the log for chain integrity, but the per-customer data key is destroyed
/// (§3.6 crypto-erasure) so the plaintext is permanently unrecoverable. Irreversible —
/// "deliberately no un-erase" (the RevocationSet posture, cap.rs:412, applied to PII).
#[derive(Debug, Clone)]
pub struct ErasureEvent { pub customer_ref: CustomerRef, pub at_unix_ms: u64 }

/// Typed erasure outcomes — never a silent partial delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErasureError { BadOwnerSig, UnknownCustomer, AlreadyErased }

// ============ G7 — multi-hub client mode (§16.18/§16.48) ============

/// One hub the owner's client connects to, authorized by a P59 child cert under the
/// owner root. `child_cert.may_delegate == false`, depth == 1 (P59 §2.4). Endpoint is
/// the hub's own tunnel/address — dowiz is NOT in this path.
#[derive(Debug, Clone)]
pub struct HubConnection {
    pub child_cert: crate::pq::cert_chain::Delegation,  // P59 — verified against owner root pubkey
    pub endpoint: String,                               // hub-owned reachable address
    pub health: HubHealth,                              // honest per-hub status (§16.14)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HubHealth { Online, Offline, Degraded }        // Offline degrades ITS tile only (§3.7)

/// The client-side merged view over N hubs. Built on the owner's device by fanning reads
/// to each HubConnection and merging. There is NO server type here by construction — a
/// dowiz-side aggregate would need a type that does not exist (§1.4-5).
pub struct MultiHubView { pub root: crate::pq::cert_chain::SelfSignedRoot, pub hubs: Vec<HubConnection> }
pub const MAX_HUBS_SOFT: usize = 64;   // client-side fan-out sanity cap; §4.2 scaling axis

// ============ G5 — marketing auto-posting (Wave-0 basic; a PANE over P22) ============

/// The ONLY net-new type G5 owns: the Wave-0 auto-post TRIGGER set. The post itself is a
/// P22 `MasterPost` (row 11) — G5 mints no post type and no poster. A trigger produces a
/// P22 Path-A template draft (AiMode::Off works); PUBLISH is P22's authority (A6), never here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoPostTrigger {
    MenuItemAdded { leaf_id: crate::catalog::LeafId },   // "new on the menu: …"
    PromoAnnounced { text_ref: WidgetId },               // owner-authored promo (P57 TextField)
}
// The pane's action = build a MasterPost{source: DraftSource::Template, status: PendingReview}
// via P22, land it in the owner's compose region, and (on owner approval) call P22's publish
// authority. G5 imports P22::SocialPoster; it does NOT define it (grep gate, §5).

// ============ shared: the owner-surface SemanticScene authorship (P58 §M7) ============
// P70 authors ONE `SemanticScene` (P58 §2) spanning G1–G7 (and P48's rendered panes). It
// imports `mirror()` and the Playwright `a11yGate(page, manifest)` — it writes NO a11y code.
// Per-screen mirror-node budget declared in each item's DoD (tightened from
// MIRROR_NODE_BUDGET_DEFAULT=256; a 40-control menu editor declares ~40, P58 §M7-2).

// ── Constants ──────────────────────────────────────────────────────────────────
pub const OWNER_ACTION_MAX_BYTES: usize = 4_096;  // refuse-typed above; never truncate (P48 §2 precedent)
```

**Rejected alternatives (DECART, one line each):** *a `theme.css`/font-upload brand model* —
rejected: §16.9 caps brand at 5 tokens precisely so the draft/live problem is a uniform swap, not a
renderer fork (R5 §4.3). *A `BrandPreview` render pass / second pipeline* — rejected: the preview is
`queue.write_buffer(sheet_draft)` on the ONE pipeline; a second pass is a P38 §12.3 scope violation
AND breaks P58 §M6's single-mirror guarantee. *A hard-delete of erasure events from the log* —
rejected: it breaks the content-address chain (row 13 `verify_chain`); crypto-erasure (§3.6)
satisfies GDPR by unrecoverability while preserving chain integrity. *A dowiz-side multi-hub
aggregate cache* — rejected: §16.18 red-line; `MultiHubView` has no server representation. *A new
`SocialPoster` in the owner lane* — rejected: P22 owns posting (P48 §1.1 "a pane, not a second
implementation"). *Storing brand/hub state in a P70 admin table* — rejected: `hub_no_shadow_store`
(§1.5-1); both are fold-derived from `BrandEvent`/`HubConnection` events.

---

## 3. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

Build order G1 → G7. G1/G2/G3 supersede P48 stubs; G4/G5/G6/G7 are net-new. Every item authors
its slice of the shared `SemanticScene` and imports `a11yGate` (P58 §M7) — stated once here, a DoD
line per item in §5.

### 3.1 G1 — Orders management surface (supersedes P48 B2)

**Spec:** a read-only fold over the venue's order events (`Order`, row 16) rendered as a P38 pane
(newest-active-first, per-order detail). The **only** mutating owner actions are **confirm** and
**cancel**, each emitted as an **owner-cap-cert-signed human intent through the existing facade**
(the same `submit_intent` seam P48 §0 row 2 uses) — legal per the kernel's transition law, refused
typed otherwise (§16.51 cancellation authority). **The P48 §10.6 gate is LAW (row 10):** confirm/
cancel are NOT a `ToolAction`, NOT agent-invocable; the `no-agent-order-authority` CI grep covers
this lane. The list is a fold; `hub_no_shadow_store` extends over it (B2 preserved).

RED→GREEN: `g1_orders_fold_read_only` (fixture order events → pane fold shows them in order, no
store outside the log) + `g1_confirm_is_capcert_human_intent` (confirm emits a signed facade intent;
a confirm without a live owner sig is refused typed) + `g1_cancel_follows_transition_law` (cancel
legal only where the machine allows; illegal cancel refused). **Adversarial:** (i) **agent cannot
confirm** — the `no-agent-order-authority` grep + `ToolAction` set (`{Read,Draft}`) proves a
model-invoked confirm/cancel is unrepresentable (P48 §10.6 wall 1, re-run over the G1 crate); (ii)
**shadow-store smell** — any G1 store not fold-derived fails `hub_no_shadow_store`; (iii)
**mid-prepare cancel** — a cancel after vendor-confirm routes to the refund/dispute channel
(§16.51), never a unilateral silent state flip.

### 3.2 G2 — Menu management surface (supersedes P48 B1)

**Spec:** CRUD over P62's **free-form vendor-authored `CatalogNode` tree** (row 8). Add/edit/delete
a category (`NodeBody::Group`) or item (`NodeBody::Leaf(PriceableLeaf)`); on save, run
`validate_tree` (no cycle, no dangling parent, leaves are tree-leaves — **structure only, NO
taxonomy**, §16.17). Every purchasable leaf is constructed **only** via `PriceableLeaf::new` — an
unpriced/uncurrencied/unattributed leaf is *unrepresentable* (X7). **Price entry rides P57's
money boundary (row 2):** the price is a `TextField` `&str`, parsed to `i64` minor units + a
`Currency` into a `Money` **at the save/submit boundary**, and displayed via
`TweenGuard::present_money` — never typed into a tweened caret. Writes ride P62's RLS (outer
`location_id` FORCE + inner `vendor_scope`, row 9); P70 adds no RLS boundary. **B1 preserved:** a
saved edit is carried by the next order's fold (orders price from menu state at placement time).

RED→GREEN: `g2_menu_edit_carried_by_next_order` (edit a leaf's price → a subsequently placed
order's fold uses the new `PriceableLeaf`; a pre-edit in-flight order is unchanged — the P48 Adv-3
property, retargeted) + `g2_validate_tree_on_save` (a save calls `validate_tree`; a cyclic/dangling
tree is refused with the exact `CatalogError`) + `g2_price_entry_parses_at_submit` (a `"7,50"`/
`"7.50"` string → `Money{750, currency}` at submit, presented via `present_money`; the caret never
animates a numeric). **Adversarial (designed to break):** (i) **negative price** — `PriceableLeaf::
new` with a negative `Money` → `CatalogError::NegativePrice`, buffer untouched (money red-line);
(ii) **cross-currency modifier** — a `Delta` component in a different `Currency` → `resolve_line`
→ `CrossCurrency` (P62 `checked_add` gives this for free); (iii) **float injection** — a pasted
`"7.5e3"`/`"NaN"` price string → parse refuses typed, no float ever reaches `Money`; (iv) **taxonomy
creep** — a save that tries to impose a fixed category enum is a review-reject (§16.17 free-form).

### 3.3 G3 — Courier management surface (supersedes P48 B3)

**Spec:** grant a courier = the owner root appends a **P59 child `Delegation`** scoped to the
courier's duty capability (`may_delegate=false`, depth 1 — P59 §2.4/§4.5), verifiable by the hub
knowing only the owner root's public key; revoke = add the courier's key to the owner's **P59
`RevocationBlob`** (row 7), which is signed, monotone (higher `seq` supersedes), and append-only
(**no unrevoke**). **NO-COURIER-SCORING red-line:** no score/rating/rank field exists on any
courier type; the CI grep `! git grep -nEi 'courier[_-]?(score|rating|reputation|rank)'` (P48 §10.0
row) binds this lane.

RED→GREEN: `g3_grant_mints_verifiable_child_cert` (owner root → courier `Delegation`; hub verifies
with owner-root pubkey only, no network — reuses P59 `red_owner_mints_child_offline`) +
`g3_revoke_rejects_next_request` (revoke → the courier's next mutating request fails
`ChainError::Revoked`; B3 preserved) + `g3_revoke_is_monotone` (a replayed stale `seq` cannot
un-revoke — P59 `red_stale_seq_cannot_unrevoke`). **Adversarial:** (i) **no scoring field** — grep
gate proves it; (ii) **cross-owner forgery** — owner A's courier chain presented under owner B's
anchor → `UnknownIssuer` (P59 `red_cross_owner_forgery`); (iii) **child re-delegation** — a courier
cert with `may_delegate=false` trying to mint a grandchild → `MaxDepthExceeded` (P59 depth ceiling).

### 3.4 G4 — Brand draft/live preview via uniform-buffer swap (NET-NEW; X5, R5 §4.3)

**Spec (the whole answer, per R5 §4.3 / X5 — one pipeline, two token buffers, a bind flag):**

1. Hold **two `Sheet` records** per hub: `published` and `draft` (`BrandState`, ~20 bytes each).
2. The wgpu field engine already reads Sheet tokens from a **uniform buffer**. Draft preview =
   `queue.write_buffer` the `draft` Sheet into the Sheet uniform and bind it **instead of**
   `published`. **Same pipeline, same shaders, same Sea layer** — only the uniform source changes.
   There is **no second render path, no preview subdomain, no staging deploy** (§1.4-3).
3. **Preview is owner-only:** the `draft` buffer is bound **only** in the owner surface behind the
   owner capability cert. Customer clients only ever fetch/bind `published` (R5 §4.3 step 3).
4. **Publish = atomic copy** `draft → published` — ONE `BrandEvent::Published` event; the Sea/
   content is untouched. **Revert = re-publish a prior record** (`BrandEvent::Reverted`), kept as
   history (R5 §4.3 step 4).
5. **Live-updating preview:** as the owner drags a color/radius slider, write straight into the
   `draft` uniform each frame; the field re-renders with the physics already running (true "real
   field-engine" WYSIWYG). Use **GPUI's glyph-atlas + damage-tracking technique for the slider-drag
   refresh** (X5 — mine the technique, don't invent a refresh strategy); the drag re-wakes the
   field via FE-14's settle gate, it does not stand up a new pipeline.

**a11y parity BY CONSTRUCTION (P58 §M6-1, X1/X5 — the invariant, not an assertion):** the a11y
mirror is `mirror(&SemanticScene)`, and the Sheet tokens live in P38's `FrameUniforms`, **not** in
the `SemanticScene`. A brand swap is `queue.write_buffer(theme/Sheet)` — it changes pixels, not the
`SemanticScene` — so the `A11yTree` is **byte-identical draft vs live**. P70 writes **zero**
brand-preview a11y code; a brand-preview-specific a11y path is *unrepresentable*, not merely
discouraged (P58 §M6, §2 rejected-alt "a second mirror for brand-preview").

RED→GREEN (the falsifiable brand-preview-parity test — §5 DoD row G4, THE required test):
`g4_brand_preview_a11y_parity` (native, pure) — build ONE owner-surface `SemanticScene`; produce
`A11yTree` under the `draft` Sheet bound and under the `published` Sheet bound; `assert_eq!` the
two trees byte-for-byte (this is P58's `a11y_tree_invariant_under_presentation_swap` retargeted to
the two `Sheet` records). **Not-done clause (P58 §M6):** any brand-preview code that authors its own
a11y = NOT done; any `mirror` overload taking a `Sheet`/token argument = NOT done. Plus
`g4_publish_is_atomic_one_event` (publish = exactly one `BrandEvent::Published`; the Sea/content
fold is unchanged) + `g4_customer_never_sees_draft` (a customer-surface fetch returns `published`
only; the draft buffer is bound only under the owner cert). **Adversarial:** (i) **draft leak** — a
customer client requesting the draft Sheet is refused (owner-cert-gated); a test drives the
customer surface and asserts it binds `published` even when a draft exists; (ii) **revert
integrity** — publish A, publish B, revert-to-A → the fold shows A as published and the full
A→B→A sequence is visible in the log (audit by construction, §1.5-2); (iii) **slider spam** — 10³
`DraftEdited` frames coalesce to per-settle events, and the field integrator does not thrash (the
FE-14 settle gate holds — one uniform write per frame, not one pipeline rebuild).

### 3.5 G5 — Marketing auto-posting pane, Wave-0 basic (NET-NEW; §16.36 — a PANE over P22)

**Spec:** an owner pane that invokes **P22's existing `SocialPoster` lane** (row 11) — "the hub
gives it a pane, not a second implementation" (P48 §1.1). **Wave-0 basic auto-posting:** an
`AutoPostTrigger` (a new menu item, or an owner-authored promo — the only two Wave-0 triggers)
produces a P22 **Path-A template `MasterPost`** (`DraftSource::Template`, `status: PendingReview`)
— which **works at `AiMode::Off`** (P22's load-bearing path). The draft lands in the owner's compose
region; **publish is P22's authority (A6 — publish authority is never the model)**, triggered by an
owner tap. Transport is P22's provider/channel adapters, never P43's transactional port (row 12).
**Boundary vs P48 H7:** G5 is **public** posts (`MasterPost`, one→many, public blast radius); P48
§10.5 H7 is **private** segmented promo *replies* into conversations (campaign lane, consent-gated).
Different types, different blast radii — G5 does not touch H7's lane.

RED→GREEN: `g5_trigger_drafts_template_masterpost` (a `MenuItemAdded` trigger → a P22
`MasterPost{Template, PendingReview}` in the compose region; green at `AiMode::Off`) +
`g5_publish_requires_owner_tap` (the pane never auto-publishes; publish routes through P22's
authority). **Adversarial:** (i) **no second poster** — grep proves the G5 lane imports
`P22::SocialPoster` and defines no poster of its own; (ii) **no bulk through P43** — a marketing
payload cannot reach P43's transactional port (`Marketing` absent, one recipient — unrepresentable,
row 12); (iii) **model cannot publish** — an AI-drafted post still lands `PendingReview`; publish
authority is the owner tap (P22 A6), never the model.

### 3.6 G6 — GDPR delete-customer tool (NET-NEW; §16.58 — vendor-triggered, dowiz-blind)

**The requirement (§16.58):** the vendor is legally responsible for erasure against their own hub's
order history; **dowiz provides a built-in "delete everything about customer X" tool in the hub
software itself** so the vendor doesn't build it from scratch. dowiz has **no visibility into WHEN
or WHY** it is used (consistent with §16.14/§16.36 data isolation and §16.60's PII boundary). The
client-side data wallet (§16.47) is self-deletable by the customer directly — G6 covers the **hub
side** only.

**The real design tension, resolved by math not policy — append-only content-addressed log (row
13) vs GDPR right-to-erasure:** a hard delete of PII-bearing events would break the content-address
chain (`verify_chain`). **Mechanism: crypto-erasure (crypto-shredding).** Each customer's PII fields
(the channel-address `peer`, name, delivery address in intake/conversation/notification events) are
sealed under a **per-customer data key** in the hub's local keystore. `CustomerErasureAction`
(owner-cap-cert-signed) appends an `ErasureEvent` and **destroys the per-customer key**. The
ciphertext remains in the log (chain integrity preserved — event-ids/hashes unchanged), but the
plaintext is **permanently unrecoverable** — GDPR erasure satisfied by construction (unrecoverability),
not by a policy promise. All PII-bearing folds decrypt-on-read; after key destruction they surface a
`[redacted]` marker. Anonymized aggregates (the vendor's own order-count) may remain — they carry
**zero** PII and are out of erasure scope. **Irreversible:** no un-erase (the `RevocationSet`
"deliberately no unrevoke" posture, cap.rs:412, applied to PII). *(Fallback if per-customer keying
is deferred: a `redaction` event all folds honor + physical shredding of the redacted payload bytes
at the next snapshot/compaction — the log keeps the erasure event + structural hashes, the PII bytes
are gone. Crypto-erasure is primary because it is immediate and chain-integrity-clean.)*

RED→GREEN: `g6_erasure_removes_all_pii_folds` (erase customer X → every PII-bearing fold — intake,
conversation, notification, order-history detail — returns `[redacted]` for X; the vendor's
anonymized order-count is unchanged) + `g6_erasure_is_owner_signed` (an `ErasureAction` without a
valid owner-cert sig → `ErasureError::BadOwnerSig`, log untouched) + `g6_chain_integrity_after_erase`
(`verify_chain` is GREEN after erasure — crypto-shredding does not break the content-address chain).
**Adversarial:** (i) **irreversible** — no code path un-erases; a second erase of an already-erased
customer → `AlreadyErased` typed, idempotent; (ii) **dowiz-blind** — grep proves the erasure event
egresses to **no** dowiz endpoint (§1.4-9); running the full hub suite with every dowiz endpoint
black-holed leaves erasure fully functional; (iii) **PII resurrection** — no fold, replay, or
snapshot restore can surface the erased plaintext (the key is gone) — a restore-from-backup test
asserts the erased customer stays `[redacted]`.

### 3.7 G7 — Multi-hub client mode (NET-NEW; §16.18/§16.48 — client-side aggregation only)

**Spec (§16.48 exact cert shape, P59 §3/§4.5):** the owner holds **one P59 `SelfSignedRoot`** (the
root credential they self-hold — no dowiz account, §16.48). For each hub they run, the owner root
mints a **child `Delegation`** (`may_delegate=false`, `MAX_DELEGATION_DEPTH=1`, P59 §2.4) — a
self-service **add** (mint), **modify** (attenuate — narrower scope, never widens, `is_subset_of`),
**revoke** (add to the owner's `RevocationBlob`). The owner's client holds N `HubConnection`s and
**fans reads out to each hub, verifying each with the owner-root public key, merging the view
CLIENT-SIDE on the owner's own device** (`MultiHubView`). **There is NO dowiz-operated aggregation
server, ever** (§16.18 red-line) — the merge type has no server representation (§2). A hub that is
`Offline` degrades **its own tile only** (honest status, §16.14), never the merged view.

RED→GREEN: `g7_owner_root_mints_n_child_certs` (owner root → 3 hub child certs; each hub verifies
its cert with the owner-root pubkey only, no dowiz — P59 `red_owner_mints_child_offline`) +
`g7_merge_is_client_side` (the full G7 suite runs with **every dowiz endpoint black-holed** and
still merges N hubs — the §1.4-5 falsifier) + `g7_revoke_drops_a_hub` (revoking a hub's child cert
→ that hub's next request from the client fails `Revoked`; the roll-up drops its tile).
**Adversarial:** (i) **no dowiz aggregator** — grep proves no G7 component depends on a dowiz
endpoint; (ii) **cross-owner forgery** — owner A's hub cert under owner B's root → `UnknownIssuer`
(P59 `red_cross_owner_forgery`); (iii) **one hub offline** — a `HubHealth::Offline` hub shows an
honest offline tile; the other N-1 hubs' tiles render fully (§16.14 no-disguised-retry, no central
queue).

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6) — reachability argued from types, never policy

- **Order confirm/cancel cannot be automated:** the agent-reachable `ToolAction` set is
  `{Read, Draft}` (P48 §10.6 wall 1, row 10) — no `Confirm`/`Cancel`/`Send` variant exists, so a
  model-invoked order mutation is *unrepresentable*. G1's confirm/cancel exist only as owner-cap-cert
  human intents through the facade. Three walls (type / config / CI grep) carried verbatim from P48
  §10.6; the `no-agent-order-authority` grep binds P70's lane. **Red-line-class.**
- **Menu price cannot go negative or float or cross currency:** a leaf is a `PriceableLeaf` whose
  `price` is a `Money` (row 14); `PriceableLeaf::new` refuses negative (`assert_non_negative`),
  `resolve_line` refuses cross-currency (`Money::checked_add`), and a float never reaches `Money`
  (parse-at-submit refuses non-integer minor units, P57 §2.2). A mispriced menu requires a type
  that does not exist.
- **Brand preview cannot desync a11y or leak to customers:** the mirror takes only `&SemanticScene`
  (P58 §M6) and the Sheet lives in `FrameUniforms` — draft/live a11y parity is *structural*; the
  draft buffer is owner-cert-gated so a customer-visible draft requires binding a buffer the
  customer surface never binds. **Two independent structural gates.**
- **GDPR erasure is irreversible and dowiz-blind:** the per-customer key is destroyed (no un-erase
  path exists — the `RevocationSet` append-only posture); the erasure event egresses to no dowiz
  endpoint (grep gate). Unrecoverability is by construction (crypto-shredding), not by policy.
- **Multi-hub cannot become a central store:** `MultiHubView` has no server representation; a
  dowiz-side aggregate needs a type that does not exist (§1.4-5).
- **Authority:** every P70 mutating action (menu, courier, brand, erasure, hub add/revoke) is
  owner-cap-cert-scoped (B4 + P59 chain); no password path exists (B4). The in-hub assistant is
  excluded from customer PII by default (§16.60) — G6's `CustomerRef` is never fed to a model.

### 4.2 Schemas & scaling axes (item 8)

- **Orders/menu/courier folds:** per-venue-day volume in the low thousands worst-case; folds are
  linear over the event log; break point joins P34B's compaction trigger (~10⁶ leaves). No axis
  break before multi-venue nodes; per-venue keying is the existing convention (P62 `VendorId`
  scoping for food-court).
- **Brand:** two `Sheet` records × ~20 bytes = trivially held; `DraftEdited` coalesced per-settle
  (not per-frame) so slider drags don't grow the log unboundedly — the axis is "settles," not
  "frames."
- **Multi-hub:** `MAX_HUBS_SOFT = 64` client-side fan-out; the axis is N hubs (breadth, not depth —
  P59 depth stays 1). Above 64, the client paginates hub tiles — no server ever enters.
- **Erasure:** per-customer key destruction is O(1); the redacted-payload compaction (fallback path)
  joins the existing snapshot/compaction pass, no new schedule.
- **Mirror-node budget (P58 §M7-2):** each screen declares a tightened budget (a 40-control menu
  editor declares ~40, not 256) so mirror bloat is caught as a DoD failure.

### 4.3 Isolation (11), mesh (12), rollback (13), living memory (15)

- **Isolation:** G1–G7 are seven independent fold-producers/consumers over the one log; each
  degrades alone (typed errors, empty-pane-with-reason, offline tile) and none can block the order
  fold. A hub-offline in G7 isolates to one tile (§3.7). The owner-surface crate depends on
  kernel + facade + the four upstream blueprints' contracts only; adapters (P22/P43/provider) stay
  at the edge (P43's crate discipline mirrored).
- **Mesh (12):** everything node-local. Brand/erasure/hub events are node-local families on
  `event_log.rs` (row 13), NOT proto-cap wire variants (P34 anti-scope — the P48 §10.1 move).
  Multi-device sync of ONE venue's owner surface rides P34B's signed-log sync lane (owner's phone +
  shop terminal); payload = the same `DeliveryEvent`-class frames already budgeted (~5–6 KB, P34
  §4.2) plus ~20-byte brand events. Multi-hub (G7) is **not** mesh gossip — it is the owner's
  client fanning reads to N independent hubs (client-side, §16.18). No new transport, no new gossip
  payload class.
- **Rollback (13, vocabulary used precisely):** **Self-Termination** — typed refusals everywhere
  (bad owner sig, negative price, cross-currency, unknown suite, offline hub); an unsafe state
  (automated confirm, un-erased PII, dowiz aggregate) is *unrepresentable*, not supervised.
  **Snapshot-Re-entry** — every pane is folds over an append-only log: any pane rebuilds from
  replay; **brand revert is `BrandEvent::Reverted` re-publishing a prior record** (§3.4) — a
  cheap regenerative recovery to a prior valid brand epoch, the textbook Snapshot-Re-entry shape.
  **Self-Healing is NOT claimed** — no redundancy math here.
- **Living memory (15):** the log IS the temporal store — orders, brand history, erasure events are
  time-ordered, content-addressed history; recall by content not location
  (`internal-retrieval-living-memory-arc-2026-07-14`'s principle, inherited via row 13's machinery,
  nothing new). Brand history and the erasure ledger are living-memory reads.

### 4.4 Error-propagation gates (item 14) + Linux discipline (item 9) + tensor/spectral (item 16)

- **Smart-index gates (item 14):** closed enums (`BrandEvent`, `ErasureError`, `HubHealth`,
  `AutoPostTrigger`, `CatalogError`); the `no-agent-order-authority` CI grep (order authority);
  the `no-courier-scoring` CI grep (G3); `hub_no_shadow_store` review-gate (every pane); a grep
  gate proving no P70 component depends on a dowiz endpoint (G7/G6); the P62 RLS FORCE gate (G2
  writes). Each turns a whole bug class into a CI-time failure, not a runtime surprise.
- **Linux discipline (item 9):** **ALREADY-EQUIVALENT** — one Law/one log for every management pane
  (folds, not an admin DB); ports-at-the-edge (P22/P43/provider adapters outside the kernel module).
  **REINFORCES** — fail-closed typed outcomes on every owner action. **EXTENDS** — the fail-closed
  doctrine to *presentation* (brand a11y parity as a type invariant) and to *erasure* (crypto-
  shredding as an irreversibility invariant). **GAP (named, deferred):** the "revert to an arbitrary
  historical brand N versions back" UX is bounded to "the last published" for Wave-0 (a deeper
  brand-history browser is v2, tracked in §5's ledger).
- **Tensor/spectral + eqc (item 16):** **N/A-honest** — no closed-form math organ on the management
  path; no decorative claim (P48 §8 item-16 precedent).

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

Every row imports P58's `a11yGate` (author `SemanticScene`, supply a tightened mirror-node budget,
carry the FE-16 WebGL2/CPU floor line verbatim — P58 §M7). RED preamble: `grep -rn "BrandEvent\|
CustomerErasureAction\|MultiHubView\|AutoPostTrigger" --include="*.rs" .` → 0 hits today.

| Item | RED (fails before) | GREEN (passes after) | Named tests (permanent, item 17) |
|---|---|---|---|
| **G1** orders mgmt | no orders-mgmt surface; B2 is a stub | read-only fold; confirm/cancel = cap-cert human intents; agent cannot confirm | `g1_orders_fold_read_only`, `g1_confirm_is_capcert_human_intent`, `g1_cancel_follows_transition_law`, `no-agent-order-authority` re-run |
| **G2** menu mgmt | B1 is a stub; no P62-backed CRUD | edit carried by next order; `validate_tree` on save; price parses at submit; negative/cross-currency/float refused | `g2_menu_edit_carried_by_next_order`, `g2_validate_tree_on_save`, `g2_price_entry_parses_at_submit`, `g2_negative_price_refused`, `g2_cross_currency_refused` |
| **G3** courier mgmt | B3 is a stub over raw `RevocationSet` | grant = verifiable P59 child cert; revoke rejects next request, monotone; no scoring | `g3_grant_mints_verifiable_child_cert`, `g3_revoke_rejects_next_request`, `g3_revoke_is_monotone`, `no-courier-scoring` grep |
| **G4** brand preview | no draft/live state | **THE brand-preview-parity test** + atomic publish + customer never sees draft | **`g4_brand_preview_a11y_parity`** (A11yTree draft ≡ published, `assert_eq!`), `g4_publish_is_atomic_one_event`, `g4_customer_never_sees_draft`, `g4_revert_integrity` |
| **G5** marketing pane | no auto-post pane | trigger drafts a P22 template MasterPost (AiMode::Off); publish = owner tap; no second poster | `g5_trigger_drafts_template_masterpost`, `g5_publish_requires_owner_tap`, `g5_no_second_poster` (grep), `g5_no_bulk_through_p43` |
| **G6** GDPR delete | no erasure concept | all PII folds redacted; owner-signed; chain integrity kept; irreversible; dowiz-blind | `g6_erasure_removes_all_pii_folds`, `g6_erasure_is_owner_signed`, `g6_chain_integrity_after_erase`, `g6_dowiz_blind` (endpoint black-hole), `g6_no_pii_resurrection` |
| **G7** multi-hub | no multi-hub client mode | owner root mints N child certs; merge client-side w/ dowiz black-holed; offline = one tile | `g7_owner_root_mints_n_child_certs`, `g7_merge_is_client_side`, `g7_revoke_drops_a_hub`, `g7_offline_hub_isolated` |
| **a11y (all)** | — | `a11yGate` passes on WebGPU AND WebGL2/CPU floors, per-screen budget respected | `a11yGate(owner_surface, manifest)` per screen (P58 §M5) |
| **§1.5 no-aggregator** | — | every P70 pane is a fold; no store outside the log; no dowiz-dependent component | `hub_no_shadow_store` (extended), `no_dowiz_endpoint_dependency` grep |

**THE required falsifiable brand-preview-parity test, stated as the standard demands (§5 row G4):**
`g4_brand_preview_a11y_parity` builds one owner-surface `SemanticScene`, produces the `A11yTree`
with the `draft` Sheet bound and with the `published` Sheet bound, and `assert_eq!`s them
byte-for-byte. It is GREEN **by construction** because `mirror(&SemanticScene)`'s signature admits
no Sheet/token input (P58 §M6-1) — the Sheet lives in `FrameUniforms`, a different type. **Not-done
clause:** any brand-preview a11y code, or any `mirror` overload taking a token, = NOT done.

Ledger obligations (`docs/regressions/REGRESSION-LEDGER.md`, red→green per its ratchet rule): (a)
"order confirm/cancel is never agent-invocable in the owner surface — guardrail:
`no-agent-order-authority` re-run over G1"; (b) "menu price cannot be negative/float/cross-currency
— guardrail: `g2_negative_price_refused` + `g2_cross_currency_refused`"; (c) "brand preview draft
and live share ONE a11y tree — guardrail: `g4_brand_preview_a11y_parity`"; (d) "GDPR erasure is
irreversible and dowiz-blind — guardrail: `g6_no_pii_resurrection` + `g6_dowiz_blind`"; (e)
"multi-hub roll-up never touches a dowiz server — guardrail: `g7_merge_is_client_side`"; (f) the
§4.4 GAP (brand-history depth bounded to last-published for Wave-0) as tracked-open.

---

## 6. Benchmark plan (item 10) — modest by design; nothing here is hot next to network I/O

1. `owner/menu_validate` — `validate_tree` over a 200-node vendor tree, **budget ≤ 2 ms** (pure
   tree walk; documents non-network cost so a future "smarter" validator can't go quadratic).
2. `owner/brand_uniform_swap` — `queue.write_buffer(draft Sheet)` + rebind, **budget ≤ 0.1 ms**
   (20 bytes; the number exists so a regression that accidentally rebuilds the pipeline shows up).
3. `owner/multihub_merge` — client-side merge of 8 hubs' order folds, **budget ≤ 5 ms** (in-memory
   merge; proves the roll-up is device-cheap, no server needed).
4. `owner/erasure_apply` — key-destroy + fold-redact over a customer with 50 orders, **budget ≤
   3 ms** (crypto-shred is O(1); documents the cost so GDPR latency is never blamed on the fold).

Numbers into the established `BENCH_HISTORY.md` convention; telemetry = the log itself plus existing
read surfaces (no third channel — item 19).

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (W2 row P70), X1/X5 (brand-preview parity + GPUI
uniform-swap posture) · `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.9/§16.18/§16.36/
§16.48/§16.58/§16.14/§16.60 (the operator rulings this executes) · **`BLUEPRINT-P48-owner-hub-
surface.md`** (the surface this SUPERSEDES-or-EXTENDS — §1.2 ledger; B1/B2/B3 superseded, H1–H8
consumed, §10.6 gate preserved) · **`BLUEPRINT-P57-canvas-text-input.md`** §2.1/§2.2/§3 (`TextField`,
money-entry boundary) · **`BLUEPRINT-P58-a11y-mirror-everywhere.md`** §2/§M6/§M7 (`SemanticScene`/
`mirror`/`a11yGate`, draft-parity invariant, P48/P52 a11y supersession) · **`BLUEPRINT-P59-
capability-cert-chain.md`** §2.4/§3/§4.5/§4.7 (`SelfSignedRoot`/`Delegation`/`RevocationBlob`, owner
multi-hub delegation) · **`BLUEPRINT-P62-catalog-multivendor-data-model.md`** §1.2/§3/§4 (leaf
invariant, `CatalogNode`/`validate_tree`) · `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md` §11.1/
§11.3/§11.5 (P22 `SocialPoster` + publish authority — G5's lane) · `BLUEPRINT-P43-external-
integration-ports.md` §1.2/§1.3/§2 (P22/P43 boundary; transactional-only) · `OPUS-R5-MULTIVENDOR-
ECOSYSTEM-OPS-2026-07-18.md` §4 (uniform-buffer-swap recommendation, verbatim in §3.4) ·
`kernel/src/{event_log.rs,money.rs,domain.rs,ports/agent/cap.rs}` (§0.B substrate). Memory files:
`integration-ports-reactive-arc-2026-07-13` (ports doctrine) · `test-integrity-rules-2026-06-27`
(money/PII red-lines; extended by §5 rows b/d) · `never-bypass-human-gates-2026-06-29` (the
order-confirm/cancel gate and owner-signed erasure are human gates by design) ·
`verified-by-math-2026-07-07` (falsifiability bar; G4/G6 unrepresentability arguments) ·
`internal-retrieval-living-memory-arc-2026-07-14` (§4.3) · `anu-ananke-strict-discipline-feedback-
2026-07-17` (style discipline) · `rust-native-bare-metal-decision-2026-07-14` (DECART one-liners).

---

## 8. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 — 16 rows, live-verified this pass: 0.A dependency contracts by exact section, 0.B kernel substrate by `file:line`, 0.C roadmap decisions |
| 2 DoD | §5 — 7 build-item RED→GREEN rows + a11y + no-aggregator; THE falsifiable brand-preview-parity test named (G4) |
| 3 spec/event-driven TDD | §2 types first; §3 per-item RED tests asserting on event sequences (BrandEvent, ErasureEvent, order folds) |
| 4 predefined types/consts | §2 — `Sheet`/`BrandState`/`BrandEvent`, `CustomerRef`/`ErasureAction`/`ErasureEvent`, `HubConnection`/`MultiHubView`, `AutoPostTrigger` + consts, DECART-rejected alternatives |
| 5 adversarial tests | §3.1–§3.7 — agent-confirm, negative/cross-currency/float price, draft-leak, no-second-poster, PII-resurrection, cross-owner forgery, offline-hub |
| 6 hazard-safety as math | §4.1 — unrepresentability arguments (no order-mutating ToolAction, Money type, mirror-takes-only-SemanticScene, key-destroyed erasure, no server aggregate type) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 — folds, brand settles, N-hubs breadth, mirror-node budgets; break points named |
| 9 Linux discipline | §4.4 — four verdicts incl. one honest GAP with a tracking row |
| 10 benchmarks+telemetry | §6 — four budgets; log as the telemetry surface |
| 11 isolation/bulkhead | §4.3 — seven independent lanes; offline-hub isolates to one tile; edge-adapter discipline |
| 12 mesh awareness | §4.3 — node-local events; multi-device sync via P34B's lane w/ payload budget; multi-hub is client fan-out, NOT gossip |
| 13 rollback/self-heal vocabulary | §4.3 — Self-Termination + Snapshot-Re-entry (brand revert = prior-record re-publish) with mechanisms; Self-Healing explicitly not |
| 14 error-propagation gates | §4.4 — closed enums, `no-agent-order-authority`, `no-courier-scoring`, `hub_no_shadow_store`, `no_dowiz_endpoint_dependency`, RLS FORCE |
| 15 living memory | §4.3 — the log as temporal store (brand history, erasure ledger); nothing new built |
| 16 tensor/spectral + eqc | N/A-honest: no closed-form math organ on the management path; no decorative claim |
| 17 regression ledger | §5 — six rows incl. the parity tooth (c), the erasure tooth (d), the no-aggregator tooth (e), and the open GAP (f) |
| 18 agent-executable instructions | §9 |
| 19 reuse-first | §1.2 (supersede-not-rebuild), §2 (P57/P58/P59/P62/P22 vocabulary reused, not redeclared), §3.5 (pane over P22, not a second poster), rejected alternatives throughout |
| 20 Hermetic citations | §9 (folded, explicit per principle) |

---

## 9. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Repo: `/root/dowiz`. **Gate check first:** P70 build items require the four W1 contracts to EXIST as
code, not just blueprints — **P57** (`TextField`), **P58** (`SemanticScene`/`mirror`/`a11yGate`),
**P59** (`SelfSignedRoot`/`Delegation`/`RevocationBlob`), **P62** (`CatalogNode`/`PriceableLeaf`/
`validate_tree`). Where a contract is still paper, STOP and flag (do not stub it P70-side — that is
the boundary). P48's H1–H8 panes are CONSUMED, not rebuilt.

1. **T1 (ports).** Create `kernel/src/ports/owner_surface.rs` (§2 verbatim: `Sheet`, `BrandState`,
   `BrandEvent`, `CustomerRef`, `CustomerErasureAction`, `ErasureEvent`, `HubConnection`,
   `MultiHubView`, `AutoPostTrigger`); register in `ports/mod.rs`. NO new kernel deps
   (`git diff kernel/Cargo.toml` → empty). Acceptance: `cd kernel && cargo test --lib` green.
2. **T2 (G1 orders).** Orders fold pane + confirm/cancel as cap-cert facade intents. Re-run
   `no-agent-order-authority` over the G1 crate (P48 §10.6). Acceptance: §3.1 tests green.
3. **T3 (G2 menu).** Menu CRUD over P62's `CatalogNode`; price entry via P57 `TextField` parsed at
   submit; `validate_tree` on save. Do NOT fork P62's catalog/price types. Acceptance: §3.2 tests
   green incl. negative/cross-currency/float adversarials.
4. **T4 (G3 courier).** Grant = P59 child `Delegation`; revoke = P59 `RevocationBlob`. `no-courier-
   scoring` grep must pass. Acceptance: §3.3 tests green.
5. **T5 (G4 brand).** Two `Sheet` records + uniform swap (R5 §4.3); publish = one `Published` event;
   revert = re-publish prior. Author the owner-surface `SemanticScene`; write ZERO brand a11y code
   (P58 §M6). Acceptance: **`g4_brand_preview_a11y_parity`** green (the required test) + §3.4 tests.
6. **T6 (G5 marketing).** Owner pane importing `P22::SocialPoster`; `AutoPostTrigger` → P22 Path-A
   template draft; publish = owner tap via P22's authority. Define NO poster. Acceptance: §3.5 tests
   green incl. `g5_no_second_poster` grep, at `AiMode::Off`.
7. **T7 (G6 GDPR).** Per-customer key crypto-erasure; `CustomerErasureAction` owner-signed; erasure
   event egresses to NO dowiz endpoint. Acceptance: §3.6 tests green incl. `g6_chain_integrity_
   after_erase`, `g6_dowiz_blind`, `g6_no_pii_resurrection`.
8. **T8 (G7 multi-hub).** Owner root mints N child certs; client-side merge; run the suite with
   every dowiz endpoint black-holed. Acceptance: `g7_merge_is_client_side` green + §3.7 tests.
9. **T9 (a11y + close-out).** Author the full owner-surface `SemanticScene`, import `a11yGate` per
   screen with tightened mirror-node budgets, carry the FE-16 floor line. Run `hub_no_shadow_store`
   + `no_dowiz_endpoint_dependency`. Append §5's six ledger rows; record benches. Do not mark P70
   done if any adversarial was weakened, `#[ignore]`d, or inverted.

**Stop-and-flag conditions (do not improvise past these):** (i) any analytics/report/chart-dashboard
surface (§1.4-1 — deferred to v2); (ii) any brand preview via a second render pass / subdomain /
staging deploy (§1.4-3); (iii) any brand-preview-specific a11y code (P58 §M6 makes it redundant);
(iv) any order-confirm/cancel made agent-invocable or a config key (P48 §10.6 red-line); (v) any
courier score/rating/rank field (no-courier-scoring); (vi) any dowiz-operated multi-hub aggregation
endpoint (§16.18 red-line); (vii) any GDPR-erasure event reaching a dowiz endpoint, or an un-erase
path (§16.58/§16.14); (viii) any second `SocialPoster` or bulk-send through P43 (P22/P43 boundary);
(ix) any P70-side catalog/cert/text type that forks P62/P59/P57 (reuse-first).

**Hermetic principles honored (item 20, explicit):** **P2 CORRESPONDENCE** — one Law/one log for
every management pane (folds, not an admin DB); one `SemanticScene` for the whole surface (one
mirror); one owner root for all authority; the brand preview is one pipeline (one mirror ⇒ draft
parity by construction). **P6 CAUSE-AND-EFFECT** — every pane is a deterministic fold of signed
events; replay reproduces it; brand revert regenerates a prior epoch. **P7 GENDER (no self-
certification)** — courier/hub certs are verified against the owner root's public key (never a
surface claiming authority); erasure unrecoverability comes from key destruction (a physical fact),
not a policy claim; posting publish-authority is the owner tap, never the model. (Other principles
not load-bearing here; not claimed decoratively.)
