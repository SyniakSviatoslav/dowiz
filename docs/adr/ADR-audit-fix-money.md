# ADR — Money-correctness remediation: inclusive-tax composition, terminal refund obligations, no-loss settlements

- **Status:** DRAFT — Triadic Council STEP 1 (architect proposal; awaiting system-breaker + counsel + operator)
- **Date:** 2026-07-03
- **Red-lines:** money, state-machine, `packages/db/migrations/`
- **Design detail:** `docs/design/audit-fix-money/proposal.md`
- **Findings:** audit-money-orders 2026-07-03 C1/C3/H5 ≡ synthesis LC1/LC6/B9; relates ADR-0005 (fee mirror), ADR-0017 (crypto C2), ADR-deliver-v2-cash-as-proof (R2-3).

## Context

Three verified money defects:

1. **LC1 (live):** `orders.ts:509-511` and the FE mirror (`packages/ui/src/lib/money.ts:81-84`) ADD the tax that `applyTax` correctly EXTRACTS from a tax-inclusive subtotal (`price_includes_tax` schema-DEFAULTS true) → every taxed inclusive order overpays by `r/(1+r)` of the cart (16.7 % at 20 % VAT). `fee-parity.test.ts` certifies the bug: its expected values are computed FROM the implementation under test (mirror-oracle).
2. **LC6 (dark, blocks crypto flag-flip):** `deliveryCompletion.ts:129-145` is the ONLY writer of `refund_due`. Five other sanctioned terminal paths (timeout sweep, owner PATCH, mark-no-show, grace-cancel, courier abort) cancel PAID orders with no obligation; the Plisio webhook flips already-CANCELLED orders to `paid` with no status check (`payments-webhook.ts:65-70`). Customer money silently kept; the owner refunds queue never learns.
3. **Settlement (live):** `app_generate_settlements` scans only `[period_start, period_end)` with `FOR UPDATE … SKIP LOCKED` and each period is generated once → a locked row or a crashed 2 AM run loses cash deliveries from reconciliation FOREVER; the payout upsert then unconditionally bumps `deliveries_count/total_earned` on payouts already `'paid'`.

## Decision

**D1 — Inclusive tax is never additive.** `total = subtotal + deliveryFee + (price_includes_tax ? 0 : taxTotal) - discountTotal`; `taxTotal` stays persisted/displayed as informational extraction. Ship as (1) an in-place hotfix at both callsites, then (2) consolidation of `applyTax` + a new pure `composeOrderTotal` into one shared module (`packages/shared-types`) so no mirror exists to drift. Correctness is proven by independent-constant vectors and the definitional invariant `inclusive ⇒ total === subtotal + fee`; the parity test is demoted to a drift detector and a mirror-oracle lint/grep ratchet bans implementation-derived expectations in money tests. Historical overcharges are enumerable (`overcharge == tax_total` on inclusive venues pre-fix); restitution is escalated to the operator, never auto-mutated.

**D2 — Terminal-state refund obligation is structural.** Inside `updateOrderStatus`, entering `CANCELLED`/`REJECTED` inserts `refund_due` for every `payments` row with `status='paid'` (idempotent via `payment_events_idem_unique`; **fail-closed** — if the obligation can't be recorded the cancel doesn't commit). The same fold is added to `app_sweep_timeout_orders()` (forward-only `CREATE OR REPLACE`, migration M-1). The webhook's 'completed' branch keeps flipping `paid` (money truth) but inserts `refund_due` in the same tx when the order is already `CANCELLED`/`REJECTED`. The R2-3 fold widens to ALL terminals: `CANCELLED`/`REJECTED` terminalize active bindings; `DELIVERED`/`PICKED_UP` with an active binding now **throw `409 ASSIGNMENT_ACTIVE`** (completion must go through `completeDelivery` — also closes money-H1's stranding). A DB trigger was rejected: it would catch raw-UPDATE bypasses but hides money control-flow; bypasses are instead closed point-wise and ratcheted by a gate banning `UPDATE orders SET status` outside the sanctioned mutator/DEFINER fns.

**D3 — Settlement generation becomes catch-up, immutable-once-paid, idempotent** (forward-only `CREATE OR REPLACE`, migration M-2): drop the lower period bound (scan = `delivered_at < period_end` + `NOT EXISTS settlement_items`, so skipped/missed rows roll into the next run — the deploy self-backfills all historically lost rows); keep SKIP LOCKED (a skip is now a deferral, not a loss); payout row locked `FOR UPDATE` and pairs skipped when `status <> 'pending'` (paid payouts never mutate; late items land in the next period's pending payout); totals recomputed as aggregates over `settlement_items` instead of incremental bumps; `pg_advisory_xact_lock` makes generation single-flight.

## Consequences

- Customer-visible: inclusive-venue totals drop by the extracted tax (the correct price); FE preview and server agree by shared code, not by mirror luck.
- `payment_status='paid'` on a CANCELLED/REJECTED order becomes a meaningful "paid-awaiting-refund" state, always paired with an unmatched `refund_due` the owner queue displays.
- **Contract change (CC-1):** owner PATCH to `DELIVERED`/`PICKED_UP` on a bound delivery order returns 409 `ASSIGNMENT_ACTIVE` instead of silently stranding — FE needs one affordance ("complete via /deliver").
- Settlement periods may contain older (caught-up) deliveries; reconciliation totals become complete and stable.
- Two forward-only migrations (M-1, M-2); no schema/column changes; no down().
- Dependency: LC3's customer-cancel fix MUST route through `updateOrderStatus` to inherit D2.
- Proofs: P1–P12 in the design doc, each red→green with independent expected values (literal constants, definitional invariants, DB-state counts) — never mirror==mirror; each lands with a regression-ledger row.

## Alternatives considered

- **Shared-module-only tax fix (no hotfix step):** rejected — couples a live-overcharge correction to build-graph refactoring; hotfix-then-consolidate keeps each diff reviewable under the money red-line.
- **Suppressing the webhook `paid` flip on terminal orders:** rejected — hides received funds; recording payment + obligation is truthful and drives the existing refunds surface.
- **DB trigger for refund obligations:** rejected as above (hidden control flow, RLS/DEFINER complexity, untestable bus semantics); revisit only if a third raw-UPDATE bypass ever appears.
- **Dropping SKIP LOCKED in settlements:** rejected — blocks the fleet-wide sweep on any long app transaction; catch-up scanning makes skipping safe instead.
- **Second payout in a closed period for late items:** rejected — violates the `(courier,location,period)` unique and complicates audit; deferral to the next pending period preserves both immutability and completeness.
