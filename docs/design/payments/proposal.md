# Payments (card · cash · crypto) — Architect Proposal (pre-council)

- **Status:** DRAFT design proposal · design-time only · **NO production code, NO migrations**
- **Date:** 2026-06-30
- **Seat:** System Architect (Triadic Council input)
- **Red-line:** 🔴 MONEY · 🔴 RLS · 🔴 MIGRATION (forward-only) · 🔴 PCI (no PAN on our servers)
- **Reads:** `docs/design/payments/research.md` (research brief, sources)
- **Grounds in:** `payment_method` ENUM (`1780310044710:15`), `orders` (`1780310074262:20-46`),
  `completeDelivery` (`apps/api/src/lib/deliveryCompletion.ts`), ADR-deliver-v2-cash-as-proof,
  ADR-stage21-reconciliation, ADR-0005 (fee SoT), ADR-0001 (pg-boss / connection budget),
  `telegram-webhook.ts` (inbound-webhook conventions), `courier_cash_ledger` (`1790000000028`).
- **ADR stub:** `docs/adr/ADR-payments.md` (DRAFT, decision pending council).

---

## 1. Problem + non-goals

### Problem
Owner directive: accept **banking card, cash, and crypto**. Today the platform is **cash-only by
construction**: `payment_method` is a Postgres ENUM with the single value `'cash'`
(`1780310044710:15`), every order defaults to it (`1780310074262:34`), and the entire
order/courier/dispatch spine is built on **cash-as-proof** — delivery completion *is* cash collection
(`deliveryCompletion.ts:51-113`; ADR-deliver-v2). There is no PSP integration, no card-data path, no
inbound payment webhook.

Adding card/crypto is **not additive**: it forks the order lifecycle into **prepaid** (paid before
fulfillment) vs **pay-on-delivery** (today's cash-as-proof), and touches a 🔴 money/contract/migration
surface end-to-end. A naive "add `paid` to the cash completion" bolt-on produces the two worst
outcomes in research §3.10: couriers asked to collect cash on already-paid orders, or paid orders
that never reach a terminal state.

### Goals (v1 design target)
- A **provider-agnostic** payment seam so the **undecided Albania acquirer choice is not baked in**.
- A **prepaid lifecycle fork** that cleanly **skips the cash-proof completion gate** without
  weakening it for COD.
- A **money-correct ledger** (`payments` + `payment_events`) that is the reconciliation source of
  truth and **unifies with** the already-council-gated refund/contra model (Stage-21).
- **Card-first, behind a flag; crypto designed but deferred.** Schema rich, runtime minimal.

### Non-goals (explicit)
- **Not** choosing the Albania acquirer/PSP (NEEDS-HUMAN — gating fact).
- **Not** building production code, migrations, or a PSP adapter implementation.
- **Not** refactoring the authoritative order-total math at `orders.ts` create-path (ADR-0005 holds
  that as a 🔴 money red-line; out of scope).
- **Not** changing the COD cash-as-proof spine, the Stage-21 hold/contra model, or
  NO-AUTO-DEDUCT / NO-COURIER-SCORING invariants.
- **Not** an earnings/wage model (Stage-21 NG-1 stands).
- **Not** holding volatile crypto, becoming an MSB, or building managed fiat settlement.

---

## 2. Back-of-envelope

**Segment (verified from Stage-21 / market brief):** 1–5-person Albanian cash shops. Small N, low
per-shop throughput. Sizing for headroom, not hyperscale.

| Quantity | v1 estimate | Growth (12mo) |
|---|---|---|
| Active locations | 20–50 | 200 |
| Peak orders/min (busy dinner, all locations) | ~50–100 | ~400 |
| Peak orders/sec | ~1.7 | ~7 |
| Prepaid share (card+crypto), once launched | ~40–60% | ~60% |
| **Peak prepaid charges/sec** | **~1** | **~4** |
| Webhook events per charge (created→authorized→captured [+refund]) | 2–4 | 2–4 |
| **Peak inbound webhook events/sec** | **~3–4** | **~16** |
| Reconciliation: provider-settlement rows/day | ≤ orders/day (~10⁴) | ~10⁵ |

**Verdict:** trivial for one Postgres instance. The binding constraint is **NOT throughput — it is the
connection budget** (ADR-0001): Supabase floor allows ~60 pooled / ~15 direct; the confirmed budget is
**operational 8 + session 3 + pg-boss 3 = 14 peak**, "leaves room for transient migrations".

**Connection-budget rule for payments (load-bearing):**
- The **inbound webhook route** runs on the **existing operational pool (8)** — it is a normal
  request handler; no new pool.
- The **reconciliation job** and **capture-on-accept sweep** reuse the **existing advisory-lock cron
  pattern** (`order-timeout-sweep.ts`), **NOT a new pg-boss queue** — app roles cannot create pg-boss
  queues (ADR-deliver-v2 grounding; `assignments.ts:372-378`), and a new worker pool would blow the
  14-conn budget. **Zero new pools, zero new queues.** (Schema rich, runtime minimal.)

Money/currency: integer minor units only, currency per location (already true; `locations.currency_code`
/ `currency_minor_unit`, ADR-0005). No new currency model.

---

## 3. Options considered (≥2, with tradeoffs)

### Option A — Bolt "paid" onto the cash-as-proof completion (REJECTED)
Add `'card'`/`'crypto'` to `payment_method`; reuse `payment_outcome='paid_full'` at the door; mark
prepaid by setting cash_amount = total at checkout.
- **Concept:** minimal-diff / no fork.
- **Tradeoff:** directly hits research §3.10 — completion still routes through the cash HOLD primitive
  (`deliveryCompletion.ts:104-110`), so a paid card order writes a phantom courier till-debt; or the
  courier UI asks to collect cash on a paid order. Conflates *payment* (when money moved) with
  *fulfillment* (when food was handed over). Cannot model auth-vs-capture, refunds, or
  awaiting-confirmation. **Rejected — couples two state machines that must be decoupled.**

### Option B — Provider-agnostic port + decoupled `payment_status` fork (RECOMMENDED)
A `PaymentProvider` port (auth/capture/refund/verify-webhook) with cash/HPP/crypto adapters; a
**`payments` + `payment_events` ledger** as money SoT; a **`payment_status` state machine on the order
decoupled from `order_status`**; prepaid orders **skip the cash-proof gate**, COD unchanged.
- **Concept:** Ports & Adapters (hexagonal) + Outbox/webhook-as-SoT + state-machine decoupling +
  ledger/CQRS-read.
- **Tradeoff:** more schema and a real fork in `completeDelivery`; higher up-front design cost. But it
  is the only shape that holds the money red-line, isolates the undecided acquirer, and lets crypto's
  "awaiting confirmation" and refunds fit without re-cutting the spine. Schema lands inert behind flags.
- **This is the recommendation.**

### Option C — Outsource the whole order to a hosted checkout / marketplace PSP (REJECTED for v1)
Redirect the entire cart to a third-party hosted order+pay flow.
- **Concept:** maximal scope-shedding.
- **Tradeoff:** abandons the cash spine, courier dispatch, and tenant model that already exist; no such
  product reliably onboards Albania merchants for *food delivery* with our courier model; loses
  server-authoritative pricing (ADR-0005). **Rejected — throws away the working system.**

---

## 4. Decision (ADR-format → mirrored in `docs/adr/ADR-payments.md`)

**Adopt Option B.** Five load-bearing pieces:

### 4.1 The `PaymentProvider` port (hexagonal seam)
A single internal interface; **the acquirer choice lives behind it** (research §1, §3.9). Conceptual
shape (design, not code):

```
interface PaymentProvider {
  readonly id: 'cash' | 'albania-hpp' | 'crypto-noncustodial';
  readonly capabilities: { authThenCapture: boolean; refund: boolean; partialRefund: boolean };

  // Begin a charge. Returns a provider ref + (for redirect/HPP) a redirectUrl, or (crypto) an address.
  createCharge(in: { orderId; amountMinor; currency; idempotencyKey; returnUrl }): ChargeInit;

  // Capture a prior auth (no-op for purchase-only gateways; guarded by capabilities.authThenCapture).
  capture(in: { providerRef; amountMinor; idempotencyKey }): CaptureResult;

  // Refund (full/partial). Crypto: may be UNSUPPORTED → manual workflow.
  refund(in: { providerRef; amountMinor; idempotencyKey }): RefundResult;

  // PURE signature verification over the raw request body — NO DB, NO side effects.
  verifyWebhook(rawBody: Buffer, headers): { verified: boolean };
  // Map a verified provider event → our normalized vocabulary (idempotent, side-effect free).
  parseEvent(rawBody: Buffer): NormalizedPaymentEvent; // { providerEventId, type, amountMinor, currency }
}
```

**Adapters (v1 design):**
- **`CashAdapter` (no-op):** `createCharge` → `payment_status='unpaid'`, no redirect; `capture`/`refund`
  are no-ops; **money truth stays in `courier_cash_ledger` (cash-as-proof, untouched).** Cash never
  enters the `payments` ledger as a captured charge — the COD spine is authoritative for cash.
- **`AlbaniaHppAdapter` (redirect / hosted payment page):** SAQ-A. `createCharge` → redirect URL; the
  PAN goes browser→PSP only; we receive a **token + provider ref**. Capture timing per the chosen
  acquirer's capabilities (many Albanian virtual-POS are **purchase-only** → `authThenCapture:false`).
- **`CryptoNonCustodialAdapter` (e.g. self-hosted BTCPay):** `createCharge` → on-chain address +
  expiry; funds settle to **our wallet** (non-custodial → out of MSB scope, research §2-crypto);
  `refund` likely `UNSUPPORTED` → manual workflow (§7).

`NormalizedPaymentEvent.type ∈ { created | authorized | captured | failed | expired | refunded |
chargeback | underpaid | overpaid }`. The **provider** vocabulary never leaks past `parseEvent`.

### 4.2 Lifecycle fork — `payment_status` decoupled from `order_status`
A **new `payment_status` state machine** on the order, **independent of fulfillment state**
(`order_status`):

```
unpaid → pending → authorized → paid
              ↘ failed
   paid → refunded   (partial → stays paid with refunded_amount > 0; full → refunded)
```

- **COD (cash):** `payment_status = 'unpaid'` for its whole life. The cash-as-proof model is the
  authoritative money record; we do **not** transition COD to `paid` (avoids two competing truths for
  the same cash). Completion is unchanged.
- **Prepaid (card/crypto):** drives the full machine. Fulfillment is **gated** on payment:
  - Order created → `payment_status='pending'`, `order_status='PENDING'` **held** (not offered to
    courier, not acceptable until payment progresses).
  - Customer completes hosted payment → **webhook** moves `pending→authorized` (auth+capture gateways:
    straight to `paid`).
  - **Capture policy (NEEDS-HUMAN, §6):** *auth-then-capture-on-accept* → owner accepts (CONFIRMED)
    triggers `capture`; webhook `captured` → `paid` → fulfillment proceeds; owner reject → void auth →
    `failed`/`refunded`. *Purchase-only / immediate-capture* → `paid` at checkout; owner reject →
    auto-refund (`refunded`).
- **The fork at completion (the central red-line fix):** `completeDelivery` branches on
  `payment_method`:
  - **`cash`** → today's path **unchanged**: `paid_full` requires `cashAmount===total`
    (`deliveryCompletion.ts:59-61`), writes the `'hold'` (`:104-110`). No change.
  - **prepaid (`card`/`crypto`)** → the door tap is a **handover confirmation only**: order →
    `DELIVERED`, **NO cash coherence check, NO `courier_cash_ledger` hold** (the order is already paid;
    a prepaid order must never create a courier till-debt). The cash-tail outcomes still apply but
    re-mean: `refused_goods` / `customer_cancelled_on_door` → order `CANCELLED` **and trigger a
    provider refund** (§5/§7); `refused_payment` is **invalid for prepaid** (already paid) → rejected at
    the edge enum. *This requires a small, explicit branch + a distinct prepaid handover outcome — see
    §10 OPEN-1.*

The two machines are orthogonal: `order_status` = fulfillment, `payment_status` = money. The **only**
coupling points are two explicit gates: (a) prepaid fulfillment cannot start until `payment_status`
clears the capture-policy threshold; (b) prepaid completion skips the cash gate.

### 4.3 Data model (RLS FORCE, integer/minor, idempotent) — design only
Two new tenant-scoped tables, both **ENABLE + FORCE ROW LEVEL SECURITY** with the canonical
`location_id IN (SELECT app_member_location_ids())` policy + the **grant-mirror DO-block** pattern
(`1790000000028:30-43`):

- **`payments`** — one row per charge attempt for an order:
  `id, order_id (FK→orders ON DELETE CASCADE), location_id, provider text, method payment_method,
   status text CHECK(status IN ('unpaid','pending','authorized','paid','failed','refunded')),
   amount_minor integer CHECK(>=0), captured_amount_minor integer CHECK(>=0) DEFAULT 0,
   refunded_amount_minor integer CHECK(>=0) DEFAULT 0, currency_code text,
   provider_charge_ref text, idempotency_key text, created_at, updated_at`.
  Invariant (trigger, like Stage-21 residual-guard): `refunded_amount_minor <= captured_amount_minor`,
  `captured_amount_minor <= amount_minor`. **All money integer minor units; no float anywhere.**
- **`payment_events`** — append-only ledger (the money SoT, like `courier_cash_ledger`):
  `id, payment_id (FK), order_id, location_id, provider text, provider_event_id text,
   type text CHECK(type IN ('created','authorized','captured','failed','expired','refunded','chargeback','underpaid','overpaid')),
   amount_minor integer CHECK(>=0), currency_code text, signature_verified boolean NOT NULL,
   payload jsonb, created_at`.
  🔴 **Idempotency by DB unique constraint, NOT check-then-act:** `UNIQUE (provider, provider_event_id)`
  — insert-wins dedup for at-least-once webhook delivery (research §2/§3.3). Append-only:
  `BEFORE UPDATE` immutability trigger, **no DELETE clause** so `orders ON DELETE CASCADE` (GDPR
  hard-erase) survives — exactly the Stage-21 pattern (`ADR-stage21:117`).
  `payload` jsonb stores the **token/provider-ref only — never a PAN** (§4.5); claim-check-clean.

- **`payment_status` on `orders`:** a new column (additive, default `'unpaid'`). The **only writer of
  `paid`/`failed`/`refunded` is the webhook handler** (§4.4). A `'pending'`/`'authorized'` write may be
  set on the create-charge path.

**Enum expansion (forward-only, additive, operator-gated):**
`ALTER TYPE payment_method ADD VALUE 'card'; ALTER TYPE payment_method ADD VALUE 'crypto';`
- Additive and forward-only (a value can never be dropped — irreversible, but additive, so acceptable).
- 🔴 **Migration-author caveat (flag for operator):** `ALTER TYPE … ADD VALUE` **cannot run inside the
  same transaction** that then uses the new value, and node-pg-migrate wraps migrations in a txn by
  default — the ADD VALUE migration must be authored to run **outside a transaction** (or split). This
  is the same Postgres-enum-churn hazard that made deliver-v2 deliberately avoid `order_status` enum
  adds (ADR-deliver-v2 §6). Document it; do not surprise the operator at deploy.

**Unification with the refund/contra model (no duplicate ledger):**
- **Prepaid refund** = a provider refund → recorded as a `payment_events('refunded')` row + bump
  `payments.refunded_amount_minor`. **No courier cash is involved.**
- **COD refund** = the existing **Stage-21 `'release'` contra** on `courier_cash_ledger`
  (ADR-stage21 §3/§5) — unchanged. A pre-delivery cancel still runs before any hold
  (`customer/orders.ts:307-326`).
- The **refund router** keys on `payment_method`: prepaid → provider refund (payments ledger); cash →
  Stage-21 contra. One refund *concept*, two *backing ledgers*, each authoritative for its money type.
  Reporting reads both. This is the "unify, don't duplicate" of research §2.

### 4.4 Money-correctness path
- 🔴 **Webhook is the source of truth, not the client redirect** (research §2/§3.2). New route
  `POST /webhook/payments/:provider` (modeled on `telegram-webhook.ts` conventions: raw body,
  always-200-to-stop-retries for *handled* events, `set_config('app.current_tenant', …, true)` for RLS),
  **but signature-verified** (`provider.verifyWebhook(rawBody, headers)` — full HMAC/signature, not the
  Telegram secret-equality shortcut). Flow:
  1. `verifyWebhook` (pure) — reject unsigned/forged → 401.
  2. `parseEvent` → `INSERT INTO payment_events … ON CONFLICT (provider, provider_event_id) DO NOTHING`
     (**insert-wins, NOT check-then-act** — kills the duplicate-webhook race, research §3.3).
  3. **Only if rowcount=1** (first time we see this event): in the **same transaction**, apply the
     guarded `payment_status` transition (rowcount>0 status-guarded UPDATE, like `updateOrderStatus`)
     and, for prepaid, release the fulfillment gate. Duplicate deliveries no-op.
- 🔴 **Idempotency keys on charge/refund:** client-generated key on every `createCharge`/`refund`,
  persisted insert-wins (reuse the `idempotency_keys` table pattern at `orders.ts:365-380`, or a
  payment-scoped twin). Prevents double charges on retry/timeout (research §3.3).
- **Capture-timing policy = NEEDS-HUMAN (§6).** The port supports both via `capabilities.authThenCapture`;
  recommendation **auth-then-capture-on-accept** (lower refund volume, mirrors the "venue commits before
  money is taken" ethos of the cash spine) — **but** if the contracted Albanian acquirer is purchase-only
  (common for legacy virtual-POS), fall back to immediate-capture + auto-refund-on-reject. The design is
  capability-driven so either works without re-cutting the schema.
- **Reconciliation job** (daily, advisory-lock cron — **no new pool/queue**, §2): pull the provider
  settlement report, match against `payment_events` by `provider_charge_ref`/`provider_event_id`; classify
  drift (partial capture, refund timing, multi-currency, missing settlement) as **owner-review facts**,
  **never auto-adjust** — mirrors the Stage-21 **NO-AUTO-DEDUCT** invariant (`ADR-stage21:82`). Surfaces a
  `payment_reconciliation_drift` counter in <1 min (observability gate, §9).
- 🔴 **Never touch the PAN (SAQ-A):** hosted fields (iframe) or full redirect/HPP only; the browser sends
  the PAN straight to the PSP; we store a **token**, never card data (research §2/§3.1). **PCI DSS v4.0.1
  req 6.4.3 + 11.6.1:** inventory + justify + change-monitor every third-party script on the checkout page
  (CSP + SRI + a script manifest). This is a real checkout-page task, not just a backend concern.

### 4.5 Crypto specifics
- **Stablecoin-only vs auto-convert = NEEDS-HUMAN** (§6). Default recommendation: **stablecoin-only
  (USDC/USDT, e.g. USDT-TRC20 low fee)** to avoid holding volatile coin (research §2/§3.5); auto-convert
  if the operator wants fiat settlement.
- 🔴 **Non-custodial only** (funds → our wallet directly) to **stay out of MSB / KYC-AML scope**
  (research §2/§3.6). A custodial gateway is out of scope for v1.
- **"Awaiting confirmation" state:** crypto confirmation latency (1–15 min) maps to
  `payment_status='pending'` + the fulfillment gate **held** — **never dispatch before on-chain
  confirmation**. The `created`→`authorized`/`paid` transition is the confirmation webhook.
- **Under/over/late payment:** `payment_events` types `underpaid`/`overpaid`; both → **owner-review
  facts** (no auto-adjust), late confirmation handled by the same idempotent webhook ingestion after
  expiry.
- 🔴 **Irreversible refunds:** confirmed crypto has no chargebacks (fraud-resistant) but refunds are
  **manual, bespoke** (`CryptoNonCustodialAdapter.refund` may be `UNSUPPORTED`) → a defined manual
  workflow recorded as a `payment_events('refunded')` fact; **refund-in-crypto-vs-fiat, valuation, and
  timing are NEEDS-HUMAN** (§6).

### 4.6 v1 scope recommendation
- **v1 = card-first, behind flags; cash unchanged (default); crypto designed-but-dark.**
  - `PAYMENTS_PREPAID_ENABLED` (default OFF) gates the whole prepaid fork at runtime.
  - `PAYMENTS_PROVIDER` selects the adapter (`cash` only until an acquirer is contracted).
  - `PAYMENTS_CRYPTO_ENABLED` (default OFF) — schema + adapter land inert; launched separately later.
- **Schema rich, runtime minimal:** the tables, the enum values, and the `payment_status` column land
  **inert**; no behavior changes until a flag flips. COD is wholly untouched in the dark state.

---

## 5. Consistency + idempotency (summary)
- **Idempotent webhook ingestion:** `UNIQUE (provider, provider_event_id)` insert-wins; state transition
  only on rowcount=1, in the same txn. At-least-once delivery → duplicates are no-ops.
- **Idempotent charge/refund:** client idempotency key, insert-wins (`idempotency_keys` pattern).
- **Status-guarded transitions:** every `payment_status` / `order_status` write is a rowcount>0 guarded
  UPDATE (the project's existing invariant; `updateOrderStatus`), so concurrent webhook + owner-action
  serialize.
- **CAP/consistency stance:** money is **CP** — the webhook write + status transition are one local
  Postgres transaction (single-node, no cross-service distributed commit). The client redirect is
  **advisory only**; truth is the webhook. No eventual-consistency window is ever fulfilled on.

---

## 6. Failures + degradation (every external call: timeout + fallback, no cascade)
- **`createCharge` (PSP) times out / 5xx:** the order stays `payment_status='pending'`,
  `order_status='PENDING'` (held); the customer sees "payment not completed, retry" — **never** a
  fulfilled unpaid order. Idempotency key makes the retry safe. Bounded timeout; circuit-breaker on the
  adapter so a dead PSP does not cascade into checkout latency.
- **`capture` / `refund` (PSP) fails:** record the attempt; surface to owner-review; **retry via the
  advisory-lock cron** (bounded, idempotent), not a tight loop. A stuck capture never blocks the cash
  path (different `payment_method`).
- **Webhook never arrives (lost):** the reconciliation job is the backstop — it finds settled-but-not-
  webhooked charges and replays the transition idempotently. **No fulfillment ever depends on the
  redirect alone.**
- **Webhook arrives but order gone (GDPR cascade-deleted):** event still inserts (or no-ops); the
  transition is a guarded UPDATE that hits 0 rows → recorded, no crash.
- **PSP totally down:** `PAYMENTS_PROVIDER` degrades — the storefront falls back to **cash-on-delivery
  only** (the always-available path). Prepaid options hidden; no checkout outage. This is the
  failure-first default: COD is the floor that never depends on any external system.
- **Crypto under/over/late:** owner-review fact, never auto-fulfilled, never auto-refunded.
- **No cascade:** each external call is bounded + isolated behind the adapter; a PSP/chain failure
  degrades prepaid to cash, it never takes down orders, courier dispatch, or the cash ledger.

---

## 7. Security + tenant isolation
- 🔴 **No PAN on our servers** — SAQ-A hosted-fields/redirect; we store tokens + provider refs only;
  PCI v4.0.1 script-monitoring on checkout (§4.4).
- 🔴 **RLS ENABLE + FORCE** on `payments` and `payment_events` with `app_member_location_ids()` tenant
  policy + grant-mirror DO-block (`1790000000028:30-43`). The webhook handler sets
  `app.current_tenant` before writing (Telegram pattern). Note (honesty, per Stage-21 §6): FORCE is
  inert against a live **BYPASSRLS** writer; the real cross-tenant closure depends on the **B3
  NOBYPASSRLS** work — keep the policy missing-GUC-tolerant so it is ready when B3 lands.
- **Webhook auth = cryptographic signature** over the raw body (`verifyWebhook`), per-provider secret in
  env (no secret in git, SOPS pattern). Not the Telegram secret-equality shortcut.
- **No PII to crypto/PSP beyond necessity; no PII in `payment_events.payload`** beyond token/provider-ref
  (claim-check discipline). JWT RS256, zero cookies — unchanged.
- **Refund authority:** owner-only; a refund is a guarded, idempotent, logged action; for COD it routes
  through the Stage-21 contra (owner-confirmed), never an auto-deduct.

---

## 8. Operability
- **Health:** a `degraded` (PSP unreachable → prepaid hidden, cash works) vs `down` distinction in
  `/health` — prepaid being unavailable is **degraded, not down** (orders still flow via cash).
- **Observability (<1 min):** counters for `payment_charge_created/authorized/captured/failed`,
  `webhook_duplicate_dropped`, `payment_reconciliation_drift`, `prepaid_fulfillment_gate_held`. Drift and
  gate-stuck are the alert signals.
- **Rollback:** schema is additive/inert; the runtime is flag-gated (`PAYMENTS_PREPAID_ENABLED`) — flip
  off to fully revert to cash-only with zero data loss. Enum values cannot be dropped (forward-only) but
  are harmless when no order uses them.
- **Scaling-gate / flag:** launch prepaid per-location or globally via the flag once an acquirer is
  contracted and the checkout PCI-script monitoring is in place.

---

## 9. Migrations (forward-only, additive, RLS FORCE, integer) — design intent only
1. `ALTER TYPE payment_method ADD VALUE 'card'` / `'crypto'` — **outside a transaction** (enum caveat,
   §4.3); forward-only, irreversible-but-additive; **operator-gated**.
2. `payments` + `payment_events` tables — ENABLE + FORCE RLS, tenant policy, grant-mirror, integer
   CHECK(>=0) money, append-only immutability trigger on `payment_events` (BEFORE UPDATE, no DELETE),
   residual/monotonicity triggers on `payments`.
3. `orders.payment_status` column — additive, default `'unpaid'`, no backfill needed (default is correct
   for all existing COD orders).
4. **No `order_status` enum churn.** **No physical rename.** **No change to** `courier_cash_ledger`,
   `delivery_trace`, or the cash completion path's existing branches.
All trivially revertible pre-launch (drop tables/column; enum values inert). Operator-gated apply on
staging-first per Ship Discipline.

---

## 10. Open risks + NEEDS-HUMAN (owner)

**OPEN (architecture-owned, resolve in council/RESOLVE):**
- **OPEN-1 — prepaid completion outcome (Architect):** the door tap for a prepaid order needs a distinct
  outcome (handover-confirmed, no cash) so `completeDelivery` does not reuse cash semantics. Either add a
  `prepaid_delivered` value to the courier completion outcome set, or branch on `payment_method` and pass
  `cash_amount=null` with the hold suppressed. Must not weaken the cash coherence check for COD. **Design
  decision for the deliver-v2 owner; carries the money red-line.**
- **OPEN-2 — refund of a prepaid order whose food was already dispatched/refused:** the `refused_goods` /
  `customer_cancelled_on_door` tail must trigger a provider refund; define the trigger point and the
  partial vs full policy. Ties to OPEN-1 and §4.3 unification.
- **OPEN-3 — tip + delivery-fee handling for prepaid:** today the courier "collect: total" door figure is
  cash (ADR-0005 §6); for prepaid there is nothing to collect. Confirm the prepaid courier screen shows
  "PAID — hand over, collect nothing" and that the Stage-21 delivery-fee bundling question
  (ADR-stage21 NEEDS-HUMAN #2) is unaffected (prepaid writes no courier till-debt).

**NEEDS-HUMAN (gating / business / legal — owner decision, not architecture):**
- **NH-1 (GATING) — Albania acquirer/PSP:** which acquirer can the operator actually contract (BKT /
  Raiffeisen / Credins / Intesa virtual-POS, vs 2Checkout/Verifone / PayPal)? Decides
  auth-vs-purchase capability, fees, 3DS support, payout constraints. **Everything else waits on this.**
- **NH-2 — Capture policy:** auth-then-capture-on-accept vs immediate-capture + auto-refund-on-reject
  (constrained by NH-1's gateway capability).
- **NH-3 — Crypto stance for v1:** support at all? Non-custodial (recommended) confirmed? Stablecoin-only
  vs auto-convert? Off-ramp/settlement choice?
- **NH-4 — Refund policy + who bears PSP fees** (merchant vs customer), and the **irreversible-crypto
  refund** workflow (crypto vs fiat, valuation, timing).
- **NH-5 — Albania legal/tax/AML/e-invoice** obligations for online card + crypto acceptance.
- **NH-6 — v1 launch scope confirmation:** card-first behind flag, crypto deferred (Architect
  recommendation) vs all three at once.

**ACCEPTED risks (with owner):**
- Mirror/denormalized `orders.payment_status` can drift from `payments` SoT → bounded by the rule that
  only the webhook writes terminal states + reconciliation backstop (Owner: API/eng).
- Enum values are forward-only/irreversible (Postgres) → accepted; additive only (Owner: Data).
- FORCE-RLS on payment tables is defense-in-depth until B3 NOBYPASSRLS lands → accepted, GUC-ready now
  (Owner: security, dependency B3).
