# BLUEPRINT P60 — Payment adapter core: provider-agnostic port, N-leg vendor-as-MoR atomicity, idempotency contract, no-PAN structural firewall (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Component:
> **PAYMENT / MONEY-FLOW**. Wave **W1** of the CORE roadmap
> (`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5). Scope = the §5 W1 table row **P60**:
> the `PaymentProvider` port, the Stripe Wave-0 server adapter, webhook verify/normalize, the
> **owned idempotency contract** (X6), the vendor-as-own-merchant-of-record **N-leg
> auth-then-capture atomicity** design (§0.2-1), refund routing (§16.29), TokenBucket anti-abuse
> (X11), and the type-level no-card-data firewall. Grounds every design claim in R2
> (`docs/research/OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md`) + live kernel code. Structural
> template + rigor precedent: `BLUEPRINT-P51-open-map-routing.md`. Sibling in-repo precedent
> reused wholesale, NOT re-derived: `kernel/src/ports/payment.rs` (the cash-settlement
> `PaymentPort` + its compile-firewall doctrine).
>
> **Operator rulings applied as inputs, NOT re-litigated** (all CLOSED per the task + synthesis
> §0.2 / §4): each vendor is their **own** merchant-of-record — dowiz is never a party to the
> money (§0.2-1); card capture = **Path C hosted redirect** on web + desktop, **Path B native
> SDK sheet** on mobile (Tauri) pending P63, wgpu canvas **never** renders a card field (§4-A);
> Eurozone/EU first, **EUR**, **Stripe Connect** primary, **Adyen** named fallback (§4-D). With
> §4-A and §4-D closed, **P60 has no remaining operator gate** — it is fully writable and its
> client card leg is unblocked.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The single most load-bearing finding:
**the kernel already carries the entire structural doctrine this port needs** — money-integer
law, event-sourced decide/fold settlement, the compile-firewall self-source-scan, the
degrade-closed TokenBucket, the P07 compensation states, and the capability-verify machinery.
P60 adds a *sibling* online-provider port beside the existing cash port; it re-derives none of
the doctrine.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| Cash `PaymentPort` EXISTS: trait + value types for the cash-on-delivery settlement rail; "the concrete adapter … owns all transport … `cargo tree -p dowiz-kernel` must show NO payment adapter dependency" | `kernel/src/ports/payment.rs:1-12` (module doc), trait `:313-329` | **VERIFIED — P60 is a sibling port, additive, never edits this file** |
| **The compile-firewall self-source-scan pattern** — `firewall_self_source_is_clean` greps the module's own `include_str!`-embedded source for forbidden crate imports assembled via `concat!`; a stray adapter/`reqwest`/`serde` import is a HARD test-run failure | `kernel/src/ports/payment.rs:508-560` (`FORBIDDEN_IMPORTS` `:516-536`, test `:542`) | **VERIFIED — this IS the no-PAN structural guarantee mechanism P60 extends (§4.1)** |
| Money red-line already law: settlement is an EVENT APPEND, fold is the only writer, integer-exact, degrade-closed; `SETTLEMENT_IDEMPOTENCY_KEY` typed; `decide_settlement` pure decide-before-commit | `kernel/src/ports/payment.rs:14-27` (doc), `:84`, `:367-443` | VERIFIED — P60's online decide/fold mirrors this shape exactly |
| Settlement reuses `verify_chain` / `RevocationSet` / `AnchorRoster` / `SignatureVerifier` — "no new crypto" | `kernel/src/ports/payment.rs:31-38`; source `kernel/src/ports/agent/cap.rs:82` (trait), `:377` (roster), `:412` (revocations), `:486` (`verify_chain`) | VERIFIED — P60 reuses the SAME machinery for webhook-secret + provider-account authority |
| Payment-rail **capability declaration** EXISTS: `PaymentRail { Fiat, Crypto, Stripe, GoogleApplePay, OtherLater }`, `PaymentCapability { rail, enabled }`, `validate()` rejects `OtherLater`; its OWN red-line self-scan `red_line_no_real_provider_references` (no client/credential in this module) | `kernel/src/ports/payment_capability.rs:39-54` (enum, `Stripe` `:47`), `:205` (`validate`), `:234` (red-line test) | **VERIFIED — this is the feature-flag that GATES P60's adapter; P60 cites it, never redefines it** |
| `TokenBucket` EXISTS, zero-dep, monotonic-clock, **degrade-closed** (`try_acquire → false` on shortfall, never a partial grant), poison-recovering | `kernel/src/token_bucket.rs:26` (struct), `:34` (`new`), `:74` (`try_acquire`, degrade-closed), `:94` (`available`) | **VERIFIED — X11's checkout-intent limiter is a configuration of this, zero new dep** |
| Order machine already has the **P07 compensation states**: `Refunding` + `CompensatedRefund` (terminal, "ledger nets to exactly zero"); every post-`Confirmed` state may transition to `Refunding`; `Refunding → CompensatedRefund` | `kernel/src/order_machine.rs:19-24` (states), `:82-92` (`allowed_next`), `:64` (`is_terminal`), `:139` (`assert_transition`) | **VERIFIED — refund routing (§16.29) rides these existing transitions; the N-leg abort/void maps onto them** |
| Money type: `Money { minor: i64, currency: Currency }`, `Currency::Eur` present, `checked_add`/`checked_sub` fail-closed on cross-currency + i64 overflow, `checked_neg` = "the compensating credit of a debit (P07 reversal primitive)" | `kernel/src/money.rs:29` (`Currency`), `:33` (`Eur`), `:59` (`Money`), `:73` (`checked_add`), `:~92` (`checked_neg`) | **VERIFIED — EUR Wave-0 currency already exists; the refund/void primitive already exists** |
| Event log primitives: `sha3_256`, append-only `append`, `commit_after_decide` (decide-before-commit discipline) | `kernel/src/event_log.rs:30`, `:302`, `:366` | VERIFIED — idempotency-key derivation + the N-leg saga log reuse these |
| Port module registry convention: `pub mod payment;` / `pub mod payment_capability;` with a one-line doc each | `kernel/src/ports/mod.rs:14`, `:21`; ports registered `kernel/src/lib.rs:186` | VERIFIED — P60 adds `pub mod payment_provider;` here, alphabetical near `payment` |
| P47's earlier wave ordering: "Wave 1 = crypto (before processors)", "Wave 2 (last) = Stripe/Payoneer/Google/Apple Pay — OFFICIAL LIBRARIES ONLY … no reimplementation of processor-side payment cryptography or card-data handling"; `RailKind` addendum names `Crypto`/`Processor` | `BLUEPRINT-P47-P50-gap-closing-phases.md:84-95`, `:107-109`, D1 `:121-124` | **VERIFIED — reconciled in §2, NOT contradicted: §16.13's "online mandatory from Wave-0" ruling reorders the ONLINE-FIAT rail forward; P60 calls Stripe's REST API and reimplements NO card crypto, so P47's binding holds** |
| R2 research verdicts (PCI SAQ-A requires provider-rendered capture; three compliant paths; split is post-tokenization; MoR collision; adapter trait shape §5.2; `async-stripe` 1.0-alpha) | `docs/research/OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md` §2, §4, §5.2, §6, §7, §9 | VERIFIED read in full — P60 consumes its findings, does not re-research |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Research verdicts consumed (R2, condensed — cited, not re-derived) + the closed rulings

R2 is the research substrate; its findings are inputs here. The load-bearing ones, each already
reconciled against the operator rulings:

1. **PCI is the one absolute external constraint (R2 §2, §6.3).** Card data must be captured by
   the provider's own surface (hosted page / iframe / native SDK). The moment a PAN transits a
   surface dowiz draws, the hub falls into SAQ-D full scope and breaks §16.49. Drawing a card
   field on the wgpu canvas is ruled out **permanently** — enforced structurally, not by policy
   (§4.1). This is why P60's only hub→client payment type is an **opaque handoff**, never a card.

2. **Card capture per platform is now RULED (§4-A, closed).** **Path C — hosted redirect** on
   web + desktop (redirect to the provider's own verified domain; short-TTL single-use signed
   session token; **the server-side webhook is the ONLY source of truth for payment success** —
   a client-side "success" redirect is never trusted to write state). **Path B — native provider
   SDK sheet** on Tauri mobile (zero DOM), pending P63's bridge-feasibility spike, with Path C as
   the mobile fallback. Lightest PCI residual (6.4.3/11.6.1 shift to the provider's page, R2 §2),
   and frees desktop to be pure `winit`+`wgpu` (X3). P60 specs the hub→client handoff for both.

3. **Vendor-as-own-MoR is now RULED (§0.2-1, closed).** dowiz is **never** a party to the money;
   there is no platform-MoR entity, not dowiz and not a lead vendor. The food-court "one
   checkout" is one UX over **N vendor-scoped money legs**, each settling to that vendor's own
   provider account. This **supersedes both** of R2 §4.3's proposed models (A single-account /
   B zero-fee-platform) as *written*, and turns R2's "sharpest open policy question" into the
   **hardest correctness item in this blueprint: N-leg atomicity** — authorize all legs, capture
   only if all authorized, void on any single-leg failure (§4.5). Single-vendor is the degenerate
   `N = 1` case, which reduces to a plain two-phase auth→capture.

4. **Split is post-tokenization and money-law-clean (R2 §4.2).** Every leg's authorize/capture
   operates on a payment handle, never on card data — §16.46 and §16.49 are fully compatible. But
   under the vendor-as-own-MoR ruling the "split" is **not** one Connect charge fanned to N
   transfers (that needs a platform); it is **N independent authorizations** against N
   independent accounts. The cross-account payment-method-reuse mechanics are the sharpest
   *technical* residual (§2 risk, handed to P72's provider matrix + a named spike).

5. **Adapter trait shape (R2 §5.2) + provider reality (R2 §3).** A thin `PaymentProvider` port in
   hub-core, provider crates behind the compile firewall; `ClientHandoff` the only hub→client
   type; **no card-data type in core**; integer minor units end-to-end; idempotency keys on every
   mutating call. Stripe ships no official Rust SDK — community `async-stripe` (1.0-alpha,
   weekly-regenerated) vs a hand-rolled ~6-endpoint REST client (§4.3 DECART).

6. **Anti-abuse is purely mechanical (R2 §7, §16.53/§16.59).** Reuse the kernel `TokenBucket` for
   checkout-intent throttling; single-outstanding-intent cap; Cloudflare **Turnstile** (already
   in stack) or self-hosted **ALTCHA** at the edge/redirect layer — never in the canvas (same DOM
   tension as X3). **No reputation, no scoring, no cross-hub blocklist, ever** (mesh red-line).

---

## 2. Scope — what P60 owns vs deliberately does NOT

**P60 owns (build items §4):**

| Item | Content |
|---|---|
| M1 | `kernel/src/ports/payment_provider.rs`: the `PaymentProvider` port trait + value types (R2 §5.2 shape) + the **no-card-data compile firewall** extending `payment.rs:542`'s pattern |
| M2 | **The idempotency contract (X6 — P60 OWNS it):** `create_with_key` + `query_status_by_key`, the `IdemLedger` reconnect authority, and the documented per-provider normalization gap |
| M3 | Stripe Wave-0 **server** adapter (out-of-kernel `payment-adapters` crate): PaymentIntent create/capture/cancel, Checkout Session (Path C), refund — thin hand-rolled REST (DECART §4.3) |
| M4 | Webhook **verify + normalize**: provider signature check (HMAC + timestamp anti-replay) → provider-agnostic `PaymentEvent`; **webhook is the sole truth writer** |
| M5 | **N-leg atomicity (§0.2-1) — the hardest correctness item:** the auth-all-then-capture-else-void saga as a pure decide/fold Law; the money-atomicity invariant; typed partial-failure states |
| M6 | Refund routing (§16.29): `refund()` routes to the vendor's provider account; dowiz stays out; maps onto the existing `Refunding`/`CompensatedRefund` order states |
| M7 | Anti-abuse (X11): `TokenBucket` checkout-intent limiter + single-outstanding-intent cap + Turnstile edge token — all degrade-closed, no reputation |
| M8 | Client card-leg **spec** (§4-A): the `ClientHandoff` hub→client contract for Path C (web+desktop) and Path B (mobile, pending P63); canvas-never-renders-card as a type invariant |

**P60 explicitly does NOT own:**

- **NO card-data type, NO PAN, ANYWHERE in hub-core — hard PCI red-line, type-enforced (§4.1),
  not a preference.** A diff that introduces a `CardNumber`/`pan`/`cvv` field in `kernel/` or
  hub-core is a scope violation regardless of test state. The client tokenizes directly with the
  provider; the kernel never deserializes a card. (R2 §6.3.)
- **NOT the checkout UI / card moment rendering** — P69 (customer storefront & checkout) owns the
  wizard and invokes the Path-C redirect / Path-B sheet at the card moment. P60 supplies the
  `ClientHandoff`; P69 consumes it. **Consumer.**
- **NOT the offline-draft / data-wallet state machine** — P66 owns `Draft`/`PaymentInflight`,
  mints the idempotency key at draft creation, and does query-before-replay on reconnect. P66
  **consumes** P60's idempotency contract (§4.2), it does not redefine it (X6). **Consumer.**
- **NOT the food-court checkout UX or the per-provider matrix** — P72 owns the multi-vendor cart
  UX, the per-vendor provider-account onboarding step, and the market-scope-per-§4-D provider
  matrix. P72 **consumes** P60's N-leg atomicity mechanism (§4.5). The cross-account
  payment-method-reuse mechanics + the "how does one checkout authorize N independent accounts"
  spike belong to P72's provider matrix — P60 defines the *atomicity law*, P72 wires the
  *provider mechanics*. **Consumer.**
- **NOT the per-vendor charge-leg derivation source** — P62 owns the catalog leaf invariant (X7:
  every purchasable leaf carries price minor-units + currency + `vendor_id`). The `NLegPlan`
  legs are *derived from* `order_item.vendor_id` fan-out over that invariant. P62 is the
  **upstream** dependency; P60 consumes the invariant, never re-specifies it.
- **NOT the capability-cert chain** — P59 owns it. P60's server side is **ruling- and
  P59-independent** (synthesis §3.2 item 4): it reuses the *existing* `SignatureVerifier` /
  `verify_chain` primitives (`cap.rs`) for webhook-secret + provider-account authority, but needs
  no biscuit-style chain. Per-hub webhook secret is simple config baked by P67 provisioning.
- **NOT crypto or Google/Apple-Pay rails** — those remain the `PaymentRail::Crypto` /
  `GoogleApplePay` capability slots (P47 Wave-1 / the wallet rail). P60 lights up the **online
  fiat / Stripe** rail only. `payment_capability.rs`'s `validate()` is the gate; P60 does not
  touch that module.
- **NOT the cash-on-delivery settlement Law** — `kernel/src/ports/payment.rs` owns it,
  unchanged. P60 is **additive** (new module); it never edits `payment.rs`.
- **NOT any platform/aggregator money custody** — no `application_fee`, no dowiz Connect account,
  no funds ever routed through a dowiz-controlled account (§0.2-1, §16.16). A diff that makes
  dowiz a Connect platform is a **money red-line violation**.

**Reconciliation with P47 (honest, not silently reordered):** P47 §2.2 sequenced processors
"Wave 2, last." The newer SOVEREIGN §16.13 ruling — "online payment **mandatory from Wave-0**,
multi-provider via adapter" — consolidated in the CORE synthesis, places the online-fiat/Stripe
adapter in **Wave W1**. P60 implements §16.13. This reorders P47's "processors last" **for the
online-fiat rail specifically**; it does **not** violate P47's binding constraint, which forbids
*reimplementing processor-side payment cryptography or card-data handling* — P60 does neither
(the client tokenizes with the provider; the adapter calls Stripe's official REST API). Crypto
(P47 Wave-1) remains its own separate rail, unaffected.

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── kernel/src/ports/payment_provider.rs — NEW module ───────────────────────
//  Compile firewall (mirrors payment.rs:1-12): ZERO network/HTTP/JSON/serde/adapter.
//  Only the abstract contract + plain value structs. `cargo tree -p dowiz-kernel`
//  shows NO payment-adapter dependency. Red-proof: `firewall_self_source_is_clean`
//  (§4.1) + `no_card_data_type_in_core` (the no-PAN teeth).

/// Money is `crate::money::Money` (i64 minor units + Currency). NEVER f64. NEVER a
/// bare integer without a currency tag. Wave-0 currency = Currency::Eur (money.rs:33).
use crate::money::Money;

/// The idempotency key — minted at draft creation by P66, domain-separated SHA3
/// (event_log::sha3_256 of `b"dowiz.pay.idem\0" || order_id || wallet_id || nonce`),
/// so a key can never be replayed across orders/wallets. Typed, never a bare String.
pub struct IdempotencyKey(pub [u8; 32]);

/// The ONLY thing that ever crosses hub → client. Opaque by construction: it carries
/// provider handles/URLs/tokens, NEVER card data. No `pan`/`cvv`/`card_*` field exists
/// or may be added (the no-PAN firewall, §4.1).
pub enum ClientHandoff {
    /// Path C (web + desktop, §4-A): the provider's hosted page URL + a short-TTL,
    /// single-use signed session token. The client opens the provider's verified domain.
    HostedRedirect { checkout_url: String, session_token: [u8; 32], ttl_s: u32 },
    /// Path B (Tauri mobile, §4-A, pending P63): opaque native-SDK session blob the
    /// device SDK consumes to render the native card sheet. Still zero card data in core.
    NativeSdkSession { session_blob: Vec<u8> },
}
pub const CLIENT_SESSION_TTL_S: u32 = 900; // 15 min single-use redirect/session window

/// Normalized, provider-agnostic status. The webhook (M4) is the ONLY writer of the
/// Authorized/Captured/Voided/Refunded truth (§4.4); a client redirect is advisory-only.
pub enum PaymentStatus {
    NoneYet, IntentCreated, Authorized, Captured,
    Voided, Refunded, Failed(FailReason),
}
pub enum FailReason { Declined, Expired, ProviderError, Cancelled }

/// One vendor money-leg (§0.2-1). Derived from order_item.vendor_id fan-out over P62's
/// leaf invariant (X7). `dest` is the VENDOR'S OWN provider account — never dowiz's.
pub struct VendorLeg { pub leg: LegId, pub vendor_id: VendorId, pub amount: Money,
                       pub dest_account: ProviderAccountRef }
pub struct LegId(pub u32);
pub struct VendorId(pub [u8; 32]);
pub struct ProviderAccountRef(pub String); // opaque per-vendor account id (their MoR)

/// The N-leg plan for one checkout. Single-vendor = the degenerate N = 1.
pub struct NLegPlan { pub order_id: String, pub currency: Currency, pub legs: Vec<VendorLeg> }
pub const MAX_LEGS_PER_CHECKOUT: usize = 32; // food-court sanity cap; §5.2 scaling axis

/// Per-leg lifecycle. A leg is exactly one of these — a partial/mixed terminal that is
/// NOT one of {all Captured, all Voided/AuthFailed, explicit NeedsReconciliation} is
/// UNREPRESENTABLE (the money-atomicity invariant, §4.5/§5.1).
pub enum LegState { Draft, Authorized, AuthFailed(FailReason), Captured, Voided, CaptureStuck }

/// The whole-checkout outcome of the atomicity Law (M5).
pub enum NLegOutcome {
    /// All legs authorized → all captured. The only "money moved" terminal.
    Committed,
    /// A leg failed to authorize → every already-authorized leg voided. No money moved.
    Aborted { void_set: Vec<LegId> },
    /// Capture began but a leg is stuck (auth succeeded, capture not confirmed). NOT silent:
    /// a typed operator-visible state; dowiz does NOT auto-resolve (§16.29). Auth holds
    /// auto-expire provider-side (~7d Stripe) ⇒ a stuck leg self-heals toward Void, never
    /// toward a phantom charge.
    NeedsReconciliation { stuck: Vec<LegId>, captured: Vec<LegId> },
}

/// Event-sourced saga log (standard item 3: tests assert on these sequences, not end-state).
pub enum NLegEvent {
    PlanCreated { order_id: String, n_legs: u32 },
    LegAuthorized { leg: LegId }, LegAuthFailed { leg: LegId, reason: FailReason },
    AllLegsAuthorized { order_id: String },
    LegCaptured { leg: LegId }, LegVoided { leg: LegId }, LegCaptureStuck { leg: LegId },
    NLegCommitted { order_id: String }, NLegAborted { order_id: String },
    NLegNeedsReconciliation { order_id: String },
}

/// Refund routing (§16.29). Routes to the VENDOR'S provider account; dowiz stays out.
/// Maps onto the existing order states Refunding → CompensatedRefund (order_machine.rs:19-24).
pub struct RefundRequest { pub charge: ChargeHandle, pub amount: Money, pub reason: RefundReason }
pub enum RefundReason { CustomerRequest, VendorInitiated, DisputeResolution }
pub struct ChargeHandle(pub String); // opaque per-leg captured-charge id

/// Typed provider-boundary error. A provider failure is ALWAYS a value here — never a panic,
/// never a silent retry (bulkhead, §5.3). Mirrors payment.rs's SettlementReject discipline.
pub enum PayError { Idempotent { key: IdempotencyKey }, Declined, Expired,
                    SignatureInvalid, Unroutable, Provider(String), CurrencyMismatch }

/// THE port. Provider-agnostic. Core knows nothing provider-specific (R2 §5.2).
pub trait PaymentProvider {
    fn id(&self) -> &str;                     // stable rail id, e.g. "stripe:eu"
    /// Create an intent to be confirmed CLIENT-SIDE (Path C/B). Idempotent on `key`:
    /// replaying the SAME key returns the SAME handoff, never a second charge (X6).
    fn create_with_key(&self, key: &IdempotencyKey, plan: &NLegPlan)
        -> Result<ClientHandoff, PayError>;
    /// Reconnect-safe status query by idempotency key (X6). Where a provider has no true
    /// query-by-key endpoint, the adapter resolves via the hub-local IdemLedger (§4.2).
    fn query_status_by_key(&self, key: &IdempotencyKey) -> Result<PaymentStatus, PayError>;
    /// Verify a provider webhook signature and normalize to a hub event (M4). The ONLY
    /// source of truth for Authorized/Captured/Voided/Refunded.
    fn verify_webhook(&self, raw: &[u8], sig: &WebhookHeaders) -> Result<PaymentEvent, PayError>;
    /// Two-phase leg controls (M5). capture/void are idempotent + provider-side.
    fn capture_leg(&self, leg: &LegId, handle: &ChargeHandle) -> Result<(), PayError>;
    fn void_leg(&self, leg: &LegId, handle: &ChargeHandle) -> Result<(), PayError>;
    /// §16.29 — refund routes to the vendor's provider account. dowiz stays out.
    fn refund(&self, req: &RefundRequest) -> Result<(), PayError>;
}
pub struct WebhookHeaders { pub sig: Vec<u8>, pub ts: i64 }
pub struct PaymentEvent { pub key: IdempotencyKey, pub leg: Option<LegId>,
                          pub status: PaymentStatus, pub provider_event_id: String }
pub const WEBHOOK_TS_TOLERANCE_S: i64 = 300; // replay window; reject older (§4.4)

// ── kernel/src/ports/payment_provider.rs — anti-abuse (X11) ─────────────────
/// Checkout-intent limiter = the EXISTING kernel TokenBucket (token_bucket.rs:26),
/// keyed by (wallet_client_id, coarse_ip). Degrade-closed: try_acquire → false ⇒ refuse.
pub const CHECKOUT_BURST: f64 = 3.0;          // TokenBucket capacity
pub const CHECKOUT_REFILL_PER_SEC: f64 = 0.05; // ~1 intent / 20 s steady state
/// A wallet may have at most ONE open (uncaptured, unexpired) intent (R2 §7 item 4).
pub const MAX_OUTSTANDING_INTENTS_PER_WALLET: usize = 1;
```

Rejected alternatives (DECART one-liners): **a Connect single-charge + N transfers** — rejected:
makes someone the platform/MoR, violates §0.2-1; N independent authorizations is the only
vendor-as-own-MoR shape. **A `Money`-less bare `i64` amount** — rejected: `money::Money` already
carries the currency tag + fail-closed cross-currency arithmetic (`money.rs:73`); dropping it
reopens the mixed-units bug class. **A stringly-typed idempotency key** — rejected: a typed
`[u8; 32]` domain-separated hash makes cross-order replay unrepresentable (matches
`SETTLEMENT_IDEMPOTENCY_KEY`'s typed discipline). **Trusting the client success redirect** —
rejected: the webhook is the sole truth writer (§4.4); a client redirect can only trigger a
`query_status_by_key` re-check.

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 4.1 M1 — `PaymentProvider` port + the no-card-data compile firewall (the PCI red-line, structural)

New module `kernel/src/ports/payment_provider.rs` per §3 (types verbatim); register
`pub mod payment_provider;` in `kernel/src/ports/mod.rs` (alphabetical, after `payment`,
before `payment_capability`). Two structural guards, both **extending the proven
`payment.rs:542` self-source-scan pattern** (nothing new invented):

1. **`firewall_self_source_is_clean`** — the module `include_str!`s its own source and asserts it
   contains none of `FORBIDDEN_IMPORTS` (`reqwest`/`hyper`/`serde`/`tokio`/`sqlx`/`*_adapters::`,
   forbidden tokens assembled via `concat!` so the scan body never self-matches). The kernel does
   not link the adapter crate, so a stray `use payment_adapters::…` is *also* a hard compile
   error — the firewall is the missing link, made a test too.
2. **`no_card_data_type_in_core`** (the **no-PAN structural guarantee**, task-mandated) — scans
   the module (and, as a `kernel/tests/` integration test, the whole `kernel/src/` tree) for the
   forbidden card-data identifiers `card_number`, `cardnumber`, `pan`, `cvv`, `cvc`, `expiry`,
   `exp_month`, `exp_year`, `card_holder` (again `concat!`-assembled). **RED→GREEN teeth:** adding
   a `struct CardNumber` or a `pan:` field anywhere in the kernel makes this test fail the build.
   The guarantee is "by construction (the kernel never deserializes a card — there is no field to
   hold one) **plus** a CI-teeth scan that keeps it that way." This is the strongest form
   available short of a custom lint, and it is exactly how `payment.rs` and
   `payment_capability.rs:234` already enforce their red-lines.

**Adversarial (designed to break):** a test that *adds* a `pan: String` field to a fixture struct
inside the scanned tree and asserts the guard fires (the teeth are real, not decorative);
`ClientHandoff` round-trips a hosted-redirect + native-session handoff carrying only opaque
handles (no card field exists to populate). RED→GREEN: `port_trait_object_safe` (the trait can be
boxed behind a hub-config selector, R2 §5.2 "core never branches on provider").

### 4.2 M2 — the idempotency contract (X6 — P60 OWNS it) + the normalization gap

`create_with_key` + `query_status_by_key` (§3) are the **single contract** spanning payment,
drafts, and reconnect (X6 — P66 consumes, never redefines). The reconnect authority is a hub-local
append-only ledger:

```rust
/// Append-only key → handle → status map. The reconnect-safety authority (X6). Event-sourced
/// (reuses event_log discipline); demote-never-mutate (§5.5 living memory).
pub struct IdemLedger { /* HashMap<IdempotencyKey, (ProviderHandles, PaymentStatus)> + log */ }
impl IdemLedger {
    pub fn record_create(&mut self, key: &IdempotencyKey, h: ProviderHandles);
    pub fn resolve(&self, key: &IdempotencyKey) -> Option<(&ProviderHandles, PaymentStatus)>;
}
```

**The documented per-provider gap (X6, task-mandated honesty):** Stripe idempotency keys make
*create* idempotent (24 h window) but Stripe has **no first-class "get resource by idempotency
key" endpoint** — the query mechanism is *replaying the create with the key*, which returns the
original result. The adapter **normalizes** this: on first `create_with_key` it records
`key → provider-handle` in the `IdemLedger`; `query_status_by_key` then resolves the handle
locally and queries the provider *by handle*, giving a provider-independent reconnect guarantee.
Where a provider (e.g. a future Adyen adapter) exposes a native query-by-key, the adapter uses it
directly. **This normalization — or the gap — is stated per provider in the adapter's README; the
reconnect-safety guarantee is provider-independent only through the ledger.** (R2 §9 risk #7.)

RED→GREEN: `idempotent_create_no_double_charge` — two `create_with_key` calls with the SAME key
against a mock provider yield ONE intent + the SAME handoff; `reconnect_query_consistent` — after
a create, `query_status_by_key` returns the ledger-resolved status. **Adversarial:** same key,
*different* plan amount ⇒ typed `PayError::Idempotent` (a key is bound to its first plan — never
silently re-priced, mirroring `payment.rs`'s AmountMismatch discipline); a key never seen ⇒
`query_status_by_key → NoneYet` (not an error, not a fabricated success); ledger lookup after a
simulated hub restart (re-fold the log) returns the same status (reconnect survives restart).

### 4.3 M3 — Stripe Wave-0 server adapter (out-of-kernel, behind the firewall)

New crate `payment-adapters` (repo root — the crate `payment.rs:8` already names as future),
`stripe` module implementing `PaymentProvider`. **DECART: `async-stripe` vs hand-rolled REST.**

| Option | Pros | Cons | Verdict |
|---|---|---|---|
| **`async-stripe`** (arlyon, weekly-regen, 1.0-**alpha**) | ~full API incl. Connect | breaking churn pre-RC (R2 §9 #5); heavy dep tree | Named alternative if endpoint surface grows |
| **Hand-rolled `reqwest` REST, ~6 endpoints** | tiny/stable/no alpha churn; firewall-clean; audit-tiny | must track Stripe API version | **Wave-0 DEFAULT** |

The dowiz server surface is small: PaymentIntent `create`(+`capture_method=manual`), `capture`,
`cancel`(void); Checkout Session `create` (Path C hosted page); `refund` create; webhook verify
is *local* HMAC (no endpoint). **Six endpoints + one local verify** — a hand-rolled thin client is
smaller than the `async-stripe` dep and immune to its 1.0 churn, and it reimplements **no**
card-data/crypto (satisfying P47's binding, §2). Pin the Stripe API version explicitly.
RED→GREEN (adapter crate, mock/Stripe-test-mode): `create_intent_manual_capture` returns a
`HostedRedirect` handoff with a real Checkout URL; `capture`/`void`/`refund` hit the right
endpoints. **Adversarial:** provider 5xx ⇒ typed `PayError::Provider`, never a panic, never an
auto-retry-that-double-charges (idempotency key makes a retry safe *only* because M2 owns it);
declined card ⇒ `Failed(Declined)`; expired session ⇒ `Failed(Expired)`.

### 4.4 M4 — webhook verify + normalize (the sole source of truth)

`verify_webhook(raw, headers)` (§3): (1) recompute the provider signature — Stripe: HMAC-SHA256
over `"{ts}.{payload}"` with the per-hub endpoint secret; (2) reject if `|now - ts| >
WEBHOOK_TS_TOLERANCE_S` (replay window); (3) dedup by `provider_event_id` (idempotent fold — a
re-delivered webhook folds once); (4) normalize to `PaymentEvent`. **Hazard invariant (§5.1):**
the normalized `PaymentEvent` is the **only** writer of `Authorized`/`Captured`/`Voided`/
`Refunded` into the fold. A client-reported status can trigger a `query_status_by_key` re-check
but can **never** write capture truth. RED→GREEN: `webhook_valid_sig_normalizes` — a fixture
Stripe `payment_intent.amount_capturable_updated` → `Authorized`, `…succeeded` → `Captured`.
**Adversarial (the load-bearing test):** **a forged client "success" with NO webhook leaves the
fold in `Authorized`/`IntentCreated`, never `Captured`** — the client cannot self-certify payment
(§4-A ruling, R2 §6.4); a webhook with a bad signature ⇒ `SignatureInvalid`, no fold write; a
replayed webhook (ts outside tolerance, or duplicate event id) ⇒ dropped, fold unchanged
(bit-compare state).

### 4.5 M5 — N-leg vendor-as-MoR atomicity (§0.2-1 — the hardest correctness item)

The Law is a pure event-sourced saga, mirroring `payment.rs`'s `decide_settlement` /
`fold_event` shape exactly (reuse of doctrine, item 19). Two phases:

**Phase 1 — Authorize all.** For each `VendorLeg`, `create_with_key` + confirm with
`capture_method = manual` (an auth-only hold on the customer's method against **that vendor's own
account** — never dowiz's). Each leg lands `LegAuthorized` or `LegAuthFailed` (via M4 webhook,
the truth writer). **Decision:** `decide_capture(state) -> Capture | Void`:

- **All legs `Authorized`** ⇒ append `AllLegsAuthorized`, proceed to Phase 2 (capture all).
- **Any leg `AuthFailed`** ⇒ **void every already-`Authorized` leg** (`void_leg`), append
  `LegVoided`×k + `NLegAborted` → `NLegOutcome::Aborted`. **No money moved. No partial capture,
  ever.**

**Phase 2 — Capture all.** `capture_leg` each authorized leg. Capture is the low-risk step (auth
already gated the money). Residual honest failure: a capture that authorized but whose confirming
webhook does not arrive ⇒ the leg is `CaptureStuck` and the order enters the typed
`NLegOutcome::NeedsReconciliation { stuck, captured }` — **operator-visible, never silent, dowiz
does not auto-resolve** (§16.29: vendor + provider own disputes). Because unclaimed auth holds
expire provider-side (~7 d Stripe), a stuck leg **self-heals toward Void**, never toward a phantom
charge — this is the Self-Healing leg claimed precisely in §5.4.

**The money-atomicity invariant (the falsifiable N-leg test, task-mandated):** for any order, the
terminal is EXACTLY one of `{ all legs Captured (Committed) }`, `{ every leg Voided/AuthFailed
(Aborted) }`, or `{ explicit NeedsReconciliation }`. **A terminal where some legs are Captured and
others Voided *without* a `NeedsReconciliation` flag is UNREPRESENTABLE** — because the only
producer of `LegCaptured` for a plan is `decide_capture`'s all-authorized arm, and its only other
arm produces `LegVoided` for *every* authorized leg. RED→GREEN (property test, standard item 3):
generate arbitrary `N ∈ 1..=MAX_LEGS_PER_CHECKOUT` and an arbitrary per-leg auth outcome vector;
run the saga; **assert the invariant holds over every generated sequence** (proptest, 400+ cases,
mirroring `payment.rs:657`'s `b3_reconciliation` property test). **Adversarial (designed to
break):** (i) leg `k+1` auth-fails after legs `1..k` authorized ⇒ exactly legs `1..k` voided,
zero captured, `Aborted` — assert the event sequence, not just end-state; (ii) capture fails at
leg `j` after `1..j-1` captured ⇒ order in `NeedsReconciliation { stuck:[j..], captured:[1..j-1]}`
— **no leg silently left mixed**; (iii) `N = 1` reduces to plain auth→capture (single-vendor
degenerate case, same Law); (iv) cross-currency legs in one plan ⇒ typed `CurrencyMismatch`
before any authorize (reuses `money.rs:73`'s cross-currency guard — a food-court plan is
single-currency Wave-0, §4-D EUR); (v) a duplicate `LegAuthorized` webhook for an already-captured
leg ⇒ folded once, state unchanged.

> **Named technical residual (handed to P72, not hidden):** true vendor-as-own-MoR requires the
> customer's payment method to authorize against N *independent* accounts. Cross-account
> payment-method reuse is provider-constrained (Stripe scopes a PaymentMethod to a customer on one
> account). The atomicity **Law** above is provider-agnostic and complete; the **mechanics** of
> presenting one checkout that produces N independent authorizations (N redirect round-trips vs a
> single SDK session minting N intents vs per-account method cloning) is P72's provider-matrix
> spike (§4-D). P60 owns the correctness contract; P72 wires the provider reality.

### 4.6 M6 — refund routing (§16.29 — dowiz stays out)

`refund(RefundRequest)` (§3) routes to the **vendor's** provider account (the charge handle is
per-leg, already bound to that vendor's account) using the vendor's own key/scoped-restricted
key — dowiz holds no platform key and initiates no money movement of its own. The order-state
side is **already built**: a refund drives the existing `Confirmed/Preparing/Ready/InDelivery →
Refunding → CompensatedRefund` transitions (`order_machine.rs:82-92`); the money side reuses
`Money::checked_neg` (`money.rs:~92`, "the compensating credit of a debit — P07 reversal
primitive"). P60 adds **no** new refund state machine — it wires the provider call to the
existing states. RED→GREEN: `refund_drives_compensation_states` — a captured leg refunded folds
`Refunding → CompensatedRefund` and the ledger nets to zero (`is_terminal(CompensatedRefund)`).
**Adversarial:** a refund exceeding the captured amount ⇒ typed reject (never over-credits,
`checked_sub` guard); a refund on an un-captured (auth-only) leg ⇒ routed as a **void**, not a
refund (nothing to credit); a partial-refund on one leg of an N-leg order touches only that
vendor's account (per-leg isolation, §5.3).

### 4.7 M7 — anti-abuse (X11 — mechanical only, no reputation)

Three degrade-closed mechanisms, no new dep:

1. **Checkout-intent TokenBucket** — the **existing** `TokenBucket::new(CHECKOUT_BURST,
   CHECKOUT_REFILL_PER_SEC)` (`token_bucket.rs:34`), keyed by `(wallet_client_id, coarse_ip)`;
   `try_acquire(1.0)` before any `create_with_key`; `false` ⇒ **refuse the intent**
   (degrade-closed, exactly the bucket's contract `token_bucket.rs:74`). Coarse IP (e.g. /24) so
   NAT'd customers share a loose bound without tracking individuals.
2. **Single-outstanding-intent cap** — a fold-state predicate: a wallet with an open (uncaptured,
   unexpired) intent is refused a second (`MAX_OUTSTANDING_INTENTS_PER_WALLET = 1`); the prior
   must capture/expire/cancel first. Event-sourced, no external dep.
3. **Turnstile at the edge/redirect layer** — Cloudflare Turnstile (already in stack, synthesis
   X11) issues a token verified server-side *before* intent creation; **never embedded in the
   wgpu canvas** (same DOM tension as X3 — it lives at the CF edge or on the Path-C hosted-redirect
   page). Self-hosted **ALTCHA** is the named sovereign alternative if a CF dependency is rejected.

**Explicitly NOT built:** vendor scoring, customer reputation, cross-hub blocklists — barred by
§16.26/§16.59 and the mesh no-reputation red-line (R2 §7 item 5). RED→GREEN: `intent_burst_refused`
— the 4th intent in a burst (bucket = 3) is refused; `second_outstanding_intent_refused` — a
second open intent per wallet is refused until the first resolves. **Adversarial:** a bucket
`try_acquire` under a poisoned lock still degrades-closed (the bucket's poison-recovery,
`token_bucket.rs:74`); an expired outstanding intent frees the slot (a stuck intent can't
permanently lock a wallet out); Turnstile token missing/invalid ⇒ intent refused at the edge,
never reaching the kernel.

### 4.8 M8 — client card-leg spec (§4-A ruling, the hub→client contract)

P60 specs (does not render) the `ClientHandoff` contract P69 consumes at the card moment:

- **Web + desktop → Path C** `HostedRedirect { checkout_url, session_token, ttl_s =
  CLIENT_SESSION_TTL_S }`. The client opens the provider's **verified domain**; on return, the
  client calls `query_status_by_key` (M2) — it does **not** write success from the redirect
  (§4.4). Desktop stays pure `winit`+`wgpu` (X3).
- **Mobile (Tauri) → Path B** `NativeSdkSession { session_blob }` (zero DOM), **pending P63's
  bridge-feasibility spike**; **Path C is the mobile fallback** if the spike comes back empty.
- **The wgpu canvas NEVER renders a card field, on any platform** — enforced structurally by the
  no-card-data firewall (§4.1): there is no card type for the canvas to bind. Turnstile lives at
  the edge/redirect, never the canvas.

RED→GREEN: `handoff_web_is_hosted_redirect` / `handoff_mobile_is_native_session`;
`session_token_single_use` (a re-presented token is refused). **Adversarial:** a handoff whose
`ttl_s` has elapsed ⇒ refused (short-TTL single-use, R2 §6.2); a client that reports success
without a webhook (§4.4 forged-redirect test) leaves the order un-captured.

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6)

Reachability arguments, not prose. **No PAN can reach hub-core:** there is no card-data type
(§4.1) — the compile firewall makes "kernel holds a PAN" a *tested-unreachable* state (the
`no_card_data_type_in_core` scan), and the client tokenizes directly with the provider, so no
deserialization path exists. **No unilateral capture:** `Captured` truth has exactly one writer —
a signature-verified webhook (§4.4); a client redirect writes nothing, so "client forges a
payment" is structurally absent. **N-leg money atomicity:** the mixed-terminal state (some
Captured, some Voided, no reconciliation flag) is unrepresentable because `decide_capture`'s two
arms produce all-capture-or-all-void; the only escape is the *explicit* `NeedsReconciliation`
(§4.5). **No dowiz money custody:** every `dest_account` is a vendor `ProviderAccountRef`; no
dowiz account type exists in the plan — "dowiz becomes a party to the money" is unrepresentable
(§0.2-1). **Money integrity:** all amounts are `Money` (i64 minor units, `money.rs:59`) with
fail-closed cross-currency + overflow arithmetic; no f64 anywhere.

### 5.2 Schemas & scaling axes (item 8)

`NLegPlan`: axis = legs/checkout, bounded `MAX_LEGS_PER_CHECKOUT = 32` (a food-court cart);
break point — a mall with >32 co-located vendors needs plan chunking (a `flags` field reserves
it). `IdemLedger`: axis = open intents/hub; a per-venue hub sees O(active carts) keys —
kilobytes; break point — a garbage-collect-on-terminal sweep at ≥10⁵ resolved keys (resolved
keys demote to a cold log, §5.5). `TokenBucket`: O(1) per key, axis = distinct
`(wallet, coarse_ip)` pairs; no break point in sight (a bucket is two `f64`s). Webhook fold:
axis = events/order, tiny; the dedup set is per-order. Payment throughput is
settlements-per-second ≪ the event-log axis (`payment.rs` §2.7's stated relation) — no new axis.

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

**Isolation/bulkhead:** the provider adapter lives in the out-of-kernel `payment-adapters` crate
behind the compile firewall (`payment.rs`'s doctrine) — a provider outage/panic reaches the
kernel only as a typed `PayError` value, never a propagating failure (the same bulkhead
`bounded_drainer.rs`/`budget.rs` degrade-closed pattern). **Per-leg isolation:** one vendor leg's
failure voids/reconciles only its own account (§4.5/§4.6) — a food-court leg cannot corrupt a
sibling vendor's money. **Mesh awareness:** payment state is **hub-local, NOT gossiped** — intents,
webhooks, and the `IdemLedger` live on the venue's own hub; the webhook arrives at the hub's own
Cloudflare-tunnel endpoint; **no cross-hub payment state, no money over the mesh, ever**. The only
external network is the hub↔provider REST/webhook (out-of-kernel). **Living memory:** the
`IdemLedger` and N-leg saga are append-only event logs, content-addressed by idempotency
key / order id — demote-never-mutate (reuse `event_log` discipline); reconnect recall =
`query_status_by_key` over the ledger (the X6 reconnect path is a living-memory read).

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

**Self-Termination leg claimed:** every provider failure is a typed `PayError`/`FailReason`
refusal; the no-card-data and no-dowiz-custody unsafe states are structurally absent (§5.1) —
unrepresentable, not supervised. **Self-Healing leg claimed narrowly:** the N-leg abort (void all
authorized legs on any auth failure) is genuine compensating error-correction; and a
`CaptureStuck` leg self-heals toward Void via provider-side auth-hold expiry (§4.5) — claimed for
the money legs only, not for arbitrary state. **Snapshot-Re-entry: claimed** — the payment fold
is re-derivable from the append-only saga + `IdemLedger` logs after a hub restart (the
reconnect-survives-restart test, §4.2), i.e. recovery is a cheap re-fold from the last valid
epoch, not a bespoke recovery path. Mechanical rollback: the whole phase is additive (one new
kernel module, one out-of-kernel adapter crate, zero edits to `payment.rs`/`order_machine.rs`/
`money.rs`) — deletion restores today's tree.

### 5.5 Linux discipline (item 9) + tensor/spectral/eqc reuse (item 16)

Verdicts per the adoption framework: **ALREADY-EQUIVALENT** — one money authority (`money::Money`),
one idempotency contract (X6, P60-owned, P66 cites), one truth-writer (the webhook), one hub→client
type (`ClientHandoff`); **REINFORCES** — the compile-firewall self-scan doctrine extended from
`payment.rs`/`payment_capability.rs` to a third module (a stable pattern for red-line modules);
**EXTENDS** — the no-card-data structural scan as a **new gate class** (identifier-absence as a
CI-teeth invariant, beyond the existing import-absence scan); **GAP** honestly named — Stripe has
no query-by-key endpoint and no official Rust SDK; Wave-0 normalizes via the `IdemLedger` and a
hand-rolled REST client (both stated, §4.2/§4.3). Item 16: tensor/spectral/eqc machinery is
deliberately **NOT** decoratively invoked — payment is integer money-law + signature verification,
where a spectral form would be ritual math (Anu/Ananke discipline forbids exactly this). The one
honest reuse: idempotency-key derivation uses the existing `event_log::sha3_256` (`:30`), not a
new hash.

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no port module; `firewall_self_source_is_clean` / `no_card_data_type_in_core` absent | port compiles; both self-scans green; adding a `pan:` field anywhere in `kernel/` fails the build | **no-card-data-type scan** (ledger row) |
| M2 | no idempotency contract; double-charge on replay | same key ⇒ one charge; reconnect query consistent; survives restart re-fold; key-rebind refused | idempotency + reconnect tests (ledger row) |
| M3 | no adapter crate | Stripe test-mode create/capture/void/refund green; provider 5xx ⇒ typed `PayError` | adapter mock-server test |
| M4 | webhook verify absent; forged-success test RED by construction | valid sig normalizes; **forged client success ⇒ NOT captured**; bad sig / replay ⇒ dropped | **webhook-sole-truth** test (ledger row) |
| M5 | N-leg saga absent; atomicity invariant unasserted | property test: every generated N-leg sequence is Committed XOR Aborted XOR NeedsReconciliation — **no silent mixed terminal**; abort voids all; N=1 degenerate | **N-leg money-atomicity** property test (ledger row) |
| M6 | refund routing absent | refund drives `Refunding → CompensatedRefund`, nets to zero; over-refund refused; auth-only ⇒ void | refund-compensation test |
| M7 | no rate limit; intent spam unbounded | 4th burst intent refused; 2nd outstanding intent refused; Turnstile-missing refused at edge | anti-abuse degrade-closed test |
| M8 | no handoff contract | web ⇒ HostedRedirect, mobile ⇒ NativeSdkSession; single-use token; expired TTL refused | handoff-shape + single-use test |

**Not-done clauses:** any card-data type/field in `kernel/` or hub-core = **NOT done** regardless
of green totals (§2 hard red-line); a client redirect that writes `Captured` without a webhook =
**NOT done** (§4.4); an N-leg terminal that leaves legs mixed without `NeedsReconciliation` =
**NOT done** (§4.5); dowiz appearing as a Connect platform / holding an `application_fee` = **NOT
done** (§0.2-1 money red-line); a bare `i64` or `f64` amount without a `Currency` tag = **NOT
done**.

---

## 7. Benchmark plan (item 10) — kernel legs micro-benched; network out-of-kernel

Criterion harness (the `payment.rs`/`token_bucket.rs` bench discipline, reused): add
`payment/decide_capture_32legs` (the atomicity Law over the max plan — target < 50 µs, pure
integer + branch), `payment/idem_ledger_resolve_1e4` (reconnect lookup at 10⁴ open keys — target
< 5 µs), `payment/webhook_hmac_verify` (one signature check — target < 20 µs), and reuse the
existing `token_bucket` bench for the intent limiter (already proven). All added RED-commit-first
so baselines auto-seed; results to `BENCH_HISTORY.md`, never prose estimates. **Out-of-kernel
network** (adapter REST/webhook round-trips) is **not** micro-benched in the kernel — it is
covered by the `payment-adapters` integration test against Stripe test mode with a stated latency
budget (a hosted-redirect create is one REST round-trip; the customer-facing latency is the
provider's, not the hub's). Telemetry: intent-create/capture/void/refund counters + webhook
verify-fail counter ride the existing native-trackers hooks (P-H's lane), so a decline-rate or
verify-fail regression surfaces without review.

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the 20-point contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §0.2-1 (MoR), §4-A (Path C), §4-D (EU/EUR/Stripe/Adyen),
X3 (path↔shell), **X6 (idempotency — P60 owns)**, X7 (leaf invariant — P62 supplies), X11
(anti-abuse), §5 W1 P60 row · `docs/research/OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md` (read in
full — §2 PCI, §4 split+MoR, §5.2 adapter shape, §6 three paths, §7 anti-abuse, §9 risks) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.13/§16.16/§16.29/§16.46/§16.49/§16.53/
§16.59 (binding rulings) · `BLUEPRINT-P47-P50-gap-closing-phases.md` §2.2 (wave reconciliation,
§2) · `HERMETIC-ARCHITECTURE-PRINCIPLES.md` (§9) · `docs/regressions/REGRESSION-LEDGER.md` (five
rows named in §6). Kernel ground-truth cites all in §0. **Consumed by (downstream, per synthesis
§5):** **P66** (data wallet & offline drafts — consumes the idempotency contract §4.2, mints the
key at draft creation) · **P69** (customer storefront & checkout — consumes the `ClientHandoff`
card moment §4.8) · **P72** (food-court checkout — consumes the N-leg atomicity mechanism §4.5,
owns the cross-account provider matrix). **Upstream dependency:** **P62** (catalog leaf invariant
X7 — the leg-derivation source). Memory: `crypto-safe-first-pass-2026-07-14` (money/RLS red-lines
preserved under autonomy — the MoR + no-custody red-line honored) ·
`test-integrity-rules-2026-06-27` (money-RLS-PII red-lines; no-f64-money) ·
`rust-native-bare-metal-decision-2026-07-14` (DECART tables §4.3; hand-rolled-vs-`async-stripe`)
· `anu-ananke-strict-discipline-feedback-2026-07-17` (style; §5.5's refusal of decorative
spectral) · `verified-by-math-2026-07-07` · `never-bypass-human-gates-2026-06-29` (money red-line
escalation — none remaining: §4-A/§4-D closed). Supersedes: nothing (additive); reorders P47's
"processors last" for the online-fiat rail per §16.13 (§2 reconciliation).

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the `PaymentProvider` trait + value types (§3) precede the
  adapter; the money-integer law is the source, the provider is the derived shadow.
- **P2 CORRESPONDENCE** (one concept, one primitive): one idempotency contract spanning payment +
  drafts + reconnect (X6), one money authority (`money::Money`), one truth-writer (the webhook),
  one hub→client type (`ClientHandoff`), one atomicity Law reused for N=1 and N-vendor.
- **P4 POLARITY** (paired inverses as law): authorize↔void and capture↔refund are the two
  compensating pairs; the N-leg abort is polarity made a fold Law (§4.5), and refund reuses the
  `checked_neg` "compensating credit of a debit" primitive (§4.6).
- **P6 CAUSE-AND-EFFECT** (determinism as law): the webhook signature is the deterministic gate;
  idempotency keys make replay a no-op; the money-atomicity invariant carries a falsifier (the
  property test, §4.5/§6).
- **P7 GENDER** (paired verification, no self-certification): a claimed "captured" is refereed by
  the *independent* provider signature + the kernel fold — the client **never** self-certifies
  payment success (§4.4); the adapter's output is refereed by the kernel's typed port contract.

(P3/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; the `payment.rs` firewall doctrine; the P07 compensation states; the TokenBucket) |
| 2 DoD | §6 |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first; §4.5 event-sequence saga assertions (`NLegEvent`) |
| 4 predefined types/consts | §3 |
| 5 adversarial/breaking tests | §4.1–4.8 (pan-field-added teeth, key-rebind, forged-success webhook, leg-auth-fails-at-k+1, capture-stuck, cross-currency, burst-refuse, expired-TTL) |
| 6 hazard-safety as math | §5.1 (no-PAN, no-unilateral-capture, N-leg atomicity, no-dowiz-custody — all reachability) |
| 7 links docs/memory | §8 (P66/P69/P72 consumers, P62 upstream named) |
| 8 scaling axes | §5.2 (each with a named break point) |
| 9 Linux discipline | §5.5 (all four verdict classes incl. an honest GAP) |
| 10 benchmarks+telemetry | §7 (kernel legs benched; network out-of-kernel stated) |
| 11 isolation/bulkhead | §5.3 (adapter firewall + per-leg isolation) |
| 12 mesh awareness | §5.3 (hub-local, never gossiped; no money over mesh) |
| 13 rollback/self-heal vocabulary | §5.4 (Self-Termination + Self-Healing + Snapshot-Re-entry claimed precisely) |
| 14 error-propagation gates | §6 (ledger rows), §5.1 (typed `PayError`/`FailReason` refusal classes) |
| 15 living memory | §5.3 (append-only `IdemLedger`, content-addressed, demote-never-mutate) |
| 16 tensor/spectral + eqc reuse | §5.5 (spectral honestly NOT invoked; sha3_256 reused) |
| 17 regression ledger | §6 (five rows) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §0/§2/§4 (payment.rs firewall, TokenBucket, order states, Money, sha3_256, verify_chain all reused; DECART §4.3; three rejected alternatives §3) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Order below is the dependency order; T1–T5 are buildable today with zero network (the kernel legs
are pure decide/fold + local HMAC). Nothing waits on an operator gate — §4-A and §4-D are closed.

1. **T1 (M1 first — the port is the contract).** Create `kernel/src/ports/payment_provider.rs`
   per §3 (types + trait verbatim); register `pub mod payment_provider;` in
   `kernel/src/ports/mod.rs` (after `pub mod payment;` at `:14`). Write the RED tests first:
   `firewall_self_source_is_clean` (copy the `FORBIDDEN_IMPORTS` + `concat!` pattern from
   `payment.rs:508-560`) and `no_card_data_type_in_core` (new — scan for `pan`/`cvv`/`card_number`
   etc., `concat!`-assembled; add a `kernel/tests/no_card_data.rs` that scans the whole
   `kernel/src/` tree). Acceptance: `cargo test -p dowiz-kernel payment_provider` green; a
   deliberately-added `pan:` field makes `no_card_data_type_in_core` fail (prove the teeth).
2. **T2 (M2 — the idempotency contract, X6).** Implement `IdempotencyKey` derivation
   (`event_log::sha3_256`, domain-separated), the `IdemLedger`, `create_with_key` +
   `query_status_by_key` against a **mock** `PaymentProvider`. RED tests: no-double-charge on
   replay; reconnect-consistent; restart re-fold; key-rebind refused. Acceptance: `cargo test -p
   dowiz-kernel` idempotency tests green. **This is the contract P66 imports — freeze its shape
   here.**
3. **T3 (M5 — N-leg atomicity, the hardest item).** Implement the saga `decide_capture` +
   `fold` + the `NLegEvent` log in the port module. Write the proptest FIRST (arbitrary N +
   arbitrary per-leg auth vector ⇒ the money-atomicity invariant), then the impl (mirror
   `payment.rs:657`'s `b3_reconciliation` property-test structure). Adversarial fixtures per §4.5
   (i)–(v). Acceptance: property test green over 400+ cases; N=1 degenerate case green.
4. **T4 (M6 — refund routing).** Wire `refund()` to the existing `Refunding →
   CompensatedRefund` transitions (`order_machine.rs:82-92`) + `Money::checked_neg`. RED:
   `refund_drives_compensation_states`; over-refund refused; auth-only ⇒ void. Acceptance:
   `cargo test -p dowiz-kernel` refund tests green. **Do NOT add a new refund state machine.**
5. **T5 (M7 — anti-abuse).** Configure the existing `TokenBucket::new(CHECKOUT_BURST,
   CHECKOUT_REFILL_PER_SEC)` as the intent limiter + the single-outstanding-intent fold predicate.
   RED: `intent_burst_refused`, `second_outstanding_intent_refused`. Acceptance: green; degrade-
   closed under a poisoned bucket. **Reuse the bucket — do NOT add a rate-limit dep.**
6. **T6 (M3 + M4 — the out-of-kernel adapter).** New crate `payment-adapters` (repo root, path-dep
   on kernel, `reqwest` allowed HERE — it is OUTSIDE the firewall). Hand-rolled Stripe REST for the
   ~6 endpoints (§4.3) + local HMAC `verify_webhook` (§4.4). Pin the Stripe API version. RED
   (Stripe test mode / recorded fixtures): create/capture/void/refund; **the forged-client-success
   test (no webhook ⇒ not captured)**; bad-sig / replay dropped. Acceptance: adapter integration
   tests green; `cargo tree -p dowiz-kernel` shows NO `payment-adapters` dependency (the firewall
   holds).
7. **T7 (M8 — client handoff spec + wiring seam).** Confirm `ClientHandoff` variants for Path C
   (web+desktop) and Path B (mobile, leave a named `P63-bridge` TODO, not a silent gap); single-use
   token + TTL tests. Hand the contract to P69 (checkout) and P66 (drafts) by blueprint number —
   they cite §4.8/§4.2, they do not redefine. Add the five §6 ledger rows to
   `docs/regressions/REGRESSION-LEDGER.md`. Acceptance: handoff tests green; ledger rows present.
