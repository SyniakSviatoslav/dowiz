# Audit — Money · Order Lifecycle · State Machine (2026-07-03)

READ-ONLY audit of the money + order-lifecycle + state-machine surface. All findings verified
against source at HEAD of `feat/phase0-safety-hardening`. Excludes the already-known items
(B3/NOBYPASSRLS deferral, prod≠staging drift, branch divergence, the fixed offer-sweep raw UPDATE).

**Counts: 3 CRITICAL · 5 HIGH · 6 MED · 4 LOW**

---

## CRITICAL

### C1 — Inclusive-tax is charged TWICE (the default configuration overcharges every taxed order)
- **Where:** `apps/api/src/routes/orders.ts:509-511` + FE mirror `packages/ui/src/lib/money.ts:81-84`
- **Scenario:** Location has `price_includes_tax=true` (the schema DEFAULT — `packages/db/migrations/1780338982014_location_commerce.ts:9`) and `tax_rate=0.2`. Cart subtotal = 1200 (VAT already inside, per "includes tax"). `applyTax(1200, 0.2, true)` correctly EXTRACTS 200 — then `total = subtotal + deliveryFee + taxTotal` adds the extracted 200 back on top → customer is charged 1400 + fee for a 1200 cart. The FE mirror does the identical double-add, and `apps/api/tests/fee-parity.test.ts` only pins FE === BE — so the parity guardrail locks the bug in on both sides instead of catching it.
- **Invariant violated:** tax-inclusive prices already contain the tax; extraction is informational, never additive.
- **Fix direction:** `total = subtotal + deliveryFee + (location.price_includes_tax ? 0 : taxTotal) - discountTotal` on both server and mirror; add a unit test asserting the inclusive-branch total equals `subtotal + fee`.

### C2 — Customer post-dispatch cancel 500s on EVERY call (writes columns that don't exist) and its event has zero subscribers
- **Where:** `apps/api/src/routes/customer/orders.ts:308-312` (`UPDATE orders SET status='CANCELLED', cancelled_at=now(), cancellation_reason=$1`)
- **Scenario:** Customer taps cancel within the 5-min window → `orders.cancelled_at` / `orders.cancellation_reason` exist in NO migration (orders table: `1780310074262_orders.ts:21-43`; grep of all migrations finds them only on `courier_assignments`) → Postgres 42703 → ROLLBACK → 500. The feature is dead-on-arrival. This is the exact defect class already fixed once in mark-no-show (`signals.ts:233-236` comment: "referenced a column that never existed (42703), so this route 500-rolled-back on EVERY call").
- **Blind spot:** the only e2e touching this route (`e2e/tests/flow-core-lifecycles.spec.ts:199-210`) asserts a **403 with an owner token** — the happy path has never been executed, so the suite stays green.
- **Compounding:** even if the columns existed — (a) the raw `UPDATE orders` bypasses `updateOrderStatus`, so no `order_status_history` row, no `timeout_at` clear, no `orderChannel`/`dashboardChannel` WS delta (owner dashboard and courier never see the cancel live); (b) `BUS_CHANNELS.ORDER_CANCEL_AFTER_DISPATCH` (`customer/orders.ts:341`) has **zero subscribers** anywhere (`registry.ts:12` + this publisher are the only references) — the courier out with the food is never notified.
- **Invariant violated:** every status transition goes through the sanctioned mutator; every published lifecycle event has a consumer.
- **Fix direction:** route through `updateOrderStatus(client, …, 'CANCELLED', {comment: reason})` (drop the nonexistent columns), keep its own assignment terminalization or rely on the R2-3 fold, and either subscribe a handler to ORDER_CANCEL_AFTER_DISPATCH or publish ORDER_CANCELLED; add a customer-token e2e asserting 200.

### C3 — Paid crypto money black hole: every cancel path except completeDelivery drops the refund obligation, and the webhook marks CANCELLED orders 'paid'
- **Where:** `apps/api/src/lib/deliveryCompletion.ts:129-145` (the ONLY writer of `refund_due`); `apps/api/src/routes/payments-webhook.ts:64-69`; `packages/db/migrations/1790000000078_phase2-sweep-fns.ts:13-22` (`app_sweep_timeout_orders`); `apps/api/src/routes/owner/signals.ts:237`; `apps/api/src/lib/bindingRelease.ts:40-43`; `apps/api/src/workers/courier-offer-sweep.ts:199-272`.
- **Scenario A (pay-then-cancel):** crypto order placed → webhook flips `payment_status='paid'` → owner never confirms → timeout sweep cancels the PENDING order. No path writes `refund_due` (only `completeDelivery` does, and only on the refused/cancelled-on-door tail). The owner refunds queue (`owner/refunds.ts:25-30` lists only `refund_due` events) never learns; the customer's money is silently kept.
- **Scenario B (cancel-then-pay):** order times out / owner rejects while payment is in flight → webhook later receives 'completed' → `UPDATE orders SET payment_status='paid' WHERE … payment_status IN ('pending','authorized')` (`payments-webhook.ts:65-69`) has **no order-status check** → a CANCELLED/REJECTED order becomes `payment_status='paid'` with no obligation recorded.
- Same hole for owner PATCH PENDING→CANCELLED / REJECTED, mark-no-show, the grace-cancel pass 4, and the courier-abort-with-food CANCELLED (`bindingRelease.ts:40-43`).
- **Invariant violated:** ADR-0017 C2 — a paid order that will not be fulfilled must always leave a `refund_due` obligation in the ledger.
- **Fix direction:** centralize "on entering a terminal non-DELIVERED state, if a `payments` row is/becomes 'paid' → insert `refund_due`" (e.g. inside `updateOrderStatus` for CANCELLED/REJECTED, plus in the webhook's 'completed' branch when the order is already terminal). Dark today (flags off) but the vertical is fully built — fix before flag-flip.

---

## HIGH

### H1 — Owner PATCH `DELIVERED`/`PICKED_UP` strands the active assignment and erases cash accountability
- **Where:** `apps/api/src/routes/orders.ts:885-892` (any non-IN_DELIVERY target goes straight to `updateOrderStatus`); fold condition `apps/api/src/lib/orderStatusService.ts:134` covers only `CANCELLED` and `IN_DELIVERY→READY`.
- **Scenario:** order IN_DELIVERY with an active `picked_up` assignment; owner PATCHes `{status:'DELIVERED'}` (allowed by the machine, allowed by `StatusUpdateInput` — full `OrderStatusEnum`; `assertOwnerTargetAllowed` only guards CANCELLED). Result: order DELIVERED but (a) NO `completeDelivery` → no cash-as-proof 'hold', no `payment_outcome`, no `delivery_trace`; (b) assignment stays `picked_up` forever and the shift stays `on_delivery`; (c) the courier is excluded from ALL future dispatch (`dispatch.ts:33-36` `NOT IN (…active statuses…)`) — permanently undispatchable; (d) settlement never counts the cash (`app_generate_settlements` requires assignment `status='delivered' AND cash_collected=true`). Same strand for READY→PICKED_UP on a delivery order carrying an `offered`/`assigned`/`accepted` binding.
- **Invariant violated:** the code's own R2-3 — "NO order leaves to a terminal … without its active courier assignment terminalized in the SAME tx" — DELIVERED and PICKED_UP are terminals not covered by the fold.
- **Fix direction:** in the PATCH route, reject owner DELIVERED on delivery orders with an active binding (force `/deliver`), or widen the fold to all terminals + require completion via `completeDelivery`.

### H2 — `mark-no-show` bypasses the SYSTEM-only cancel guard (and brands customers no-show for undelivered orders)
- **Where:** `apps/api/src/routes/owner/signals.ts:198-252` — calls `updateOrderStatus(…, 'CANCELLED')` at :237 with **no `assertOwnerTargetAllowed` and no order-status precondition**.
- **Scenario:** owner is 403'd (`CANCEL_NOT_PERMITTED`) on PATCH for a CONFIRMED/PREPARING/READY order — then calls `POST /:locationId/orders/:orderId/mark-no-show` on the same order: it cancels straight through the widened machine edges the addendum declared SYSTEM-only, AND increments `customers.no_show_count` / `last_no_show_at` for an order that was never even dispatched (a PENDING order qualifies). Future preflight signals then punish that customer.
- **Invariant violated:** `orderAuthz.ts` — "an OWNER must not be able to drive [CONFIRMED/PREPARING/READY→CANCELLED] by piping a request-supplied status"; a no-show verdict requires a delivery attempt.
- **Fix direction:** restrict mark-no-show to `IN_DELIVERY` (or DELIVERED-refused) orders and/or call `assertOwnerTargetAllowed` on the locked current status.

### H3 — Crypto-paid override erases an explicit refusal: refused food is recorded DELIVERED and the refund obligation is never written
- **Where:** `apps/api/src/routes/courier/assignments.ts:338-340`.
- **Scenario:** prepaid crypto order; customer refuses the food at the door; courier taps `refused_goods`. The route overrides ANY outcome — including explicit refusals — to `delivered_prepaid` whenever `payment_method='crypto' && payment_status='paid'` → `completeDelivery` marks the order DELIVERED, and the `refund_due` branch (which fires only on `!isDelivered`) is skipped.
- **Invariant violated:** deliver-v2's founding rule — "the customer never sees 'Delivered' for refused food" — plus ADR-0017 C2 (refund obligation on refused prepaid).
- **Fix direction:** only auto-resolve to `delivered_prepaid` when the courier reported a DELIVERED-class outcome (`paid_full`/absent); preserve explicit refusal outcomes and let the C2 `refund_due` branch fire.

### H4 — Owner-proxy `/deliver` fabricates a cash attestation by default and writes false till-debt for prepaid orders
- **Where:** `apps/api/src/routes/owner/dashboard.ts:462` (`cashCollected = body?.cash_collected ?? true`), `:479` (`finalCashAmount = cashAmount ?? total`), `:454` (enum omits `delivered_prepaid`).
- **Scenario A:** owner POSTs `{}` → outcome `paid_full` with `cashAmount` defaulted to exactly `total` → the `cash===total` coherence check passes **by construction** → a `courier_cash_ledger` 'hold' (till-debt) is created against the courier with zero attestation anyone collected cash. Compare the courier path where `{}` defaults to `refused_payment` — opposite defaults on the two "parity" bodies.
- **Scenario B:** crypto-paid order proxy-delivered → `delivered_prepaid` isn't in the owner enum, so it completes as `paid_full` and writes a cash hold for money already received on-chain → double-counted revenue + unjust courier debt at reconciliation.
- **Invariant violated:** cash-as-proof — a hold represents cash actually in a courier's hands; a paid order must never create a till-debt (`deliveryCompletion.ts:59`).
- **Fix direction:** require an explicit `payment_outcome` on the proxy route; mirror the courier crypto auto-resolve (add `delivered_prepaid`); drop the `?? true` / `?? total` defaults.

### H5 — Settlement generation: once-only period + `SKIP LOCKED` silently drops cash deliveries; paid payouts mutate afterwards
- **Where:** `packages/db/migrations/1790000000078_phase2-sweep-fns.ts:160-195` (`app_generate_settlements`), `apps/api/src/lib/settlement-period.ts` (daily = exactly [yesterday, today)), `apps/api/src/workers/settlement-cron.ts:29-49`.
- **Scenario A (lost money rows):** the 2AM run for yesterday selects items `FOR UPDATE OF ca SKIP LOCKED`. Any assignment row locked at that instant (courier app write, recon query) is skipped — and since each period is generated exactly once and no later run re-scans past periods, that delivered-cash row **never enters any payout**. A crashed/failed 2AM job loses the entire day the same way.
- **Scenario B (paid payout mutates):** the payout upsert keeps whatever status exists (`ON CONFLICT … SET status = courier_payouts.status`) and then unconditionally bumps `deliveries_count`/`total_earned` — including on a payout already `'paid'` via `owner/settlements.ts:162-203`. A late item lands inside a closed, already-paid payout; the courier was paid the pre-bump total and the delta is invisible.
- **Invariant violated:** every `delivered+cash_collected` assignment reaches exactly one payout; a paid payout is immutable.
- **Fix direction:** make the item scan period-independent (drive off `NOT EXISTS settlement_items` + `delivered_at < period_end` catch-up), and route late items to a new `pending` payout when the target payout is not `'pending'`.

---

## MED

### M1 — Owner-cancel guard is TOCTOU-bypassable
- **Where:** `apps/api/src/routes/orders.ts:862-891` — `assertOwnerTargetAllowed` runs on a status read in one query; `updateOrderStatus` re-reads and guards its UPDATE on the *re-read* status.
- **Scenario:** owner sends CANCELLED while the order is PENDING (guard passes); concurrently the courier legacy-accept path (`assignments.ts:164`) confirms it. `updateOrderStatus` re-reads CONFIRMED, the machine now allows CONFIRMED→CANCELLED, the guarded UPDATE matches — the owner has driven the SYSTEM-only edge the guard exists to block.
- **Fix direction:** move the actor check into `updateOrderStatus` (actor param) or re-assert on the row read inside the same statement (`WHERE status = $expected`).

### M2 — Crypto charge init runs post-COMMIT in autocommit; idempotent replay permanently loses the invoice
- **Where:** `apps/api/src/routes/orders.ts:641-666` (after the `COMMIT` at :579) and the replay early-return at :381-391.
- **Scenario:** crash between COMMIT and the crypto block → the order proceeds as a live cash order despite the customer choosing crypto. Separately: any client retry of the POST hits the idempotency replay, which returns the bare order row with **no `payment.redirectUrl`** and never re-creates a charge — a crypto order whose first response was lost is unpayable and dies at timeout.
- **Fix direction:** persist the payment intent inside the main tx; on idempotent replay, re-attach the existing invoice URL (read `payments` by order_id) or create the charge if missing.

### M3 — Plisio webhook amounts recorded with hardcoded minor-unit=2; the claimed amount validation doesn't exist; 'mismatch' has no review surface
- **Where:** `apps/api/src/lib/payments/plisio.ts:91-92` (`amountStringToMinor(…, 2)`); `apps/api/src/routes/payments-webhook.ts:45-85`.
- **Scenario:** ALL has `currency_minor_unit=0`. A 1250-ALL invoice comes back as `source_amount="1250"` → parsed with unit 2 → `payment_events.amount_minor=125000` (100× the `payments.amount_minor=1250`). The code comment says "the route validates against payments.amount_minor" — no such comparison exists; 'completed' flips paid purely on Plisio's status. 'mismatch' events are recorded but no endpoint/UI lists them (`owner/refunds.ts` lists only `refund_due`) — an underpaid order just dies at timeout with partial customer money kept.
- **Fix direction:** pass the location minor-unit into `parseEvent` (or store provider units separately); compare event amount to `payments.amount_minor` before flipping paid; surface `mismatch` events to the owner.

### M4 — `refunds/:paymentId/sent` has no state guard: 500s on pending payments, silently "refunds" any paid one
- **Where:** `apps/api/src/routes/owner/refunds.ts:53-70`.
- **Scenario:** owner posts `sent` for a payment with no `refund_due` and status `pending` → `refunded_amount_minor = amount_minor` violates `payments_money_residual` (refunded ≤ captured=0) → raw 500. For a legitimately captured, delivered payment → silently flips `payments.status` and `orders.payment_status` to 'refunded' with no obligation ever recorded — ledger corruption by one wrong click.
- **Fix direction:** `WHERE status='paid'` + require an unmatched `refund_due` event before accepting `refund_sent`.

### M5 — Concurrent dispatch double-books one courier (no lock on the shift row)
- **Where:** `apps/api/src/lib/dispatch.ts:27-53`.
- **Scenario:** two owners PATCH two different orders to IN_DELIVERY concurrently; both transactions snapshot-read the same courier as `available` (no `FOR UPDATE` on `courier_shifts`, and the active-assignment `NOT IN` can't see the other's uncommitted INSERT). The per-ORDER partial unique doesn't cover per-COURIER → one courier ends up with two active assignments.
- **Fix direction:** `SELECT … FOR UPDATE SKIP LOCKED` on the shift row, or a partial unique on active assignments per courier.

### M6 — Cancel fan-out is inconsistent across the five cancel paths
- **Where:** `orderStatusService.ts:211-216` publishes lifecycle events only for CONFIRMED/REJECTED; ORDER_CANCELLED is published only by `signals.ts:248` and the grace sweep (`courier-offer-sweep.ts:261`); the timeout sweep sends only its own telegram enqueue; owner PATCH PENDING→CANCELLED publishes nothing beyond the WS delta.
- **Scenario:** dwell-alert resolution and escalation-job cancellation (`lifecycle-handlers.ts:27`) run for some cancels and not others; customer terminal-push behavior differs per path.
- **Fix direction:** publish ORDER_CANCELLED centrally (post-commit hook or outbox) for every path entering CANCELLED.

### M7 — Courier `/delivered` with an empty body defaults to `refused_payment` → cancels the order
- **Where:** `apps/api/src/routes/courier/assignments.ts:312`.
- **Scenario:** a legacy/malfunctioning client POSTs `{}` to "delivered" → outcome derives to `refused_payment` → `completeDelivery` CANCELs the order, writes NO cash hold. If the courier actually delivered and collected cash: customer sees CANCELLED for delivered food and the cash vanishes from till accountability.
- **Fix direction:** make `payment_outcome` (or `cash_collected`) required; reject an outcome-less body instead of defaulting to the money-losing branch.

---

## LOW

### L1 — Shift resurrection: unguarded `SET status='available'` in four places
- **Where:** `orderStatusService.ts:141-142`, `courier-offer-sweep.ts:139`, `bindingRelease.ts:29`, `deliveryCompletion.ts:92` — none guard on the current shift state (`courier_shifts` CHECK: offline/available/on_delivery). Only the customer-cancel route does it right (`customer/orders.ts:333` `AND status='on_delivery'`).
- **Scenario:** a shift flipped `offline` (heartbeat-stale sweep / courier logout) while its assignment is being terminalized gets resurrected to `available` → ghost dispatch eligibility for a courier who left.
- **Fix direction:** add `AND status='on_delivery'` everywhere a shift is freed.

### L2 — Owner `/verify` line math omits modifiers
- **Where:** `apps/api/src/routes/owner/dashboard.ts:557` — `(oi.price_snapshot * oi.quantity) AS subtotal` ignores `order_item_modifiers` deltas, so item lines don't sum to `orders.subtotal` for modifier orders; misleading during money disputes.
- **Fix direction:** join/aggregate `order_item_modifiers.price_delta_snapshot` into the line subtotal.

### L3 — Owner settlements list swallows every DB error as an empty list
- **Where:** `apps/api/src/routes/owner/settlements.ts:69-71` (`catch { return { payouts: [] } }`) — a money surface that renders "no payouts" on infrastructure failure.
- **Fix direction:** let it 500/503 like every other money endpoint.

### L4 — `/settlements/regenerate` is a cross-tenant global trigger with an arbitrary date
- **Where:** `apps/api/src/routes/owner/settlements.ts:301-317` — any single-location owner triggers `app_generate_settlements` for ALL tenants (the comment admits it), with an unvalidated `referenceDate`. Item-level `NOT EXISTS` prevents double-count, but it's an unauthorized cross-tenant side effect and a cheap heavy-scan DoS (5/5min).
- **Fix direction:** scope generation by location_id or restrict to a platform-admin role.

---

## Cross-cutting observations

- **The sanctioned-mutator funnel leaks.** `updateOrderStatus` is the "central fold", but the customer cancel (raw UPDATE, C2), `app_sweep_timeout_orders` (by design, but with divergent fan-out, M6), and owner PATCH terminals not covered by the fold (H1) all sit outside its guarantees. Guarantees claimed "for EVERY caller, present and future" hold only for CANCELLED/READY-revert.
- **The refund ledger has exactly one writer on a many-writer surface** (C3) — every new cancel path silently reopens the hole.
- **Two guardrails certify their own bug:** the FE/BE fee-parity test (C1) and the 403-only e2e for customer cancel (C2) both keep CI green while the behavior is wrong. Proof tests must assert against an independently computed expectation, not against the mirrored implementation.
