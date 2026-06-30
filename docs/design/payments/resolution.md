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
