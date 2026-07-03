# Proposal — Route the offer-sweep grace-cancel through the sanctioned mutator

Status: PROPOSED (design-time; no production code in this doc). REVISED after breaker round 1
(breaker-findings.md, counsel-opinion.md) — decision flipped from Option B to **Option A + coupling-fix**.
Owner: System Architect
Blocks: prod deploy (RED-LINE guardrail `scripts/guardrail-deliver-v2.mjs` R3-3)
Related: ADR-deliver-v2-cash-as-proof (addendum ships WITH the code), ADR-dispatch-recovery,
`packages/domain/src/order-machine.ts`

---

## 1. Problem + non-goals

### Problem
`scripts/guardrail-deliver-v2.mjs` (R3-3, no-new-raw-cancel) fails on
`apps/api/src/workers/courier-offer-sweep.ts:221`:

```
UPDATE orders SET status = 'CANCELLED', timeout_at = NULL WHERE id = $1 AND status = $2
```

This is **Pass 4 — grace-window auto-cancel** (`graceCancelExhausted`): a dispatch-exhausted order the
owner ignored for the full `DISPATCH_OWNER_GRACE_MS` grace window transitions to the customer-honest
terminal `CANCELLED`. The pass is **flag-off dark** (`DISPATCH_OWNER_GRACE_ENABLED=false`,
R-NEEDS-HUMAN-1, pending STOP-ETHICS) but the guardrail is a **static scan**, so it blocks the deploy.

**Root cause:** the state machine (`order-machine.ts:18`) does not own a
`CONFIRMED/PREPARING/READY → CANCELLED` edge — only `PENDING→CANCELLED` and `IN_DELIVERY→CANCELLED` are
legal. A dispatch-exhausted order sits pre-IN_DELIVERY (its `NOT EXISTS` clause excludes active
assignments). So `updateOrderStatus` cannot express the transition today — `assertTransition` throws.
That is why the worker author wrote a raw guarded UPDATE. Genuine gap between worker need and domain
machine.

### Non-goals
- Not turning on grace-cancel (`DISPATCH_OWNER_GRACE_ENABLED` stays false; STOP-ETHICS owns enablement).
- Not adding an **owner-facing** "cancel a preparing order" capability — the widened edge is SYSTEM-only,
  blocked at the owner route (§ coupling fix). Whether to expose it to owners is a deferred product call.
- Not touching Pass 1-3.

---

## 2. Back-of-envelope (blast radius)
- **Live volume today: ZERO** (flag-off). The guardrail protects the invariant, not a hot path.
- **When enabled:** a triple-tail (dispatch-exhausted × owner-ignored-15min × no active assignment) →
  single digits/day system-wide at MVP scale, most days zero.
- **Connection budget:** runs in the sweep's existing single pooled client, per-row short tenant-pinned
  txns. No new pool/queue. Negligible.
- **Conclusion:** correctness/invariant integrity dominates. Optimise for keeping the state machine the
  single transition authority, not for throughput.

---

## 3. Options (revised after breaker round 1)

The breaker (F2) demonstrated that the previously-chosen Option B does **not** achieve containment: a
second exported cancel function in `orderStatusService.ts` is (a) machine-illegal (bypasses
`assertTransition`), (b) **owner-callable** — the module is imported by `routes/orders.ts`,
`owner/signals.ts`, `courier/assignments.ts`, `telegram-webhook.ts` — and (c) **guardrail-blind** (the
gate scans raw-UPDATE *text* in files, never *calls* to an export). It is Option-C laundering relocated
into a *more* trusted file, and one import away from the exact unflagged owner-cancel the proposal
rejected Option A for — with no STOP-ETHICS gate and a green scan. B is withdrawn.

### Option A (+ coupling-fix) — widen the machine; owner exposure closed at the route layer (CHOSEN)
1. Widen `TRANSITIONS`: add `CANCELLED` to `CONFIRMED`, `PREPARING`, `READY` (documented as the
   dispatch-exhausted terminal edges — same additive-machine-edge pattern the ADR already used for
   `IN_DELIVERY→{CANCELLED,READY}`).
2. **Coupling-fix (the decisive addition):** the two owner PATCH sites that pipe request-supplied
   `newStatus` straight into `updateOrderStatus` (`orders.ts:879-885`, `owner/dashboard.ts:625-653`) get a
   route-layer authz guard `assertOwnerTargetAllowed(from, to)` that rejects owner-requested `CANCELLED`
   from `{CONFIRMED,PREPARING,READY}` → `403 CANCEL_NOT_PERMITTED`. Existing owner cancels
   (`PENDING→CANCELLED`, `IN_DELIVERY→CANCELLED` via `signals.ts`) are preserved.
3. Route Pass 4 through `updateOrderStatus` (system actor, no route guard) inside the worker's tx.
- **Concept:** state machine = *what is possible* (the edge is a true domain fact — a kitchen that can't
  fulfil an order it is preparing must be able to terminate it); route = *who is allowed*. The correct
  separation the codebase already espouses.
- **Pro:** machine stays **sole transition authority**; NO new raw UPDATE and NO new export anywhere →
  guardrail passes with **zero allowlist change, zero laundering** (F2 dissolved); history rows now match
  legal machine edges (F6 dissolved); cash-safety is `updateOrderStatus`'s existing property, not a new
  boundary claim (F3 dissolved); `fetchOrderDelta` gives the canonical dashboard shape for free (F5
  dissolved). The coupling-fix is a net-positive hardening that should exist anyway.
- **Con:** touches the domain machine (broad-authority surface) → requires an exhaustive transition test
  (pin) + the ADR addendum shipping WITH the code; the owner-route guard is a second place authz lives
  (mitigated by a single shared helper).

### Option B (hardened) — dedicated `cancelUndispatchableOrder` primitive (WITHDRAWN)
To mitigate F2 you would need the export to be un-callable by owners (impossible for an exported symbol in
a shared module) or teach the guardrail to scan call-sites (fragile scope-creep). B cannot actually reach
containment, and it introduces a shadow transition table (`ALLOWED_FROM`) that drifts from the canonical
machine (F6). Withdrawn.

### Option C — extend the guardrail allowlist to the worker file (REJECTED, unchanged)
Launders a machine-forbidden edge past the very gate meant to enforce the funnel; leaves every future
cancel edit in the worker unguarded. Violates "never weaken a gate."

---

## 4. Decision + justification

**Chosen: Option A + coupling-fix + F1 fan-out fix.** Widen the machine for the three CANCELLED edges;
close owner exposure at the route layer; route Pass 4 through `updateOrderStatus`; publish
`ORDER_CANCELLED` post-commit so lifecycle side-effects fire. The machine remains the single source of
transition legality. Grace-cancel stays flag-off.

**Why A over B (honest comparison the coordinator asked for):**
- **Single transition authority (the deciding axis).** A keeps `order-machine.ts` + `assertTransition` as
  the one gate every mutation funnels through. B knowingly creates a *second* authority (a hardcoded
  `ALLOWED_FROM`) that bypasses it. The coordinator's tie-breaker ("pick the one that keeps the state
  machine the single transition authority") points at A.
- **F2 is unmitigable in B, dissolved in A.** B's exported primitive is owner-callable and
  guardrail-blind by construction. A has no new export and no new raw UPDATE — the guardrail passes
  because `updateOrderStatus` genuinely *is* the mutator; owner exposure is closed by explicit route
  authz (which the guardrail need not see because there is nothing to launder).
- **The coupling-fix is real hardening, not a workaround.** Piping request `newStatus` blindly into a
  transition mutator is a latent authz weakness regardless of this change; fixing it is net-positive.
- **F1/F3/F5/F6 all resolve more cleanly under A** (one mutator carries the semantics; edges are legal;
  canonical delta for free) — see the resolution table in resolution.md.

**ADR addendum: ships WITH the code (merge-gate, per counsel).** A green R3-3 scan must not be allowed to
*mean* "nothing changed in the machine." The addendum to `ADR-deliver-v2-cash-as-proof.md` records: the
three new SYSTEM-only CANCELLED edges; the owner-route authz guard; the `ORDER_CANCELLED` fold; the
`updateOrderStatus` assignment-terminalize fold extension; and the pre-registered
**STOP-REFUND-BEFORE-GRACE** ethics condition. Merge is gated on the addendum landing in the same PR.

---

## 5. Data / migrations
**None.** Domain change only (`order-machine.ts` `TRANSITIONS`) + route/worker code + one bus publish.
No schema change → forward-only/atomic/RLS-FORCE/integer all N/A. `orders`, `order_status_history` already
carry FORCE RLS; the worker pins `app.current_tenant` per row. `timeout_at=NULL` is handled by
`updateOrderStatus`'s CANCELLED branch (falls through to the `else` at `orderStatusService.ts:114`).

---

## 6. Consistency + idempotency
- **Widened edges are legal + funneled:** `assertTransition('PREPARING','CANCELLED')` now passes; the write
  is the mutator's existing status-guarded `UPDATE ... WHERE status=$current` (409 on race, caught by the
  worker → skip). No shadow authority.
- **Assignment-terminalize fold extended (self-found, closes a widened-edge strand):** today
  `updateOrderStatus`'s R2-3 fold only runs when `currentStatus === 'IN_DELIVERY'`. A widened
  `PREPARING→CANCELLED` on an order that has an active `offered/assigned/accepted` assignment would strand
  it. Extend the fold condition so it terminalizes any active assignment on **any** `newStatus==='CANCELLED'`
  (idempotent no-op when none active — unchanged for `PENDING→CANCELLED`). This makes the widened edge
  robust for every caller, not just the grace worker's `NOT EXISTS`. Cash-safe: terminalizing an assignment
  writes no ledger row (the `'hold'` is written only by `completeDelivery` at DELIVERED).
- **F7 under-lock anti-race (fix):** the worker re-checks `NOT EXISTS(active assignment)` inside the per-row
  `FOR UPDATE` tx immediately before calling the mutator; if a fresh binding appeared (dispatch drain
  earlier in the same tick), ROLLBACK + continue — never cancel an order a courier just took.
- **Idempotent:** a re-run over an already-terminal order → the mutator throws 409 → worker skips; no
  duplicate history/publish.

---

## 7. Failures + degradation
- **F1 fix — consequential fan-out is POST-commit** (matches the sanctioned `signals.ts:237-248` pattern):
  `updateOrderStatus(CANCELLED)` inside tx → COMMIT → `messageBus.publish(BUS_CHANNELS.ORDER_CANCELLED,
  {orderId, locationId, reason:'dispatch_exhausted'})` + `NOTIFY_CUSTOMER_STATUS`. This drives
  `lifecycle-handlers.ts:27` → `app_resolve_order_alerts` (dwell alerts resolved) + `boss.cancel` (every
  pending `notify.dispatch.*` escalation job cancelled). Publishing post-commit is *required*: you must not
  cancel escalation jobs / resolve alerts before the order row is durably CANCELLED.
- **F4 disposition (split):** the live WS delta (`order.status` + dashboard) is published *inside*
  `updateOrderStatus` pre-commit — this is the pre-existing, whole-codebase mutator property (every caller:
  owner PATCH, courier, signals, `completeDelivery`). The self-reconciling window (COMMIT failing after a
  successful guarded UPDATE on a held-lock row) is negligible; a subsequent poll/fetch corrects it. The
  *consequential* side-effects (alert-resolve, job-cancel, customer push) are post-commit (above), so the
  dangerous ones can never fire on a rolled-back cancel. ACCEPT the pre-commit live-delta as the canonical
  mutator behavior; register the cross-cutting phantom-live-delta as a known risk on `updateOrderStatus`
  itself, not this path.
- **409 / race lost:** caught → ROLLBACK + continue, no cascade.
- **History / bus / notify failures:** `updateOrderStatus` savepoint-guards history; the worker's post-commit
  publishes stay in its existing try/catch → logged, non-fatal (state already durable). No external call on
  the commit critical path.

---

## 8. Security + tenant isolation
- **Owner-exposure closed (coupling-fix):** `assertOwnerTargetAllowed(from,to)` at both owner PATCH sites
  rejects owner `CANCELLED` from `{CONFIRMED,PREPARING,READY}` (403) — the widened edge is SYSTEM-only.
  Single shared helper = one authz source of truth.
- No new PII/secrets/external surface. Actor `system:dispatch_grace`. Tenant isolation unchanged (per-row
  `app.current_tenant`; FORCE-RLS tables only). Non-PII bus payloads (claim-check preserved).

---

## 9. Operability
- **Observability:** existing grace-window log kept; the R3-3 invariant is enforced statically in CI (green
  guardrail) without relying on the flag; dwell-alert resolution (F1) is now observable via
  `DWELL_ALERT_RESOLVED`.
- **Rollback:** pure code + domain-table change, no migration → plain deploy rollback.
  `DISPATCH_OWNER_GRACE_ENABLED` (default false) is the runtime kill-switch.
- **Scaling gate:** none (see §2).

---

## 10. Open / accepted risks
- **ACCEPTED — F4 pre-commit live-delta:** inherited from `updateOrderStatus`'s existing whole-codebase
  behavior; consequential effects are post-commit; negligible self-reconciling window. Owner: whoever later
  proposes a cross-cutting publish-timing change to `updateOrderStatus`.
- **ACCEPTED/FLAG — F8 lock order:** `updateOrderStatus`'s fold locks `orders` then `courier_assignments`
  (pre-existing for the IN_DELIVERY edge). Serialized by the sweep advisory lock + low volume. Implementer to
  confirm the accept/dispatch bind path locks `orders` before `courier_assignments`; if not, note for a
  follow-up (LOW). Owner: implementer.
- **ACCEPTED — prepaid refund gap → escalated to a pre-registered ETHICAL-STOP:** cancelling a paid prepaid
  order via `updateOrderStatus` writes no `refund_due` (only `completeDelivery` does). Pre-existing for
  `PENDING/IN_DELIVERY→CANCELLED`; both grace + crypto flags off today. Registered as
  **STOP-REFUND-BEFORE-GRACE** in the ADR (no co-enable until a paid grace-cancel writes `refund_due` or is
  proven impossible). Owner: payments council + grace-cancel council jointly.
- **DEFERRED-FLAG — owner-facing cancel-a-preparing-order:** the machine now *permits* the edge; exposing it
  to owners is a product decision owned by the grace-cancel STOP-ETHICS council. Until then the route guard
  keeps it SYSTEM-only.
- **OPEN — customer copy (counsel dignity condition):** grace-cancel customer message must attribute cause
  truthfully ("no courier was available" — not customer fault, not a kitchen rejection) and, when prepaid,
  state a refund is coming. i18n al/en. Implementer at build.

---

## Appendix — exact code-change shape (for the build step)

**1. Domain — `packages/domain/src/order-machine.ts` `TRANSITIONS`:**
```
CONFIRMED: ['PREPARING', 'IN_DELIVERY', 'CANCELLED'],   // + system-only dispatch-exhausted terminal
PREPARING: ['READY', 'CANCELLED'],                       // + system-only dispatch-exhausted terminal
READY:     ['IN_DELIVERY', 'PICKED_UP', 'CANCELLED'],    // + system-only dispatch-exhausted terminal
// IN_DELIVERY unchanged (already ['DELIVERED','CANCELLED','READY'])
```
Comment the three additions as SYSTEM-only (owner exposure closed at the route; see ADR addendum).

**2. Mutator — `apps/api/src/lib/orderStatusService.ts`: extend the R2-3 fold** so the assignment
terminalize runs on any `newStatus === 'CANCELLED'` (not only `currentStatus === 'IN_DELIVERY'`); keep the
`IN_DELIVERY → READY` revert branch as-is. Idempotent (`WHERE status IN (active)` → 0 rows = no-op). Do
**NOT** fold the `ORDER_CANCELLED` publish into the mutator (the mutator publishes pre-commit;
alert-resolve/job-cancel must be post-commit — keep it a caller responsibility). Leave the mutator's own
publish set unchanged.

**3. Route coupling-fix — new shared helper** (e.g. `apps/api/src/lib/orderAuthz.ts`):
```
const OWNER_FORBIDDEN_CANCEL_FROM = new Set(['CONFIRMED','PREPARING','READY']);
export function assertOwnerTargetAllowed(from: string, to: string) {
  if (to === 'CANCELLED' && OWNER_FORBIDDEN_CANCEL_FROM.has(from))
    throw { statusCode: 403, error: 'Cancelling an order in preparation is not available', code: 'CANCEL_NOT_PERMITTED' };
}
```
Call it in `routes/orders.ts` (before line 885, after the current-status read at 861-870) and in
`owner/dashboard.ts` `transitionOrder` (after the read at 631-635, before 639). Preserves
`PENDING→CANCELLED` + `IN_DELIVERY→CANCELLED`.

**4. Worker — `courier-offer-sweep.ts` `graceCancelExhausted`** (replace ~213-253): inside the per-row
`BEGIN` + `set_config`, `SELECT status ... FOR UPDATE`; **re-check `NOT EXISTS(active assignment)` under the
lock** (F7) → if bound, ROLLBACK + continue; call
`await updateOrderStatus(client, row.id, row.location_id, 'CANCELLED', {messageBus:this.messageBus,
comment:'dispatch_exhausted'})` (wrap in try/catch: 409 → ROLLBACK + continue); `COMMIT`; then **post-commit**:
`messageBus.publish(BUS_CHANNELS.ORDER_CANCELLED, {orderId, locationId, reason:'dispatch_exhausted'})` (F1) +
`NOTIFY_CUSTOMER_STATUS` (event CANCELLED) + keep the log. **Delete** the raw UPDATE, the manual history
insert, and the manual WS `order.status` publishes (now carried by `updateOrderStatus`, with the canonical
`fetchOrderDelta` shape — F5). Import `updateOrderStatus` + `BUS_CHANNELS`.

**5. Guardrail `RAW_CANCEL_ALLOW`: unchanged** — no new entry, no new raw UPDATE, no new export. Green because
the funnel is real.

---

## Red → green proof plan
1. **RED (baseline):** `node scripts/guardrail-deliver-v2.mjs` → exit 1 citing `courier-offer-sweep.ts:221`.
2. **GREEN (after fix):** exit 0; diff shows `RAW_CANCEL_ALLOW` unchanged (3 entries) AND no new exported
   cancel fn (F2 assertion — the fix is not a relocated raw cancel).
3. **Machine-edge PIN (F6 / counsel):** an exhaustive `assertTransition` test asserting the exact legal edge
   set incl. the three new `→CANCELLED` edges — so the widening is a conscious, tested change and any future
   drift (new state, removed edge) fails red. (Replaces the withdrawn "shadow transition table" pin.)
4. **Coupling-fix test (F2):** owner PATCH `CANCELLED` on a PREPARING order → `403 CANCEL_NOT_PERMITTED`;
   owner PATCH `CANCELLED` on a PENDING order → still succeeds (no regression).
5. **Functional test** (`DISPATCH_OWNER_GRACE_ENABLED=true`, order PREPARING, exhausted, no assignment): run
   the sweep. Assert: `orders.status='CANCELLED'` AND `timeout_at IS NULL`; exactly one
   `PREPARING→CANCELLED` history row (`actor='system:dispatch_grace'`); **`courier_cash_ledger` count = 0**
   (load-bearing cash-safety); `delivery_trace` count = 0.
6. **F1 test (the fix the old plan missed):** seed the order with an open dwell alert + a pending
   `notify.dispatch.<id>` escalation job; run grace-cancel; assert the dwell alert is RESOLVED
   (`DWELL_ALERT_RESOLVED` emitted / `location_alerts` row resolved) AND the escalation job is cancelled
   (`boss.cancel` called / job not in `created/active`).
7. **F7 anti-race test:** with the order having a freshly-`accepted` assignment, run grace-cancel → order
   NOT cancelled (skipped), no history row, no ORDER_CANCELLED publish.
8. **Idempotency:** second run over the now-terminal order → 409 caught, no second history/publish.
9. **Regression ledger row:** red→green recorded; the guardrail + the exhaustive transition test are the
   deterministic guardrails for this fix.
