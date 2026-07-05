# ADR-0017 — Payments (crypto-first, non-custodial · Plisio); cash unchanged; card deferred

- **Status:** 🟢 **APPROVED** (Triadic Council converged + RESOLVE round + operator decisions 2026-06-30).
  Build **schema-rich / runtime-minimal, DARK** behind `PAYMENTS_PREPAID_ENABLED` + `PAYMENTS_CRYPTO_ENABLED`
  (both default OFF). The launch flip is gated on the residual NEEDS-HUMAN below. Design-time artifact — code
  + migrations are a separate, operator-gated act under Ship Discipline.
- **Date:** 2026-06-30
- **Red-line:** 🔴 MONEY · 🔴 RLS · 🔴 MIGRATION (forward-only) · 🔴 PCI (no PAN on our servers)
- **Resolution:** `docs/design/payments/resolution.md` § "RESOLVE round (Plisio, crypto-first)" (the
  build-ready C1/C2/C3 + H1/H2/H3 + crypto-specific resolutions; this ADR records the decisions).
- **Proposal:** `docs/design/payments/proposal.md`
- **Research:** `docs/design/payments/research.md`
- **Bound by / extends:** `ADR-deliver-v2-cash-as-proof.md` (the cash-as-proof completion + `'hold'`
  primitive — **unchanged**), `ADR-stage21-reconciliation.md` (refund/`'release'` contra,
  **NO-AUTO-DEDUCT / NO-COURIER-SCORING** — preserved, the prepaid refund unifies with it),
  `ADR-0005-delivery-fee-source-of-truth.md` (server-authoritative integer money), `0001-queue-in-postgres.md`
  (connection budget — no new pool/queue).
- **Supersedes:** the deliver-v2 §D "card seam stays explicit and unbuilt" note (`ADR-deliver-v2:117-119`)
  — this ADR is that seam's design.

## Context

Cash-only by construction: `payment_method` ENUM = `('cash')` (`1780310044710:15`); the order/courier
spine is cash-as-proof (`deliveryCompletion.ts`). Adding card/crypto **forks** the lifecycle into
**prepaid** (paid before fulfillment) vs **pay-on-delivery** (today's cash). The undecided fact is the
**Albania acquirer/PSP** (NEEDS-HUMAN) — the architecture must keep that choice un-baked behind one port.

## Operator decision (RESOLVE round, ratified)

**Crypto-first, non-custodial, provider = Plisio** (hosted non-custodial, funds **direct to the merchant
wallet**, **USDT-TRC20 + USDC**, **stablecoin-only**, signature-verified HMAC webhook). **Card deferred**
(`AlbaniaHppAdapter` designed-but-unbuilt; NH-1 acquirer open for a later round). `PAYMENTS_PROVIDER=plisio`
selects `CryptoNonCustodialAdapter` behind the unchanged port. Crypto is the **first** prepaid rail, so the
Counsel crypto-STOP (irreversibility disclosure + written refund SLA) is **live** and gates the launch flip.

## Decision (APPROVED)

1. **Provider-agnostic `PaymentProvider` port** (createCharge/capture/refund/verify-webhook) with adapters:
   `CashAdapter` (no-op; money truth stays in `courier_cash_ledger`), `AlbaniaHppAdapter`
   (SAQ-A redirect/HPP — **deferred, unbuilt**), `CryptoNonCustodialAdapter` (**Plisio**, non-custodial,
   awaiting-confirmation; `refund` = **UNSUPPORTED** → manual owner-review). Provider vocabulary never leaks
   past `parseEvent`. **Acquirer choice is not baked in.**
2. **`payment_status` state machine decoupled from `order_status`:**
   `unpaid → pending → authorized → paid → refunded` (+ `failed`). **COD stays `unpaid`** (cash spine
   authoritative, unchanged). **Prepaid** drives the full machine; fulfillment is **gated** on payment
   and **completion skips the cash-proof gate** (no cash check, no `'hold'` — a paid order never creates
   a courier till-debt).
3. **`payments` + `payment_events` ledger** — ENABLE + FORCE RLS, grant-mirror, **integer minor units**,
   append-only events with `UNIQUE (provider, provider_event_id)` **insert-wins** idempotency
   (NOT check-then-act). The money SoT; **unifies** with Stage-21: prepaid refund → provider refund
   (payments ledger); COD refund → Stage-21 `'release'` contra (`courier_cash_ledger`). One refund
   concept, two backing ledgers, routed by `payment_method`.
4. **Webhook is the source of truth, not the client redirect** — signature-verified, idempotent;
   the **only writer of `paid`/`failed`/`refunded`**. Client-idempotency keys on charge/refund.
   Reconciliation cron (advisory-lock, **no new pool/queue**) is the backstop; drift = **owner-review,
   never auto-adjust** (mirrors NO-AUTO-DEDUCT). **Never touch the PAN** (SAQ-A) + PCI v4.0.1
   script-monitoring on checkout.
5. **Crypto:** non-custodial only (out of MSB scope), stablecoin-only or auto-convert (NEEDS-HUMAN),
   "awaiting confirmation" = `pending` + held fulfillment, under/over/late = owner-review,
   irreversible refunds = defined manual workflow.
6. **v1 = card-first behind `PAYMENTS_PREPAID_ENABLED` (default OFF); cash unchanged; crypto designed
   but dark.** Schema rich, runtime minimal — tables/enum-values/column land inert.

## Red lines preserved

Money integer-only `CHECK(>=0)`, no float · RLS ENABLE + FORCE + grant-mirror on every new payment table ·
migrations forward-only/additive/operator-gated (enum `ADD VALUE` outside-txn caveat noted) · **no PAN on
our servers** (SAQ-A) · **cash-as-proof spine and `'hold'` primitive untouched** · NO-AUTO-DEDUCT /
NO-COURIER-SCORING intact · webhook-as-SoT + insert-wins idempotency · claim-check (no PII/PAN in
`payment_events.payload`) · no new connection pool / pg-boss queue (ADR-0001 budget).

## Resolved (RESOLVE round → resolution.md)

- **OPEN-1 / C1 — RESOLVED.** `delivered_prepaid` outcome in `completeDelivery`: skips the cash assert, writes
  **no** courier `'hold'`; precondition `orders.payment_status='paid'` (409 `PREPAID_NOT_PAID` otherwise).
  Cash path untouched.
- **OPEN-2 / C2 — RESOLVED (reframed for irreversible crypto).** No auto provider refund. Refused/cancelled
  tail appends a `payment_events('refund_due')` **owner-review obligation**; owner sends crypto back
  out-of-band then records `refund_sent` → `refunded` (sticky). Unifies with Stage-21 NO-AUTO-DEDUCT.
- **OPEN-3 — RESOLVED.** Prepaid courier screen = "PAID — hand over, collect nothing"; the immutable
  `delivery_trace` (`payment_outcome='delivered_prepaid'`) is the dignified proof-of-handover (Counsel §5).
  No courier till-debt; delivery-fee/tip bundling (Stage-21 NH#2) unaffected.
- **C3 — RESOLVED.** Plisio HMAC-verified webhook; tenant via SECURITY DEFINER `payment_location_by_provider_ref`
  → `set_config('app.current_tenant')`; RLS policy admits **both** member reads **and** the GUC writer
  (`WITH CHECK` on `app.current_tenant`); insert-wins `UNIQUE(provider,provider_payment_id,type)`. **Depends on
  B3 (NOBYPASSRLS+GUC)** — stated, not bypassed.
- **H1/H2/H3 — RESOLVED.** Minute-cadence advisory-lock poll for `pending` prepaid (no new pool/queue);
  status-guarded monotonic terminal states; Plisio HTTP never holds an operational-pool connection.
- **NH-1 acquirer / NH-2 capture / NH-3 crypto stance / NH-6 scope — DECIDED by operator** (crypto-first,
  non-custodial Plisio, stablecoin-only, card deferred).

## Residual NEEDS-HUMAN (launch-gate, not build blockers)

- **NH-RES-1** off-ramp (USDT-TRC20 Binance P2P / USDC regulated rail) — merchant treasury op, gates cash-out.
- **NH-RES-2** refund-SLA copy + value of **Y** + irreversibility disclosure copy — gates the
  `PAYMENTS_CRYPTO_ENABLED` flip (Counsel ETHICAL-STOP-1).
- **NH-RES-3** non-custodial wallet-key custody procedure (backup / multisig / hardware) — gates launch.
- **Carried:** NH-5 Albania legal/tax/AML/e-invoice for crypto (gating a real consumer launch);
  NH-1 acquirer reopens for the deferred card round.

## Consequences

**Positive:** holds the money red-line (separate ledger, never re-routes paid orders through the cash
HOLD); acquirer choice isolated behind one port; crypto fits without re-cutting the spine; additive/inert
schema + flag → zero-loss rollback to cash-only; reuses existing patterns (idempotency_keys, grant-mirror,
advisory-lock cron, Stage-21 contra) — zero new pools/queues.

**Negative / accepted:** a real fork in `completeDelivery` (OPEN-1); `orders.payment_status` is a
denormalized mirror that can drift (bounded by webhook-only-writer + reconciliation); enum values are
forward-only/irreversible (additive only); FORCE-RLS is defense-in-depth until B3 NOBYPASSRLS lands.

**APPROVED — build-ready, dark.** The council converged, the RESOLVE round (resolution.md) closed C1/C2/C3 +
H1/H2/H3 + the crypto-specifics (await-confirmation, mismatch=owner-review, reorg/depeg/key-custody accepted
or NEEDS-HUMAN), and the operator decided crypto-first / non-custodial Plisio / stablecoin-only / card
deferred. The three residual NEEDS-HUMAN (NH-RES-1..3) gate the **flag flip to a real customer**, not the
schema-rich/runtime-minimal build. Cash spine untouched; cash stays the default + failure-first floor.
