# Payments — Breaker findings (Triadic Council)

Returned inline by the system-breaker seat (its run-harness blocked the file write); persisted here for the
council artifact set. All anchored to live code. **15 findings — 3 CRITICAL · 3 HIGH · 5 MED · 4 LOW.**

## CRITICAL
- **C1 · Prepaid completion structurally blocked (OPEN-1 is the design's own missing fix).** `completeDelivery`
  (`apps/api/src/lib/deliveryCompletion.ts:5-9`) is the single completion primitive for both callers; its only
  success outcome `paid_full` hard-requires `cashAmount === total` (`:59-61`) then unconditionally writes the
  courier till-debt `'hold'` (`:104-110`). A prepaid card order → no cash → `CASH_AMOUNT_MISMATCH` 422 →
  **paid-but-never-completed**; or force `cashAmount=total` → **phantom till-debt on an already-paid order**.
  The proposal defers the fix to OPEN-1 → the design doesn't contain its own load-bearing fix.
- **C2 · Charged-then-refused prepaid keeps the customer's money.** Non-`paid_full` outcomes set order
  `CANCELLED` + write NO ledger row + make NO refund call (`:63-90`). Prepaid refused-at-door → money retained,
  no provider refund. §4.2's "trigger a provider refund" is undefined (OPEN-2).
- **C3 · The webhook (money source-of-truth) can't INSERT under the chosen RLS policy without the BYPASSRLS
  escape the proposal warns against.** §4.3 copies `courier_cash_ledger`'s `location_id IN
  (SELECT app_member_location_ids())` (`1790000000028:26-27`), which is JWT-member-derived
  (`1780310071220:76-79`). But §4.4's webhook is unauthenticated and sets `app.current_tenant` (telegram
  pattern). USING-only policies reuse USING as INSERT WITH-CHECK → webhook has no member → empty set → INSERT
  rejected by FORCE-RLS → forced onto a BYPASSRLS role (which §7 admits voids isolation). Two incompatible
  tenancy schemes on the SoT writer.

## HIGH
- **H1 · Lost-webhook recovery ≤24h vs a minutes-long food SLA.** A dropped webhook leaves `pending`,
  fulfillment held, customer charged; the only backstop is the *daily* reconciliation cron. One lost webhook =
  paid order undispatched ~a day. Cadence mismatched.
- **H2 · Out-of-order refund/chargeback before capture is dropped → order ships as paid.** rowcount-guarded
  `paid→refunded` hits 0 rows if the refund arrives at `pending`/`authorized`; §6 treats 0-rows as "no crash";
  a later `captured` then moves `pending→paid` → dispatched despite a recorded refund. State machine diverges
  from the money SoT in the unsafe direction.
- **H3 · `createCharge` on the operational pool (8) blows the 14-conn budget under a slow PSP.** ~4 charges/s ×
  10s PSP timeout = 40 conn-seconds vs 8 conns ≈ 5× over; pool saturates, starves checkout, before the
  circuit-breaker trips. Violates the proposal's own stated binding constraint (§2).

## MEDIUM
- **M1 · Payment idempotency has no amount/request fingerprint** (the cited orders pattern compares
  `request_hash`, `orders.ts:365-380`) → key reuse after cart change → stale charge or double charge.
- **M2 · `payload jsonb` stored raw contradicts "claim-check clean / never PAN" (§4.5/§7)** — provider payloads
  carry cardholder name/last-4/billing PII; unfiltered + logged.
- **M3 · `payment_status='unpaid'` is overloaded** — COD-for-life AND prepaid-pre-payment → "chase unpaid"
  reports can't tell a delivered cash order from an unpaid prepaid one.
- **M4 · Partial capture not representable in the binary `paid`** → fulfillment gate opens on full `paid` while
  only a partial amount captured.
- **M5 · Crypto chain-reorg after "confirmed" unhandled** → reversed on-chain after handover, non-custodial = no
  recourse.

## LOW
- **L1** no stated index on `payment_events(provider_charge_ref)` (reconciliation match). **L2** stablecoin
  depeg in confirm→off-ramp window unpriced. **L3** non-custodial wallet private-key custody = single
  catastrophic-loss point, unaddressed. **L4** copying telegram's "always 200" risks swallowing provider
  retries on transient/signature errors.

## Top criticals to resolve first
C1, C2, C3 — all genuine, all in the 🔴 money/RLS/state surface.
