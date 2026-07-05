# Payments — Council Resolution (synthesis)

**Seats:** Architect (`proposal.md` + `docs/adr/ADR-payments.md` DRAFT) · Breaker (`breaker-findings.md`,
15 findings) · Counsel (`counsel-opinion.md`, 3 ETHICAL-STOPs). Research: `research.md`.

## Verdict — ARCHITECTURE SOUND, NOT YET BUILD-READY (NOT-YET-CONVERGED)
The shape is right and all three seats agree on it: **`payment_status ⊥ order_status`** (prepaid forks from
the cash-as-proof spine, COD untouched), a **provider-agnostic port** (the undecided Albania acquirer isn't
baked in), **webhook-as-source-of-truth + insert-wins idempotency**, **no-PAN/SAQ-A**, **FORCE-RLS ledger**,
**non-custodial crypto**, and **v1 = card-first behind `PAYMENTS_PREPAID_ENABLED` (default OFF), crypto dark**.
Counsel calls the `⊥` "ethically literate" — the aesthetic and the ethic coincide.

But the Breaker proved **3 CRITICALs** that mean the design as-written cannot be built safely. So this is **not
an APPROVAL to code** — it is an approved *direction* with a mandatory RESOLVE round + human inputs first.

## The 3 criticals — resolve-direction (design-time, before any code)
- **C1 (prepaid completion blocked / phantom till-debt).** The fix the proposal deferred (OPEN-1) must be IN
  the design: `completeDelivery` gets a distinct **prepaid terminal outcome** (e.g. `delivered_prepaid`) that
  (a) does NOT assert `cashAmount===total` and (b) writes NO courier `'hold'` till-debt. The cash `paid_full`
  path is untouched. This is the single load-bearing change to a 🔴 primitive — it cannot stay an OPEN.
- **C2 (charged-then-refused keeps the money).** The non-delivered prepaid terminal tail (refused/cancelled
  at door) MUST emit a **provider refund command** (via the payments ledger), with a defined trigger point and
  an idempotent refund. No prepaid terminal state may leave the customer charged with goods undelivered.
- **C3 (webhook can't write under the chosen RLS without BYPASSRLS).** The two tenancy schemes must be
  reconciled on the payments tables: a policy that admits BOTH member reads (`app_member_location_ids()`) AND
  the **unauthenticated webhook writer** — i.e. an explicit `WITH CHECK` keyed on `app.current_tenant` set from
  the order's location (telegram-webhook pattern), NOT a fallback to a BYPASSRLS role. Depends on B3
  (NOBYPASSRLS+GUC) — already a known hard dependency.

## HIGHs to fold into the resolve
- **H1** lost-webhook backstop is daily vs a minutes SLA → add a **fast reconcile/poll for `pending` prepaid
  orders** (minute-cadence or provider-status poll), not just the daily cron.
- **H2** out-of-order refund-before-capture → terminal **`refunded` must be sticky/monotonic** (a recorded
  refund can never be overtaken by a later `captured`→`paid`).
- **H3** `createCharge` on the 8-conn operational pool blows the 14-conn budget → keep the charge **off the
  server hot path** (redirect/HPP so the browser→PSP, server only ingests the webhook) or give the outbound a
  bounded dedicated path; do not saturate the operational pool.

MEDs/LOWs (M1 idempotency fingerprint, M2 raw-payload PII filter, M3 overloaded `unpaid`, M4 partial capture,
M5 crypto reorg, L1–L4) → addressed in the resolve or explicitly accepted with reason.

## Counsel ETHICAL-STOPs (friction, gating launch flips — already flag-dark, so cost ≈ 0)
1. No **crypto** consumer launch without honest irreversibility disclosure + a written refund SLA.
2. No **card** launch without a recorded **PSP-fee-bearer** decision; the **courier never bears it**.
3. Latent Charter line: the platform must **not degrade or default-away-from cash** to chase its own PSP
   volume (serves-the-unbanked). Not currently crossed — recorded.
Plus: **non-custodial-only** locked (no customer KYC for a meal); and Counsel's unasked question —
**what dignified, dispute-proof proof-of-delivery does the courier hold once the cash handshake is gone?** —
must be answered before COD is ever de-emphasised.

## NEEDS-HUMAN (gates BOTH rails — nothing builds until these land)
- **NH-1 Albania acquirer/PSP** the operator can actually contract (the gating fact).
- **NH-2 capture policy** (auth-then-capture-on-accept vs immediate+auto-refund).
- **NH-3 crypto stance** (in v1 at all? stablecoin-only? non-custodial confirmed? — recommend: dark in v1).
- **NH-4 refund + fee-bearer policy** (incl. irreversible-crypto workflow).
- **NH-5 Albania legal/tax/AML/e-invoice** for online card + crypto.
- **NH-6 v1 scope** — recommend **card-first behind the flag; cash unchanged; crypto dark**.

## Recommendation
1. **Do not code yet.** Get NH-1/NH-2/NH-6 from the operator first (acquirer choice changes the adapter).
2. Then run **one RESOLVE round** that bakes C1/C2/C3 + H1/H2/H3 into the proposal and flips the ADR from
   DRAFT → APPROVED.
3. Build **card-first behind `PAYMENTS_PREPAID_ENABLED` (default OFF)**, schema-rich/runtime-minimal, cash
   spine untouched; crypto stays dark behind its own flag pending NH-3/NH-5 + the Counsel crypto-STOP.

---

# RESOLVE round (Plisio, crypto-first) — 2026-06-30

**Operator decision (supersedes NH-1/NH-2/NH-3/NH-6 and the card-first recommendation):**
**crypto-first, non-custodial, provider = Plisio** (hosted non-custodial, funds settle **direct to the
merchant wallet**, **USDT-TRC20 + USDC**, **stablecoin-only**, signature-verified HMAC webhook). **Card is
deferred** (the `AlbaniaHppAdapter` stays designed-but-unbuilt; NH-1 acquirer remains open for a later round).
`PaymentProvider` port is **unchanged** — `PAYMENTS_PROVIDER=plisio` selects the `CryptoNonCustodialAdapter`;
the seam is exactly what the council approved. Crypto is now the **first** prepaid rail launched, so the
Counsel crypto-STOP (irreversibility disclosure + written refund SLA) is **live, not future**, and is baked
in below. This section makes the design **build-ready**: each CRITICAL/HIGH/crypto-specific is resolved to
inputs→state, grounded in live code. **Still design-only — no production code, no migration files.**

This RESOLVE assumes **B3 (NOBYPASSRLS + GUC tenancy)** as the security closure for C3 (stated, not bypassed).

## C1 (RESOLVED) — `delivered_prepaid` outcome in `completeDelivery` (the load-bearing fix)

Add a **distinct terminal outcome** to the single completion primitive
(`apps/api/src/lib/deliveryCompletion.ts`). Extend the `PaymentOutcome` union (`:12-16`) with
**`delivered_prepaid`**. Cash path (`paid_full` + the no-cash refusal tail) is **byte-for-byte untouched**.

**Inputs (`CompleteDeliveryArgs`):** for `delivered_prepaid` the caller's Zod **forbids `cashAmount`**
(must be `null`); `total` is carried for the `delivery_trace` crumb only.

**Caller routing (server-authoritative).** Both callers (courier `delivered` handler + owner-proxy
`/deliver`) branch on `orders.payment_method`:
- `cash` → today's outcomes (`paid_full` / `refused_goods` / `customer_cancelled_on_door` /
  `refused_payment`) — **unchanged**.
- `card`/`crypto` (prepaid) → the door tap is a **handover confirmation only** → `delivered_prepaid`.
  `refused_payment` is **invalid for prepaid** (money already moved) → rejected at the edge enum.

**Precondition (replaces the cash coherence gate).** `delivered_prepaid` requires
`orders.payment_status = 'paid'` (Plisio `completed`, server-authoritative). If not paid → **409
`PREPAID_NOT_PAID`** before any mutation. This is the prepaid mirror of `cashAmount===total`: for cash,
coherence = full cash in hand; for prepaid, coherence = the **payments ledger says captured**. A courier
can never mark "delivered_prepaid" an unpaid order.

**State produced (the fix):**

| `paymentOutcome` | cash assert | assignment | order_status | `cash_collected` | `cash_amount` col | courier `'hold'` |
|---|---|---|---|---|---|---|
| `paid_full` (cash) | **yes** (`===total`, `:59-61`) | `delivered` | `DELIVERED` | `true` | `=total` | **yes** (`:104-110`) |
| **`delivered_prepaid`** | **NO** | `delivered` | `DELIVERED` | `false` | `null` | **NONE** |
| `refused_goods` / `customer_cancelled_on_door` (prepaid) | no | `cancelled` | `CANCELLED` | `false` | `null` | **NONE** → C2 refund obligation |

The branch is small and explicit: `isPaidFull` stays the **only** condition that (a) runs the cash assert
and (b) writes the `'hold'` (`:105`). `delivered_prepaid` falls through both — **no phantom till-debt on an
already-paid order**, exactly the C1 harm. `delivery_trace` records `payment_outcome='delivered_prepaid'`,
`cash_amount=null` (the dignified prepaid proof-of-handover crumb — answers Counsel §5: the courier holds an
immutable, signed, server-stamped handover trace, not a cash receipt).

## C2 (RESOLVED) — crypto is IRREVERSIBLE → owner-review manual refund, NOT an auto provider refund

**Reversal of the council's original C2 direction.** The old C2 said "emit a provider refund command."
**For crypto that is wrong** — confirmed on-chain stablecoin has **no chargeback and no provider clawback**;
Plisio cannot reverse a settled payment, and funds are already in the merchant wallet. So there is **no
`provider.refund()` call** on the crypto path; `CryptoNonCustodialAdapter.refund` is **`UNSUPPORTED`**.

**The non-delivered prepaid terminal tail (refused/cancelled-at-door) = recorded obligation + manual send:**
1. `completeDelivery` runs the refusal tail (order `CANCELLED`, no hold — per C1). **In the same caller
   transaction** it appends a **`payment_events` row of type `refund_due`** (amount = captured minor units) —
   an **owner-review FACT**, not an executed refund. `payments.refunded_amount_minor` is **not** bumped yet.
2. The owner sees a **refund-obligation queue** (owner-only, RLS member-scoped). The owner sends the
   stablecoin back **out-of-band from the merchant wallet** (we never hold keys → we cannot send for them).
3. The owner records completion via an **owner-only authenticated action** → appends `refund_sent` →
   bumps `refunded_amount_minor` (≤ captured) → sets `payment_status='refunded'` (sticky, H2).

This **unifies with Stage-21 NO-AUTO-DEDUCT**: the platform never moves money automatically; it records the
obligation and surfaces it for a human. Mirrors the cash `'release'` contra (owner-confirmed), routed by
`payment_method`: cash → Stage-21 contra; crypto → `refund_due`/`refund_sent` events. **No prepaid terminal
state leaves the customer charged with the obligation un-recorded.**

**Ties to the Counsel refund-SLA STOP:** the customer was told *before paying* (see "Counsel STOP" below)
that refunds are **manual, owner-initiated, within SLA Y**. `refund_due` is the start of that SLA clock; the
SLA copy + the value of **Y** are **NEEDS-HUMAN** (NH-RES-2).

## C3 (RESOLVED) — Plisio webhook tenancy: HMAC-verified, GUC `WITH CHECK`, definer resolver

New route **`POST /webhook/payments/plisio`** (modeled on `telegram-webhook.ts` conventions: raw body,
own try/catch). Differences from Telegram — **stronger**:

1. **Signature (not secret-equality).** Verify Plisio's **`verify_hash`** = HMAC over the callback body keyed
   by the **Plisio secret API key** (env, SOPS — no secret in git). Mismatch → **401** (and we do **NOT**
   `200` it — see L4: a forged/garbled body must not be silently swallowed; only *handled, signature-valid*
   events return 200 to stop redelivery).
2. **Tenant resolution without a member (the C3 crux).** The callback carries our `order_number` (the
   `payments.id` we passed at `createCharge`) and Plisio's `txn_id` (`provider_payment_id`). To learn
   `location_id` the webhook calls a **SECURITY DEFINER resolver**
   `payment_location_by_provider_ref(provider text, provider_payment_id text) RETURNS uuid` — `search_path`
   statically pinned (the DEFINER guardrail, ledger #33), returns **only** the `location_id`, no other
   columns. This is the same pattern the provisioning ownership-transfer used for a row not SELECT-visible
   under RLS. Then `SELECT set_config('app.current_tenant', <location_id>, true)` (Telegram pattern,
   `telegram-webhook.ts:281`).
3. **RLS policy admits BOTH readers and the GUC writer.** On `payments` + `payment_events`, the policy is
   **not** the bare member USING that C3 proved excludes the webhook. It is:
   ```
   USING (
     location_id IN (SELECT app_member_location_ids())
     OR location_id = nullif(current_setting('app.current_tenant', true), '')::uuid
   )
   WITH CHECK (
     location_id IN (SELECT app_member_location_ids())
     OR location_id = nullif(current_setting('app.current_tenant', true), '')::uuid
   )
   ```
   Members read their locations; the webhook writes the single tenant it set via the GUC. `nullif(...,'')`
   makes it **missing-GUC-tolerant** (a member request with no GUC set still works; an unauthenticated
   request with no GUC set matches nothing → fail-closed). Explicit `WITH CHECK` (not USING-reused-as-CHECK)
   so the INSERT is admitted by the GUC clause.
4. **Idempotent ingest (insert-wins, NOT check-then-act).** Reuses the `idempotency_keys`/Stage-21 discipline:
   - `payments`: **`UNIQUE (provider, provider_payment_id)`** — one charge row per Plisio invoice.
   - `payment_events`: **`UNIQUE (provider, provider_payment_id, type)`** — **deliberate divergence from the
     literal "`UNIQUE(provider, provider_payment_id)`"**: Plisio **resends the same `txn_id` across status
     changes** (`pending` → `completed`), so a bare 2-col unique would reject the legitimate
     `pending→completed` second callback and we would never learn the payment confirmed. The composite
     `(provider, provider_payment_id, type)` kills **same-status** replays (at-least-once delivery) while
     **admitting the progression**. `INSERT … ON CONFLICT DO NOTHING`; the guarded status transition fires
     **only on rowcount=1**, in the same transaction.

**B3 dependency (stated, not bypassed).** The GUC `WITH CHECK` is the **real** cross-tenant closure **once
B3 removes BYPASSRLS** from the writer role. Until B3 lands, FORCE-RLS + this policy is defense-in-depth (a
BYPASSRLS role would void it — Stage-21 §6 honesty). The policy is **GUC-ready now** so B3 flips it to
load-bearing with zero rework. Grant-mirror DO-block (`1790000000028:30-43`) clones `orders` grants onto both
tables.

## H1 (RESOLVED) — fast poll for pending prepaid, not just the daily cron

A **minute-cadence** advisory-lock sweep (reuses the `order-timeout-sweep.ts` pattern — **no new pool, no new
pg-boss queue**, ADR-0001) polls Plisio invoice status for `payments` in `status='pending'` whose order is
held > ~60s with no `completed` event. It feeds the **same** transition path as the webhook and writes a
**synthetic `payment_events` row with the identical `(provider, provider_payment_id, type)` key** → if the
real webhook later arrives it **insert-wins no-ops**. So a lost/late webhook self-heals within ~1 min, not
~24h. The daily reconciliation cron stays as the deep backstop (settlement match → owner-review drift,
never auto-adjust).

## H2 (RESOLVED) — terminal states sticky/monotonic

`payment_status` transitions are **status-guarded UPDATEs with an explicit allowed-from set**, never a blind
write:
- `UPDATE payments SET status='paid' WHERE id=$1 AND status='pending'` — a late/out-of-order `completed`
  callback that arrives **after** `refunded`/`failed`/`expired` hits **0 rows** → recorded as owner-review
  drift, **never resurrects** a refunded order into fulfillment (kills the H2 ship-as-paid race).
- `refunded`, `failed`, `expired` are **terminal sinks** — no transition leaves them.
- `refunded_amount_minor` is **monotonic non-decreasing**, capped `≤ captured_amount_minor` by the residual
  trigger (Stage-21 pattern). A reorg reversal (M5) **records a fact**, it never flips `paid→unpaid` to
  silently re-charge.

## H3 (RESOLVED) — Plisio HTTP stays off the 8-conn operational pool

**Invariant: never hold an operational-pool connection across a Plisio HTTP round-trip.**
- `createCharge` (invoice creation): the only server→Plisio call at checkout. Pattern: open conn → `INSERT`
  the `pending` payment row → **release** → call Plisio (bounded timeout + circuit breaker on the adapter) →
  re-acquire conn → `UPDATE` with `invoice_url`/`txn_id`. The slow PSP HTTP occupies **zero** pooled slots.
  Plisio is a **hosted invoice (redirect/HPP)** — the customer's browser talks to Plisio for the actual
  payment, so the server never proxies funds traffic.
- The H1 poll sweep: connect → read the `pending` list → **release** → HTTP to Plisio → connect → write.
- Net effect: ~4 charges/s × a 10s Plisio stall = **0** operational conn-seconds (H3's 40-conn-second blowup
  is eliminated). The 8+3+3=14 budget (ADR-0001) is preserved; **no new pool, no new queue.**

## Crypto specifics (RESOLVED)

- **`awaiting_confirmation`.** Plisio `pending` → `payment_status='pending'` + the **fulfillment gate HELD**
  (order not offered to a courier). **Never dispatch before Plisio `completed`** (1–2 confirmations, ~2–5
  min). `completed` → `payment_status='paid'` → gate released. Customer-facing copy is **calm progress**
  (Counsel §3): "Confirming payment — your order is reserved," never an anxious spinner.
- **Under/over/late (Plisio `mismatch`).** Recorded as `payment_events` `underpaid`/`overpaid` →
  **owner-review fact**, **no auto-fulfill, no auto-refund**. Late confirmation after invoice expiry is
  ingested idempotently and surfaced for owner-review (fulfill or `refund_due`).
- **Chain reorg (M5).** Mitigated up-front by requiring Plisio's 1–2 confirmations before `paid` (deep reorg
  past 1–2 confs on Tron/ETH/Base/Sol is rare). If a post-`completed` reversal callback arrives, it is
  recorded as a **`reorg_reversed` owner-review fact** (non-custodial → no auto recourse; if food already
  shipped it is an owner-borne loss event). Monotonicity (H2) forbids a silent `paid→unpaid` re-charge.
  **Accepted risk** (owner: API/eng) — confirmation threshold + owner-review.
- **Stablecoin depeg (L2).** The invoice is denominated in the order's **fiat minor units** (`amount_minor`);
  Plisio computes the stablecoin amount at invoice time and the customer pays that. Depeg in the
  **confirm→off-ramp** window is the **merchant's treasury risk** at cash-out, **not** the customer's
  (customer paid the invoiced amount). We store `amount_minor` (fiat) + the asset/crypto-amount as recorded
  facts. **Accepted risk** (owner: treasury) — off-ramp is the merchant's op, outside our code.
- **Wallet-key custody (L3).** Non-custodial = funds settle to the **merchant's** receiving wallet
  (address/xpub configured **in Plisio**). **We never store the private key** — the single catastrophic-loss
  point lives entirely in the merchant's custody. **NEEDS-HUMAN (NH-RES-3):** a written key-custody/backup
  (ideally multisig or hardware) procedure before launch. Code-side: nothing to store, nothing to leak.

## Counsel STOP (live, no longer future) — checkout disclosure + refund SLA

Because crypto is the **first** rail launched, **ETHICAL-STOP-1 gates the `PAYMENTS_CRYPTO_ENABLED` flip**:
before the pay action the checkout MUST render, **server-authoritative**, both:
1. an honest **"crypto payments are irreversible — once sent, funds cannot be reversed"** disclosure, and
2. the **written refund policy/SLA**: refunds are **manual, owner-initiated, within Y**, sent back to the
   customer's wallet out-of-band (the C2 `refund_due`→`refund_sent` flow).

This is a **launch-gate checklist item**, not a build blocker: the schema/runtime can be built **dark** under
`PAYMENTS_PREPAID_ENABLED=OFF` + `PAYMENTS_CRYPTO_ENABLED=OFF`. The flip is blocked until the disclosure +
SLA copy are authored (NH-RES-2). **ETHICAL-STOP-3 (don't default away from cash):** crypto launches as an
*additional* method; cash stays the failure-first floor and the default — recorded, not crossed.

## Discipline preserved
Integer minor units `CHECK(>=0)`, no float · RLS **ENABLE+FORCE**+grant-mirror on `payments` +
`payment_events` · migrations **forward-only / additive / operator-gated**; `payment_method ADD VALUE 'crypto'`
(+ `'card'` inert) **outside a transaction** (enum caveat) · **schema-rich / runtime-minimal** behind
`PAYMENTS_PREPAID_ENABLED` (default OFF) + `PAYMENTS_CRYPTO_ENABLED` (default OFF), `PAYMENTS_PROVIDER=plisio`
· claim-check (no PII in `payment_events.payload` — Plisio callbacks carry no PAN; strip any address/email to
provider-ref + txn_id only, M2) · cash-as-proof spine + `'hold'` primitive **untouched** · NO-AUTO-DEDUCT
intact (crypto refund is owner-initiated) · no new pool / pg-boss queue (ADR-0001 14-conn budget).

## Residual NEEDS-HUMAN (launch-gate, not build blockers)
- **NH-RES-1 — off-ramp.** How the merchant turns received USDT-TRC20 / USDC into ALL/EUR (Binance P2P for
  USDT in the Balkans; USDC for any regulated rail). Merchant treasury op, outside our code; gates *cash-out*,
  not the build. (Owner: merchant/treasury.)
- **NH-RES-2 — refund-SLA copy + value of Y** and the irreversibility disclosure copy. Gates the
  `PAYMENTS_CRYPTO_ENABLED` flip (ETHICAL-STOP-1). (Owner: operator + counsel.)
- **NH-RES-3 — wallet-key custody procedure** (backup / multisig / hardware) for the non-custodial receiving
  wallet (L3). Gates launch, not the build. (Owner: operator.)
- **Carried-forward:** NH-5 Albania legal/tax/AML/e-invoice for crypto acceptance (gating for a real
  consumer launch); NH-1 acquirer stays open for the deferred card round.

## Build-ready verdict
**YES — build-ready, dark.** All three CRITICALs and all three HIGHs are resolved to concrete inputs→state
against Plisio; C1 is a small explicit branch in the one completion primitive; C2 is reframed correctly for
irreversible crypto (owner-review obligation, no fake auto-refund); C3 has a working RLS scheme (definer
resolver + GUC `WITH CHECK`) with the B3 dependency stated. The three residual NEEDS-HUMAN are **launch-gate**
items (off-ramp, refund-SLA copy, key custody) — they block the **flag flip to a real customer**, not the
schema-rich/runtime-minimal build. Cash spine is untouched and remains the default + failure-first floor.
