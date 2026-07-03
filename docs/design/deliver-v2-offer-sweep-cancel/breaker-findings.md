# Breaker findings — Option B `cancelUndispatchableOrder`

Target: `docs/design/deliver-v2-offer-sweep-cancel/proposal.md` §4 chosen fix (Option B: a
`cancelUndispatchableOrder` primitive colocated in `apps/api/src/lib/orderStatusService.ts`).
RED-LINE surface: order state machine + cash-as-proof. Grounded in `order-machine.ts`,
`orderStatusService.ts`, `courier-offer-sweep.ts`, `deliveryCompletion.ts`, `lifecycle-handlers.ts`,
`registry.ts`, `guardrail-deliver-v2.mjs`.

Verdict up front: **NO CRITICAL confirmed** (the cash-leak hypothesis does not fire — see F3). **Two
HIGH blockers** (F1 missed-event → false post-cancel escalations; F2 gate-laundering + guardrail-blind
export). Rank: F1, F2, F3 HIGH · F4, F5, F6 MEDIUM · F7, F8 LOW.

---

## [HIGH] F1 · B-CONSIST / B-OPS · emit-drift: primitive omits `ORDER_CANCELLED` → dwell alerts never resolve, escalation jobs fire AFTER the honest cancel

**Break.** The canonical cancel side-effect chain is: caller publishes `BUS_CHANNELS.ORDER_CANCELLED`
(`owner/signals.ts:248`) → `lifecycle-handlers.ts:27` `handleTransition(msg,'CANCELLED')` →
`app_resolve_order_alerts()` resolves `dwell_pending/dwell_confirmed/dwell_preparing/dwell_en_route`
AND `boss.cancel('notify.dispatch.<alertId>')` cancels every pending escalation notify job
(`lifecycle-handlers.ts:9-10, 55-57`).

The grace path does **not** publish `ORDER_CANCELLED`. The current worker (`courier-offer-sweep.ts:243-252`)
and the proposal's Pass-4 emit set (§246: `orderChannel` + `dashboardChannel` + `NOTIFY_CUSTOMER_STATUS`)
publish to `orderChannel(id)` = `order:<id>` and to `dashboardChannel`. Note `orderChannel(id)` is the
per-order channel, **not** the topic `BUS_CHANNELS.ORDER_STATUS = 'order.status'` that lifecycle-handlers
also subscribes to (`registry.ts:6,47`) — so neither subscription in lifecycle-handlers ever fires for a
grace-cancel.

Concrete scenario: an order reaches Pass 4 precisely *because* it was dispatch-exhausted and the owner
was alerted for the full grace window — i.e. dwell alerts and `notify.dispatch.*` escalation jobs almost
certainly already exist. Grace-cancel terminalizes the order but:
1. the dwell alerts stay OPEN forever (dashboard shows "order stuck" on an order that is CANCELLED);
2. the pending escalation notify jobs are **never cancelled** → they fire minutes later, notifying the
   owner about an order that is already terminal-CANCELLED — a false, contradictory alert *after* the
   honest terminal customer push.

**Invariant violated.** Proposal §4 "one audited mutator carries the cancel semantics" and §180
observability claim. The primitive is presented as semantically complete for a cancel; it is not — it
drops the `ORDER_CANCELLED` fan-out that every other cancel path drives. (This gap pre-exists in the raw
worker, but Option B claims to *close* the semantics gap and does not.)

---

## [HIGH] F2 · B-SEC / B-ANTIPATTERN · colocation launders a machine-forbidden edge + creates a guardrail-blind, owner-callable cancel export

**Break — self-refuting justification.** `RAW_CANCEL_ALLOW` is **file-granular**
(`guardrail-deliver-v2.mjs:22-26,56`): `orderStatusService.ts` is allowlisted because it *is* the central
`updateOrderStatus` fold that runs `assertTransition`. Option B adds a **second** function in that file
that does a raw `UPDATE orders … 'CANCELLED'` while **bypassing `assertTransition`** for an edge the
machine explicitly forbids (`order-machine.ts:20-22`: CONFIRMED/PREPARING/READY have no CANCELLED edge).
The guardrail only checks the filename, so it cannot see that a machine-illegal transition now lives in a
"blessed" file. This is exactly the sin the proposal uses to **reject Option C** (§99-103: "launders an
illegal transition past the very gate meant to enforce the funnel"). B commits the same laundering, merely
relocated into a *more* trusted file — worse for reviewer trust, since anyone reading the guardrail assumes
every cancel in `orderStatusService.ts` funnels through `assertTransition`.

**Break — guardrail-blind export.** `cancelUndispatchableOrder` is an **exported** function in a module
already imported by `routes/orders.ts`, `owner/signals.ts`, `courier/assignments.ts`,
`telegram-webhook.ts`. The guardrail scans only for raw `UPDATE orders … CANCELLED` text — a *call* to
`cancelUndispatchableOrder(...)` is never scanned. So the primitive is a cancel channel that (a) skips the
machine, (b) skips the guardrail forever, (c) is one import away from any owner route. The proposal's
decisive reason for rejecting Option A (`orders.ts:849-885` passes request `newStatus` straight into
`updateOrderStatus`, so widening the machine unflags owner cancel) is **not eliminated** by B — it is
relocated into an unguarded, guardrail-invisible export and hidden from the state machine. A future dev
adding "owner cancels a preparing order" via this primitive gets the exact unflagged behavior Option A was
rejected for, with *no* STOP-ETHICS gate and a green guardrail.

**Invariant violated.** "Never weaken a gate" (proposal §103, self-improvement loop #1); order-machine as
sole transition authority.

---

## [HIGH] F3 · B-DATA (cash) · "cash-safe BY CONSTRUCTION" is false at the primitive boundary

**Break.** The primitive's doc comment (§213-214) asserts "there is no courier → never a till-hold …
cash-as-proof-safe BY CONSTRUCTION." But the primitive's own contract contradicts this: `ALLOWED_FROM`
**includes `IN_DELIVERY`** (§221) and it carries the R2-3 assignment-terminalize fold (§228-231) —
i.e. it *explicitly anticipates being called on an order that has an active courier assignment*. So
cash-safety is **not** a property of the primitive; it is a property of the single current caller's
`NOT EXISTS` SELECT (`courier-offer-sweep.ts:202-205`) plus the fact that `completeDelivery` writes the
`'hold'` ledger row and the order-terminal transition in the **same tx**
(`deliveryCompletion.ts:78-124`: hold at step 4 is atomic with `updateOrderStatus`→DELIVERED/CANCELLED
at step 2). Neither invariant is referenced or enforced by the primitive.

**Why it does NOT (yet) leak — honest disposition.** Because `completeDelivery` terminalizes atomically,
a non-terminal order (CONFIRMED/PREPARING/READY/IN_DELIVERY) provably has **no** `'hold'` row, so the
current single caller cannot strand a till-hold. The cash claim *holds transitively* — for reasons living
in a different file the primitive does not own. This is a **latent trap**, not a live leak: the deferred
Option-A promotion (§118-122) or any second caller of this exported primitive against an `IN_DELIVERY`
order breaks the transitive guarantee, and the ADR addendum records only "writes no ledger/trace" — which
is NOT the same claim as "no hold can exist." Severity HIGH because it is a RED-LINE (cash) invariant
asserted at the wrong boundary; downgrade to MEDIUM only if the primitive drops `IN_DELIVERY` from
`ALLOWED_FROM` or hard-asserts "no active assignment" internally.

**Invariant.** cash-as-proof: a hold must never be orphaned by a cancel. Held today only by external
coupling the primitive advertises as intrinsic.

---

## [MEDIUM] F4 · B-FAIL · publishes moved pre-commit = phantom terminal broadcast on COMMIT failure

**Break.** Proposal §152 states "WS publishes happen pre-commit, matching how every `updateOrderStatus`
caller already behaves," and the appendix puts the WS/dashboard publish inside the primitive (§233-235),
with the worker committing *after* (§245). The **current** worker deliberately publishes **post-commit**
(`courier-offer-sweep.ts:240-252`). Option B therefore *downgrades* this path: if `COMMIT` fails after the
primitive has already published `order.status`=CANCELLED and enqueued the customer push, the customer/
dashboard are told CANCELLED while the DB row rolls back to PREPARING. Next poll/fetch reconciles to
PREPARING → a terminal-state flip-flop and a customer push for a cancel that didn't happen. "Matching
`updateOrderStatus`" is true but is a regression from the safer post-commit shape the worker already had.

**Invariant.** honest-terminal (never tell the customer a terminal state that isn't durable).

---

## [MEDIUM] F5 · B-CONSIST · dashboard delta shape drift; no test binds the two emitters

**Break.** Canonical dashboard emit calls `fetchOrderDelta` → a full payload
(`orderStatusService.ts:20-51,196-204`: `total, currency, itemCount, shortId, createdAt`, all `*_at`
fields, `statusUpdatedAt`). The current worker publishes a **minimal** delta `{orderId,status,statusUpdatedAt}`
(`courier-offer-sweep.ts:247-249`), and the appendix pseudocode (§233-235) elides the delta entirely
("… messageBus.publish …"). So the implementer coin-flips between re-calling `fetchOrderDelta` and shipping
an impoverished delta. Either way there is **no test binding the primitive's emit to the canonical shape**
— the recent ORDER-TRACKING `*_at` additions to `fetchOrderDelta` show this delta evolves; the grace path
will silently drift stale. Proposal §10 accepts "overlap with history/bus logic" but never names the
delta-*shape* coupling.

**Invariant.** WS/dashboard contract parity across all transition emitters.

---

## [MEDIUM] F6 · B-ANTIPATTERN · silent second transition authority; machine says the edge is impossible while the DB records it

**Break.** `order-machine.ts` is documented/used as the sole transition authority
(`orderStatusService.ts:74-86` funnels every mutation through `assertTransition`). After Option B the DB
`order_status_history` will contain `CONFIRMED/PREPARING/READY → CANCELLED` rows
(`courier-offer-sweep.ts:232-235`, `actor='system:dispatch_grace'`) — transitions `assertTransition`
throws `IllegalTransitionError` for. No test or guardrail reconciles history against the machine (grep
found no exhaustive order-machine transition test; `test-stage26.ts:98` and `dashboard.ts:12` only assert
status *membership*, not edge legality). Any future audit/analytics/reconciliation that trusts
`assertTransition` as ground truth for "which transitions are real" is now wrong, silently.

**Invariant.** single-source-of-truth for legal transitions (`order-machine.ts`).

---

## [LOW] F7 · B-CONSIST (race) · TOCTOU between the batch SELECT and per-row FOR UPDATE; primitive re-checks orders.status but not the assignment

**Break.** Pass 4's `NOT EXISTS`(active assignment) SELECT (`courier-offer-sweep.ts:197-207`) runs
unlocked over the whole batch; the primitive then locks only the **orders** row (`FOR UPDATE`, §219) and
re-guards only `orders.status = $from`. The async COURIER_DISPATCH worker (fed by Pass 3 drain earlier in
the same tick) can bind a courier (`offered`/`assigned`/`accepted`, even `picked_up` if the loop is slow)
in the gap. Accepting/binding does **not** move `orders.status`, so the guard still passes and the
primitive cancels the order and terminalizes the fresh live binding via the fold. The courier is left
holding food on a CANCELLED order; a subsequent `completeDelivery` then fails
(`CANCELLED→DELIVERED` is illegal), so no orphan hold is created (see F3) — but the customer was told
CANCELLED while a courier was en route. Low probability (single-digit/day volume per §2, ms-scale window
for a single row) but the `NOT EXISTS` guarantee the cash-safety rests on is **not** re-validated under
the lock.

**Invariant.** anti-race: a guarded cancel must re-check the condition it relies on (here: no active
assignment) under the row lock, not only `orders.status`.

---

## [LOW] F8 · B-SCALE (deadlock) · lock order orders→courier_assignments

**Break.** The primitive acquires the `orders` row lock (`FOR UPDATE`) then updates
`courier_assignments` (fold). Any concurrent path that locks `courier_assignments` before `orders`
(courier accept / dispatch bind) is an ABBA deadlock candidate. Mitigated in practice by the sweep's
session advisory lock (`courier-offer-sweep.ts:56`) serializing the sweep itself and by low volume, so the
only counterparty is the dispatch/accept path — real but low-likelihood. Flagged for the implementer to
confirm the accept path's lock order.

---

### Regression note (for the build's red→green plan §255)
The proposal's proof plan (§262-269) asserts `courier_cash_ledger`/`delivery_trace` count = 0 and one
history row — it does **not** cover F1 (no assertion that dwell alerts resolve / escalation jobs cancel),
F4 (post- vs pre-commit publish ordering), or F6 (no history-vs-machine reconciliation test). The green
guardrail proof (§256-259) is exactly the laundering in F2 — a green scan is not evidence the transition
is legal.
