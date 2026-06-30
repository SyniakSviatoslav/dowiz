# ADR — Payments (card · cash · crypto)

- **Status:** 🟡 **DRAFT — decision pending Triadic Council** (design-time; NO production code, NO migrations)
- **Date:** 2026-06-30
- **Red-line:** 🔴 MONEY · 🔴 RLS · 🔴 MIGRATION (forward-only) · 🔴 PCI (no PAN on our servers)
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

## Decision (DRAFT — to be ratified/amended by council)

1. **Provider-agnostic `PaymentProvider` port** (auth/capture/refund/verify-webhook) with adapters:
   `CashAdapter` (no-op; money truth stays in `courier_cash_ledger`), `AlbaniaHppAdapter`
   (SAQ-A redirect/HPP), `CryptoNonCustodialAdapter` (non-custodial, awaiting-confirmation). Provider
   vocabulary never leaks past `parseEvent`. **Acquirer choice is not baked in.**
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

## Open / NEEDS-HUMAN (see proposal §10)

- **OPEN-1** prepaid completion outcome (distinct from cash `paid_full`; carries money red-line).
- **OPEN-2** prepaid refund trigger on refused/cancelled-on-door tail.
- **OPEN-3** prepaid courier "collect nothing" screen + delivery-fee/tip handling.
- **NH-1 (GATING)** Albania acquirer/PSP · **NH-2** capture policy · **NH-3** crypto stance ·
  **NH-4** refund policy + fee bearer + irreversible-crypto workflow · **NH-5** Albania legal/tax/AML/
  e-invoice · **NH-6** v1 scope confirmation.

## Consequences

**Positive:** holds the money red-line (separate ledger, never re-routes paid orders through the cash
HOLD); acquirer choice isolated behind one port; crypto fits without re-cutting the spine; additive/inert
schema + flag → zero-loss rollback to cash-only; reuses existing patterns (idempotency_keys, grant-mirror,
advisory-lock cron, Stage-21 contra) — zero new pools/queues.

**Negative / accepted:** a real fork in `completeDelivery` (OPEN-1); `orders.payment_status` is a
denormalized mirror that can drift (bounded by webhook-only-writer + reconciliation); enum values are
forward-only/irreversible (additive only); FORCE-RLS is defense-in-depth until B3 NOBYPASSRLS lands.

**Decision pending council.** This stub records the proposed shape; the council ratifies/amends, then the
RESOLVE round addresses OPEN-1..3 and the human records NH-1..6 before any code.
