# BLUEPRINT P72 — Food-court checkout (N-leg, vendor-as-MoR): one unified cart over N independent money legs, auth-all-then-capture, per-vendor refund & KDS (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Component:
> **DELIVERY / multi-vendor checkout**. Wave **W4** (multi-vendor + hardening) of the launch-blocker
> build sequence (`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5, row **P72**; build order §3.1 —
> "**→ M4 gate**: first food-court order"). Structural template + rigor precedent:
> `BLUEPRINT-P51-open-map-routing.md`; sibling W1/W2 rigor: `BLUEPRINT-P60-payment-adapter-core.md`,
> `BLUEPRINT-P62-catalog-multivendor-data-model.md`, `BLUEPRINT-P69-customer-storefront-checkout.md`.
>
> **This blueprint is pure composition, not new primitives.** Every hard mechanism it needs already
> exists as a completed W1/W2 contract: the **N-leg auth-then-capture atomicity Law** is P60's
> (`decide_capture`, `NLegPlan`, `NLegOutcome`); the **per-vendor charge-leg derivation** and the
> **`order_item.vendor_id` fan-out** are P62's (`charge_legs`, `kitchen_tickets`, the X7 leaf
> invariant); the **single-vendor checkout journey** it extends is P69's (`Journey`,
> `JourneyStep::{Cart,Payment,Suspended,Placed}`, `SuspendState`, `ReturnSignal`); the **per-vendor
> provider-account connect hook** is P67's (§2.2 claim/vendor-setup) surfaced in P70's owner UI.
> P72 wires them into the concrete food-court case (N > 1), owns the **cross-account authorization
> mechanic** P60 explicitly handed it (P60 §4.5 "named technical residual"), the **partial-failure
> UX**, the **per-vendor refund router**, the **per-vendor KDS routing**, the **payability gate**,
> and the **Wave-0 provider matrix**.
>
> **Operator rulings applied as inputs, NOT re-litigated** (both CLOSED per the task + synthesis):
> **(1) Merchant-of-record — each vendor is their OWN MoR** (§0.2-1): dowiz is *never* a party to the
> money; there is no platform-MoR entity, not dowiz and not a lead vendor; the food-court "one
> payment" is one checkout UX over **N vendor-scoped money legs**, each settling to that vendor's own
> provider account. **(2) Market scope (§4-D):** Eurozone/EU first, **EUR** currency, **Stripe
> Connect** (separate-charges style, zero platform fee) primary + **Adyen** named fallback, for this
> feature's Wave-0 proof only — per §16.20 the overall PLATFORM architecture stays market-agnostic;
> §4-D scopes only where the food-court FEATURE is proven first, never the platform.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads. The single most load-bearing finding:
**P72 introduces no new atomicity, money, or catalog primitive — it composes three completed
contracts and the landed kernel money/order authorities.** Two axes matter: what is **landed kernel
code** (immutable, reused verbatim) and what is an **on-paper W1/W2 contract** (P60/P62/P69 types
this document cites by section, which land before P72 builds, per SYNTHESIS §5 write-order).

| Claim | Fresh `file:line` / artifact (this pass) | Status |
|---|---|---|
| **Money authority LANDED** (immutable): `Money { minor: i64, currency: Currency }`, `Currency::Eur` present, `checked_add`/`checked_sub` fail-closed on cross-currency + i64 overflow, `checked_neg` = the compensating-credit reversal primitive | `kernel/src/money.rs:59` (struct), `:29`/`:33` (`Currency`/`Eur`), `:71` (`checked_add`), `:92` (`checked_neg`), `:105` (`checked_sub`) | **VERIFIED — EUR Wave-0 currency exists; every leg amount is this type; the void/refund reversal primitive exists** |
| **Order FSM LANDED** (immutable): `OrderStatus` with `Refunding` + `CompensatedRefund`; `allowed_next` admits `Refunding` from every post-`Confirmed` state; `Refunding → CompensatedRefund` (terminal, ledger nets to zero) | `kernel/src/order_machine.rs:8` (enum), `:21`/`:24` (states), `:78-92` (`allowed_next`) | **VERIFIED — per-leg refunds ride these existing transitions; the whole-order → CompensatedRefund only when ALL legs refunded (§4.4)** |
| **Cash settlement Law LANDED**: settlement is an event append, `decide_settlement` pure decide-before-commit, `SETTLEMENT_IDEMPOTENCY_KEY` typed, `AmountMismatch` never silently adjusts, `PaymentPort` trait | `kernel/src/ports/payment.rs:84`, `:114` (`AmountMismatch`), `:177`, the trait | VERIFIED — P60's online saga mirrors this shape; P72 executes P60's saga, touches `payment.rs` never |
| **`OrderItem` LANDED but vendor-blind** today: `product_id/modifier_ids/quantity/unit_price`, **no `vendor_id`** yet | `kernel/src/domain.rs:30` (struct), `place_order_priced` `:198` | **VERIFIED — the `vendor_id` fan-out axis is P62's additive change (P62 M4); P72 consumes the extended `OrderItem`, never adds the field itself** |
| **`catalog.rs` is currency+vendor-blind today**: only `PriceEntry`/`PriceCatalog` exist; **no `PriceableLeaf`/`VendorId`/`CatalogNode`/`charge_legs`** | `kernel/src/catalog.rs:21`/`:30` (existing); `PriceableLeaf`/`VendorId` absent this pass | **VERIFIED — these are P62's on-paper contracts (P62 §3); P72 cites them, does not define them** |
| **`payment_provider.rs` is NOT on disk** — the `PaymentProvider` port, `NLegPlan`, `VendorLeg`, `NLegOutcome`, `decide_capture` are **P60's on-paper W1 contract** | `ls kernel/src/ports/payment_provider.rs` → absent; `BLUEPRINT-P60-payment-adapter-core.md` §3/§4.5 | **VERIFIED — P60 lands before P72 (W1 → W4); P72 cites P60 §3/§4.5 by name, never re-specifies the saga** |
| **P60 owns the N-leg atomicity Law**: `decide_capture(state) -> Capture | Void`; all-authorized ⇒ capture all; any `AuthFailed` ⇒ void every authorized leg ⇒ `NLegOutcome::Aborted { void_set }`; capture-stuck ⇒ `NeedsReconciliation`; the money-atomicity invariant (terminal ∈ {Committed, Aborted, NeedsReconciliation}, mixed is unrepresentable); `MAX_LEGS_PER_CHECKOUT = 32` | `BLUEPRINT-P60-payment-adapter-core.md` §4.5, §3 (`NLegPlan`/`VendorLeg`/`NLegOutcome`/`LegState`), `:213` const | **VERIFIED — P72's food-court checkout is the concrete N > 1 instance of THIS Law; P72 owns the mechanic that feeds it, not the Law** |
| **P60 explicitly HANDED P72 the cross-account mechanic**: "true vendor-as-own-MoR requires the customer's payment method to authorize against N independent accounts … the mechanics of presenting one checkout that produces N independent authorizations … is P72's provider-matrix spike (§4-D). P60 owns the correctness contract; P72 wires the provider reality." | `BLUEPRINT-P60-payment-adapter-core.md` §4.5 (the boxed "Named technical residual") | **VERIFIED — §4.5 of THIS blueprint discharges exactly that residual** |
| **P60 owns refund routing**: `refund(RefundRequest { charge: ChargeHandle, amount, reason })` routes to the vendor's own account via the per-leg captured-charge handle; maps onto `Refunding → CompensatedRefund` | `BLUEPRINT-P60-payment-adapter-core.md` §3 (`RefundRequest`/`ChargeHandle`), §4.6 | **VERIFIED — P72's per-vendor refund router calls THIS per leg; it invents no refund state machine** |
| **P62 owns the leg derivation + KDS fan-out**: `charge_legs(order) -> Result<Vec<ChargeLeg>, String>` (`group_by(vendor_id)`, sum via `Money::checked_add`, deterministic `VendorId` order); `kitchen_tickets(order) -> BTreeMap<VendorId, Vec<&OrderItem>>`; `order_item.vendor_id` is **catalog-authoritative** in `place_order_priced` (a client cannot forge which vendor's leg an item belongs to) | `BLUEPRINT-P62-catalog-multivendor-data-model.md` §3 (`ChargeLeg`/`charge_legs`/`kitchen_tickets`), §4.4 | **VERIFIED — P72 maps `charge_legs` → P60's `NLegPlan` and routes `kitchen_tickets` to N KDS views; it re-derives neither** |
| **P62 owns the RLS inner filter**: outer `location_id` FORCE RLS deny-on-unset (RED-LINE, existing) + **inner `app.vendor_scope` opt-in narrowing** (NOT a second tenant boundary); a KDS/vendor connection sets `app.vendor_scope` → sees only its own rows | `BLUEPRINT-P62-catalog-multivendor-data-model.md` §3 (RLS predicate), §4.6 (M6) | **VERIFIED — per-vendor KDS isolation IS this inner filter; P72 adds no RLS boundary** |
| **P69 owns the single-vendor journey P72 extends**: `Journey`/`JourneyStep::{Cart,Payment,Suspended,Placed}`, `SuspendState`, `ReturnSignal`, the C.2 **Inciting** beat on `Captured`, `HubStatus`, the Path-C hosted-redirect card moment; P69's cart is ALREADY P62's cross-vendor `Cart`, and its M3 already shows the `charge_legs` **split preview** | `BLUEPRINT-P69-customer-storefront-checkout.md` §3 (types), §4.3 (M3 split preview), §4.5 (M5 suspend/resume) | **VERIFIED — P72 extends P69's Payment/Suspend for N > 1; the cross-vendor cart itself is already P69/P62's, not new** |
| **P67 exposes the per-vendor account-connect hook**: "§0.2-1's 'food-court vendor is payable only after connecting their own provider account' is a named step in the claim/vendor-setup flow … P67 exposes the hook; P60 fills it. No card data ever touches the claim service." | `BLUEPRINT-P67-hub-provisioning-claim.md` §2.2 (`:146-149`) | **VERIFIED — the onboarding STEP lives in P67's claim/vendor-setup flow + P70's owner UI; P72 consumes the resulting `ProviderAccountRef` and enforces the payability precondition** |
| **P70 is the owner surface where the vendor connects & manages**: G2 menu management over P62's `CatalogNode` tree; owner root `SelfSignedRoot`; the owner authors the catalog P69 renders and manages the hub | `BLUEPRINT-P70-owner-surface.md` §2 rows 6/8, G2 | **VERIFIED — the vendor's account-connect UI + catalog authoring live in P70; P72 consumes the connected-account state, renders no owner UI** |
| Merchant-of-record ruling (post-research, CLOSED): each vendor is own-MoR; "dowiz never becomes a party to the money"; supersedes R2 §4.3 models A/B **as written**; the hard new item is **N-leg atomicity** (P60/P72) | `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md:29-41` (§0.2-1) | VERIFIED — the binding money ruling; §2 anti-scope encodes "dowiz is never a payment party" as a red-line |
| Market-scope ruling (§4-D, CLOSED): food-court Wave-0 = which market(s) the FEATURE is proven in; "§16.20 requires the *architecture* to be market-agnostic — this decision only scopes where the food-court *feature* is proven first" | `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md:373-377` (§4-D) | **VERIFIED — §4.6 provider matrix is EUR/Stripe-Connect + Adyen-fallback as a FEATURE-scope limit, explicitly not a platform limit** |
| R2 split-payment + MoR research (read in full): Stripe Connect separate-charges names the restaurant-delivery split precedent; split is post-tokenization, money-law-clean; Mollie splits EUR/GBP-only; the MoR collision | `docs/research/OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md` §4, §8, risk #3/#4 | VERIFIED read in full — consumed, not re-researched |
| R5 food-court data-model research (read in full): one hub DB, `vendor_id`-row-scoping, ONE `order` + `order_item.vendor_id` fan-out, ONE shared `courier_pool`, per-item KDS routing, split settlement via provider transfers; risk #3 = split failure/refund semantics (this doc's §4.3/§4.4) | `docs/research/OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md` §3, §7 risk #3 | VERIFIED read in full — the food-court shape P72 wires |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Design verdicts — the load-bearing decisions, argued not asserted

### 1.1 P72 is composition: it EXECUTES P60's Law, it does not re-author it (standard item 19)

The single most important framing: **the correctness of a food-court checkout is entirely P60's
N-leg atomicity Law** (`decide_capture`, `NLegOutcome`, the money-atomicity invariant, P60 §4.5).
P72 does **not** re-specify authorize-all-then-capture, void-on-failure, or the reconciliation
terminal — those are proven once, in P60, over an *arbitrary* `N ∈ 1..=MAX_LEGS_PER_CHECKOUT`
(P60 §4.5's proptest generates arbitrary N). The food-court case is simply **N > 1**; the
single-vendor M1 order (P69) is **N = 1**, the degenerate pole of the *same* Law (P60 §5.1 P4
POLARITY). What P72 adds is everything *around* the Law that the concrete food-court case needs and
that P60/P62/P69 each deliberately deferred to it:

| P72 genuinely owns (new) | Cited contract it composes |
|---|---|
| The **plan-derivation glue** `charge_legs (P62) + per-vendor ProviderAccountRef (P67/P70) → NLegPlan (P60)` | P62 `charge_legs`; P60 `NLegPlan`/`VendorLeg` |
| The **cross-account authorization mechanic** (N independent auths against N vendor accounts) — the residual P60 §4.5 handed it | P60 §4.5 (residual), R2 §4 (Stripe Connect) |
| The **payability gate** (a vendor with no connected account is an unrepresentable leg destination) | P60 `ProviderAccountRef`; P67 §2.2 hook |
| The **partial-failure customer UX** (what the customer sees when leg 3 of 4 fails) | P60 `NLegOutcome::Aborted` |
| The **per-vendor refund router** (refund one vendor's items only, never one undifferentiated refund) | P60 `refund`; `order_machine` FSM |
| The **per-vendor KDS routing** (one order → N kitchen views, each vendor sees only its own) | P62 `kitchen_tickets`; P62 RLS `app.vendor_scope` |
| The **N > 1 extension of P69's Payment/Suspend journey** | P69 `Journey`/`SuspendState`/`ReturnSignal` |
| The **Wave-0 provider matrix** (EUR/Stripe-Connect + Adyen-fallback, feature-scope) | §4-D; R2 §8 |

### 1.2 "N independent authorizations against N independent accounts" is a RULED shape — the mechanic is the residual (§0.2-1 ↔ P60 §4.5)

§0.2-1 forbids a platform-MoR entity. P60 §1 verdict 4 already drew the consequence and ruled the
*shape*: "the split is **not** one Connect charge fanned to N transfers (that needs a platform); it
is **N independent authorizations** against N independent accounts." So P72 does **not** re-decide
whether to use destination-charges-with-transfers (R2 model B) — that was rejected by the ruling
because it makes dowiz (or a lead vendor) the platform-MoR. P72's job is the *mechanic* P60 handed
it (§4.5 boxed residual): **how does one customer card-entry produce N independent auth holds, each
against a different vendor's own account, when a payment method is provider-scoped to one account?**
That is a real technical question with a real Stripe answer (§4.5), and it is **the** thing P72
owns that nobody else could.

### 1.3 dowiz is a zero-fee technical facilitator, NOT the merchant-of-record — and that boundary is a type, not a policy (§0.2-1, the money red-line)

The delicate distinction P72 must nail precisely, because it is the money red-line: under Stripe
Connect a **platform account** exists as a technical integration root, but the **merchant-of-record
is the connected (vendor) account** for a *direct charge* — funds settle **directly into the
vendor's account**, dowiz's platform balance **never holds the money**, and
`application_fee_amount = 0` so dowiz takes **no cut** (§16.16). Stripe's "platform" role
(technical facilitator) is **not** the same as "merchant-of-record" (the money party). §0.2-1
forbids a platform-**MoR** entity; a zero-fee technical facilitator that never touches funds is
**not** an MoR. P72 encodes this as **type-level structure, not a promise** (§5.1): every
`VendorLeg.dest_account` is a *vendor* `ProviderAccountRef` (P60 §3); **there is no dowiz-account
type in the plan**, so "dowiz becomes a party to the money" is unrepresentable — the same
construction P60 §5.1 already proves. §4-D having closed "Stripe Connect as the primary matrix"
means this facilitator mechanic is the operator-accepted path; P72 states the boundary explicitly
so no implementer later reaches for a destination-charge that would silently route funds through a
dowiz balance (the money red-line, §6 not-done clause).

### 1.4 The unified cart is already cross-vendor; P72 extends the PAYMENT step, not the cart (§16.46 ↔ P62/P69)

A subtle reuse win: the "one unified cart across N vendors" (§16.46) is **already built** — P62's
`Cart` is cross-vendor by construction (P62 §0: "the unified cross-vendor cart is this same
`Cart`"), and P69's M3 already renders it and shows the `charge_legs` **split preview** (P69 §4.3).
So P72 does **not** build a multi-vendor cart from scratch; the cart, the split preview, and the
menu render are P69/P62's, working at N > 1 the same as N = 1. What genuinely differs at N > 1 is
**the payment moment**: P69's M5 handles a single `ClientHandoff` (N = 1); P72 extends it to the
**N-leg handoff** — N independent auth holds, P60's auth-all-then-capture saga, and the
partial-failure UX. P72's journey change is therefore a *specialization of `JourneyStep::Payment`
and `Suspended`* (P69 §3), not a new wizard. This is the falsifiable form of "cite P69's journey
and extend it, don't redesign the checkout."

### 1.5 Reuse-first: one small kernel glue module + one web extension, nothing forked (standard item 19)

P72 writes almost no new *mechanism*. New this pass: (a) `kernel/src/foodcourt.rs` — the pure
derivation glue (`charge_legs → NLegPlan`), the payability gate, and the per-vendor refund router
(consumes P60/P62 types, defines no money/atomicity primitive); (b) `web/src/storefront/foodcourt.mjs`
— the N > 1 extension of P69's payment step + the per-vendor status/partial-failure UX; (c) the
**cross-account auth mechanic** requirement spec'd for the out-of-kernel `payment-adapters` Stripe
crate (P60 M3's crate, extended here, behind P60's compile firewall). Everything else is cited.

Rejected alternatives (DECART one-liners): **one Connect charge + N transfers (destination charges,
R2 model B)** — rejected: makes dowiz/a lead vendor the platform-MoR, violates §0.2-1 (P60 §1
verdict 4 already ruled it out). **A new food-court atomicity saga** — rejected: forks P60's Law;
the food-court case is N > 1 of the *same* `decide_capture` (§1.1). **A new multi-vendor cart type**
— rejected: P62's `Cart` is already cross-vendor (§1.4). **A single undifferentiated refund across
the whole order** — rejected: each vendor is its own MoR with its own `ChargeHandle`; a refund must
route per-leg (§4.4, task-mandated). **A dowiz-held escrow/settlement balance** — rejected: dowiz is
never a money party; there is no dowiz-account type in the plan (§1.3, money red-line). **Silently
dropping a not-yet-connected vendor's items at checkout** — rejected: dishonest; the payability gate
refuses the checkout for that vendor with an honest status, never a silent drop (§4.2).

---

## 2. Scope — what P72 owns vs deliberately does NOT

### 2.1 P72 owns (build items §4)

| Item | Content |
|---|---|
| M1 | **`kernel/src/foodcourt.rs` — plan-derivation glue + payability gate**: `derive_nleg_plan(order, vendor_accounts) -> Result<NLegPlan, FoodCourtError>` mapping P62's `charge_legs` legs onto P60's `NLegPlan`/`VendorLeg` by attaching each vendor's connected `ProviderAccountRef`; a vendor with no connected account ⇒ `FoodCourtError::VendorNotPayable` (the payability gate, structural) |
| M2 | **The cross-account authorization mechanic (P60 §4.5 residual)**: spec the out-of-kernel Stripe-Connect adapter path that produces **N independent auth holds** (one direct manual-capture charge per vendor account) from **one** customer card-entry via a shared/cloned `PaymentMethod` under a **zero-fee technical facilitator** — dowiz balance never holds funds, `application_fee_amount = 0` (§1.3 money red-line) |
| M3 | **Partial-failure / void semantics + customer UX**: execute P60's `decide_capture` over the food-court plan; on any leg `AuthFailed`, void every already-authorized leg (P60 `NLegOutcome::Aborted { void_set }`); render the honest "no charge — some vendors couldn't accept payment" UX; **the falsifiable N-leg partial-failure test** (leg 3 of 4 fails ⇒ legs 1-2 voided) |
| M4 | **Per-vendor refund router (§16.29)**: `refund_vendor_leg(order, vendor_id, amount, reason) -> RefundRequest` routing to **only** that vendor's `ChargeHandle` via P60 `refund`; the whole-order → `CompensatedRefund` transition only when **all** legs refunded; **the falsifiable per-vendor-refund test** (refund vendor 2 ⇒ vendors 1 & 3 untouched) |
| M5 | **Per-vendor KDS routing**: `kds_route(order) -> BTreeMap<VendorId, KitchenTicket>` over P62's `kitchen_tickets` fan-out; each vendor's KDS connection reads only its own tickets via P62's inner `app.vendor_scope` RLS filter; one food-court order → N kitchen views, no item duplicated or dropped |
| M6 | **The N > 1 extension of P69's journey** (`web/src/storefront/foodcourt.mjs`): specialize `JourneyStep::{Payment,Suspended}` for the N-leg handoff; a per-vendor payment-status view (each leg Authorized/Captured/Voided); resume into P69's Inciting beat only when **all** legs `Captured` via the webhook (never client self-certified) |
| M7 | **The Wave-0 provider matrix** (§4-D): EUR / Stripe Connect primary + Adyen named fallback (design-in, stubbed not integrated); the explicit statement that this is a **feature-scope** limit, **not** a platform-architecture limit (§16.20) |

### 2.2 P72 explicitly does NOT own

- **NOT the N-leg atomicity Law.** `decide_capture`, the auth-all-then-capture two-phase saga, the
  `NLegOutcome::{Committed,Aborted,NeedsReconciliation}` money-atomicity invariant, and `LegState`
  are **P60** (§4.5). P72 *feeds* the saga an `NLegPlan` and *renders* its outcome; a diff that
  re-implements `decide_capture` in `foodcourt.rs` is a scope violation regardless of test state.
- **NOT the charge-leg derivation source or the KDS fan-out primitive.** `charge_legs`,
  `kitchen_tickets`, and the `order_item.vendor_id` catalog-authoritative fan-out are **P62**
  (§3/§4.4). P72 *maps* their output onto P60's plan and P58/P70's KDS view; it re-derives neither.
- **NOT the catalog / leaf invariant / the RLS mechanism.** The X7 `PriceableLeaf`, the free-form
  `CatalogNode` tree, and the `location_id`+`vendor_scope` RLS predicate are **P62** (§1.2/§4.6).
  P72 uses `app.vendor_scope` for KDS isolation; it adds no RLS boundary and forks no price authority.
- **NOT card data / PAN, on any platform — hard PCI red-line, type-enforced (P60 §4.1).** P72
  consumes only `ClientHandoff` (opaque handles/URLs). The wgpu canvas renders no card field because
  there is no card type to bind. A `card_*`/`pan`/`cvv` field anywhere is a scope violation.
- **NOT the single-vendor journey / M1.** The `/s/:slug` journey shell, menu render, cart, fields,
  Path-C suspend/resume, honest hub-offline, and bot pack are **P69**. P72 *extends* the Payment/
  Suspend steps for N > 1; it re-authors none of the other journey items.
- **NOT the vendor's account-connect UI or the claim/vendor-setup flow.** The **onboarding step**
  (a vendor connecting their own Stripe/Adyen account) lives in **P67**'s claim/vendor-setup hook
  (§2.2) and **P70**'s owner surface. P72 *consumes* the resulting `ProviderAccountRef` and enforces
  the payability precondition; it renders no owner UI and touches no claim service.
- **NOT dowiz becoming a party to the money — money red-line (§0.2-1).** No dowiz-held balance, no
  `application_fee`, no dowiz-controlled account in any settlement path. Every `dest_account` is a
  *vendor* `ProviderAccountRef`; there is no dowiz-account type in the plan (§5.1). A diff that makes
  dowiz the merchant-of-record, holds funds in a dowiz balance, or takes a transaction cut is a
  **money red-line violation** regardless of test state.
- **NOT courier dispatch, the map, or delivery.** One food-court order = **one delivery** over the
  **shared** hub-scoped `courier_pool` (§16.15/§16.46). Dispatch is **P65**, map **P51**, courier
  surface **P71**. P72 records the pool is hub-scoped (never vendor-scoped) and routes no courier.
- **NOT cross-currency food-court carts.** One order = one currency in Wave-0 (EUR, §4-D); a cart
  mixing vendors in different currencies is refused fail-closed via P62's `charge_legs`
  cross-currency guard (`CatalogError::CrossCurrency` / `Money::checked_add`), never silently
  converted — the §4-D market boundary, honestly surfaced (§4.1).
- **NOT tax computation.** Each vendor sets its own rate in the free-form schema (§16.49); dowiz
  calculates nothing tax-related. P72 carries per-vendor amounts as opaque, already-resolved `Money`.

### 2.3 Dependencies (standard §2 item 7 — named by exact contract)

**Hard upstream (must land first; all W1/W2, all present on paper this pass):**
- **P60** `BLUEPRINT-P60-payment-adapter-core.md` — the `PaymentProvider` port; `NLegPlan`,
  `VendorLeg { leg, vendor_id, amount, dest_account: ProviderAccountRef }`, `LegId`,
  `NLegOutcome::{Committed, Aborted { void_set }, NeedsReconciliation }`, `LegState`,
  `decide_capture` + the N-leg saga (§4.5), `capture_leg`/`void_leg`, `refund(RefundRequest)`,
  `ChargeHandle`, `RefundReason`, `ClientHandoff`, `IdempotencyKey`, `create_with_key`,
  `query_status_by_key`, `MAX_LEGS_PER_CHECKOUT = 32`, `CLIENT_SESSION_TTL_S = 900`, the
  webhook-sole-truth writer (§4.4), the no-card-data compile firewall (§4.1), the idempotency
  contract (X6). **The atomicity residual P72 discharges: P60 §4.5 boxed "named technical residual."**
- **P62** `BLUEPRINT-P62-catalog-multivendor-data-model.md` — `VendorId(u64)`; `PriceableLeaf`;
  `ChargeLeg { vendor_id, amount, line_count }`; `charge_legs(order) -> Result<Vec<ChargeLeg>, String>`;
  `kitchen_tickets(order) -> BTreeMap<VendorId, Vec<&OrderItem>>`; `order_item.vendor_id`
  catalog-authoritative in `place_order_priced` (§4.4); the inner `app.vendor_scope` RLS filter
  (§4.6); `CatalogError::{CrossVendor, CrossCurrency, Overflow}`.
- **P69** `BLUEPRINT-P69-customer-storefront-checkout.md` — `Journey`,
  `JourneyStep::{Cart, Payment, Suspended, Placed}`, `SuspendState { key, session_token,
  ttl_deadline_unix_s, await_since_unix_s, resume_step }`, `ReturnSignal::{DeepLink, Poll}`,
  `HubStatus`, the C.2 **Inciting** beat, the Path-C hosted-redirect card moment (§4.5), the
  `charge_legs` split preview (§4.3 M3). **P72 extends `JourneyStep::Payment`/`Suspended` for N > 1.**
- **P67** `BLUEPRINT-P67-hub-provisioning-claim.md` §2.2 — the **per-vendor account-connect hook**:
  "the food-court vendor is payable only after connecting their own provider account" is a named
  step in the claim/vendor-setup flow; P67 exposes the hook, P60 fills the payment mechanics,
  **no card data touches the claim service**. P72 consumes the connected `ProviderAccountRef`.
- **P70** `BLUEPRINT-P70-owner-surface.md` — the owner surface where the vendor **connects their
  account** and authors the catalog (G2 over P62's `CatalogNode` tree; owner `SelfSignedRoot`).
  P72 consumes the connected-account state; it renders no owner UI.

**Landed kernel (immutable, reused verbatim):** `kernel/src/money.rs` (`Money`/`Currency::Eur`/
`checked_*`), `kernel/src/order_machine.rs` (`Refunding`/`CompensatedRefund` + `allowed_next`),
`kernel/src/domain.rs` (`OrderItem`, `place_order_priced`), `kernel/src/ports/payment.rs`
(settlement-fold discipline, cited for shape).

**Consumed by / hands off to (downstream):** **P61** (fires per-vendor status notifications on the
food-court order), **P65/P51/P71** (the single shared-pool delivery beyond the pickup path), **P74**
(moderation reports, unrelated to money). **→ M4 gate:** P72 live = first food-court order.

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ════════════════════════════════════════════════════════════════════════════
//  P72 is a COMPOSITION surface. The types below are the DERIVATION GLUE, the
//  PAYABILITY GATE, and the PER-VENDOR REFUND ROUTER — the only genuinely new
//  kernel code (§1.5). Everything a leg/plan/saga/refund/KDS touches is a CITED
//  type from P60/P62/P69, NEVER redefined. Cited types are shown as `use` lines.
// ════════════════════════════════════════════════════════════════════════════

// ── cited, never redefined (single-owner contracts) ─────────────────────────
use crate::money::{Money, Currency};                       // kernel money.rs:59/:29 (i64 minor + Currency)
use crate::vendor::VendorId;                               // P62 (§3) — the intra-hub partition key
use crate::domain::{Order, OrderItem};                     // kernel domain.rs (OrderItem gains vendor_id per P62 M4)
use crate::catalog::{ChargeLeg, charge_legs, kitchen_tickets}; // P62 (§3) — the derivation + KDS fan-out
use crate::order_machine::OrderStatus;                     // kernel order_machine.rs:8 (Refunding/CompensatedRefund)
// P60 (kernel/src/ports/payment_provider.rs, on-paper W1 contract — cited, not redefined):
//   NLegPlan { order_id, currency: Currency, legs: Vec<VendorLeg> }
//   VendorLeg { leg: LegId, vendor_id: VendorId, amount: Money, dest_account: ProviderAccountRef }
//   LegId(u32), ProviderAccountRef(String), NLegOutcome::{Committed, Aborted{void_set}, NeedsReconciliation}
//   decide_capture(state) -> Capture | Void ; capture_leg / void_leg ; refund(RefundRequest)
//   RefundRequest { charge: ChargeHandle, amount: Money, reason: RefundReason }, ChargeHandle(String)
//   IdempotencyKey, create_with_key, query_status_by_key, ClientHandoff, PaymentStatus
//   MAX_LEGS_PER_CHECKOUT = 32, CLIENT_SESSION_TTL_S = 900
// P69 (web journey — cited): Journey, JourneyStep::{Cart,Payment,Suspended,Placed}, SuspendState, ReturnSignal

// ── kernel/src/foodcourt.rs — NEW module (the composition glue) ─────────────

/// A vendor's payment-account connection state, resolved from the P67/P70 onboarding step.
/// A vendor is a valid charge-leg destination ONLY when `Connected`. This is the type-level
/// form of §0.2-1's "a food-court vendor is payable only after connecting their own provider
/// account": a `NotConnected`/`Pending` vendor CANNOT produce a `VendorLeg`, so a checkout that
/// includes their items is structurally unpayable for that vendor (§5.1), never a silent drop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayabilityStatus {
    /// The vendor connected their OWN provider account (their MoR). Carries the vendor's opaque
    /// account ref — NEVER a dowiz account (§1.3 money red-line: no dowiz-account type exists).
    Connected(/* ProviderAccountRef */ String),
    /// Onboarding started but the provider has not confirmed the account is chargeable.
    Pending,
    /// No provider account connected. The vendor is not payable; their leg is unrepresentable.
    NotConnected,
}

/// Per-vendor account map for one hub, populated from the P67/P70 onboarding step. Deterministic
/// `VendorId` order (BTreeMap) so plan derivation and KDS routing are reproducible (P6 determinism).
pub type VendorAccounts = std::collections::BTreeMap<VendorId, PayabilityStatus>;

/// Typed food-court refusals — every failure names itself (never a partial plan / silent drop).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoodCourtError {
    /// A vendor in the cart has no `Connected` account (the payability gate, §4.2). Names the
    /// vendor so the UX can say "vendor X isn't accepting payment yet" — honest, not a drop.
    VendorNotPayable(VendorId),
    /// The cart mixes vendors in different currencies (Wave-0 = one currency, §4-D). Fail-closed
    /// via P62's `charge_legs` cross-currency guard; never a silent conversion.
    CrossCurrencyCart,
    /// More vendors than P60's `MAX_LEGS_PER_CHECKOUT`. Bounded refusal, never an unbounded plan.
    TooManyVendors { got: usize, max: usize },
    /// P62's `charge_legs` returned an error (overflow / cross-vendor / cross-currency). Wrapped
    /// verbatim, never re-derived.
    LegDerivation(String),
}

/// THE food-court derivation glue (M1). Map P62's per-vendor `charge_legs` onto P60's `NLegPlan`
/// by attaching each vendor's connected `ProviderAccountRef`. This is the ONLY new composition:
/// P62 says HOW MUCH each vendor is owed; P67/P70 say WHICH ACCOUNT is theirs; P60 EXECUTES the
/// resulting plan. N=1 (single-vendor) yields a one-leg plan by the SAME code path as N=30 — there
/// is NO food-court branch (a group-by of a 1-key input is a 1-entry map; §1.1 P4 POLARITY).
///
/// Fail-closed: any vendor not `Connected` ⇒ `VendorNotPayable` BEFORE any authorization (the
/// payability gate); cross-currency ⇒ `CrossCurrencyCart`; > MAX_LEGS ⇒ `TooManyVendors`.
pub fn derive_nleg_plan(order: &Order, accounts: &VendorAccounts)
    -> Result</* NLegPlan */ (), FoodCourtError>;   // returns P60::NLegPlan; unit shown to keep the port boundary explicit

/// Per-vendor KDS routing (M5). Thin wrapper over P62's `kitchen_tickets`: one food-court order
/// fans to N kitchen views, each keyed by `VendorId`. A vendor's KDS connection reads only its own
/// via P62's inner `app.vendor_scope` RLS filter (§4.5); this fn is the PURE derivation the view
/// consumes. Σ(ticket line-counts) == order item count — no line duplicated or dropped (§4.5 test).
pub fn kds_route(order: &Order)
    -> std::collections::BTreeMap<VendorId, /* KitchenTicket */ Vec<()>>; // wraps kitchen_tickets

/// Per-vendor refund router (M4, §16.29). Route a refund to EXACTLY one vendor's captured charge —
/// never one undifferentiated refund across the order. `charge` is that vendor's own per-leg
/// `ChargeHandle` (bound to the vendor's account at capture, P60 §3), so the money reversal lands
/// ONLY in that vendor's account. The whole-order `OrderStatus` moves to `CompensatedRefund` ONLY
/// when EVERY leg is fully refunded (§4.4); a single-vendor partial refund leaves the order
/// non-terminal (per-leg reversal, not a whole-order compensation).
pub fn refund_vendor_leg(order: &Order, vendor_id: VendorId, amount: Money,
                         reason: /* RefundReason */ ())
    -> Result</* RefundRequest */ (), FoodCourtError>;  // returns P60::RefundRequest for that vendor's ChargeHandle

/// Does refunding this leg complete the whole order's refund? True ⇒ the caller may drive
/// `Refunding → CompensatedRefund` (order_machine.rs:91); False ⇒ the order stays non-terminal
/// (a partial, per-vendor refund). Pure predicate over the per-leg refund ledger.
pub fn all_legs_refunded(order: &Order, refunded: &[VendorId]) -> bool;
```

```
// ── kernel/src/foodcourt.rs — constants ──────────────────────────────────────
// (reuse P60's cap — do NOT introduce a second food-court cap; §1.1 one authority)
pub use crate::ports::payment_provider::MAX_LEGS_PER_CHECKOUT;   // = 32 (P60 §3)
```

```
// ── payment-adapters (out-of-kernel Stripe crate, behind P60's firewall) ─────
//  The cross-account auth MECHANIC (M2). Spec, not kernel code — reqwest lives HERE.
//  Requirement: produce N INDEPENDENT manual-capture auth holds from ONE customer card-entry,
//  each a DIRECT charge on a DIFFERENT vendor's connected account (the vendor is MoR), via a
//  PaymentMethod shared/cloned from the zero-fee technical facilitator to each connected account.
//  Invariants the adapter MUST satisfy (asserted by P60's firewall + this doc's §6 not-done):
//   - application_fee_amount == 0 on every leg (dowiz takes no cut; §16.16)
//   - funds settle DIRECTLY to the vendor connected account (dowiz balance never holds them; §1.3)
//   - each leg is a manual-capture (auth-only) PaymentIntent → feeds P60's auth-all-then-capture
//   - no card data crosses the kernel (P60 §4.1); the client tokenizes on the provider's hosted page
```

Rejected alternatives (DECART one-liners): **a `FoodCourtPlan` type distinct from P60's `NLegPlan`**
— rejected: forks the plan authority; the food-court plan IS `NLegPlan` at N > 1 (§1.1). **A
`DowizFacilitatorAccount` in the plan** — rejected: no dowiz-account type may exist in a settlement
path (§1.3 money red-line); the plan holds only vendor `ProviderAccountRef`s. **A second
`MAX_LEGS_FOODCOURT` cap** — rejected: reuse P60's `MAX_LEGS_PER_CHECKOUT = 32` (one authority).
**A whole-order refund helper that always moves to `CompensatedRefund`** — rejected: a per-vendor
partial refund must NOT terminate the order (§4.4); `all_legs_refunded` gates the transition.
**Silently omitting a `NotConnected` vendor's items** — rejected: `VendorNotPayable` is an honest
typed refusal, not a drop (§4.2).

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

Dependency order: M1 → M2 → M3 → M4 → M5 → M6 → M7. M1/M4/M5 (pure kernel glue over P62/P60 types)
are buildable the moment P60+P62 land, with zero network. M2/M3/M6 add the out-of-kernel adapter
mechanic + the web UX; M7 is the provider-matrix record.

### 4.1 M1 — plan-derivation glue + payability gate (`kernel/src/foodcourt.rs`)

New module `kernel/src/foodcourt.rs` per §3; register `pub mod foodcourt;` in `kernel/src/lib.rs`
(alphabetical, near `format_money`/`geo`). `derive_nleg_plan(order, accounts)`: (1) call P62's
`charge_legs(order)` → `Vec<ChargeLeg>` (fail-closed cross-currency/overflow, wrapped as
`LegDerivation`/`CrossCurrencyCart`); (2) for each `ChargeLeg`, look up the vendor's
`PayabilityStatus` — `Connected(acct)` ⇒ build a P60 `VendorLeg { leg, vendor_id, amount,
dest_account: acct }`; **anything else ⇒ `VendorNotPayable(vendor_id)`, no plan produced** (the
payability gate); (3) refuse `> MAX_LEGS_PER_CHECKOUT` with `TooManyVendors`; (4) return P60's
`NLegPlan`. **The vendor→leg mapping preserves P62's deterministic `VendorId` order.**

RED→GREEN (pure kernel, zero network): `derive_plan_three_vendors_all_connected` — a 3-vendor order
with all `Connected` yields a 3-leg `NLegPlan`, leg amounts summing to the order subtotal, in
ascending `VendorId` order; `single_vendor_is_one_leg` — an N=1 order yields a one-leg plan by the
same code path (no food-court branch). **Adversarial (designed to break):** (i) one vendor
`NotConnected` ⇒ `VendorNotPayable(that_id)`, **no partial plan** (the payability gate refuses the
whole checkout for that vendor, never a silent drop — the task's onboarding-step consequence made a
test); (ii) a cart mixing EUR + a second currency ⇒ `CrossCurrencyCart` (P62's cross-currency guard,
never a silent conversion — the §4-D boundary); (iii) 33 vendors ⇒ `TooManyVendors { got: 33,
max: 32 }`; (iv) a `Pending` vendor ⇒ `VendorNotPayable` (pending ≠ payable — an unconfirmed account
cannot be a leg destination); (v) **no dowiz-account ref can appear as a `dest_account`** — asserted
structurally: `PayabilityStatus::Connected` carries only the vendor's ref and there is no
constructor path from a dowiz identity to a `dest_account` (the money red-line, §5.1).

### 4.2 M2 — the cross-account authorization mechanic (P60 §4.5 residual, out-of-kernel)

**This discharges the residual P60 §4.5 boxed and handed to P72.** The requirement (spec, not kernel
code — it lives in the `payment-adapters` Stripe crate behind P60's firewall): from **one** customer
card-entry, produce **N independent manual-capture auth holds**, each a **direct charge on a
different vendor's connected account** (the vendor is MoR). The Wave-0 Stripe mechanic (DECART below):

| Mechanic | How | Verdict |
|---|---|---|
| **N direct charges via shared PaymentMethod** (Wave-0 DEFAULT) | Customer tokenizes once on the Path-C hosted page → a `PaymentMethod` on the zero-fee **technical facilitator**; the facilitator **shares/clones** it to each connected vendor account; N manual-capture `PaymentIntent`s, one per vendor account (`Stripe-Account` = vendor), `application_fee_amount = 0`, funds settle **directly** to each vendor | **DEFAULT** — true "N independent auths against N independent accounts" (P60 §1 v4); dowiz balance never holds funds (§1.3) |
| One platform charge + N transfers (destination charges) | One charge on a platform balance, N `Transfer`s to vendors | **REJECTED** — needs platform-MoR + funds transit a platform balance; violates §0.2-1 (P60 §1 v4) |
| N hosted-redirect round-trips (one Checkout Session per vendor) | Customer pays N times | **REJECTED** — dishonest UX; a "unified cart" that charges N times is not one checkout (§16.46) |

The N auth holds feed **P60's `decide_capture` saga unchanged** (auth-all-then-capture-else-void).
**The mechanic changes nothing about the Law** — it only produces the N `LegAuthorized`/`LegAuthFailed`
inputs P60 already handles. **Money red-line invariants the adapter MUST satisfy** (§3 spec, checked
by §6 not-done + P60's firewall): `application_fee_amount == 0` per leg; funds settle directly to the
vendor account; no dowiz balance ever holds the money; the client tokenizes on the provider's hosted
domain (no PAN in the kernel, P60 §4.1).

RED→GREEN (adapter integration test, Stripe **test mode**, `#[ignore]`-gated on the crate like P60 M3):
`n_direct_charges_zero_fee` — a 3-vendor plan produces 3 manual-capture direct charges, each on a
distinct connected test account, each `application_fee_amount == 0`; `funds_never_touch_facilitator`
— assert the facilitator balance delta is zero (funds land on the connected accounts). **Adversarial:**
a leg whose `application_fee_amount != 0` ⇒ the adapter test fails (the money red-line teeth — dowiz
must never take a cut); a mechanic that routes through a platform balance ⇒ fails
`funds_never_touch_facilitator`; a PaymentMethod that cannot clone to a connected account ⇒ that leg
`AuthFailed` (feeds P60's abort arm, never a silent success).

### 4.3 M3 — partial-failure / void semantics + customer UX (the task-mandated N-leg test)

**The scenario the task names:** "if leg 3 of 4 fails to authorize, what happens to legs 1-2 (already
authorized)?" The answer is **entirely P60's `decide_capture` Void arm** (P60 §4.5) — P72 executes it
over the food-court plan and renders the outcome honestly. Flow: Phase 1 authorizes all N legs (M2's
N direct charges); if **any** leg lands `LegAuthFailed`, P60's decide arm **voids every
already-`Authorized` leg** (`void_leg` on each vendor's own account) → `NLegOutcome::Aborted {
void_set }`. **No money moves. No partial capture, ever.** P72's UX renders the honest customer
message ("we couldn't complete your order — no payment was taken; vendor X isn't accepting payment
right now") and returns the journey to the cart/payment step (P69 `resume_step`); the customer is
**never** charged for the vendors that did authorize.

**The falsifiable N-leg partial-failure test (`foodcourt_leg3_fails_voids_legs_1_2`, task-mandated):**
a 4-vendor order; legs 1 and 2 authorize; leg 3 `AuthFailed`; **assert:** exactly legs 1 and 2 are
voided (each `void_leg` called on that vendor's own account), leg 4 is **never authorized** (Phase 1
short-circuits to abort on the first failure per P60's Law), **zero legs captured**, outcome
`Aborted { void_set: [1, 2] }`, and the customer-facing state is the honest no-charge UX — asserted on
the **event sequence** (`LegAuthorized{1}`, `LegAuthorized{2}`, `LegAuthFailed{3}`, `LegVoided{1}`,
`LegVoided{2}`, `NLegAborted`), not just end-state (standard item 3). **Adversarial:** (i) leg 1
fails first ⇒ nothing was authorized yet ⇒ `void_set` empty, `Aborted`, zero charges (early-abort
degenerate); (ii) the LAST leg (N) fails after 1..N-1 authorized ⇒ all N-1 voided, zero captured
(the maximal void set); (iii) a duplicate `LegAuthFailed` webhook for leg 3 ⇒ folded once, void set
unchanged (idempotent fold, P60); (iv) a leg that authorizes but whose confirming webhook never
arrives during capture ⇒ P60's `NeedsReconciliation` (operator-visible, self-heals toward Void via
auth-hold expiry, P60 §4.5) — **the food-court order is never left silently half-charged**.

### 4.4 M4 — per-vendor refund router (the task-mandated per-vendor-refund test, §16.29)

**The scenario the task names:** "if a customer wants a partial refund covering only one vendor's
items, how does this route through P60's refund mechanism per-leg, not as a single undifferentiated
refund?" Because each vendor is its own MoR, **each captured leg has its own `ChargeHandle` bound to
that vendor's account** (P60 §3). `refund_vendor_leg(order, vendor_id, amount, reason)` builds a P60
`RefundRequest { charge: <that vendor's ChargeHandle>, amount, reason }` and P60's `refund` routes the
money reversal to **only that vendor's account** — never a single refund across the whole order. The
order-state side: a per-leg refund folds through the existing P07 ledger (`checked_neg`, the
compensating credit) **per leg**; the whole-order `OrderStatus` moves to `Refunding →
CompensatedRefund` (order_machine.rs:91) **only when `all_legs_refunded` is true**. A single-vendor
partial refund is a **per-leg reversal that leaves the order non-terminal** (still `Confirmed`/
`Delivered` for the other vendors) — the honest form of "refund one vendor, not the order."

**The falsifiable per-vendor-refund test (`foodcourt_refund_one_vendor_leaves_others_untouched`,
task-mandated):** a 3-vendor order, all legs captured; refund vendor 2's items; **assert:** the
`RefundRequest.charge` equals vendor 2's `ChargeHandle` (not vendor 1's or 3's), vendors 1 and 3's
`ChargeHandle`s are **never touched** (no reversal on their accounts), vendor 2's per-leg ledger nets
to zero for the refunded amount, and the whole-order `OrderStatus` **does NOT** reach
`CompensatedRefund` (only one of three legs refunded ⇒ `all_legs_refunded == false`). **Adversarial:**
(i) refund ALL three vendors ⇒ `all_legs_refunded == true` ⇒ the order legitimately reaches
`CompensatedRefund`, whole-order ledger nets to zero; (ii) a refund exceeding vendor 2's captured
amount ⇒ typed reject (never over-credits, P60's `checked_sub` guard); (iii) a refund on a vendor
whose leg was only **auth-only** (never captured — e.g. an Aborted order) ⇒ routed as a **void**, not
a refund (nothing to credit, P60 §4.6); (iv) a refund naming a `VendorId` **not in the order** ⇒
`FoodCourtError` (never fabricates a leg); (v) two concurrent refunds on the same vendor leg ⇒
idempotent via P60's refund idempotency (never double-credits).

### 4.5 M5 — per-vendor KDS routing (§16.15/§16.46 — one order, N kitchens, each sees only its own)

`kds_route(order)` is a thin wrapper over P62's `kitchen_tickets(order)` (`group_by(vendor_id)`): one
food-court order fans to **N kitchen views**, each keyed by `VendorId`. The isolation is **P62's inner
`app.vendor_scope` RLS filter** (P62 §4.6): each vendor's KDS connection **SETs `app.vendor_scope` =
its own `vendor_id`** → reads only its own `order_items` rows (the GoTab "tenants see only their own
KDS tickets" pattern, R5 §3.2); a hub-wide/owner connection leaves it unset → sees all (the unified
view). **P72 adds no RLS boundary** — it uses P62's mechanism verbatim. Routing is by
`order_item.vendor_id`, which is **catalog-authoritative** (P62 §4.4) — a client cannot misroute an
item to another vendor's kitchen (it cannot forge the `vendor_id`).

RED→GREEN (pure kernel over P62's `kitchen_tickets`): `kds_routes_each_item_to_its_vendor` — a
3-vendor order routes item *i* to exactly `vendor(i)`'s ticket; `kds_no_item_lost_or_duplicated` —
Σ(ticket line-counts across all vendors) == order item count (nothing dropped, nothing double-routed).
**Adversarial:** (i) two items same vendor ⇒ one ticket with both lines (not two tickets); (ii) an
attempt to route via a client-supplied `vendor_id` ⇒ impossible: `vendor_id` is re-derived from the
trusted catalog in `place_order_priced` (P62 §4.4), so the KDS routing keys off catalog truth, not the
request; (iii) **vendor A's KDS connection with `app.vendor_scope = A` cannot read vendor B's ticket**
— a DB-gated test (P62's `#[ignore]` RLS pattern) asserting the inner filter narrows; (iv) an
unset-scope owner connection sees all N tickets (the unified hub-wide view) — the same predicate, no
special case.

### 4.6 M6 — the N > 1 extension of P69's payment journey (`web/src/storefront/foodcourt.mjs`)

Extend P69's `JourneyStep::Payment`/`Suspended` (P69 §3) for the N-leg case — **specialize, do not
rewrite** the journey (P69 §2.2 charter). At Payment: derive the plan via M1 (`derive_nleg_plan`);
mint/create via P60 `create_with_key(key, plan)` (the key was minted by P66 at draft creation, X6);
suspend into P69's `SuspendState` exactly as N=1 does (the same Path-C hosted redirect, single
customer card-entry — M2's mechanic produces the N legs behind the one entry). On return
(`ReturnSignal::DeepLink` or poll `query_status_by_key`, P69 §4.5), resume into P69's **Inciting**
beat **only when ALL legs are `Captured`** (the webhook is the sole truth writer, P60 §4.4 — a client
redirect never self-certifies). A **per-vendor payment-status view** shows each leg
Authorized/Captured/Voided during the brief saga window; on `Aborted` (M3) it renders the honest
no-charge message and returns to the cart.

RED→GREEN (Lane A state machine + Stripe test mode, per P69's lane structure): `foodcourt_all_legs_captured_is_placed`
— an N-leg order where every leg captures resumes into the Inciting beat (P69's C.2 amber burst);
`foodcourt_partial_failure_is_no_charge` — an `Aborted` outcome (M3) never reaches `Placed`, renders
the honest no-charge UX. **Adversarial:** (i) a **forged client "success" with NO webhook** ⇒ the
journey never reaches `Placed`, no leg is `Captured` (the P69/P60 forged-success red-line, applied to
N legs); (ii) legs 1-3 Captured but leg 4 `NeedsReconciliation` ⇒ an honest "confirming…" state, never
a fake `Placed` (P60 §4.5); (iii) app killed mid-redirect ⇒ P66 query-before-replay restores the
draft + key, `query_status_by_key` resolves the true per-leg status, **no double charge** (X6, applied
to the N-leg plan — the idempotency key covers the whole plan, P60 §4.2); (iv) a stale `session_token`
(TTL elapsed) ⇒ refused, re-mint (P60 `session_token_single_use`).

### 4.7 M7 — the Wave-0 provider matrix (§4-D — feature-scope, NOT platform-scope)

Record the matrix, explicitly bounded: **EUR / Stripe Connect (zero-fee facilitator, direct charges)
is the primary, proven mechanic; Adyen is the named fallback** (design-in, **stubbed not integrated**
in Wave-0 — the port trait is proven to hold by stubbing an `AdyenProvider: PaymentProvider`, exactly
as P60 M3 / R2 §8 prescribe). Adyen's equivalent shape is split-at-authorization to sub-merchant
balance accounts (R2 §4.1) — a different mechanic behind the **same** `PaymentProvider` port, so
`foodcourt.rs`'s `derive_nleg_plan` and P60's saga are **provider-agnostic** (they see only
`NLegPlan`/`VendorLeg`, never a Stripe/Adyen type). **The explicit scope statement (standard
honesty):** per §16.20 the **platform architecture stays market-agnostic** — `Currency` is not
hardcoded to EUR (the `Money`/`Currency` authority is currency-general, money.rs:29), the port branches
on no provider, and a new market/provider is a new adapter crate + a config change, **zero
`foodcourt.rs`/P60/P62 kernel change**. §4-D scopes only **where the food-court FEATURE is proven
first** (EU/EUR), **never** the platform. Mollie's EUR/GBP-only split limit (R2 §4.1) is noted as a
reason it is *not* the Wave-0 primary, not a platform constraint.

RED→GREEN: `adapter_port_holds_for_adyen_stub` — a stubbed `AdyenProvider` compiles against the same
`PaymentProvider` port and `foodcourt.rs` derives a plan for it with **zero code change** (the port
holds); `currency_not_hardcoded` — `derive_nleg_plan` over a non-EUR single-currency order produces a
valid plan (the currency-general path; the EUR limit is a *config/market* choice, not a code
constraint). **Adversarial:** a `foodcourt.rs`/P60 code path that branches on `"stripe"` vs `"adyen"`
⇒ a source-audit test fails (provider-agnostic by construction — mirrors P62's `no_single_vendor_special_path`
audit shape).

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6)

Reachability arguments, not prose:

- **dowiz cannot become a party to the money.** Every `VendorLeg.dest_account` is a vendor
  `ProviderAccountRef` (P60 §3); `PayabilityStatus::Connected` carries only the vendor's ref, and
  **there is no dowiz-account type anywhere in `NLegPlan`/`foodcourt.rs`.** "dowiz holds funds / is
  MoR / takes a fee" is **unrepresentable**, not policed (the money red-line, §1.3). The zero-fee +
  direct-settlement adapter invariants (M2) are the runtime teeth on the out-of-kernel half.
- **A not-yet-onboarded vendor cannot be silently charged or dropped.** A non-`Connected` vendor
  yields `VendorNotPayable` in `derive_nleg_plan` (M1) — **no plan is produced**, so there is no code
  path from an unconnected vendor to an authorization or a dropped line. The onboarding-step
  precondition is a type gate, not a runtime check that could be skipped.
- **A food-court order is never left half-charged.** The terminal is exactly one of P60's
  `{Committed, Aborted, NeedsReconciliation}` — a mixed terminal (some legs captured, some voided,
  no reconciliation flag) is unrepresentable in P60's `decide_capture` (P60 §5.1), and P72 only
  *executes* that Law, adding no capture/void path of its own.
- **A refund cannot leak across vendors.** `refund_vendor_leg` routes to exactly one vendor's own
  `ChargeHandle` (M4); the other vendors' handles are never passed to `refund`. Per-vendor isolation
  is the charge-handle binding, not a filter that could be misapplied.
- **A KDS ticket cannot leak across vendors.** Per-vendor KDS reads are the inner `app.vendor_scope`
  RLS narrowing within the already-`location_id`-scoped set (P62 §4.6); the outer cross-hub boundary
  is untouched and deny-on-unset. Cross-vendor KDS leakage is unreachable through the inner filter
  (AND, not OR — P62 §5.1).
- **A card cannot reach the canvas.** No card-data type exists (P60 §4.1 firewall); P72 consumes only
  opaque `ClientHandoff`. Unchanged from P69/P60 — inherited, not re-proven.
- **Money integrity.** Every leg amount is `Money` (i64 minor units) with fail-closed cross-currency
  + overflow arithmetic (money.rs); no f64 anywhere; cross-currency carts fail closed (M1).

### 5.2 Schemas & scaling axes (item 8)

- **`NLegPlan` (food-court):** axis = vendors/order, bounded `MAX_LEGS_PER_CHECKOUT = 32` (reused
  from P60 — a realistic food-court is 2–30 vendors, R5 §3). Break point: a mall > 32 co-located
  vendors ⇒ P60's reserved plan-chunking `flags` (named, not built).
- **`derive_nleg_plan`:** O(vendors) over P62's O(items) `charge_legs` group-by — microseconds for a
  30-vendor / 300-item order (P62 §7 bench). No break in sight.
- **`kds_route`:** O(items) fan-out over P62's `kitchen_tickets`; axis = items/order. Same shape.
- **Per-vendor refund ledger:** axis = legs/order (≤ 32); a per-leg refund is O(1). No break point.
- **Cross-account auth (M2):** axis = auth round-trips/checkout = N vendors (out-of-kernel network);
  N ≤ 32 provider calls per checkout — the customer-facing latency is the provider's, not the hub's
  (§7). Break point: a very large food-court would batch the N auths (a provider-side concern).

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

**Isolation/bulkhead:** `foodcourt.rs` is a **pure derivation/routing** module — a bad cart yields a
typed `FoodCourtError`, never a propagating failure; the atomicity saga's provider I/O is out-of-kernel
behind P60's firewall (a provider outage reaches the kernel only as a typed `PayError`/leg
`AuthFailed`, P60 §5.3). **Per-vendor bulkhead:** one vendor's leg failure voids/reconciles only its
own account (M3/M4) and one vendor's KDS is scoped to its own rows (M5) — a food-court vendor cannot
corrupt a sibling's money or kitchen view. **Mesh awareness:** the food-court order is **hub-local,
NOT gossiped** — the N legs settle hub↔provider (out-of-canvas), the KDS routing is hub-local RLS,
and `order_item.vendor_id` rides the **existing** order event on the P34/P37 wire (an additive field
P62 already carries — no new payload budget, P62 §5.3). **Zero mesh payload originates in P72; no
money over the mesh.** **Living memory:** the per-leg saga + refund ledgers are append-only,
content-addressed by idempotency key / order id (demote-never-mutate, reused from P60's `IdemLedger`
discipline); a superseded price is preserved as the `unit_price` snapshot on `order_item` (P62 §5.3) —
a placed food-court order keeps its priced-at per-vendor numbers even after a vendor re-prices.

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

- **Self-Termination leg claimed:** the typed refusals (`VendorNotPayable`, `CrossCurrencyCart`,
  `TooManyVendors`; the unrepresentable dowiz-account-as-dest; the unrepresentable mixed-capture
  terminal inherited from P60) — hard invariant boundaries, not a supervisor's decision.
- **Self-Healing leg claimed narrowly:** (a) the **N-leg abort** — voiding all authorized legs on any
  auth failure (M3) is genuine compensating error-correction (P60's Law); (b) a **`CaptureStuck` leg
  self-heals toward Void** via provider-side auth-hold expiry (P60 §4.5); (c) the **resume-from-draft**
  path on an app-kill during the N-leg redirect (P66 query-before-replay, M6). Claimed for the money
  legs + the journey resume only, not for arbitrary state.
- **Snapshot-Re-entry: claimed via P60/P66** — the N-leg saga is re-derivable from the append-only
  saga + `IdemLedger` logs after a hub restart (P60 §5.4), and the journey resumes from the P66 draft
  snapshot. Recovery is a cheap re-fold from the last valid epoch, not a bespoke path. Mechanical
  rollback: every P72 change is **additive** (new `kernel/src/foodcourt.rs`, new
  `web/src/storefront/foodcourt.mjs`, an extension to P60's `payment-adapters` Stripe crate) — deletion
  restores the single-vendor (N=1) path exactly, since N=1 never routed through the food-court glue.

### 5.5 Error-propagation gates (item 14) + Linux discipline (item 9) + tensor/spectral/eqc (item 16)

**Named gates that turn P72's bug classes into compile/CI failures:** the money red-line teeth
(`application_fee == 0` + `funds_never_touch_facilitator` adapter tests, M2; the unrepresentable
dowiz-account-as-dest, §5.1); the `foodcourt_leg3_fails_voids_legs_1_2` partial-failure test (M3);
the `foodcourt_refund_one_vendor_leaves_others_untouched` per-vendor-refund test (M4); the
`kds_no_item_lost_or_duplicated` + inner-scope RLS narrowing test (M5); the `foodcourt_partial_failure_is_no_charge`
+ forged-success test (M6); the provider-agnostic source audit (M7). **Linux-discipline verdicts:**
**ALREADY-EQUIVALENT** — one atomicity Law (P60), one leg-derivation (P62 `charge_legs`), one money
authority (`Money`), one refund primitive (P60 `refund`), one KDS fan-out (P62 `kitchen_tickets`), one
leg cap (P60 `MAX_LEGS_PER_CHECKOUT`); **REINFORCES** — the vendor-as-own-MoR ruling encoded as a
type (no dowiz-account in the plan) extends P60's no-dowiz-custody proof to the multi-vendor case;
**EXTENDS** — the **payability gate** (a not-yet-onboarded vendor is an unrepresentable leg
destination) is a new gate class this doc adds for the onboarding-step precondition; **GAP** honestly
named — the **cross-account PaymentMethod-clone mechanic** (M2) is provider-specific and unproven
until the `payment-adapters` Stripe-test-mode integration runs (P60 §9 risk #3's Tauri-bridge cousin
for the money leg); Wave-0 proves it against Stripe test mode with connected test accounts, and Adyen
stays stubbed (§4.7). **Item 16 (tensor/spectral/eqc): NOT load-bearing, stated not decoratively
invoked** — food-court checkout is integer money-law + group-by fan-out + provider REST; there is no
closed-form organ, so `eqc-rs` does not apply and no spectral machinery is summoned (the Anu/Ananke
discipline forbids manufacturing a spectral form where none is load-bearing). The one honest reuse of
math is P62's `Money::checked_add` fold for leg sums — cited, not re-derived.

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 | no `foodcourt.rs`; plan-derivation + payability tests absent | 3-vendor plan derives (legs sum to subtotal, ascending order); N=1 = one leg same path; **`NotConnected` ⇒ `VendorNotPayable`, no partial plan**; cross-currency ⇒ `CrossCurrencyCart`; no dowiz-account as dest | **payability-gate** test (ledger row) |
| M2 | no cross-account mechanic; zero-fee/direct-settlement unasserted | 3 direct manual-capture charges, distinct connected accounts, `application_fee == 0`; **facilitator balance delta == 0** | **money-red-line (zero-fee, funds-never-touch-facilitator)** test (ledger row) |
| M3 | no partial-failure execution; N-leg abort unasserted | **`foodcourt_leg3_fails_voids_legs_1_2`**: legs 1-2 voided, leg 4 never authed, zero captured, `Aborted{void_set:[1,2]}`, honest no-charge UX (asserted on event sequence) | **N-leg partial-failure** test (ledger row — task-mandated) |
| M4 | no per-vendor refund router; cross-vendor-leak untested | **`foodcourt_refund_one_vendor_leaves_others_untouched`**: only vendor 2's `ChargeHandle` refunded, vendors 1&3 untouched, order NOT `CompensatedRefund`; all-legs-refunded ⇒ terminal | **per-vendor-refund** test (ledger row — task-mandated) |
| M5 | no KDS routing; item-loss/leak untested | each item → its vendor's ticket; Σ line-counts == item count; vendor A scope cannot read B's ticket (inner RLS) | KDS-routing + inner-scope tests |
| M6 | no N-leg journey extension; forged-success RED by construction | all-legs-captured ⇒ Inciting beat; **partial failure ⇒ NO charge, never `Placed`**; forged success (no webhook) ⇒ not placed; app-kill no double-charge | **webhook-sole-truth (N-leg)** test (ledger row) |
| M7 | no provider matrix; provider-branch untested | Adyen stub compiles against the same port with zero kernel change; currency not hardcoded; provider-branch source audit fails on any `"stripe"`/`"adyen"` branch | provider-agnostic audit + currency-general test |

**Not-done clauses:** dowiz appearing as merchant-of-record, holding funds in a dowiz balance, or
taking any `application_fee`/cut = **NOT done** (§0.2-1 money red-line) regardless of green totals; a
food-court leg captured while a sibling leg is voided **without** a `NeedsReconciliation` flag = **NOT
done** (P60 atomicity); a refund routed as one undifferentiated whole-order refund instead of
per-vendor = **NOT done** (§4.4); a not-yet-connected vendor's items silently dropped or charged =
**NOT done** (payability gate, §4.2); a client redirect that lets the journey reach `Placed`/`Captured`
without the webhook = **NOT done** (§4.6); a KDS view that leaks another vendor's tickets = **NOT
done** (§4.5); a `foodcourt.rs`/P60 code path branching on provider name = **NOT done** (§4.7); a
bare `i64`/`f64` amount without a `Currency` tag, or a cross-currency cart silently converted = **NOT
done**; a re-implementation of `decide_capture` / `charge_legs` inside `foodcourt.rs` = **NOT done**
(§2.2 scope violation).

---

## 7. Benchmark plan (item 10) — kernel glue micro-benched; provider network out-of-kernel

Reuse the kernel Criterion harness (P60 §7 / P62 §7 discipline). Add: `foodcourt/derive_plan_30v_300items`
(the food-court plan derivation over a 30-vendor / 300-item order — target < 100 µs, pure group-by +
checked-adds + account lookup, measured **beside** `derive_plan_1v` to prove the cost shape is one
function scaling with vendors, not a food-court branch — the §1.1 P4 POLARITY made a bench);
`foodcourt/kds_route_30v` (the KDS fan-out — target < 50 µs, reuses P62's `kitchen_tickets` bench
shape); `foodcourt/refund_router_resolve` (per-leg handle resolution — target < 5 µs). All added
**RED-commit-first** so baselines auto-seed; results to `BENCH_HISTORY.md`, never prose estimates.
**Out-of-kernel network** (the N cross-account auth round-trips, M2) is **not** micro-benched in the
kernel — it is covered by the `payment-adapters` Stripe-test-mode integration test with a stated
latency budget (N auth holds is N provider round-trips; the customer-facing latency is the
provider's, not the hub's — the auth-all phase can fan the N calls concurrently). Telemetry: per-leg
authorize/capture/void/refund counters + a `vendor_not_payable` refusal counter + the
`application_fee != 0` guard counter ride the existing native-trackers hooks (P-H's lane), so a
money-red-line regression or a payability-refusal spike surfaces without review.

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the 20-point contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §0.2-1 (MoR ruling — each vendor own-MoR), §4-D (market
scope — EU/EUR/Stripe-Connect + Adyen fallback, feature-scope not platform-scope), §5 W4 P72 row
(§3.1 "→ M4 gate: first food-court order"), X6 (idempotency — P60 owns), X7 (leaf invariant — P62
owns), X11 (anti-abuse) · `docs/research/OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md` (read in full — §4
split/Connect, risk #3 re-scoped to the new ruling, §8 provider matrix, risk #4 multi-currency split
limit) · `docs/research/OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md` (read in full — §3 food-court
data model / per-item KDS routing / split settlement, §7 risk #3 split failure/refund semantics =
this doc's §4.3/§4.4) · `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.15 (shared courier
pool), §16.16 (no transaction %, vendors keep 100%), §16.20 (market-agnostic architecture — the
§4.7 scope statement), §16.29 (refunds = vendor+provider responsibility), §16.46 (unified cart, split
required), §16.49 (hub never sees card data). **Upstream contracts consumed (by exact name):**
`BLUEPRINT-P60-payment-adapter-core.md` (`NLegPlan`/`VendorLeg`/`NLegOutcome`/`decide_capture`/
`capture_leg`/`void_leg`/`refund`/`RefundRequest`/`ChargeHandle`/`ProviderAccountRef`/`ClientHandoff`/
`create_with_key`/`query_status_by_key`/`MAX_LEGS_PER_CHECKOUT`; **the §4.5 boxed residual P72
discharges**; the no-card firewall §4.1; the webhook-sole-truth §4.4) ·
`BLUEPRINT-P62-catalog-multivendor-data-model.md` (`VendorId`/`ChargeLeg`/`charge_legs`/
`kitchen_tickets`/`order_item.vendor_id` catalog-authoritative §4.4/ the inner `app.vendor_scope` RLS
§4.6) · `BLUEPRINT-P69-customer-storefront-checkout.md` (`Journey`/`JourneyStep::{Cart,Payment,
Suspended,Placed}`/`SuspendState`/`ReturnSignal`/the Inciting beat/ the Path-C card moment §4.5/ the
`charge_legs` split preview §4.3) · `BLUEPRINT-P67-hub-provisioning-claim.md` §2.2 (the per-vendor
account-connect hook in the claim/vendor-setup flow) · `BLUEPRINT-P70-owner-surface.md` (the owner
surface where the vendor connects the account + authors the catalog) · `BLUEPRINT-P51-open-map-routing.md`
(structural template) · `HERMETIC-ARCHITECTURE-PRINCIPLES.md` (§9) · `docs/regressions/REGRESSION-LEDGER.md`
(the rows named in §6). Landed kernel ground-truth cites in §0. **Consumed by / hands off to:** **P61**
(per-vendor status notifications), **P65/P51/P71** (the single shared-pool delivery). Memory:
`crypto-safe-first-pass-2026-07-14` (money/RLS red-lines preserved under autonomy — the MoR +
no-custody + zero-fee red-line honored) · `test-integrity-rules-2026-06-27` (money-RLS-PII red-lines;
no-f64-money) · `rust-native-bare-metal-decision-2026-07-14` (DECART tables §1.5/§4.2; extend-don't-fork)
· `never-bypass-human-gates-2026-06-29` (the money red-line is honored by construction — dowiz is never
a money party; the two closed operator rulings are applied, not re-litigated) ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (§5.5's honest "spectral N/A", no ritual math).
Supersedes: nothing — additive; it is the concrete N > 1 realization of the food-court gap §16.46/R5
§3 named, built entirely from prior contracts.

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the food-court checkout is composed from prior *specs*
  (P60's Law, P62's derivation, P69's journey) — P72 adds glue that derives the plan from those
  sources; the concrete order is the derived shadow of the ruled shape (§1.1).
- **P2 CORRESPONDENCE** (one concept, one primitive): one atomicity Law (P60), one leg-derivation
  (P62 `charge_legs`), one refund primitive (P60 `refund`), one KDS fan-out (P62 `kitchen_tickets`),
  one money authority — the food-court case re-derives none of them (§1.1/§5.5).
- **P4 POLARITY** (one axis, two poles): single-vendor (N=1) and food-court (N>1) are the **same**
  checkout at two poles of the vendor-count axis — `derive_nleg_plan`, `decide_capture`, and
  `kds_route` are total over `1..N` with no food-court branch (§1.1, machine-proven by the
  `single_vendor_is_one_leg` + provider-agnostic audit tests).
- **P6 CAUSE-AND-EFFECT** (determinism as law): deterministic `VendorId`-ordered legs, deterministic
  KDS routing, the webhook as the sole deterministic gate for `Captured`, integer money end-to-end —
  every determinism claim carries a falsifier (§4/§6).
- **P7 GENDER** (paired verification, no self-certification): a food-court payment is refereed by the
  *independent* provider webhook + the kernel fold across **all N legs** — the client never
  self-certifies (§4.6); the money red-line is refereed by the zero-fee/direct-settlement adapter
  teeth + the type-level absence of a dowiz-account (§5.1), not by a policy promise.

(P3/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; landed money/order FSM vs on-paper P60/P62 contracts; P60's boxed residual; P67 §2.2 hook) |
| 2 DoD | §6 (incl. the two task-mandated N-leg partial-failure + per-vendor-refund tests) |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first; §4.3 asserts on the leg event sequence, not end-state |
| 4 predefined types/consts | §3 (derivation glue + payability + refund router; cited types shown as `use` boundaries) |
| 5 adversarial/breaking tests | §4.1–4.7 (unconnected-vendor, cross-currency, leg-3-fails, refund-leak, KDS-leak, forged-success, provider-branch, zero-fee teeth) |
| 6 hazard-safety as math | §5.1 (dowiz-as-money-party / unconnected-charge / mixed-capture / cross-vendor-refund-leak / cross-vendor-KDS-leak all unreachable by construction) |
| 7 links docs/memory | §8 (P60/P62/P69/P67/P70 named by exact contract; consumers named) |
| 8 scaling axes | §5.2 (each with a named break point; MAX_LEGS reused from P60) |
| 9 Linux discipline | §5.5 (all four verdict classes incl. an honest GAP — the cross-account mechanic) |
| 10 benchmarks+telemetry | §7 (kernel glue benched beside N=1; provider network out-of-kernel; money-red-line telemetry) |
| 11 isolation/bulkhead | §5.3 (pure derivation module; per-vendor money + KDS bulkhead; adapter behind P60 firewall) |
| 12 mesh awareness | §5.3 (hub-local; no money over mesh; vendor_id additive on the existing order event) |
| 13 rollback/self-heal vocabulary | §5.4 (three legs claimed precisely — abort, capture-stuck self-heal, resume-from-draft) |
| 14 error-propagation gates | §5.5 (named gates), §5.1 (typed `FoodCourtError` refusals), §6 (ledger rows) |
| 15 living memory | §5.3 (append-only per-leg saga/refund ledgers; price snapshot demote-never-mutate) |
| 16 tensor/spectral + eqc reuse | §5.5 (spectral honestly NOT invoked; `Money::checked_add` fold reused, no manufactured equation) |
| 17 regression ledger | §6 (rows named, incl. the two task-mandated tests + the money-red-line teeth) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §1.1/§1.5 (the compose-not-build table; P60/P62/P69 all consumed, not forked; six rejected alternatives §1.5/§3) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Order below is the dependency order. **Prerequisite: P60 + P62 must be landed** (their on-paper types
this doc cites must exist as kernel code); **P69 landed** for the journey extension; **P67/P70 landed**
for the connected `ProviderAccountRef`. **Lane A (buildable the moment P60+P62 land, zero network):**
T1, T4, T5 (pure kernel glue over P60/P62 types). **Lane B (gated on the out-of-kernel adapter +
Stripe test mode):** T2, T3, T6. T7 is the matrix record.

1. **T1 (M1 — the derivation glue is the spine).** Create `kernel/src/foodcourt.rs` per §3
   (`PayabilityStatus`/`VendorAccounts`/`FoodCourtError`/`derive_nleg_plan`); register
   `pub mod foodcourt;` in `kernel/src/lib.rs` (alphabetical). `derive_nleg_plan` calls P62's
   `charge_legs`, attaches each vendor's `Connected` `ProviderAccountRef` → P60's `VendorLeg`, and
   refuses non-`Connected` with `VendorNotPayable`. Write RED first: `derive_plan_three_vendors_all_connected`,
   `single_vendor_is_one_leg`, `NotConnected ⇒ VendorNotPayable` (no partial plan), cross-currency ⇒
   `CrossCurrencyCart`. **Do NOT re-derive `charge_legs` or define a plan type — cite P60/P62.**
   Acceptance: `cargo test -p dowiz-kernel foodcourt` green.
2. **T2 (M2 — the cross-account mechanic, out-of-kernel).** Extend P60's `payment-adapters` Stripe
   crate (repo root, `reqwest` allowed HERE — outside the firewall) with the N-direct-charge mechanic
   (§4.2): one customer PaymentMethod on the zero-fee facilitator, cloned to N connected accounts, N
   manual-capture direct charges, `application_fee_amount = 0`. RED (Stripe test mode, connected test
   accounts): `n_direct_charges_zero_fee`, `funds_never_touch_facilitator`. **Freeze the money
   red-line here: dowiz balance never holds funds, no fee, ever.**
3. **T3 (M3 — partial-failure, the task-mandated N-leg test).** Wire `foodcourt.rs`'s plan into P60's
   `decide_capture` saga (P60 owns the Law — call it, do not re-implement). Write `foodcourt_leg3_fails_voids_legs_1_2`
   FIRST (4 vendors, leg 3 auth-fails ⇒ legs 1-2 voided, leg 4 never authed, zero captured, `Aborted`
   — assert the event sequence), then the UX. Adversarial per §4.3 (i)–(iv). Acceptance: the N-leg
   partial-failure test green.
4. **T4 (M4 — per-vendor refund, the task-mandated refund test).** Add `refund_vendor_leg` +
   `all_legs_refunded` per §3 (route to one vendor's `ChargeHandle` via P60 `refund`; whole-order
   `CompensatedRefund` only when all legs refunded). Write `foodcourt_refund_one_vendor_leaves_others_untouched`
   FIRST. Adversarial per §4.4 (i)–(v). **Do NOT add a refund state machine — reuse P60 `refund` +
   `order_machine`.** Acceptance: the per-vendor-refund test green.
5. **T5 (M5 — KDS routing).** Add `kds_route` wrapping P62's `kitchen_tickets` per §3. RED:
   `kds_routes_each_item_to_its_vendor`, `kds_no_item_lost_or_duplicated`; add the DB-gated inner-scope
   RLS narrowing test (P62's `#[ignore]` pattern) asserting vendor A's scope cannot read B's ticket.
   Acceptance: KDS tests green.
6. **T6 (M6 — the N>1 journey extension).** Create `web/src/storefront/foodcourt.mjs`: specialize P69's
   `JourneyStep::Payment`/`Suspended` for the N-leg handoff + the per-vendor status view + the
   partial-failure no-charge UX. **Extend P69's journey, do NOT rewrite it.** Write the forged-success
   test FIRST (no webhook ⇒ never `Placed`, applied to N legs). RED per §4.6. Acceptance: Lane-A state
   machine + Stripe test-mode green.
7. **T7 (M7 — the provider matrix).** Record EUR/Stripe-Connect primary + Adyen stub fallback (§4.7);
   add `adapter_port_holds_for_adyen_stub` + `currency_not_hardcoded` + the provider-agnostic source
   audit (fails on any `"stripe"`/`"adyen"` branch in `foodcourt.rs`/P60). State the feature-scope /
   not-platform-scope boundary explicitly. Add the §6 ledger rows to `docs/regressions/REGRESSION-LEDGER.md`
   (naming the two task-mandated tests + the money-red-line teeth). Acceptance: matrix tests green;
   ledger rows present.
