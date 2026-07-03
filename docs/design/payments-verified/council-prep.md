# Payments — Verified & Reliable (card + crypto): Council Prep

- **Status:** 🟡 RESEARCH + COUNCIL-PREP ONLY. No code, no commit, no flag flip in this lane.
- **Red-line:** 🔴 MONEY · 🔴 RLS · 🔴 MIGRATION (forward-only) · 🔴 PCI. This document is **design/analysis
  only** — it proposes NO money code and grants NO enablement authority.
- **Date:** 2026-07-02
- **Standing gate:** the current crypto vertical stays **DARK** behind `PAYMENTS_PREPAID_ENABLED` +
  `PAYMENTS_CRYPTO_ENABLED` (both default OFF). No autonomous money enablement — every launch flip is a
  Triadic-Council + human act under Ship Discipline.
- **Program goal:** working / verified / reliable **CARD + CRYPTO** payments.
- **Grounds on:** ADR-0017 (`docs/adr/ADR-payments.md`), `docs/design/payments/{research,proposal,
  breaker-findings,counsel-opinion,resolution,crypto-providers}.md`, migration
  `packages/db/migrations/1790000000083_payments-ledger.ts`, and the built-dark code cited inline below.

---

## 0. Executive summary (for the impatient)

- **The 3 council CRITICALs (C1/C2/C3) are structurally addressed in the built-dark code** — a
  `delivered_prepaid` outcome (C1), an owner-review `refund_due`/`refund_sent` path (C2), and a DEFINER +
  `app.current_tenant` webhook tenancy (C3). **But none is verified** — there is not a single automated test
  over the entire payments vertical, and C2 has a **real, confirmed money-loss hole** (below).
- **Card is greenfield.** Stripe is **not available to an Albania-domiciled merchant** (confirmed). No card
  adapter exists (`AlbaniaHppAdapter` is designed-only). Recommended path: **per-tenant local-bank Virtual
  POS / hosted payment page (HPP) redirect (SAQ-A)** behind the existing port — restaurant is the merchant of
  record, dowiz stays a conduit. Merchant-of-Record PSPs (2Checkout/Verifone, Paddle) are available to
  Albania but structurally wrong for a per-tenant marketplace (reject as primary).
- **Reliability bar is not met.** Zero tests; the H1 "reconciliation poll" claimed RESOLVED in ADR-0017 is
  **not built** (the nightly recon worker checks `payment_method='cash'` only — crypto has no drift net); the
  ledger is single-entry (acceptable for non-custodial crypto **iff** reconciliation is built; **not**
  acceptable the moment any custodial rail lands).
- **Nothing ships until the council ratifies the proof bar in §6 and a human clears the NEEDS-HUMAN list.**

---

## 1. The 3 open CRITICALs + NEEDS-HUMAN — current status

Source of the CRITICALs: `payments-council-2026-06-30` memory + `docs/design/payments/breaker-findings.md`,
resolved in `resolution.md` / ADR-0017. Status below is verified against **live code** on branch
`feat/phase0-safety-hardening` (2026-07-02).

| # | CRITICAL (Breaker) | ADR claim | Verified in code? | True status |
|---|---|---|---|---|
| **C1** | `completeDelivery` only had `paid_full` → a prepaid order would hit `CASH_AMOUNT_MISMATCH` (never completes) or a phantom courier till-debt (`hold`) on already-paid money | RESOLVED via `delivered_prepaid` outcome (skip cash assert, no `hold`, precond `payment_status='paid'` → 409 `PREPAID_NOT_PAID`) | **Yes, code present** — `deliveryCompletion.ts` (`isPaidFull` gates the `hold`; the non-delivered tail runs), `courier/assignments.ts` auto-resolves crypto+paid | 🟡 **Code-complete, UNVERIFIED.** No test exercises `delivered_prepaid`, the 409 precondition, or the "no `hold` on a paid order" invariant. |
| **C2** | Charged-then-refused prepaid keeps the money — no refund in the completion path | RESOLVED (reframed): irreversible crypto → no auto-refund; refused/cancelled tail writes `payment_events('refund_due')`; owner sends crypto back → records `refund_sent` → `refunded` | **Partially** — `deliveryCompletion.ts:126-145` writes `refund_due`, `owner/refunds.ts` lists/settles it | 🔴 **OPEN — real gap.** `refund_due` is written **only** in `deliveryCompletion.ts` (the courier terminal path). Any paid crypto order cancelled via another path strands the customer's money with **no obligation recorded** (see §3.2). Also UNVERIFIED (no test). |
| **C3** | The unauthenticated webhook (money SoT writer) can't INSERT under member-derived RLS without a `BYPASSRLS` role the proposal warns against | RESOLVED: DEFINER `payment_location_by_provider_ref()` → `set_config('app.current_tenant')`; dual RLS policy `WITH CHECK` on the GUC; insert-wins `UNIQUE(provider,provider_payment_id,type)`. **Depends on B3 (NOBYPASSRLS+GUC).** | **Yes, code present** — mig 083 (FORCE RLS + dual policy + DEFINER resolver + grant-mirror), `payments-webhook.ts` sets the GUC | 🟡 **Code-complete, but load-bearing dep OPEN + UNVERIFIED.** The `WITH CHECK` GUC path is **only enforcement once B3 removes `BYPASSRLS`** — until then it's defense-in-depth over a role that still bypasses RLS. No test proves a forged/cross-tenant webhook is rejected. B3 status must be re-confirmed by the council. |

**HIGHs (fold-ins) status:**
- **H1** (lost-webhook backstop → fast pending-poll): ADR-0017 says "RESOLVED — minute-cadence advisory-lock
  poll." **NOT BUILT.** The nightly `reconciliation.ts` worker's money checks (`M2/M3/M4…`) are all scoped
  `payment_method='cash'`; there is **no** poll or drift check for `payments`/`payment_events`. A `pending`
  crypto invoice whose webhook is lost stays `pending` forever with no detection. 🔴 OPEN.
- **H2** (refund monotonic/sticky): partially — `payments_money_residual CHECK` + status-guarded transitions
  + insert-wins on `refund_sent` give stickiness; UNVERIFIED. 🟡
- **H3** (createCharge off the 8-conn operational pool): the Plisio call is `fetch` to a hosted invoice and
  the webhook only ingests; charge happens inside the order-create transaction (`orders.ts:641-664`) — this
  **does** hold a pool connection across an external HTTP call (a latency/pool-starvation risk the ADR wanted
  avoided). ⚠️ Re-examine — see §4.6. 🟡

**NEEDS-HUMAN (launch-gate) status — all still OPEN:**
- **NH-RES-1** off-ramp procedure (USDT-TRC20 Binance P2P / USDC rail) — merchant treasury op. OPEN.
- **NH-RES-2** refund-SLA copy + value of **Y** (max days) + irreversibility disclosure copy — gates the
  `PAYMENTS_CRYPTO_ENABLED` flip (Counsel ETHICAL-STOP-1). OPEN.
- **NH-RES-3** non-custodial wallet-key custody procedure (backup / multisig / hardware). OPEN.
- **NH-5** Albania legal/tax/AML/e-invoice for crypto — gates a real consumer launch. OPEN.
- **NH-1** Albania card acquirer/PSP — reopens for the card round (see §2). OPEN.
- **B3** NOBYPASSRLS + GUC — the load-bearing dep under C3. Status must be confirmed before crypto enablement.

**Verdict on §1:** the *design* converged and the *dark code exists*, but "resolved-in-ADR" ≠ "verified-in-
code." Of the three CRITICALs, **C2 is genuinely still open** (money-loss on non-courier cancel paths), C1/C3
are code-complete but unproven, and the H1 backstop that reliability depends on was never built.

---

## 2. CARD — processor research + recommendation (greenfield)

### 2.1 The gating fact (confirmed, current)
**Stripe does not serve Albania-domiciled merchants.** As of 2026 Stripe supports ~46 merchant countries;
Albania is not among them, and there is no announced expansion. Any "just use Stripe" plan requires an
EU/US-domiciled legal entity — a corporate-structure decision, not an integration decision (NEEDS-HUMAN).

### 2.2 The real options for Albania

| Path | What it is | Fit for a per-tenant food marketplace | PCI | Verdict |
|---|---|---|---|---|
| **Local-bank Virtual POS / HPP** (BKT Virtual POS, Raiffeisen, Credins, Intesa Sanpaolo AL, Tirana Bank) | Restaurant signs a card-acquiring contract with its own Albanian bank; dowiz redirects the customer to the bank's hosted 3DS payment page; bank settles to the restaurant's account in ALL | ✅ **Best fit** — each restaurant is its own merchant of record; dowiz never touches funds (conduit, mirrors the non-custodial crypto model + the Charter "commons/conduit" stance) | **SAQ-A** (redirect → no PAN on our servers) | ✅ **RECOMMENDED primary** |
| **Merchant-of-Record PSP** (2Checkout/Verifone, Paddle) | The PSP becomes the legal seller of record, handles tax/VAT/PCI, pays out to one merchant account | ❌ **Structurally wrong** — MoR makes the PSP (not the restaurant) the seller of the food; settlement is to one account, not per-tenant; fees ~3.5–6%; conflicts with the multi-tenant "each restaurant owns its money" model | SAQ-A | ❌ Reject as primary (note as a fallback only if per-tenant bank contracts prove unobtainable) |
| **International PSP via EU entity** (Stripe/Adyen/Mollie) | Requires dowiz or the tenant to hold an EU-domiciled company | Depends entirely on the corp-structure decision (NEEDS-HUMAN) | SAQ-A | ⏸ Defer — reopen only if an EU entity exists |

### 2.3 Recommendation
**`AlbaniaHppAdapter` = per-tenant local-bank hosted payment page (redirect), SAQ-A, behind the existing
`PaymentProvider` port.** This is the honest architecture for the market and is consistent with everything
already decided: provider-agnostic port, webhook/return-status as source of truth, restaurant-as-merchant
(conduit not custodian). It is **greenfield** — no card adapter exists today.

### 2.4 What building card actually requires (the port fits; the specifics differ from crypto)
- **Integration shape:** hosted payment page **redirect** (customer leaves to the bank's 3DS/ACS page), not
  Stripe Elements/embedded fields — keeps us firmly in **SAQ-A** (no PAN, no card fields, ever, on dowiz
  origins). PCI DSS v4.0.1 client-side **script-monitoring** on the checkout page is still required (SAQ-A
  reqs 6.4.3 + 11.6.1) even with redirect.
- **Per-tenant credentials:** unlike the single Plisio key, each restaurant has its **own** bank merchant
  ID + terminal + signing secret. The registry/config model must become **per-location** (a real schema +
  secrets-management delta — `location_payment_config` or similar, secrets in SOPS/vault, never in the DB in
  plaintext). This is the single biggest structural difference from crypto and a council decision.
- **3DS / SCA:** European card rules require Strong Customer Authentication; the bank HPP handles the 3DS
  challenge, but our flow must handle the **challenge round-trip** (redirect out → 3DS → redirect back →
  server-side status verification), never trusting the return redirect as proof (webhook/server-query is SoT,
  same rule as crypto).
- **Webhooks vs return-redirect:** many Albanian bank HPPs have **weak or no async webhooks** — settlement
  confirmation is often a **server-to-server status query** after the return redirect, or a signed return
  form. The reliability design (idempotency, reconciliation) must not assume a Plisio-style push webhook.
- **Idempotency:** same discipline as crypto — DB-UNIQUE insert-wins on `(provider, provider_payment_id,
  type)`; client idempotency key on charge init; capture is a distinct guarded transition.
- **Capture model:** decide **auth-only + later capture** vs **immediate capture** (NH-2 was deferred). For
  food where the order can be rejected pre-fulfillment, **auth-then-capture-on-accept** avoids the entire C2
  refund class for card — a strong argument to differ from the crypto (capture-immediately-then-refund) model.
- **PSP fee-bearer:** Counsel ETHICAL-STOP-2 — a recorded decision that the **courier NEVER bears the PSP
  fee**; who does (restaurant vs platform vs customer) is a business decision to log before card launch.
- **Refunds:** unlike crypto, card **supports programmatic refunds** — the `refund()` port method becomes
  real for the card adapter (vs `UNSUPPORTED` for Plisio). This changes the C2 shape for card (auto-refund
  possible) and argues for auth-then-capture to avoid refunds entirely where possible.

---

## 3. CRYPTO — what's built vs what's unverified

### 3.1 Built (dark, per `crypto-payments-build-2026-06-30`)
Full buy→deliver→refund vertical, committed + pushed, dark behind flags:
- **Schema** (mig 083): `payment_method += crypto,card`; `orders.payment_status`
  (`unpaid|pending|authorized|paid|failed|refunded`, decoupled from `order_status`); `payments` +
  `payment_events` ledger (integer minor, residual-guard `CHECK refunded<=captured<=amount`,
  `UNIQUE(provider,provider_payment_id)` + `payment_events UNIQUE(provider,provider_payment_id,type)`);
  ENABLE+FORCE RLS dual policy; grant-mirror; DEFINER `payment_location_by_provider_ref()`.
- **Port + Plisio adapter** (`provider.ts`, `plisio.ts`, `php-serialize.ts`): HMAC `verify_hash` fail-closed;
  integer-only money helpers; `refund()` → `UNSUPPORTED` (owner-review).
- **Webhook** (`payments-webhook.ts`): HMAC fail-closed (401) → DEFINER tenant-resolve → `app.current_tenant`
  → insert-wins `payment_events` → guarded monotonic transition; **only** writer of `payment_status='paid'`;
  404 when dark.
- **Order fork** (`orders.ts:635-665`): crypto order → `payments` row + `createCharge` → hold
  (`payment_status='pending'`) → return Plisio invoice URL; charge failure swallowed.
- **Completion** (`deliveryCompletion.ts`): `delivered_prepaid` (no cash, no `hold`, precond paid).
- **Refund** (`owner/refunds.ts`): GET pending / POST `.../sent` → `refunded`.
- **FE** (`CheckoutPage`): cash/crypto selector + irreversibility disclosure + refund-SLA + redirect + i18n.

### 3.2 🔴 The confirmed money-loss gap (C2 is NOT fully closed)
`refund_due` is written in **exactly one place** — `deliveryCompletion.ts:129` (`if (!isDelivered)`), which
runs only when a **courier** drives an order to a non-delivered terminal state. A paid crypto order can reach
a terminal cancelled state via **at least three other paths that never call `completeDelivery`**, each of
which leaves the customer paid with **no `refund_due` recorded** (money silently kept):

1. **Customer self-cancel** — `routes/customer/orders.ts:307-319` sets `orders.status='CANCELLED'` directly.
   No payments check, no `refund_due`.
2. **Owner cancel / reassign** — `routes/owner/dashboard.ts` cancel paths.
3. **Auto-timeout / offer-sweep** — `workers/order-timeout-sweep.ts`, `workers/courier-offer-sweep.ts` set
   `cancelled` for stuck orders.

Because the webhook can flip `payment_status='paid'` **independently** of fulfillment (the whole point of the
prepaid fork), a customer can pay, then the order auto-cancels (kitchen never accepts) → paid, cancelled, and
**no refund obligation exists**. This is the "STOP-REFUND-BEFORE-GRACE" gap: the refund obligation must be
tied to **every** terminal-cancel of a paid order, not just the courier completion path. **Fix belongs in the
central order-cancel primitive** (a single choke point that emits `refund_due` whenever a paid payment exists
on any order transitioning to a terminal non-delivered state), not scattered across four call sites — but
that is a design/council decision for the money code round, **not** enacted here.

### 3.3 Unverified surface (the test gap — confirmed)
`find … -name '*.spec.ts' -o -name '*.test.ts' | grep -iE 'payment|crypto|refund|plisio'` → **zero results.**
The entire money vertical has **no automated proof**. Specifically untested:
- `verify_hash` php-serialize key-order (adapter comment itself flags "NEEDS-VALIDATION against a real Plisio
  test invoice"; `verifyWebhook` is fail-closed so a wrong serialization **rejects all real webhooks** — a
  silent total failure that only a real test invoice surfaces).
- Webhook idempotency / replay / monotonic transitions / cross-tenant rejection.
- The `delivered_prepaid` 409 precondition (C1).
- The refund lifecycle and the residual-guard invariant.
- The dark-mode guarantees (404 when off; cash spine byte-for-byte unchanged).

### 3.4 Reliability gaps specific to crypto
- **No pending-invoice reconciliation** (H1, §1) — a lost webhook = a permanently stuck order.
- **Under/over/late payment (`mismatch`)** → event recorded, no status flip, routed to owner-review — the
  owner-review UI/queue for `mismatch` (distinct from `refund_due`) is unverified/possibly absent.
- **Reorg / depeg** — accepted-risk per ADR but with no monitoring.

---

## 4. Reliability & verification bar for money

### 4.1 Idempotency (both rails)
DB-UNIQUE **insert-wins** (never check-then-act) is the correct pattern and is present. **Proof required:**
a test that fires the same webhook N× and asserts exactly one state transition + one ledger row.

### 4.2 Single-entry vs double-entry ledger — the decision
**Current state: single-entry / event-sourced.** `payment_events` is an append-only log of events against a
single `payments` summary row; there is no balanced debit/credit across two accounts.

**Assessment:**
- **For non-custodial crypto v1 this is DEFENSIBLE** — funds settle **direct to the restaurant wallet**;
  dowiz never holds the money, so there is no dowiz cash position to keep balanced. dowiz is a **recorder**,
  not a bookkeeper of its own funds. Single-entry event log + the residual-guard invariant
  (`refunded<=captured<=amount`) + **external reconciliation** (recorded truth vs provider/on-chain truth) is
  sufficient, **provided reconciliation is actually built** (it is not — H1).
- **Double-entry becomes MANDATORY the moment any rail is custodial** — i.e. money flows *through* a dowiz
  account: card settlement into a platform account, platform-fee withholding, or per-tenant payouts. At that
  point you must be able to prove `sum(debits)==sum(credits)` and reconstruct each account's balance, which
  single-entry cannot do. The recommended card path (§2, per-tenant bank settlement direct to the restaurant)
  is **non-custodial too**, so it can stay single-entry — **but this coupling must be an explicit, ratified
  invariant**: *"single-entry is permitted only while every rail is non-custodial; introducing custody
  requires a double-entry migration first."*

**Council must ratify:** single-entry-with-reconciliation for v1 (non-custodial only) + the tripwire that any
custodial rail re-opens the ledger design as double-entry.

### 4.3 Reconciliation (the missing backstop)
Required before enablement: a **crypto/payments reconciliation check** in the nightly recon worker (or a
minute-cadence poll) that (a) finds `pending` payments older than a threshold and queries the provider for
true status, (b) finds `paid` orders whose fulfillment stalled, (c) finds paid+cancelled orders **with no
`refund_due`** (the C2 tripwire), (d) reports DRIFT via the existing Telegram-ops channel — **drift =
owner-review, never auto-adjust** (mirrors NO-AUTO-DEDUCT). This closes H1 and turns C2 into a detected
condition even if the primitive fix in §3.2 regresses.

### 4.4 Refund correctness
- Crypto: manual owner-review, sticky/monotonic (`refund_sent` insert-wins → `refunded`). **Proof:** the
  refund obligation is created on **every** terminal-cancel of a paid order (§3.2 fix), and double-settle is
  impossible.
- Card: programmatic refund via the port; prefer **auth-then-capture-on-accept** to avoid refunds entirely
  where possible. **Proof:** refunded amount can never exceed captured (residual-guard) and a refund is
  idempotent.

### 4.5 Security / RLS proof
FORCE-RLS + DEFINER + GUC path must be proven: a forged-HMAC webhook → 401; a valid webhook for tenant A
cannot write tenant B's rows; B3 (NOBYPASSRLS) confirmed so the `WITH CHECK` is actually load-bearing (not
defense-in-depth over a bypassing role). No PAN, no PII, no secret in `payment_events.payload` (claim-check).

### 4.6 Connection-pool discipline (H3 re-examine)
`orders.ts:641-664` performs the external `createCharge` **inside** the order-create DB transaction, holding
an operational-pool connection across a network round-trip to Plisio. Under the ADR-0001 8-connection budget
this is a starvation risk. Council should decide whether charge-init moves **after** commit (order committed
held/unpaid, then a separate short-lived charge step) — a reliability refinement, flagged not fixed.

---

## 5. Proof suite required before ANY enablement (the go-live gate)

No flag flips to a real customer until **all** of the following are green and pasted (Mandatory Proof Rule).

**A. Automated (red→green, committed as guardrails):**
1. **Plisio `verify_hash` validated against a REAL test invoice** — the one thing no unit test can fake;
   fail-closed means a wrong serialization silently rejects all production webhooks.
2. Webhook: valid → 200 + exactly one transition; forged HMAC → 401; replay/resend → idempotent (one row,
   one transition); cross-tenant write → rejected under RLS.
3. `delivered_prepaid`: completes with no `hold`; 409 `PREPAID_NOT_PAID` when not paid; cash path unchanged.
4. **C2 full matrix** — paid order cancelled via **each** path (customer-cancel, owner-cancel, auto-timeout,
   courier-refuse) records a `refund_due`; refund settle is sticky + idempotent; no double-refund.
5. Residual-guard invariant holds under refund (refunded ≤ captured ≤ amount).
6. Reconciliation: a stuck `pending`, a stalled `paid`, and a paid+cancelled-without-refund_due each raise
   DRIFT.
7. Dark-mode: all endpoints 404/empty when flags off; cash spine byte-for-byte unchanged (existing lifecycle
   E2E stays green).
8. `pnpm typecheck` + the money unit/integration suite green.

**B. Live staging E2E (against `dowiz-staging`):**
9. crypto buy → Plisio invoice → (simulated/real) webhook → `payment_status=paid` → order offered to
   fulfillment → `delivered_prepaid`; asserted on real DOM/`request.*` (Mandatory Proof Rule).
10. refuse-after-paid → owner sees a pending refund → records sent → `refunded`.

**C. Human sign-offs (NEEDS-HUMAN — cannot be automated):**
11. NH-RES-1 off-ramp procedure documented. 12. NH-RES-2 refund-SLA copy + **Y** value + irreversibility
disclosure finalized. 13. NH-RES-3 wallet-key custody procedure. 14. NH-5 Albania legal/tax/AML/e-invoice.
15. B3 (NOBYPASSRLS+GUC) confirmed live. 16. PSP fee-bearer decision recorded (card, courier never bears it).

---

## 6. COUNCIL AGENDA — what the Triadic Council must decide before any money code ships

The council (Architect + Breaker + Counsel) must converge on each of these **before** a money-code build
round is authorized. Each is a red-line decision; none may be defaulted by an agent.

**Crypto (verification round — the code exists, prove or fix it):**
1. **Ratify C2 as OPEN** and approve the fix shape: a **single central order-cancel primitive** that emits
   `refund_due` on every terminal-cancel of a paid order (vs today's single-path write). (Architect designs,
   Breaker attacks the choke point, Counsel checks the customer-money-safety promise.)
2. **Approve the proof suite (§5) as the mandatory go-live gate** and that crypto enablement is blocked until
   §5-A + §5-B are green and §5-C signed.
3. **H1 reconciliation:** ratify that a payments reconciliation/poll (§4.3) is a **build blocker**, not a
   post-launch nicety (correct the ADR's "RESOLVED" claim).
4. **H3 pool discipline** (§4.6): decide charge-init inside-txn vs after-commit.
5. **Confirm B3 status** — is NOBYPASSRLS+GUC actually live? Until it is, C3's `WITH CHECK` is not the real
   enforcement boundary; decide whether that blocks enablement.

**Ledger (the money-integrity spine):**
6. **Ratify single-entry-with-reconciliation for v1 (non-custodial only)** + the **tripwire**: any custodial
   rail (card into a platform account, fee-withholding, per-tenant payouts) re-opens the ledger as
   **double-entry** before that rail ships. (§4.2)

**Card (new build round — greenfield):**
7. **Ratify the card architecture:** per-tenant local-bank HPP redirect (SAQ-A), restaurant-as-merchant,
   dowiz-as-conduit — vs the rejected MoR path. Confirm NH-1 (which banks/acquirers, per-tenant onboarding).
8. **Per-tenant credential model** — a new `location_payment_config` schema + secrets management (SOPS/vault,
   never plaintext in DB). This is a schema + RLS red-line delta.
9. **Capture model** — auth-then-capture-on-accept (avoids the C2 refund class for card) vs immediate capture.
10. **PSP fee-bearer** decision recorded (courier never bears it — Counsel ETHICAL-STOP-2).
11. **3DS/SCA round-trip + return-vs-webhook SoT** for banks with weak async webhooks (§2.4).

**Counsel ETHICAL-STOPs (must be live before either launch flip):**
12. No crypto launch without irreversibility disclosure + written refund SLA (ETHICAL-STOP-1).
13. No card launch without a recorded fee-bearer decision (ETHICAL-STOP-2).
14. **Do not degrade or default away from cash** to chase PSP/crypto volume — cash stays the default +
    failure-first floor (serves-the-unbanked; Charter "commons"). Non-custodial-only locked; no customer KYC
    for a meal.

**Standing constraint (non-negotiable):** everything stays DARK behind the existing flags; no autonomous
money enablement; every flag flip is a human act under Ship Discipline after the §5 proof bar is green.

---

## Appendix — evidence index (files read for this prep)

- `docs/adr/ADR-payments.md` (ADR-0017, APPROVED, dark)
- `packages/db/migrations/1790000000083_payments-ledger.ts` (schema, RLS, DEFINER resolver)
- `apps/api/src/routes/payments-webhook.ts` (money SoT writer)
- `apps/api/src/routes/owner/refunds.ts` (owner refund review)
- `apps/api/src/lib/deliveryCompletion.ts:117-145` (C1 `hold` gate + C2 `refund_due` — the single write site)
- `apps/api/src/routes/orders.ts:635-665` (prepaid fork, in-txn createCharge — H3)
- `apps/api/src/lib/payments/{provider,plisio,registry}.ts` (port + adapter + flags)
- `apps/api/src/routes/customer/orders.ts:307-319` (customer self-cancel — a refund_due-less cancel path)
- `apps/api/src/workers/reconciliation.ts:147-165` (money checks scoped `payment_method='cash'` — no crypto)
- Web: Stripe supported-countries 2026 (Albania excluded); Albania acquiring options (BKT Virtual POS,
  Raiffeisen, 2Checkout/Verifone MoR)
- Doc drift noted: ADR §Decision-3 says `UNIQUE(provider, provider_event_id)`; migration uses
  `UNIQUE(provider, provider_payment_id, type)` — the migration is authoritative.
</content>
</invoke>
