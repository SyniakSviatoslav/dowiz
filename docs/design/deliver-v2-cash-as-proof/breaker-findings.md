# Breaker Findings — `deliver` v2 (Cash-as-Proof) + Courier Accept/Decline

> BREAKER seat, Triadic Council. Attack-only — no fixes (architect's job).
> Grounded against live source @ HEAD (feat/mvp-sensor-seams). Each finding:
> severity · vector · concrete break/number · violated invariant · file:line.

Verdict: the capacity/atomicity claims hold, but **the §A "no-trap" red-line is refuted three
ways**, all rooted in one un-modeled constraint (`courier_assignments_order_uniq`) plus a missing
terminal transition. The proof "offer-timeout/decline/reassign can NEVER touch customer order state"
is false in the live code the design extends, and the v2 migration does not close it.

---

## CRITICAL

### C-1 · B-CONSIST/B-DATA · The re-offer / reassign loop is physically impossible — `order_uniq` blocks the second assignment row
- **Fact:** `courier_assignments_order_uniq` is a **FULL** unique index on `order_id`
  (`1780421100041_courier-assignments.ts:23` — no `WHERE`), so an order has **at most one
  assignment row, forever.** All six `INSERT INTO courier_assignments` sites use **plain INSERT, no
  `ON CONFLICT`** (`server.ts:653`, `orders.ts:803`, `owner/dashboard.ts:303`,
  `workers/courier-dispatch.ts:84`, `dev/mock-auth.ts:149,536`).
- **Break sequence (already live today):** courier rejects → `assignments.ts:185` sets the row
  `status='rejected'` (row stays, keeps its `order_id`) → re-enqueues to `courier_dispatch_queue`
  (`:194`) → dispatch worker fires → `courier-dispatch.ts:84` `INSERT … (order_id …)` → **unique
  violation on `courier_assignments_order_uniq` → txn 500 → job retried forever → order never
  reassigned.** 100% of rejected-then-redispatched orders are stuck.
- **v2 makes it load-bearing:** §A's offer→decline→**re-offer to next courier**, and B2's
  `offered_expired`→requeue, both require creating a **new** `'offered'` row for the same `order_id`.
  The §5 migration only rewrites `courier_one_active_assignment` (the `courier_id` partial index) and
  **never touches `courier_assignments_order_uniq`** (proposal.md:198-200). The central loop of the
  whole feature cannot create its second row.
- **Violated invariant:** no-trap-state (decline/timeout must return order assignable); "deterministic
  single result" (§6:241-244). Back-of-envelope: re-dispatch success rate = **0%** for any order that
  ever had a prior assignment row.

### C-2 · B-CONSIST/B-FAIL · §A no-trap REFUTED — courier-cancel of an owner-direct delivery leaves the order permanently stuck `IN_DELIVERY`
- **Fact:** the owner-direct path sets the **order-side mirror**: `updateOrderStatus(…,'IN_DELIVERY')`
  + `UPDATE orders SET courier_id=$1` (`dashboard.ts:315-319`). The courier `cancel` path
  (`assignments.ts:428-446`) updates **only** `courier_assignments.status='cancelled'` and
  `courier_shifts='available'` — it **never reverts `orders.status` or clears `orders.courier_id`.**
  The machine has **no `IN_DELIVERY→CANCELLED`** (order-machine.ts:23 — `IN_DELIVERY: ['DELIVERED']`).
- **Break sequence:** owner force-assigns O (order → `IN_DELIVERY`, `courier_id=A`, assignment
  `'accepted'`) → courier A taps cancel within the 5-min window → assignment `'cancelled'`, shift
  freed, **order still `IN_DELIVERY` with `courier_id=A` and no live assignment.** To deliver, someone
  needs a `'picked_up'` row; the owner pickup-proxy (`dashboard.ts:359-367`) requires an `'accepted'`
  row (now cancelled → 409). Owner reassign → `INSERT` collides with the cancelled row's `order_id`
  (C-1) → 500. Any bus-driven `IN_DELIVERY→CANCELLED` → `assertTransition` throws
  `IllegalTransitionError`. **The order is trapped `IN_DELIVERY` forever; the customer sees "out for
  delivery" indefinitely.**
- **Violated invariant:** 🔴 §A red-line — "timeout/decline never touch the customer's order, only the
  courier binding is rolled back." Here the courier binding rolls back but the **customer order is
  left in a terminal-stuck non-recoverable state.** The only `IN_DELIVERY→READY+courier_id=NULL`
  revert that exists (`dashboard.ts:264-268`) fires **only** in the busyCheck branch when the *same*
  courier is reassigned to a *different* order — it does not cover cancel/decline.

### C-3 · B-CONSIST · Concurrent decline ↔ owner-reassign is non-deterministic and 500s — not "first guarded UPDATE wins"
- **Claim under test (§6:241-244):** "both target the same single `courier_assignments` row; the first
  status-guarded UPDATE wins … Deterministic single result." **False for the reassign path.**
- **Why:** owner-reassign (`dashboard.ts:212-330`) is **not a guarded UPDATE of courier A's row** — it
  is a fresh **INSERT** for courier B. Its only collision check is `busyCheck` on the *new courier's*
  id `WHERE ca.courier_id=$1 AND status IN ('accepted','picked_up')` (`:246-251`) — it does **not**
  inspect the order's existing `'offered'`/`'assigned'` row, and the status set does **not include
  `'offered'`**.
- **Break sequence:** order O is `'offered'` to A. At T=0 the 5-min sweep fires (A→`'offered_expired'`,
  order requeued) AND owner taps reassign O→B. Owner's `INSERT (O,B …)` races the leftover/expired A
  row on `order_id`: if A's row still exists → **unique violation → 500** (owner told "reassign
  failed"); if the sweep already produced a requeue → dispatch worker ALSO tries to INSERT (O, next)
  → second collision. Two writers, one constraint, no guard → **outcome depends on commit ordering,
  not on a rowcount authority.** Add a simultaneous courier decline and there are three writers on a
  table that allows exactly one row per order.
- **Violated invariant:** status-guarded transitions (rowcount>0) — the reassign path has **no**
  rowcount-0→409 guard on the prior binding; "double-offer impossible" relies on
  `courier_one_active_assignment` (courier_id) which says nothing about two writers contending the
  same `order_id`.

---

## HIGH

### H-1 · B-CONSIST · No-cash tail mislabels the order as `DELIVERED` — there is no failure terminal
- **Fact:** `IN_DELIVERY → ['DELIVERED']` is the only transition (order-machine.ts:23). The delivered
  handler calls `updateOrderStatus(…,'DELIVERED')` **regardless of `cash_collected`**
  (`assignments.ts:340`). There is no `FAILED`/`RETURNED`/door-cancel terminal.
- **Break:** `refused_goods` / `customer_cancelled_on_door` → courier taps completion with
  `cash_collected=false` → **order becomes `DELIVERED`** and the customer's status page shows
  "Delivered" for food they refused / a door-cancel. The alternative (leave it `IN_DELIVERY`) is a
  permanent stuck state. Either branch is a trap or a lie.
- **Violated invariant:** no-trap-state + customer-facing honesty. The §7 table (proposal.md:261)
  claims these "trip closes, food returns, never blocks the courier" — true for the *courier*, but it
  silently asserts the order reaches a clean terminal; the live machine cannot express a non-delivered
  terminal, so the immutable customer record is **wrong**.

### H-2 · B-CONSIST/B-DATA · `paid_partial` has no honest representation → 422 trap or silent un-surfaced courier debt
- **Fact:** completion requires `cash_collected && cash_amount === total` else **422
  `CASH_AMOUNT_MISMATCH`** (`assignments.ts:324-327`). The `'hold'` ledger row is written **only** when
  `cash_collected` (`:353-359`) and always for `cash_amount` (= exact total).
- **Break:** courier collected partial cash (e.g. customer paid 800 of 1000).
  - `cash_collected=true, cash_amount=800` → `800 !== total(1000)` → **422; cannot complete** → order
    stuck `IN_DELIVERY` (H-1).
  - `cash_collected=false` → completes, **no ledger row**, the 800 ALL collected is **unrecorded** →
    the shortfall is invisible; reconciliation (which runs off `'hold'` rows) sees **zero** owed →
    **silent courier debt with no surfacing.**
- **Violated invariant:** money must be reconcilable; "the cash bond is the proof." `paid_partial` is
  named as a first-class tail outcome (proposal.md:25,261,322) but the ledger model can represent only
  full-or-nothing → the proof mechanism has a hole exactly where partial fraud lives.

### H-3 · B-CONSIST/B-SEC · Tail outcomes collapse to one boolean — the "immutable record" cannot store the distinction the proof model depends on
- **Fact:** the delivered body schema is `z.object({ cash_collected: z.boolean(), cash_amount:
  z.number().optional() }).strict()` (`assignments.ts:276-279`). There is **no field** for
  `payment_outcome` or a refusal reason. `payment_outcome` is read in `orders.ts:46`/`dashboard.ts:124`
  but is **never written** by the delivered handler (grep: zero writes in the completion path).
- **Break:** `refused_goods`, `refused_payment`, `customer_cancelled_on_door` all map to the single
  signal `cash_collected=false`. A courier who pockets the food and lies "refused" is **byte-identical**
  in the record to a genuine refusal. The proposal's §6 "UPDATE orders SET payment_outcome=$1" has no
  input channel — the courier app cannot send the distinguishing value the design says it records.
- **Violated invariant:** "crumbs recorded, burden on the accuser" presupposes a record that
  distinguishes outcomes; here the distinguishing crumb is **not collectable**. Quantifies R-3: the
  hole is not "1-in-N content disputes" — it is **every** no-cash completion.

### H-4 · B-ANTIPATTERN/B-CONSIST · R-2 "unify the paths" is not structurally enforced — owner-direct still produces `DELIVERED` with no offer/accept handshake
- **Fact:** the handshake ships behind `COURIER_OFFER_HANDSHAKE_ENABLED` **default-off**
  (proposal.md:303), and the owner-direct path (`dashboard.ts:302-320`) is a **separate code path**
  that INSERTs `'accepted'` and force-drives `IN_DELIVERY`. Adding `'offered'` to the CHECK does not
  make it **mandatory** — nothing in the machine or a constraint requires `offered→accepted` before
  `delivered`.
- **Break:** with the flag off (the launch default), behavior is **unchanged**: an order reaches
  `DELIVERED` having never been `'offered'`, with `courier_assignments.status` jumping
  `accepted→picked_up→delivered` by owner proxy. The §9 guardrail only checks that crumbs don't gate
  state; it does **not** assert every delivered order passed a handshake. The dual-write divergence the
  ADR claims to close (Decision §7) persists in production until a future, unscheduled flag flip.
- **Violated invariant:** "single formalized path"; the design's headline fix is a doc intention, not a
  guarded transition.

---

## MEDIUM

### M-1 · B-SEC · The R-1 `FORCE` fix does NOT close the cross-courier IDOR it is sold against
- **Fact:** `courier_assignments` policy isolates by **location** only (`location_id =
  current_setting('app.current_tenant')`, `1780421100041:28-29`). Cross-**courier** hijack (courier B
  acting on courier A's offer in the *same* location) is prevented **solely** by the inline `AND
  courier_id=$me` predicate (`courierAssignmentService.ts:24,50`; `assignments.ts:175,236,298`). RLS —
  location-scoped — cannot gate intra-location cross-courier access whether or not `FORCE` is on.
- **Break:** any v2 query that forgets `AND courier_id=$me` (e.g. a new `decline`/`accept` on
  `'offered'`) → courier B accepts/declines A's offer. The ADR (Red lines, §8/R-1) frames adding
  `FORCE` as bringing "the offer handshake (a cross-courier IDOR surface)" to canon — but `FORCE`
  closes only the **owner/BYPASSRLS** bypass, **not** the cross-courier vector the offer actually
  introduces. The real defense remains a hand-written predicate on every new query.
- **Violated invariant:** RLS-as-isolation is mis-attributed; the isolation guarantee for the new
  surface is app-code discipline, not the DB.

### M-2 · B-DATA/B-SEC · `cash_amount` Zod is unvalidated (`z.number().optional()`) — float/negative is latent once the equality guard is relaxed
- **Fact:** schema is `cash_amount: z.number().optional()` (`assignments.ts:278`) — **not**
  `.int().nonnegative()`. Today the `cash_amount === total` guard (`:324`) incidentally rejects floats
  and negatives (total is integer). The DB has `CHECK (amount >= 0)` (ledger:17) but no integer-domain
  guard at the edge.
- **Break:** H-2/H-3 force the design to add a tail/`paid_partial` path that **drops the equality
  guard** (partial ≠ total by definition). With the guard gone, `cash_amount: -5` or `100.5` reaches
  the insert → negative trips the DB CHECK as an **ungraceful 500** (not a 422), float `100.5` →
  integer column → `invalid input syntax` 500. Money validation lives in a guard that the new
  requirements must remove.
- **Violated invariant:** money integer `CHECK(>=0)` enforced at the boundary with a graceful contract,
  not via a coincidental equality.

### M-3 · B-ANTIPATTERN/B-SEC · The §9 "crumbs-passive" guardrail is unfalsifiable as specified — gps in `delivery_trace` can become a silent threshold
- **Fact (verified):** `delivery_trace` is currently read by **no** decision path (grep: only the
  INSERT at `assignments.ts:345`; the only other match is a comment). So the passive claim holds
  **today**. But §9's guardrail is described as "a lint/test asserting no code path reads
  `delivery_trace`/`order_sensor_events`/`customer_signals` to branch state" — while §8 **explicitly
  allows** writing+**displaying** them, and v2 adds `gps_lat/gps_lng` to the trace (proposal.md:206-207).
- **Break:** a future Stage-21 reconciliation or owner alert that *reads* `delivery_trace.gps_lat`,
  computes venue-proximity, and **surfaces "courier was 4 km away at delivery"** is a `read` that the
  lint cannot distinguish from an allowed "display" — yet an owner acting on that surfaced number IS
  the verdict-gate by another name (the exact thing being removed). The guardrail draws no
  line between "display" and "owner-actionable signal," so the discipline rule cannot be mechanically
  enforced.
- **Violated invariant:** "never build the verdict engine" needs a falsifiable gate; as written it is
  advisory prose.

---

## LOW

### L-1 · B-CONSIST · Double-tap `delivered` returns 404 on a legitimate post-commit network retry
- The delivered guard `SELECT … WHERE status='picked_up' FOR UPDATE` (`assignments.ts:298`) returns 0
  rows after the first commit → **404 `…NOT_PICKED_UP`** to a client that simply retried after a
  dropped response. No double-HOLD (good — see note), but the courier UI sees an error on a successful
  delivery. Cosmetic-class, not a money bug.

### L-2 · B-ANTIPATTERN/B-SEC · R-4 concrete card break — completion never reads `payment_method`, so "cash=proof" mis-fires the instant card flips on
- The delivered handler keys entirely off `cash_collected` (`assignments.ts:285,353`) and **never
  reads `payment_method`** (not in the query at `:292-299`, not in the schema). When `payment_method`
  gains `'card'`, a prepaid order completed with `cash_collected=false` writes **no ledger hold** and
  goes `DELIVERED` with **no proof artifact at all** — while card-scheme/Albanian-consumer-law shift
  the chargeback burden to the merchant. The "burden-on-the-accuser" model inverts silently with zero
  code signal. R-4 is named as deferred, but the seam (a hard dependency on `cash_collected` with no
  `payment_method` branch) is being **built now**, not deferred.

---

## Verified NON-findings (attacked, held — stated to avoid severity theatre)

- **HOLD atomicity is SOUND.** The status-guarded order→`DELIVERED` UPDATE, `delivery_trace` insert,
  and `courier_cash_ledger` `'hold'` insert all execute inside **one** `BEGIN…COMMIT`
  (`assignments.ts:289-361`). A mid-txn failure `ROLLBACK`s everything — **no partial completion**.
- **Double-HOLD is prevented.** Ledger insert is `ON CONFLICT (order_id, type) DO NOTHING`
  (`:357`); trace is `ON CONFLICT (order_id) DO NOTHING` (`:348`); the `status='picked_up'` guard
  makes the second tap a no-op (it 404s before any write). **No idempotency hole on the money path.**
- **Claim-check holds.** Bus payloads in this flow are id-only — `ORDER_DELIVERED {orderId,
  locationId, courierId, cashCollected, cashAmount}` (`:364-370`), reject's `ORDER_CONFIRMED
  {orderId, locationId}` (`:208`), `fetchOrderDelta` strips item names (`orderStatusService.ts:21-31`).
  No coords/PII on the bus; GPS/`name_snapshot` stay in RLS-FORCE `delivery_trace`. Sweep payload as
  designed is `order_id` only.
- **Back-of-envelope is honest.** 6.7 orders/s @ 10×, ~7 short-lived conns, +0 pools, +0 queues —
  capacity is genuinely a non-issue; the design's "optimize for correctness not throughput" is correct.
  (The correctness side is where C-1..C-3 live.)
</content>
</invoke>

---

## RE-ATTACK round 2

> BREAKER seat, regression round. Verified each fix against live source @ HEAD
> (feat/mvp-sensor-seams). Verdict: the three CRITICALs are genuinely closed at the DB-shape
> level, **but the fix-set patched only ONE of the system's TWO completion paths and widened a
> SHARED transition map without auditing its other callers** — re-opening the no-trap and
> money-proof red lines on paths the resolution never names. Three HIGH regressions below.

### Verified CLOSED (no inflation)
- **C-1 migration index-swap is SAFE.** `courier_assignments_order_uniq` is a FULL unique on
  `order_id` (`1780421100041_courier-assignments.ts:23`, no WHERE) → there can be **zero** duplicate
  `order_id`s in the table → the partial `… WHERE status IN (active)` subset **cannot** fail on
  existing rows. The active set `('offered','assigned','accepted','picked_up')` covers every
  non-terminal state incl. `picked_up` (the IN_DELIVERY/en-route equivalent on the binding row) — no
  gap. **Closed.**
- **C-3 single-winner holds.** Two writers contend the **same** row via the guarded
  `UPDATE … WHERE order_id=$o AND status IN ('offered','assigned','accepted')`; under READ COMMITTED
  the second waits on the row lock then re-evaluates the WHERE against the now-`offered_expired`
  row → rowcount 0 → 409, **no INSERT**. Only the winner inserts the new active row → no double-active
  window. **Closed.**
- **M-3b customer-snapshot IDOR is closed.** The snapshot read piggybacks the existing customer route
  `WHERE o.id=$1 AND o.customer_id=$2` (JWT `sub`, `customer/orders.ts:46-47`) → no cross-order /
  cross-tenant read. **Closed.**

### HIGH (new / regression)

#### R2-1 · B-CONSIST/B-DATA · The fix-set patched the courier deliver handler only — the **owner-proxy deliver path bypasses every H-1/H-2/H-3/M-2 fix AND the ledger `'hold'` the whole security model rests on**
- **Fact:** there are **two** completion paths. The resolution rewrites only the courier one
  (`courier/assignments.ts:285-380`). The **owner-proxy deliver** (`owner/dashboard.ts:408-484`):
  - writes **no `payment_outcome`** (H-3 hole reopens for owner-completed orders),
  - writes **no `delivery_trace`** row (no INSERT in the block) → the immutable crumb / GPS / customer
    evidence is **absent** for owner completions,
  - writes **no `courier_cash_ledger` `'hold'`** even when `cashCollected=true` (`:446`) → **collected
    cash with zero reconciliation record**,
  - has **no `cash_amount===total` guard** and **no Zod int/nonneg** (`body?.cash_amount` raw, `:413`),
  - always drives `DELIVERED` (no `CANCELLED` no-cash tail).
- **Break:** every owner-tapped completion silently skips the `'hold'` that "makes lying cost money."
  The cash-as-proof primitive (Stage-21 HOLD atomic with DELIVERED) is **not written** here →
  reconciliation sees `0` owed → the exact silent-courier-debt the design claims to close, on a live
  path the resolution never mentions.
- **Invariant:** 🔴 HOLD-atomic-with-DELIVERED / money reconcilable / `payment_outcome` recorded.

#### R2-2 · B-CONSIST/B-FAIL · C-2 mirror-revert is gated behind the 5-min cancel window → the realistic stuck-`IN_DELIVERY` case is NOT recovered
- **Fact:** the courier `cancel` handler returns **410 `CANCEL_WINDOW_EXPIRED`** when
  `now − assigned_at > CANCEL_AFTER_DISPATCH_WINDOW_MS` (default **300000** = 5 min)
  (`assignments.ts:434-438`). The C-2 mirror-revert lives **inside** this handler, **after** that gate.
- **Break:** a `picked_up` order en route **>5 min** whose courier cannot deliver (unreachable,
  accident, refusal): `cancel` → **410** (mirror-revert never reached); reassign terminalize excludes
  `picked_up` → **409**; the only remaining exit is an owner-proxy **deliver lie** (R2-1) or a no-show
  cancel that strands the assignment (R2-3). The order is **stuck `IN_DELIVERY`** for the common
  long-delivery case. C-2 is FIXED only for cancels **inside** the 5-min window — i.e. precisely
  *not* the case where a courier is already out with the food.
- **Invariant:** 🔴 §A no-trap (decline/cancel never leaves the customer order non-recoverable).

#### R2-3 · B-CONSIST · The C-2 edge-widening (`IN_DELIVERY→CANCELLED`) is global in the shared machine but only the courier-cancel handler terminalizes the assignment → the **no-show cancel path now creates a dangling ACTIVE assignment** (courier blocked forever)
- **Fact:** the fix widens the **shared** map to `IN_DELIVERY:['DELIVERED','CANCELLED','READY']`
  (`order-machine.ts:23`). Every `updateOrderStatus(…,'CANCELLED')` caller inherits the new source.
  The no-show route `owner/signals.ts:234` calls exactly that — it previously threw
  `IllegalTransitionError` for an `IN_DELIVERY` order (safely no-op), but now **succeeds** and **never
  terminalizes the `courier_assignments` row**.
- **Break:** owner marks no-show on an `IN_DELIVERY` (picked_up) order → order `CANCELLED`, but the
  assignment stays `'picked_up'` (**active**) → `courier_one_active_assignment`
  (`proposal §5:218`, partial-unique on active) **blocks that courier from every future assignment
  forever**, and an active binding now points at a `CANCELLED` order. A **new trap the fix introduced**
  by widening a shared transition without auditing its other callers.
- **Invariant:** single-active-binding / no-trap.

### MEDIUM

#### R2-4 · B-DATA · H-2 silent debt is MOVED, not closed — partial-cash-with-goods-delivered has no ledger home
- Forbidding `paid_partial` forces a courier who **physically handed over the food** and collected
  partial cash to tap `refused_payment` (→`CANCELLED`, "food returns"). In COD Albania, completing a
  delivery on partial pay-with-goods is routine, not a refusal. Result: order `CANCELLED`, **zero**
  ledger `'hold'`, courier holds N units **unrecorded** → reconciliation under-counts → the same
  silent debt H-2 named, relocated onto the `refused_payment` tail. The honest-record claim
  ("no courier ever holds unrecorded cash") fails on the most common partial case.
- **Invariant:** money reconcilable / no unrecorded cash.

#### R2-5 · B-CONSIST · Courier-cancel mirror-revert (→READY) collides with the handler's `ORDER_CANCELLED` bus publish
- The cancel handler unconditionally publishes `BUS_CHANNELS.ORDER_CANCELLED`
  (`assignments.ts:448-452`) at its tail. With the C-2 revert the order is now `READY`, yet the bus
  emits **CANCELLED** → the customer receives a contradictory "cancelled" signal for an order that is
  actually back to assignable (the `updateOrderStatus(READY)` revert emits its own READY event too).
  Two conflicting events from one transaction.
- **Invariant:** customer-facing honesty / state↔event coherence.

#### R2-6 · B-CONSIST · Reassign's other-order revert is a RAW `UPDATE` bypassing the machine + WS — the resolution claims reverts route through `updateOrderStatus`
- The busyCheck branch reverts the **new** courier's previously-`IN_DELIVERY` order via
  `UPDATE orders SET status='READY', courier_id=NULL WHERE id=$1` (`dashboard.ts:264-268`) — **not**
  `updateOrderStatus` → no `order_status_history`, **no WS event** → that order's customer is stranded
  on stale "out for delivery". This sits in the exact handler the C-3 terminalize-then-insert rewrite
  touches, but the resolution's "reverts route through `updateOrderStatus` (audit + WS)" claim
  (§5:252-257) does not cover it.
- **Invariant:** audit + realtime coherence on every transition.

#### R2-7 · B-DATA/B-SEC · The "90-day GPS retention, then anonymize to NULL" red-line has no enforcing artifact
- `delivery_trace` is referenced by **zero** workers (`grep apps/api/src/workers` → none; existing
  retention sweeps cover acquisition / access-request / anonymizer only). The §8 claim
  "retention = 90 days, then GPS anonymized to NULL (🔴 anonymize-not-delete)" is a **documented
  intention with no sweep** → `gps_lat/gps_lng` are retained **indefinitely**. A red-line asserted in
  prose with no mechanism is not a control.
- **Invariant:** data-minimization / anonymize-not-delete (claimed §E red-line).

### LOW

#### R2-8 · B-SEC · M-1 guardrail phrasing ("no `courier_assignments` mutation lacks `AND courier_id=$authed`") mis-scopes the owner paths
- Owner reassign / pickup / deliver legitimately mutate assignment rows with **no** `courier_id=$me`
  (owner authority, location-scoped — `dashboard.ts:256,377,446`). The §9 guardrail as written either
  red-flags valid owner code or must carve them out; stated universally it gives a false sense the
  predicate is global. Precision, not a hole.

#### R2-9 · B-SEC · The route M-3b extends already leaks courier PII to the customer (pre-existing)
- The "customer sees their OWN snapshot, **NOT** the courier `gps`/`name` crumbs" framing is already
  contradicted by the live route it piggybacks: `customer/orders.ts:64-75` decrypts and returns
  courier `full_name`/`phone` + live `courier_positions` GPS during active delivery. Out of this
  change's scope, but the §8 minimization claim is already false on that surface.

### Net round-2 verdict
CRITICALs C-1/C-2/C-3 hold at the DB-shape level (index-swap safe, single-winner deterministic,
re-offer unblocked). **But C-2's no-trap guarantee is breached two new ways** — gated behind the 5-min
cancel window (R2-2) and the shared-edge widening stranding the no-show path (R2-3) — and the
**money-proof red line is unguarded on the owner-proxy completion path the resolution never named**
(R2-1). H-2's silent debt is relocated, not removed (R2-4). The fixes are correct where they were
applied; the misses are **the second completion path and the unaudited callers of the widened machine**.

---

## RE-ATTACK round 3 (final regression — did centralization create new holes?)

> BREAKER seat, convergence check. Attacked ONLY the round-2 fixes (`completeDelivery`, `/abort`,
> the `updateOrderStatus` fold, the two guardrails, the retention worker) against live source @ HEAD.
> Verdict: the centralization closes the round-2 HIGHs at design level, but the **two NEW runtime
> artifacts it introduced (the cross-tenant retention sweep + the `/abort` accepted-branch) each carry
> a concrete break the resolution does not cover.** One HIGH, one MED, one LOW below. Not converged.

### CONFIRMED-CLOSED (attacked, genuinely held — no inflation)

- **R2-1 owner-proxy completion + the null/owner-courier HOLD hypothesis → CLOSED.** The hypothesized
  hole (owner-proxy completion with NO courier → `courier_cash_ledger.courier_id` NOT NULL violation or
  a HOLD mis-attributed to the owner) **cannot occur**: the owner-proxy `/deliver`
  (`owner/dashboard.ts:432-440`) **requires** an assignment `WHERE order_id=$1 AND status IN
  ('accepted','picked_up')`, rowcount-0 → 409, and resolves `courierId` from
  `assignmentRes.rows[0].courier_id`. The courier path resolves it identically. There is **no** path
  reaching `completeDelivery` without a real, non-null `courier_id` (every `IN_DELIVERY` order has an
  assignment with a courier — `assign-courier` requires `courierId`; the owner PATCH auto-assign only
  inserts when a courier is available). So the HOLD is attributed to the **assigned** courier on both
  paths — correct, no constraint violation. (`courier_cash_ledger:13` `courier_id uuid NOT NULL`;
  `dashboard.ts:442`.)
- **R2-2 `/abort` ↔ `/deliver` race + picked_up→CANCELLED honesty → CLOSED (for the picked_up branch).**
  Concurrent `/abort` and `/deliver` by the same courier serialize on the **same** `courier_assignments`
  row: both are status-guarded UPDATEs, `/deliver` takes `… WHERE status='picked_up' FOR UPDATE`. Under
  READ COMMITTED the second re-evaluates its WHERE against the committed new status → rowcount-0 → 404.
  No double-terminal, no HOLD-then-cancel. The `picked_up`→`CANCELLED` exit is the honest terminal
  (reason `courier_aborted_en_route` on the assignment + `courier_aborted` on the order), routes through
  `updateOrderStatus(CANCELLED)` (the central fold terminalizes the binding idempotently), and the
  customer is informed via the `order.status` CANCELLED delta.
- **R2-3 fold covers `signals.ts:234` + `orders.ts:779` → CLOSED for those callers.** Both verified to
  route through `updateOrderStatus` (`owner/signals.ts:234`; `orders.ts:778-781`, owner-role-gated PATCH),
  so the in-tx terminalize-the-binding fold applies. The no-show no-longer-strands a `picked_up` row.
- **Parity + Stage-21 guardrails are falsifiable.** The R2-1 parity test (every `DELIVERED` ⇒
  `delivery_trace` row, `paid_full` ⇒ `'hold'`) is red against the current inline owner-proxy body and
  green only when both route through `completeDelivery` — falsifiable **provided the test exercises the
  owner-proxy path, not only the courier path** (the one residual implementation risk; flagged, not a
  design hole). The Stage-21 `stage21-no-auto-deduct.invariant.test.ts` (asserts
  `ADR-stage21-reconciliation.md` exists + contains `NO-AUTO-DEDUCT` & `NO-COURIER-SCORING`) is genuinely
  RED-on-disk today and goes green only when the ADR is authored — an honest forcing-tripwire (it gates
  the *record*, not the *mechanism* — acknowledged in the resolution).

### HIGH (new — introduced by the round-2 retention worker)

#### R3-1 · B-SEC/B-DATA/B-OPS · The 14-day GPS-anonymize sweep matches **zero rows** under `delivery_trace`'s tenant-scoped FORCE RLS — the 🔴 anonymize-not-delete red-line silently does not fire, and the boot-assert cannot detect it
- **Fact:** `delivery_trace` is `ENABLE + FORCE ROW LEVEL SECURITY` with a **tenant-scoped** policy
  `USING ( location_id IN (SELECT app_member_location_ids()) )` (`1790000000027_delivery-trace.ts:22-25`).
  `app_member_location_ids()` is derived from the request's `app.user_id`/`app.current_tenant`. The
  round-2 worker (`delivery-trace-retention.ts`) is specified to **"mirror `access-request-retention.ts`"**
  — which runs on `createOperationalPool()` and sets **no tenant context** (verified: neither
  `access-request-retention.ts` nor `acquisition-retention.ts` issues any `set_config('app.user_id'…)`).
- **Why the precedent does not transfer:** `access_requests` uses an **ops allow-all** policy
  `FOR ALL USING(true)` (`1790000000041_access-requests.ts:47-49`) → its global sweep passes RLS
  regardless of context. `delivery_trace`'s policy is **not** `USING(true)` — it is tenant-scoped. A
  worker on the operational role with no tenant context set → `app_member_location_ids()` returns
  **empty** → `UPDATE delivery_trace SET gps_lat=NULL … WHERE delivered_at < now()-$1` matches **0 rows**
  → GPS/`name_snapshot`/`price_snapshot` are **retained indefinitely**, the exact red-line R2-7 claimed
  to convert "from prose to an enforced control."
- **The guardrail does not falsify this:** `assertDeliveryTraceSchedule()` asserts the **schedule exists**,
  not that the sweep **anonymizes ≥1 row**. So the boot-assert stays GREEN, the cron fires nightly, and
  it anonymizes nothing — a silent false-green on a 🔴 PII red-line.
- **Contingency, stated honestly:** the sweep works **only if** the operational role carries `BYPASSRLS`
  (which bypasses even FORCE). Project memory flags this as an *uncertain env artifact*
  (`verify:rls` anomaly). Relying on an undocumented `BYPASSRLS` attribute for a PII-anonymization
  control is itself fragile: the day the operational role is correctly stripped of `BYPASSRLS`, the
  retention red-line breaks **silently**. Either way the design's stated mechanism ("mirror
  access-request-retention") is **insufficient** — it provides neither a `USING(true)` ops policy, a
  per-tenant context loop, nor a justified `BYPASSRLS` for a cross-tenant sweep of a tenant-scoped FORCE
  table.
- **Violated invariant:** 🔴 anonymize-not-delete / data-minimization, asserted (§E, R2-7) as an
  *enforced* control; it is not enforced, and the guardrail measures schedule-existence, not efficacy.

### MEDIUM (new — introduced by the round-2 `/abort` action)

#### R3-2 · B-CONSIST · `/abort` from an **`accepted`** assignment forces `updateOrderStatus(…,'READY')` on an order that never left its pre-pickup status → `IllegalTransition`/`SameStatus` 400 → the whole abort tx rolls back, the assignment is never freed (manifests under flag-ON)
- **Fact:** under `COURIER_OFFER_HANDSHAKE_ENABLED` (the flag-on unified model the round-2 fix is built
  around), accept does **not** move the order — "the order's status stays `CONFIRMED`/`READY` … it
  advances to `IN_DELIVERY` only at pickup" (proposal §3 A2:137-138). So an `'accepted'` assignment
  coexists with an order at **CONFIRMED / PREPARING / READY**. The round-2 `/abort` spec routes the
  `accepted` branch as *"order→`READY` (re-offerable)"* via `updateOrderStatus` (resolution R2-2,
  proposal §5:279-285).
- **Break:** `assertTransition` in `updateOrderStatus` (`orderStatusService.ts:76`) rejects it:
  - order `CONFIRMED` → `CONFIRMED:['PREPARING','IN_DELIVERY']` (v2 does **not** widen CONFIRMED;
    `order-machine.ts:19`) → `READY` not allowed → `IllegalTransitionError` → `{statusCode:400}` thrown.
  - order `READY` → `from===to` → `SameStatusError` → `{statusCode:400}` thrown.
  The throw propagates out of `/abort` → tx `ROLLBACK` → the assignment is **not** terminalized, the
  shift not freed → the courier **cannot abort** a pre-pickup accepted offer; retry loops on the same
  400. Only the `PREPARING`→`READY` sub-case happens to be legal.
- **Why flag-off masks it:** with the flag off, owner-direct `assign-courier` creates `'accepted'` **and**
  drives the order to `IN_DELIVERY` (`dashboard.ts:304,315`), so `accepted` ⟺ `IN_DELIVERY` and
  `IN_DELIVERY→READY` (widened) is legal. The bug lives **only** on the flag-on path — i.e. precisely the
  unification the round-2 fix exists to enable. MEDIUM (deferred flag, but the `/abort` code ships now and
  its `accepted` branch is specified incoherently with the live machine; no current guardrail exercises
  flag-on accept).
- **Violated invariant:** no-trap / status-guarded transitions are sound only because the design assumed
  `accepted ⟺ IN_DELIVERY`, which the A2 model it adopts explicitly negates.

### LOW (still-open — the central fold is not literally exhaustive)

#### R3-3 · B-CONSIST · One raw-`UPDATE` caller bypasses the "central fold in `updateOrderStatus`" — `customer/orders.ts:300-304` reaches `CANCELLED` from `IN_DELIVERY` outside the fold, relying on a **duplicated** terminalize block
- **Fact:** the round-2 headline claim is *"terminalize folded into `updateOrderStatus` … covers every
  present and future caller."* But `customer/orders.ts:300-304` self-cancel issues a **raw**
  `UPDATE orders SET status='CANCELLED'` (guarded only by `status==='IN_DELIVERY'` at `:286`) and does
  **not** call `updateOrderStatus`. It carries its **own** terminalize (`:309-318`, set
  `('assigned','accepted','picked_up')` — note it omits `'offered'`).
- **Why it is LOW, not a live strand:** the duplicate keeps the post-condition (zero active binding) true
  for `IN_DELIVERY` orders today, and the R2-3 no-strand guardrail is a **post-condition** test (passes
  via the duplicate). The architect's own audit table marks this path SAFE. Residual risk: (a) the
  centralization is **partial** — this path also skips `order_status_history`, the standard `order.status`
  WS delta, and the eta-synthesis savepoint that `updateOrderStatus` provides (the customer self-cancel
  emits only `ORDER_CANCEL_AFTER_DISPATCH`), so a customer self-cancel of an `IN_DELIVERY` order is
  audit/realtime-inconsistent with every other cancel; (b) drift — the two terminalize blocks now carry
  **different** status sets and must be kept in sync by hand. Pre-existing and acknowledged; flagged so the
  "every present and future caller" claim is not over-credited.
- **Violated invariant:** the *claim* of central exhaustiveness (the post-condition still holds via the
  duplicate).

### Net round-3 verdict
**converged: no — 1 open HIGH, 0 open CRITICAL** (plus 1 MED, 1 LOW).
The three round-1 CRITICALs and the round-2 HIGHs (R2-1/2/3) are **confirmed closed** at design level,
and the hypothesized null/owner-courier HOLD violation does **not** exist (both completion paths resolve
a real assigned `courier_id`). Centralization is sound *where it lands*. The two open regressions are
**properties of the two new artifacts the round-2 fix introduced**: the cross-tenant retention sweep
(R3-1, HIGH — silently anonymizes 0 rows under `delivery_trace`'s tenant-scoped FORCE policy, guardrail
measures schedule-existence not efficacy) and the `/abort` `accepted`-branch (R3-2, MED — forces an
illegal/same-status `READY` transition on a pre-pickup order under flag-on). R3-3 (LOW) notes the fold's
exhaustiveness claim is carried by a duplicate, not the central path, on the one customer raw-UPDATE
caller.

---

## RE-ATTACK round 4 (loop-exit gate — attacked ONLY the two round-3 fixes)

> BREAKER seat, convergence check. Re-grounded against live source @ HEAD. Attacked only R3-1
> (`anonymize_stale_delivery_trace` DEFINER sweep + outcome guardrail) and R3-2 (abort:
> terminalize-first-then-conditional). Verdict: **both round-3 fixes are substantively CLOSED; 0 new
> CRITICAL/HIGH.** Residuals are 2 MED + 1 LOW, all flag-gated / guardrail-precision / cosmetic.

### R3-1 — CLOSED (with a guardrail-precision MED + a stated owner-privilege assumption)

CONFIRMED-CLOSED:
- **Right columns, no over-null.** The fn NULLs exactly `gps_lat,gps_lng,name_snapshot,price_snapshot`
  and retains the non-PII facts (`total,delivered_at,courier_id,payment_outcome,route_distance_m`).
  Direction `delivered_at < now() - p_window` is correct — it can only touch rows *past* the window,
  never inside it. No over-null in the SQL.
- **search_path / privesc clean.** `SET search_path=public`, `REVOKE ALL … FROM PUBLIC`, grant-mirrored
  to the read_public_menu grantee only; `p_window` is a typed `interval` param (no injection); body uses
  unqualified `now()`/`delivery_trace` safe under the pinned path; no dynamic SQL. No RPC/PostgREST
  surface (Fly app, not Supabase-exposed) → no end-user invocation. Mechanism mirrors the proven
  `app_is_shadow_location`/`read_preview_menu` posture (`1790000000070:33-34,59-63`).

**Correction of a load-bearing claim (does NOT change the disposition, but the prose is wrong):**
`SECURITY DEFINER` ownership does **not**, by itself, bypass `FORCE ROW LEVEL SECURITY`. FORCE exists
precisely to subject the *table owner* to RLS. The DEFINER sweep reaches all-tenant rows **only because
its OWNER role (the migration `postgres`) carries `BYPASSRLS`/superuser** — confirmed by migration 070's
own crux comment ("*the operational role bypasses RLS today … becomes the boundary once the role is
NOBYPASSRLS*"). So the resolution's "*executes as the privileged owner → bypasses FORCE*" is imprecise,
and "*no reliance on … `BYPASSRLS`*" is half-true: it correctly decouples from the **operational** role's
future NOBYPASSRLS (a genuine improvement — the fn runs as the owner, not the caller), but it **does**
depend on the **function-owner** role being superuser/`BYPASSRLS`. This is an **unstated assumption**:
if migrations are ever run by a NOBYPASSRLS non-superuser role, the sweep silently anonymizes 0 rows
again — the exact R3-1 false-green. On the standard Supabase deploy (owner = privileged `postgres`) it
holds → CLOSED, assumption noted.

#### R4-1 · [MEDIUM] B-OPS/B-SEC · the outcome-based efficacy guardrail does not actually DISCRIMINATE unless the test runs the operational pool under a NOBYPASSRLS role
- **Break:** the guardrail invokes the sweep "*the way the worker does — operational pool, NO app.user_id
  set*." But by migration 070's own statement the operational pool role **has `BYPASSRLS` today**. Under
  `BYPASSRLS`, the **round-2 raw context-free `UPDATE`** (the regression this guardrail must catch) ALSO
  anonymizes every cross-tenant row → the test is **GREEN against both** the broken raw-UPDATE and the
  DEFINER fix. The guardrail therefore proves "rows got anonymized," not "the DEFINER routing is what
  reached them," and would **not** go RED if someone reverts to the raw context-free `UPDATE`. Its
  discriminating power requires an **unstated precondition**: the operational caller must be NOBYPASSRLS
  (as `provision-rls.test.ts` deliberately arranges). As specified ("operational pool, no tenant set"),
  the red→green claim is not guaranteed.
- **Invariant:** a guardrail must be RED against the failure it names; efficacy-under-BYPASSRLS is
  non-discriminating. (Not a runtime data break — prod sweep works via the owner's privilege — hence MED,
  not HIGH.)

#### R4-2 · [LOW] B-OPS · `p_window` has no floor — a mis-set `DELIVERY_TRACE_GPS_RETENTION` (e.g. `'1 day'`) anonymizes GPS *inside* the 7-day dispute window, destroying evidence early
- Worker passes `env.DELIVERY_TRACE_GPS_RETENTION || '14 days'` straight into `p_window`; the fn enforces
  no `>= dispute-window` floor. A malformed value throws on `::interval` cast (safe), but a *valid-but-too-
  small* interval silently over-anonymizes within the dispute window. Operational-config risk, not a code
  defect — LOW.

### R3-2 — CLOSED (atomicity + concurrency + illegal-transition all genuinely fixed)

CONFIRMED-CLOSED:
- **Atomicity / "crash between terminalize and order-action":** the whole flow is one `BEGIN…COMMIT`. A
  crash after step-2 terminalize but before the step-3 order-action → full `ROLLBACK` → assignment NOT
  terminalized AND order untouched. There is **no window** where the binding is terminalized while the
  order still carries `courier_id`. Closed.
- **Concurrent abort ↔ owner-reassign on a flag-ON `accepted` order = single winner.** Both contend the
  **same** assignment row: abort `SELECT … FOR UPDATE WHERE status IN ('accepted','picked_up')` vs the C-3
  reassign `UPDATE … WHERE status IN ('offered','assigned','accepted')`. Under READ COMMITTED the second
  blocks on the row lock then re-evaluates its qual against the committed new status → abort-wins leaves
  the row `'cancelled'` (reassign re-eval → 0 rows → 409); reassign-wins leaves it `'offered_expired'`
  (abort FOR UPDATE re-eval → 0 rows → 404). Deterministic single winner. Closed.
- **Illegal/same-status `READY` (the R3-2 bug) is gone:** `updateOrderStatus` is now called **only** when
  `ord_status='IN_DELIVERY'` — the one state from which `→READY`/`→CANCELLED` is a legal widened edge — so
  it can never throw `IllegalTransition`/`SameStatus`. The flag-ON `accepted` (CONFIRMED/PREPARING/READY)
  case takes the no-transition `SET courier_id=NULL` branch. Closed.

#### R4-3 · [MEDIUM] B-CONSIST/B-FAIL · the flag-ON no-transition branch emits NO event and does NOT re-enqueue → owner/customer go stale and the order is not auto-re-offered (asymmetric with decline)
- **Break (directly answers the "does the no-transition branch emit the right signal" attack — it does
  not):** the decline/reject path **re-enqueues** the order to `courier_dispatch_queue`
  (`assignments.ts:193-197`) so the dispatch worker auto-re-offers, AND publishes. The round-3 abort
  flag-ON branch does **neither** — its only writes are assignment→`'cancelled'`, shift→`'available'`,
  `orders.courier_id=NULL`, with **no `updateOrderStatus` (so no order WS/status delta)** and **no
  `courier_dispatch_queue` INSERT / `COURIER_DISPATCH` send** (the *only* enqueue site in the codebase is
  the reject handler — grep-verified). Result under flag-ON: a courier aborts a pre-pickup `accepted`
  offer → the customer/owner receive **zero realtime signal** that the courier bailed (order status never
  changed), and the order is **not auto-re-offered** the way a decline is — it sits in its current status
  with no active binding until a manual owner re-offer that nothing prompts. "Re-offerable" is true only
  in the *eligibility* sense, not the *mechanism* sense.
- **Why MED not HIGH:** flag-gated (`COURIER_OFFER_HANDSHAKE_ENABLED` is OFF; flip is NEEDS-HUMAN), the
  data is **consistent and recoverable** (no dangling active binding, no trap), and the IN_DELIVERY
  branches DO emit via `updateOrderStatus`. The gap is signal/coherence + missing auto-redispatch on the
  one new branch, not a stuck-state. Invariant: state↔event coherence + symmetry with the decline recovery.

#### R4-4 · [LOW] B-CONSIST · abort's `IN_DELIVERY → READY` (legacy flag-OFF) branch leaves `orders.courier_id` stale — diverges from the C-2 revert which clears it
- `updateOrderStatus` does not touch `orders.courier_id` (grep-verified, none in `orderStatusService.ts`),
  and the abort `accepted-while-IN_DELIVERY→READY` branch (unlike the flag-ON branch and unlike the
  original C-2 revert `SET status='READY', courier_id=NULL`) does not clear it → a READY, re-offerable
  order keeps pointing at the departed courier. **Not a trap:** dispatch selection keys off
  `courier_assignments` active rows (`courier-dispatch.ts:57`), not `orders.courier_id`, and customer PII
  is gated on `courierActive` (no live active assignment after abort) — so the stale mirror is cosmetic
  drift, harmless until the next pickup overwrites it. LOW.

### Net round-4 verdict
**converged: YES — 0 open CRITICAL / 0 open HIGH.** R3-1 (DEFINER sweep + correct columns + pinned
search_path + no over-null) and R3-2 (one-tx atomicity, single-winner concurrency, conditional order-side
action that never forces an illegal transition) are both substantively closed. Carried, non-blocking:
**R4-1 (MED** — efficacy guardrail is non-discriminating unless the test exercises a NOBYPASSRLS
operational caller; plus the unstated function-owner-must-be-privileged assumption), **R4-3 (MED** —
flag-ON no-transition abort branch is signal-silent and not auto-re-offered, asymmetric with decline),
**R4-2 / R4-4 (LOW** — `p_window` floor; stale `orders.courier_id` on the legacy READY branch). None
re-open a red line; all are flag-gated, guardrail-precision, or cosmetic.
