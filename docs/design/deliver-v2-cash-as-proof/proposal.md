# Design Proposal — `deliver` v2 (Cash-as-Proof) + Courier Accept/Decline

> ARCHITECT seat, Triadic Council. Design-time only — NO production code in this change.
> Supersedes the Stage-20 correlation engine. Extends Stage-18 (assignment), Stage-19 (GPS),
> Stage-21 (cash HOLD). Pairs with ADR `docs/adr/ADR-deliver-v2-cash-as-proof.md`.

## 0. Grounding: exists-vs-spec (verified against live source)

Before designing, the single most load-bearing fact: **the dangerous Stage-20 parts the contract
says to "REMOVE" were never built.** Verified by grep across `apps/api/src` + `packages/db/migrations`:

| Artifact contract says REMOVE | Reality | Verb changes to |
|---|---|---|
| Correlation engine (signals → {autoConfirm \| friction}) | **Does not exist** in `apps/api/src` (only docs/specs match `correlat`/`autoConfirm`) | **NEVER BUILD** |
| Friction-UX in courier app | **Does not exist** in code | NEVER BUILD |
| `proof_photo` / `proof_photo_key` | **Does not exist** in migrations or code. (`delivery_photo_key`, `apps/api/src/routes/courier/assignments.ts:50,64`, is the *customer's* entry-anchor photo — UX-3, not proof) | NEVER BUILD |
| `delivery_flags` table | **Does not exist** — zero migrations, zero code references | NEVER CREATE |
| heartbeat-as-completion-signal | Not wired as completion logic | NEVER BUILD |
| auto-raising flags on completion | N/A — no flags table | NEVER BUILD |

What **does** exist and is already aligned with the Cash-as-Proof philosophy:

| Artifact | Where | Status |
|---|---|---|
| `payment_outcome` enum (`pending,paid_full,paid_partial,refused_payment,refused_goods,customer_cancelled_on_door`) | `packages/db/migrations/1780310044710_extensions-and-enums.ts:16` | EXISTS. DDL name is **`paid_full`** (resolves §G-2 — "paid_cash" is a docs typo). Column on orders defaults `'pending'` (`1780310074262_orders.ts:35`) but is **never written** by the delivered handler today. |
| `delivery_trace` (immutable, 1/order, RLS ENABLE+FORCE, `ON CONFLICT DO NOTHING`) | `packages/db/migrations/1790000000027_delivery-trace.ts` + cols `route_distance_m,expected_delivery_min` added in `1790000000066_sensor-bus-now.ts:43-47` | EXISTS. Written in the DELIVERED txn at `assignments.ts:345-349`. |
| `courier_cash_ledger` `'hold'` row **atomic** with DELIVERED | `1790000000028_courier-cash-ledger.ts` + write at `assignments.ts:353-359` | EXISTS (Stage-21 shipped). Same txn as the status-guarded UPDATE. |
| `courier_assignments.status` text CHECK `('assigned','accepted','picked_up','delivered','cancelled','rejected')` | `1780421100041_courier-assignments.ts:11` | EXISTS. No `'offered'` value yet. |
| Accept / Reject / Picked-up / Delivered / Cancel routes | `apps/api/src/routes/courier/assignments.ts:122-453` | EXIST, all status-guarded (`WHERE … status=$expected`, rowcount checked). |
| `order_status` enum — **uppercase**, no offered/accepted | `1780310044710:14` + `packages/domain/src/order-machine.ts:3-29` | EXISTS. Transition map: `PENDING→{CONFIRMED,REJECTED,CANCELLED}`, `CONFIRMED→{PREPARING,IN_DELIVERY}`, … `IN_DELIVERY→DELIVERED`. |
| GPS stream / geofence / `order_sensor_events` (passive, exactly-once per `(order,event_type)`) | `1790000000066_sensor-bus-now.ts:60-93` | EXISTS (Stage-19/sensor-bus). |
| PENDING-timeout = **timestamp column + cron sweep** (NOT a pg-boss delayed job) | `apps/api/src/workers/order-timeout-sweep.ts:63-85` (`UPDATE … status='CANCELLED' WHERE status='PENDING' AND timeout_at<now()`) | EXISTS. **The pg-boss `order.timeout` queue is UNREGISTERED** and silently no-ops — see the known-debt note at `assignments.ts:372-378`. |
| `customer_signals` (advisory, **"NEVER used for auto-block"**, owner ack/dismiss only) | `1780421100057_anti-fake-signals.ts:6-41,104` | EXISTS — already a passive record, consistent with the philosophy. Keep, do not wire to any gate. |

**Consequence for the whole proposal:** Cash-as-Proof is achieved mostly by *additive recording*
(two columns on `delivery_trace`, one write of `payment_outcome`, one new assignment sub-state) plus
a *discipline rule* — **never build the verdict engine.** The "delete" surface is near-zero.

A second load-bearing fact, also from grounding: there are **two divergent assignment paths today**,
and they disagree. The dispatch/queue path uses `courier_assignments.status='assigned'` then a real
courier accept handshake (`assignments.ts:122-217`). The owner-direct path (`owner/dashboard.ts:302-320`)
INSERTs `status='accepted'` **and immediately drives the order to `IN_DELIVERY`** — **no accept handshake,
no offer**. §A formalizes one path and removes this divergence.

---

## 1. Problem + non-goals

**Problem.** Delivery confirmation must be a *timestamp recorded by a human tap*, not an automated
verdict. The Stage-20 plan would have correlated passive signals into an `autoConfirm|friction|block`
decision — the riskiest, most failure-prone surface (false-positive friction punishes honest couriers;
false-negative defeats the point; the correlation thresholds are unowned config). The contract replaces
technical verification with **economic embedding** (courier = restaurant staff; a "delivered + cash
collected" tap creates **collected-cash accountability** — the courier already physically holds that
cash and is reconciled for it at shift close like any cashier's till, so lying "delivered+collected"
costs money. This is **till-accountability, NOT posted personal surety** — the courier never fronts
their own capital), **reputation**
(local repeat actors), and **burden-of-proof-on-the-accuser**. We need to (a) add a real courier
accept/decline handshake without ever trapping the customer, and (b) make completion a single tap that
records an outcome + cash, with passive crumbs and zero gate.

**Non-goals (explicit):**
- **No verdict engine** — no signal→decision logic of any kind. Crumbs are recorded, never evaluated.
- **No card flow** (§D). `payment_method` enum is `('cash')` only (`1780310044710:15`). Card is scaffold.
- **No auto-dispatch.** For a 1–5 courier shop, the owner reassigns manually (contract §A).
- **No platform adjudication of disputes** (§C). Platform keeps the immutable record; owner↔customer
  settle off-platform.
- **No new pg-boss queue** (infra cannot `create_queue` from app roles — `assignments.ts:372-378`).
- Not changing the customer-facing `order_status` enum values (uppercase MVP enum stays canonical truth).

---

## 2. Back-of-envelope

**Domain:** Albania, cash-on-delivery, single-restaurant tenants with **1–5 couriers each**.

- **Scale target (MVP → 10×):** 50–200 active locations. Per-location dinner-peak ≈ 0.2 orders/min
  (a busy single shop ≈ 1/min, most idle). Not-all-peaking-together ⇒ system peak
  ≈ `200 × 0.2 = 40 orders/min ≈ 0.67 orders/s`. At **10× growth: 400/min ≈ 6.7 orders/s.** Trivial for one Postgres.
- **Delivery-completion txn cost:** the existing DELIVERED handler (`assignments.ts:288-361`) is
  1 connection, ~7 statements (status-guarded UPDATE assignment, UPDATE shift, `updateOrderStatus`,
  `delivery_trace` insert, `courier_cash_ledger` insert, `order_status_history`, eta-synthesis) — < 50 ms.
  At 6.7/s peak that is **≤ ~7 connections momentarily held**, each < 50 ms. v2 adds **one column write**
  (`payment_outcome`) and **two `delivery_trace` columns** — sub-millisecond, no new statement.
- **Offer + offered-timeout:** owner taps assign → 1 INSERT (`courier_assignments`, `status='offered'`,
  `offered_expires_at = now()+5min`). Courier accept/decline → 1 status-guarded UPDATE. The **timeout** is a
  **cron sweep** (reuse `order-timeout-sweep`), every 30–60 s, one indexed scan:
  `WHERE status='offered' AND offered_expires_at < now()`. At 200 locations the live offer set is tens of
  rows — partial-index scan, < 5 ms, runs in the worker pool.
- **Connection budget (the real ceiling — Supabase pooler):** API pool **10** + worker pool **5** +
  analytics **3** + migrations **2** = **~20 concurrent**, against a typical 60–100 ceiling. v2 adds **zero**
  new pools and **zero** new long-lived connections. The offered-sweep shares the worker pool with the
  existing PENDING-sweep.
- **Storage:** `delivery_trace` = 1 row/order, ~200 B. Even at the 10× extreme (≈ 576k orders/day) that is
  ~115 MB/day *if every shop ran flat-out 24h* — realistic MVP is a few thousand orders/day ⇒ **single-digit
  MB/day.** The 7-day customer history (§C) reads *existing* `orders`/`order_items` rows — **zero new storage.**

**Conclusion:** capacity is a non-issue for years; the design must optimize for **correctness,
no-trap-states, and tenant isolation**, not throughput. Boring-and-proven wins.

---

## 3. Options (≥2, named concept + tradeoffs)

### Central fork — completion posture

**Option 1 — "Verdict Engine" (Stage-20 correlation/friction/proof-photo/auto-flag).**
Concept: *signals-as-control* — passive signals (GPS proximity, velocity, heartbeats, proof photo) feed a
correlator that emits `autoConfirm | friction | block`.
- (+) Looks rigorous; auto-confirms the easy cases.
- (−) Unowned thresholds; false-positive friction punishes honest staff couriers (red-line: *friction-not-verdict*);
  false-negative defeats it; proof-photo = PII + storage + a doorstep-friction step; the correlator is a
  trap-state generator. **Already the riskiest Phase-3 part — and never built.** Rejecting it costs nothing.

**Option 2 — "Cash-as-Proof / Passive Crumbs" (CHOSEN).**
Concept: *economic embedding + record-don't-judge*. Completion = one courier tap carrying
`payment_outcome` + cash. The tap IS the timestamp AND the completion — no gate, no correlation. The
strongest costly-to-fake signal is **collected-cash accountability (till-accountability)**: recording
"delivered + paid_full" creates a real `courier_cash_ledger` `'hold'` for cash the courier **already
holds**, reconciled at shift close — not posted surety. Signals (GPS proximity, amount, order snapshot) are written
to the append-only `delivery_trace` as a **passive record**, never evaluated.
- (+) Works day-1, needs no reputation history; zero doorstep friction; no trap-states; no PII photo;
  reuses shipped Stage-21 HOLD machinery; matches the actor model (staff/repeat/local).
- (−) A one-time customer who pays without inspecting vs a rare bad-actor owner on a content dispute is an
  **accepted residual** (§C, niche-scoped).

### Sub-decision A — where the offered/accepted sub-states live (§G-1)

**Option A1 — "Aggregate-enum extension."** Add `OFFERED`/`ACCEPTED` to the `order_status` enum + domain
map. Concept: *single state machine*.
- (−) Pollutes the **customer-facing** enum with courier-internal binding state; a Postgres enum value add is
  forward-only and irreversible; the customer status page would have to map/hide them; risks the "customer
  punished for courier inaction" trap because the order's own status now moves on offer.

**Option A2 — "Assignment-row sub-state" (CHOSEN).** Keep offer/accept on `courier_assignments.status`
(add `'offered'` to the CHECK). The **order's** status stays put (`CONFIRMED`/`READY`) through the entire
offer→accept→decline/timeout dance; it advances to `IN_DELIVERY` only at **pickup**. Concept:
*model the handshake on the binding entity, keep the aggregate-root enum stable*.
- (+) 🔴 **Structurally guarantees the contract's red-line**: offered-timeout/decline can never touch customer
  state, because customer state lives on a different row. (+) No enum churn. (+) `'assigned'` today already
  means "offered" in the queue path — we make it explicit and unify both paths.
- (−) Two status columns to reason about (order vs assignment) — but that split already exists and is the
  correct DDD boundary.

### Sub-decision B — offered-timeout mechanism

**Option B1 — "pg-boss delayed job."** `boss.send('offer.timeout', {orderId}, {startAfter: 300})`,
claim-check payload. The contract's literal suggestion.
- (−) **Infeasible on this infra:** app roles cannot `create_queue` (`assignments.ts:372-378`); the existing
  `order.timeout` queue is unregistered and silently no-ops. Building on a queue that doesn't fire = a silent
  trap-state (offers never time out → couriers/owners stuck). Hard reject.

**Option B2 — "Timestamp + cron sweep" (CHOSEN).** Add `offered_expires_at timestamptz` to
`courier_assignments`; reuse the proven `order-timeout-sweep` worker pattern (`order-timeout-sweep.ts:63-85`):
`UPDATE courier_assignments SET status='offered_expired' … WHERE status='offered' AND offered_expires_at<now()`,
then return the order to assignable. Concept: *durable deadline as data + idempotent sweep* — the exact pattern
already shipped and proven for PENDING.
- (+) Reuses a working, owned worker; survives restarts (deadline is in the row, not a timer); idempotent;
  the `WHERE status='offered'` guard IS the transition authority. (−) Up to one sweep-interval (≤60 s) of
  latency before an offer is reclaimed — irrelevant for a 5-min offer window.

---

## 4. Decision + rationale (ADR-format → also in docs/adr/)

**Adopt Option 2 (Cash-as-Proof / Passive Crumbs) + A2 (assignment-row sub-state) + B2 (timestamp+sweep).**
Reject the Verdict Engine in full — and because it was never built, "reject" = *a standing discipline rule
to never build it*, encoded as the ADR + a guardrail (§9). The completion path is unchanged in shape from
the shipped Stage-21 handler; v2 is the **minimum additive delta**: record `payment_outcome`, two trace
columns, the `'offered'`/accept handshake, and the unification of the two assignment paths. Full ADR in
`docs/adr/ADR-deliver-v2-cash-as-proof.md`.

**§G resolutions (grounded):**
1. **order_status reconciliation** → **A2**: do **not** add offered/accepted to `order_status`. Add `'offered'`
   (and `'offered_expired'` for the swept terminal) to `courier_assignments.status` CHECK. Order enum
   untouched. Reconcile the two assignment paths so the owner-direct path (`dashboard.ts:302`) also creates an
   `'offered'` row and **stops** force-driving the order to `IN_DELIVERY` before pickup.
2. **payment_outcome name** → unify on **`paid_full`** (DDL is authoritative; `1780310044710:16`). Treat any
   "paid_cash" in prose as a typo. No migration needed for the enum.
3. **delivery_flags** → **never create it.** It does not exist; its only intended consumer (Stage-21
   reconciliation) already runs off `courier_cash_ledger` `'hold'` rows. Zero value, deleted from scope.

---

## 5. Data / migrations (forward-only, atomic, RLS FORCE, integer)

One forward-only migration (`<ts>_deliver-v2-cash-as-proof.ts`), additive + idempotent:

```
-- 1. Offer sub-state on the binding entity (A2). Forward-only CHECK swap.
--    (the inline CHECK is auto-named `courier_assignments_status_check`; verify via pg_constraint
--    in a DO-block for idempotency before DROP.)
ALTER TABLE courier_assignments DROP CONSTRAINT courier_assignments_status_check;
ALTER TABLE courier_assignments ADD  CONSTRAINT courier_assignments_status_chk
  CHECK (status IN ('offered','assigned','accepted','picked_up','delivered',
                    'cancelled','rejected','offered_expired'));
ALTER TABLE courier_assignments
  ADD COLUMN IF NOT EXISTS offered_at         timestamptz,
  ADD COLUMN IF NOT EXISTS offered_expires_at timestamptz;

-- 🔴 C-1 FIX (the no-trap red-line): the live constraint `courier_assignments_order_uniq`
-- (1780421100041:23) is a FULL unique on order_id → one assignment row per order FOREVER →
-- a rejected/cancelled row permanently blocks any re-offer (today: 0% redispatch success for
-- any order that ever had a row; courier-dispatch.ts:84 INSERT collides → job retried forever).
-- Replace it with a PARTIAL unique on ACTIVE states only, so terminal rows
-- (rejected/cancelled/offered_expired/delivered) never block a re-offer, while still guaranteeing
-- at most ONE active binding per order at the DB level.
DROP INDEX courier_assignments_order_uniq;
CREATE UNIQUE INDEX courier_assignments_order_active_uniq
  ON courier_assignments (order_id)
  WHERE status IN ('offered','assigned','accepted','picked_up');

-- Partial index powering the sweep.
CREATE INDEX IF NOT EXISTS courier_assignments_offered_due_idx
  ON courier_assignments (offered_expires_at) WHERE status = 'offered';
-- Extend the existing single-active-assignment guard (1790000000066:127-131) to include 'offered'
DROP INDEX IF EXISTS courier_one_active_assignment;
CREATE UNIQUE INDEX courier_one_active_assignment ON courier_assignments (courier_id)
  WHERE status IN ('offered','assigned','accepted','picked_up');

-- 🔴 R-1 FIX (M-1): FORCE RLS + align policy to app_member_location_ids() (canon). FORCE closes the
-- owner/BYPASSRLS bypass ONLY — the cross-courier-same-location vector stays closed by the inline
-- `AND courier_id=$me` predicate in app code (see §8), NOT by RLS.
ALTER TABLE courier_assignments FORCE ROW LEVEL SECURITY;
DROP POLICY isolate_courier_assignments ON courier_assignments;
CREATE POLICY isolate_courier_assignments ON courier_assignments
  USING (location_id = ANY (SELECT app_member_location_ids()));

-- 2. Passive crumbs onto the immutable trace (append-only; already RLS FORCE).
ALTER TABLE delivery_trace
  ADD COLUMN IF NOT EXISTS payment_outcome   payment_outcome,   -- reuse existing enum
  ADD COLUMN IF NOT EXISTS cash_amount       integer CHECK (cash_amount IS NULL OR cash_amount >= 0),
  ADD COLUMN IF NOT EXISTS gps_lat           double precision,  -- passive proximity, NOT thresholded
  ADD COLUMN IF NOT EXISTS gps_lng           double precision,
  ADD COLUMN IF NOT EXISTS name_snapshot     jsonb,             -- immutable order snapshot link
  ADD COLUMN IF NOT EXISTS price_snapshot    integer CHECK (price_snapshot IS NULL OR price_snapshot >= 0);

-- 🔴 R3-1 FIX (anonymize-not-delete red-line MUST actually fire): delivery_trace is tenant-scoped FORCE
-- (1790000000027:22-25, `USING (location_id IN (SELECT app_member_location_ids()))` — NOT access_requests'
-- `USING(true)`). A context-free operational-pool UPDATE sees 0 rows → GPS retained forever. The sweep MUST
-- run through a SECURITY DEFINER fn. 🔴 PRECISE MECHANISM (R4 correction — do NOT mis-state): SECURITY DEFINER
-- alone does NOT bypass FORCE — FORCE exists precisely to subject the table OWNER to RLS. The sweep reaches
-- all-tenant rows ONLY because the function's OWNER role (the migration `postgres`/admin) carries
-- BYPASSRLS/superuser. 🔴 STATED ASSUMPTION (function-owner-is-privileged): migrations run as a privileged
-- (BYPASSRLS/superuser) owner — the standard Supabase/Fly deploy. If migrations are ever run by a NOBYPASSRLS
-- non-superuser role, this fn silently anonymizes 0 rows again (the R3-1 false-green). It depends on the
-- OWNER's privilege, NOT the OPERATIONAL pool role's BYPASSRLS (an uncertain env artifact). Same canon as
-- read_public_menu / app_is_shadow_location (1790000000070:33-34). No per-tenant loop. Pinned search_path
-- (closes the DEFINER search_path class). Narrowly scoped: NULLs only PII crumbs, returns a count, no row exfil.
-- 🔴 R4-2 dispute-window floor: clamp p_window to >= the 7-day dispute window so a mis-set
-- DELIVERY_TRACE_GPS_RETENTION (e.g. '1 day') can NEVER anonymize evidence INSIDE the dispute window.
CREATE OR REPLACE FUNCTION anonymize_stale_delivery_trace(p_window interval)
RETURNS integer LANGUAGE plpgsql VOLATILE SECURITY DEFINER SET search_path = public AS $$
DECLARE
  v_count  integer;
  v_window interval := GREATEST(p_window, interval '7 days');  -- R4-2 floor = dispute window
BEGIN
  WITH anon AS (
    UPDATE delivery_trace
       SET gps_lat=NULL, gps_lng=NULL, name_snapshot=NULL, price_snapshot=NULL
     WHERE delivered_at < now() - v_window
       AND (gps_lat IS NOT NULL OR gps_lng IS NOT NULL
            OR name_snapshot IS NOT NULL OR price_snapshot IS NOT NULL)
    RETURNING 1)
  SELECT count(*)::int INTO v_count FROM anon;
  RETURN v_count;
END;
$$;
REVOKE ALL ON FUNCTION anonymize_stale_delivery_trace(interval) FROM PUBLIC;
-- grant-mirror EXECUTE to whatever role executes read_public_menu_all_locales (1790000000070:114-129 DO-block).
```

- **RLS:** `delivery_trace` and `courier_cash_ledger` already `ENABLE+FORCE` with tenant policies
  (`1790000000027:22-25`, `1790000000028:24-27`). 🔴 `courier_assignments` today is `ENABLE` **without FORCE**
  and uses `current_setting('app.current_tenant')` (`1780421100041:26-29`) — **flagged risk R-1 below**; this
  migration adds `… FORCE ROW LEVEL SECURITY;` to bring it to the canon (no other table relaxes the standard).
- **Money:** every amount column is `integer` (minor units) with `CHECK (>= 0)` — matches `cash_amount`
  (`1780421100041:19`) and the ledger (`1790000000028:17`).
- **No enum value adds to `order_status`** → avoids the irreversible/forward-only Postgres enum pain entirely.
- **No backfill required** — all new columns are nullable additive observability; historical traces stay valid.
- Grants for the new `courier_assignments` columns are implicit (column-level grants follow the table grant).
- **Domain map change (NOT a migration — `packages/domain/src/order-machine.ts:23`):** widen
  `IN_DELIVERY: ['DELIVERED']` → **`IN_DELIVERY: ['DELIVERED', 'CANCELLED', 'READY']`**. (a) `CANCELLED`
  is the no-cash-tail terminal (H-1) — **reuses an existing terminal, no `order_status` enum value add**,
  preserving A2. (b) `READY` is the revert target for courier-cancel / owner-reassign of an order the
  owner-direct path force-drove to `IN_DELIVERY` (C-2) — lets the revert route through `updateOrderStatus`
  (audit + WS events) instead of the v1 raw `UPDATE … status='READY'` (`dashboard.ts:264-268`). Both edges
  are downgrades to assignable/terminal states the machine already owns; no new state value is introduced.
- **C-2 mirror-revert (courier `cancel`, same tx, status-guarded):** after freeing the assignment + shift,
  the order revert routes through `updateOrderStatus(…,'READY')` (audit + WS) — no-op for queue-path orders
  still `CONFIRMED`/`READY`. Closes the owner-direct `IN_DELIVERY`-forever trap.
- 🔴 **R2-3 shared invariant (central, not per-caller):** the assignment-terminalize is folded into
  **`updateOrderStatus` itself** (`lib/orderStatusService.ts`): on any `IN_DELIVERY→{CANCELLED,READY}`
  transition, in the **same tx** after the guarded order UPDATE, terminalize the active binding + free its
  shift —
  ```
  WITH freed AS (
    UPDATE courier_assignments SET status='cancelled', cancelled_at=now(),
           cancellation_reason = COALESCE($comment, 'order_'||lower($newStatus))
     WHERE order_id=$orderId AND status IN ('offered','assigned','accepted','picked_up')
    RETURNING shift_id)
  UPDATE courier_shifts SET status='available' WHERE id IN (SELECT shift_id FROM freed);
  ```
  Idempotent (already-terminal rows are a no-op; `DELIVERED ∉ {CANCELLED,READY}` so a `'delivered'` row is
  never reverted). **Invariant: no order leaves `IN_DELIVERY` without its active assignment terminalized in
  the same tx** — covers the audited callers `owner/signals.ts:234` (no-show), `orders.ts:779` (owner PATCH),
  the dashboard reassign revert (R2-6), and the courier `/cancel`+`/abort`, at every present and future site.
- 🔴 **R2-1 shared completion function:** both `delivered` paths (courier `assignments.ts` + owner-proxy
  `dashboard.ts:408`) call **one** `lib/deliveryCompletion.ts::completeDelivery(client, args, {messageBus})`
  (takes a client; runs in the caller's tx) — the single place that writes the `'hold'`, `payment_outcome`,
  and `delivery_trace` crumb. This structurally guarantees the cash-as-proof primitive on **both** paths
  (the owner-proxy path previously wrote none — `dashboard.ts:444-462`, verified).
- **R2-2 en-route abort (NEW courier action, NO time gate):** the 5-min `CANCEL_AFTER_DISPATCH_WINDOW_MS`
  gate (`assignments.ts:423-426`) is for **offer/accept regret**, not an en-route failure. Add `/assignments/
  :id/abort` — guard `WHERE id=$1 AND courier_id=$me AND status IN ('accepted','picked_up')` (rowcount-0 →
  404). **R3-2 — the order-side action is CONDITIONAL on the order's actual status, never a forced
  transition** (under flag-ON an `'accepted'` order is still `CONFIRMED/PREPARING/READY`, never
  `IN_DELIVERY` — forcing `READY` would throw `IllegalTransition`/`SameStatus` (`order-machine.ts:20,35-42`)
  → rollback → the assignment is never freed). Sequence: **(1)** terminalize the binding UNCONDITIONALLY
  (`status='cancelled'`, `reason='courier_aborted_en_route'`) + free shift — so abort **always** frees the
  assignment, independent of any order transition; **(2)** order-side, guarded on the locked order status —
  `ord_status='IN_DELIVERY' ∧ picked_up` → `updateOrderStatus(CANCELLED)` (food is out → honest terminal);
  `ord_status='IN_DELIVERY' ∧ accepted` (legacy flag-OFF force) → `updateOrderStatus(READY)` (legal widened
  edge, food at venue) **plus `UPDATE orders SET courier_id=NULL` (R4-4 — `updateOrderStatus` does not touch
  the mirror; clear it so a re-offerable READY order does not point at the departed courier)**;
  **`ord_status ∈ {CONFIRMED,PREPARING,READY}`** (flag-ON accept, order never advanced) → **NO
  `updateOrderStatus` call** (the order was never moved → forcing a transition would throw) — just
  `UPDATE orders SET courier_id=NULL`. `updateOrderStatus` is invoked **only** from `IN_DELIVERY` (the one
  state with a legal widened exit), so abort can **never** throw on a no-op transition. No time gate → the
  >5-min stuck-`IN_DELIVERY` case is recoverable, and the flag-ON pre-pickup abort no longer 400s.
- 🔴 **R4-3 — the flag-ON no-transition branch must converge with the decline path (broadcast + auto-re-offer),
  not go silent.** A decline re-enqueues the order to `courier_dispatch_queue` (the only enqueue site,
  `assignments.ts:193-197`) so the dispatch worker auto-re-offers, **and** publishes a status delta. The
  abort no-transition branch (above) writes only `courier_id=NULL` and — because it does **not** call
  `updateOrderStatus` — emits no WS event and does **not** re-enqueue → owner/customer go stale and the order
  is not auto-re-offered (asymmetric with decline). Fix: that branch must, in the same tx, **(a)** publish a
  binding-change broadcast over `MessageBus` (id-only payload — the `ORDER_STATUS` delta for the order's
  unchanged status so owner/customer realtime reconverges and the "courier dropped" change is visible), and
  **(b)** `INSERT INTO courier_dispatch_queue (order_id, …)` exactly as the decline path does, so the
  dispatch worker re-offers it through the **same** re-offer mechanism. After this, abort and decline
  converge: terminalized binding + freed shift + broadcast + back in the assignable pool. ("Re-offerable" now
  holds in the *mechanism* sense, not merely the *eligibility* sense.)

---

## 6. Consistency + idempotency

- 🔴 **Single completion primitive (R2-1):** both the courier `delivered` handler and the owner-proxy
  `/deliver` handler call **`lib/deliveryCompletion.ts::completeDelivery`** — the only function that writes
  the assignment terminal + shift + `updateOrderStatus` + `delivery_trace` crumb + ledger `'hold'`. Neither
  route has an inline completion body, so the HOLD/`payment_outcome`/trace are **structurally guaranteed on
  every `delivered`** (the owner-proxy path wrote none today — `dashboard.ts:444-462`). The owner-proxy body
  gains the same first-class `payment_outcome` + `.int().nonnegative()` `cash_amount` as the courier body.
- **Revert broadcast (R2-5):** an exit that reverts the order to `READY` (`/cancel`, `/abort`-from-`accepted`,
  owner-reassign) must **not** hand-publish `ORDER_CANCELLED`; `updateOrderStatus` already emits the correct
  `READY` `ORDER_STATUS` event. Only an exit that actually terminalizes the order to `CANCELLED` (no-cash
  tail, picked-up `/abort`) emits `ORDER_CANCELLED`. The unconditional publish at `assignments.ts:440-444` is
  removed. One event, matching the real resulting status.
- **Completion atomicity (unchanged, KEPT):** the `'hold'` ledger row and the order→`DELIVERED` transition
  already commit in **one txn** (`assignments.ts:329-361`), HOLD-atomic-with-delivered (Stage-21). v2 adds the
  `payment_outcome`/trace-column writes **inside the same txn** — no batching. The `delivery_trace` insert is
  `ON CONFLICT (order_id) DO NOTHING` (idempotent; first observed record wins). The ledger insert is
  `ON CONFLICT (order_id,type) DO NOTHING` (`assignments.ts:354-358`).
- **payment_outcome write (H-3 FIX):** the delivered body schema gains a first-class field —
  `payment_outcome: z.enum(['paid_full','refused_goods','refused_payment','customer_cancelled_on_door'])`
  and `cash_amount: z.number().int().nonnegative().optional()` (M-2 FIX) — so the distinguishing crumb is
  **collectable**, not collapsed to one boolean. In the DELIVERED txn the handler **persists** it to
  **both** `orders.payment_outcome` and `delivery_trace.payment_outcome`. **Server-authoritative coherence:**
  `cash_collected ⟺ payment_outcome==='paid_full' ∧ cash_amount===total`; any incoherent body, float/negative
  `cash_amount`, or **`paid_partial`/`pending`** → **422** (`CASH_AMOUNT_MISMATCH` / `PARTIAL_NOT_SUPPORTED`)
  **before** any mutation. The server owns the mapping; no recomputed client value is trusted.
- **`paid_partial` is FORBIDDEN as a delivered outcome (H-2 / counsel-C3 FIX).** The ledger is full-or-nothing;
  a partial collection has no honest `'hold'` row → forbidding it removes the one silent-courier-debt trap. A
  customer short on cash is handled as a **`refused_payment` no-cash tail** (order→`CANCELLED`, food returns,
  dispute off-platform), never "delivered but short." No courier ever holds unrecorded cash.
- 🔴 **Carried no-partial-handover rule (R2-4 / counsel-Q1 — what makes forbidding honest, not relocating):**
  forbidding the *representation* does not abolish the *event* unless the doorstep protocol prevents it. The
  carried courier rule (ADR + courier-UX + training): **"No partial handover — full cash in hand before the
  food changes hands; short on cash → no goods, tap `refused_payment`."** The completion UI offers only
  `paid_full` / the no-cash tails — **there is no partial-amount affordance** — so partial collection is
  *operationally prevented*, not merely enum-rejected. Without this rule the silent debt would simply move
  from the system to the doorstep (counsel Q1); with it, `refused_payment → CANCELLED, food returns` is a
  *true* record.
- **No-cash tail terminal (H-1 FIX):** `refused_goods`/`refused_payment`/`customer_cancelled_on_door` →
  assignment `'cancelled'` (`cancellation_reason=payment_outcome`), order **`CANCELLED`** (new edge
  `IN_DELIVERY→CANCELLED`), **no** ledger `'hold'`, trace crumb written, shift freed. The customer sees
  **Cancelled** — never "Delivered" for refused food. `paid_full` → assignment `'delivered'`, order
  `DELIVERED`, ledger `'hold'`. Both terminal, courier free → no trap.
- **Status-guarded transitions (KEPT, red-line):** every assignment transition keeps
  `WHERE id=$1 AND courier_id=$2 AND status=$expected` + rowcount-0 → 404/409 (`assignments.ts:173-181,234-242,
  292-304`). The new offer transitions follow the identical pattern:
  - accept: `WHERE status='offered' AND courier_id=$me` → `'accepted'`.
  - decline: `WHERE status='offered' AND courier_id=$me` → `'offered_expired'`, order untouched.
  - sweep: `WHERE status='offered' AND offered_expires_at<now()` → `'offered_expired'`.
- **Re-offer / reassign = a status-guarded *terminalize-then-insert*, NOT an unguarded fresh INSERT
  (C-3 FIX).** The v1 owner-reassign was a fresh INSERT whose only check was a `busyCheck` on the *new*
  courier's id (`dashboard.ts:246-251`) — two writers contending the full `order_id` unique → 500, outcome
  decided by commit order. v2 makes reassign a single tx with a rowcount authority:
  ```
  -- WINNER guard: terminalize the current ACTIVE row (rowcount=1 wins; rowcount=0 → 409 RACE_LOST, no-op)
  UPDATE courier_assignments SET status='offered_expired', cancelled_at=now(), cancellation_reason='reassigned'
   WHERE order_id=$order AND status IN ('offered','assigned','accepted')   -- not picked_up
  RETURNING id, shift_id;
  -- only the winner: revert order mirror if IN_DELIVERY (§C-2), then INSERT the new 'offered' row
  ```
  Because the prior row is moved **terminal first**, `courier_assignments_order_active_uniq` (C-1 FIX) is
  free for the new INSERT — no constraint race. A concurrent courier-decline targets the **same row**: the
  first guarded UPDATE wins (rowcount=1), the loser sees rowcount=0 → 409/idempotent.
- **The race the contract names (courier-declines ↔ owner-reassigns):** all writers contend the **same
  active row** via the guarded UPDATE above; rowcount is the single authority (not commit ordering). The
  `courier_one_active_assignment` unique (extended to include `'offered'`) makes a double-offer to the same
  courier impossible. **Deterministic single winner.**
- **Every transition writes `order_status_history`** via `updateOrderStatus` (`orderStatusService.ts:129-140`,
  SAVEPOINT-guarded so an audit-insert failure can't roll back canonical state).
- **N-safe:** all fan-out is via `MessageBus.publish` (`orderStatusService.ts:164-186`, `assignments.ts:257,
  364`) — no in-process state, safe across N API instances.

---

## 7. Failures + degradation (every external touch: timeout + fallback, zero cascade)

| Touch / failure | Behavior | Why no cascade |
|---|---|---|
| **Offer never answered** (courier offline) | cron sweep flips `'offered'→'offered_expired'` after `offered_expires_at`; order returns assignable. 🔴 **Customer order untouched.** | Deadline is data; sweep is idempotent; no live timer to lose. |
| **Sweep worker down** | Offers sit `'offered'` (stale) but harmless; owner can still manually reassign (the manual path is the primary, sweep is a safety net). On worker recovery the overdue set is reclaimed in one scan. | Degraded ≠ down; manual reassignment is always available. |
| **MessageBus.publish fails** post-commit | The order/assignment state is already committed (DB is authoritative); the WS event is best-effort. Customer page reconverges on next poll/refetch. | Realtime is advisory; canonical truth is `orders.status` (`orderStatusService.ts:138`). |
| **delivery_trace / sensor / eta-synth insert fails** | SAVEPOINT-wrapped (`orderStatusService.ts:146-152`) — never rolls back the applied transition (observe-don't-control). Crumb is lost, completion stands. | Crumbs are passive; losing one cannot block a courier. |
| **GPS coord missing at delivery** | `gps_lat/lng` written NULL; `route_distance_m`/`expected_delivery_min` already null-safe (`assignments.ts:312-322`). | Crumb is never thresholded; null is a valid record. |
| **No-cash tail** (`refused_goods`/`refused_payment`/`customer_cancelled_on_door`) | courier taps outcome → assignment `'cancelled'`, order → **`CANCELLED`** (new `IN_DELIVERY→CANCELLED` edge), no ledger `'hold'`, trace crumb (`payment_outcome`+gps) written, shift freed. 🔴 **Customer sees Cancelled, never "Delivered"; courier never blocked.** | Terminal state, frictionless completion; the distinguishing reason is recorded on `payment_outcome`. |
| **Courier can't complete en route** (unreachable/accident/vehicle), any elapsed time (R2-2) | `/abort` (NO 5-min gate): assignment `'cancelled'` (`courier_aborted_en_route`), shift freed + `courier_id=NULL` (R4-4); `IN_DELIVERY ∧ picked_up`→order `CANCELLED`, `IN_DELIVERY ∧ accepted`→order `READY`, both via `updateOrderStatus`. **Flag-ON pre-pickup `accepted` (order still `CONFIRMED/PREPARING/READY`):** no transition, but **R4-3** — publish the `ORDER_STATUS` delta (no-stale) **and** `INSERT INTO courier_dispatch_queue` (auto-re-offer, same mechanism as decline). Central R2-3 fold terminalizes the binding on the `IN_DELIVERY` paths. 🔴 **No stuck `IN_DELIVERY`; abort and decline converge (broadcast + re-offer).** | Distinct from the offer-regret `/cancel`; the time gate is for accept-regret only, never an en-route failure. The no-transition branch is not signal-silent. |
| **Owner cancels an `IN_DELIVERY` order** (no-show `signals.ts:234` / PATCH `orders.ts:779`) | `updateOrderStatus(CANCELLED)` now also terminalizes the active assignment + frees shift in the same tx (R2-3 central fold). | The widened edge can never strand a `'picked_up'` row → `courier_one_active_assignment` never blocks a courier forever. |
| **`paid_partial` attempted** | **422 `PARTIAL_NOT_SUPPORTED`** — forbidden as a delivered outcome (H-2). Customer short on cash → courier instead taps `refused_payment` (→`CANCELLED`, food returns). | No courier ever holds unrecorded cash → no silent-debt trap. |
| **Cash amount ≠ total / float / negative** | 422 `CASH_AMOUNT_MISMATCH` (server-authoritative; integer-`nonnegative` Zod at the edge, M-2), courier re-taps; nothing half-written (txn rolled back before any state change). | Validation precedes mutation; integer-money guard at the boundary, not via a coincidental equality. |

**The only intentional friction in the whole system = shift reconciliation** (Stage-21 owner-alert on the
courier's cash debt). **Zero doorstep friction.** No external call (no payment gateway — cash; no photo
upload — removed; no correlator — never built), so there is nothing to time out or to cascade from.

---

## 8. Security + tenant isolation

- 🔴 **R-1 (must-fix in this migration):** `courier_assignments` is `ENABLE` **without FORCE** and policies
  off `current_setting('app.current_tenant')` (`1780421100041:26-29`), unlike the canon (`SELECT
  app_member_location_ids()` + FORCE). The migration adds `FORCE ROW LEVEL SECURITY` and aligns the policy.
- **Two distinct isolation guards — do not conflate (M-1 honesty fix):** `FORCE` + `app_member_location_ids()`
  closes the **owner/BYPASSRLS** bypass **only**. It is **location-scoped** and therefore **cannot** gate
  **cross-courier-same-location** access — courier B acting on courier A's `'offered'` row in the same
  location is closed **solely** by the inline predicate **`AND courier_id = $authenticatedCourier`** on every
  offered mutation. That is **app-code discipline, not the DB**. (The ADR prose previously over-credited
  FORCE for closing the offer-handshake IDOR; corrected.)
- **The cross-courier guard for accept/decline/complete on `'offered'`:** `WHERE status='offered' AND
  courier_id=$me` (status-guarded **and** courier-scoped), rowcount-0 → 404 — the acting courier MUST be the
  offered courier. Accept already routes through `acceptCourierAssignment` (`assignments.ts:141`, courier-
  scoped); decline keeps the identical predicate. A §9 guardrail asserts no `courier_assignments` mutation
  lacks the `AND courier_id=$<authed>` predicate (red→green: courier B's decline of A's offer → 404).
- **Tenant context** is set per-request (`SET app.current_tenant` / `app.user_id` via `set_config(...,true)`,
  `assignments.ts:79,109,137`). All v2 reads/writes inherit it.
- **Crumbs carry no new PII to anyone untrusted:** `gps_lat/lng` and `name_snapshot` live in
  `delivery_trace` (RLS FORCE, tenant-scoped, owner-readable only). **Claim-check preserved** — the bus/queue
  payloads stay id-only (`orderStatusService.ts:24-31` already strips item names; the sweep payload is
  `order_id` only).
- **Zero cookies / RS256 JWT / Zod `.strict()` / `crypto.randomUUID()` / parameterized SQL** — all preserved
  (`assignments.ts:276-279` body is `.strict()`; `dashboard.ts:300` uses `crypto.randomUUID()`; the
  `STATUS_AT_COLUMN` allowlist at `orderStatusService.ts:111` keeps column names off user input).
- **No PII to AI** — there is no AI surface in this flow.
- **Customer evidence + data-minimization (counsel C2 / M-3b):** burden-of-proof on the customer (§C) is
  only fair if the accuser can see the evidence. Give the **customer read-access to their OWN immutable
  order snapshot** — items, integer price, and **`delivered_at`** — via their existing authenticated order
  read (their own data; **NOT** the courier `gps`/`name_snapshot` crumbs). This recovers the "independent
  signal nobody controls" kernel of the rejected Option 1. The owner-only `delivery_trace` crumbs
  (`gps_lat/lng`, `name_snapshot`, `price_snapshot`) get a **declared purpose = human dispute-adjudication
  evidence** and a **retention bound = `DELIVERY_TRACE_GPS_RETENTION` = 14 days (7-day §C dispute window +
  a stated 7-day off-platform settlement buffer — derived from the purpose, not picked), then GPS +
  `name_snapshot` + `price_snapshot` anonymized to NULL** (🔴 anonymize-not-delete; the non-PII delivery
  facts — total, `delivered_at`, distance, `payment_outcome` — are retained). **R2-7: enforced by a real
  worker** (`workers/delivery-trace-retention.ts` — advisory lock, `.catch`-wrapped cron, boot-assert), not
  prose. 🔴 **R3-1 (the sweep must actually reach the rows):** the worker does **NOT** run a context-free
  operational `UPDATE` (that mirrors `access-request-retention` but `access_requests` is `USING(true)` while
  `delivery_trace` is **tenant-scoped FORCE** `USING (location_id IN (SELECT app_member_location_ids()))`,
  `1790000000027:22-25` — a context-free pool would match **0 rows** and the schedule-existence boot-assert
  could not detect it). Instead it calls the **`SECURITY DEFINER` `anonymize_stale_delivery_trace($window)`**
  (§5 — the canonical cross-tenant mechanism; no per-tenant loop, no dependence on the *operational* role's
  `BYPASSRLS`) and logs/asserts the returned count. 🔴 **Precise mechanism + stated assumption (R4):**
  `SECURITY DEFINER` alone does **not** bypass `FORCE` — FORCE exists to subject the table OWNER to RLS. The
  sweep reaches all-tenant rows **only because the function's OWNER role (the migration `postgres`/admin)
  carries `BYPASSRLS`/superuser**. Assumption made explicit: *migrations run as a privileged
  (BYPASSRLS/superuser) owner* (standard Supabase/Fly deploy); if migrations are ever run by a NOBYPASSRLS
  non-superuser role, the sweep silently anonymizes 0 rows again — which is exactly why the R3-1 efficacy
  guardrail (§9) runs the operational caller as NOBYPASSRLS, to prove the DEFINER *routing* (owner privilege),
  not the *caller's* privilege, is what reaches the rows. Customer snapshot read follows the existing §C
  7-day window.
- **The accused must see the accusation (counsel Q4 — inverse of C2):** the customer order read also surfaces
  the customer's **own** `orders.payment_outcome` + `orders.cancellation_reason` (their own data, RLS-scoped),
  rendered humanely (i18n) — `refused_payment`→"Cancelled — payment was not completed",
  `refused_goods`/`customer_cancelled_on_door`→"Cancelled — recorded as refused at the door",
  `courier_aborted`→"Cancelled — the delivery could not be completed". A courier who pockets the food and taps
  `refused_goods` records the *customer* as refuser; surfacing the customer's own recorded reason lets the
  accused **see and contest** it (server stays authoritative; UI no longer omits the accusation).
- **R2-9 honesty (scope correction):** the minimization claim above covers the **v2-added** crumbs only. The
  customer order route this read piggybacks **already** returns masked courier `full_name`(first-char+`***`)/
  `phone`/live GPS during an active order (`customer/orders.ts:63-87`) — **pre-existing, masked,
  active-order-only**, not introduced by v2. Flagged for separate Privacy review (R2-9 below); out of this
  change's surface.

---

## 9. Operability

- **Health: degraded-vs-down.** Sweep-worker liveness is a *degraded* signal (manual reassignment still
  works), reported distinctly from API/DB *down*. Wire the offered-sweep into the same worker-liveness
  heartbeat as `order-timeout-sweep`/`courier-cron`.
- **Observability (< 1 min):** an offer stuck in `'offered'` past `offered_expires_at + 1 sweep` is a
  metric (`count(*) WHERE status='offered' AND offered_expires_at < now()-interval '90s'`) → alert. Every
  transition is on `order_status_history` + `courier_audit_log` (`assignments.ts:200-203`) with
  `correlationId` in logs (`server.ts:204`).
- **Rollback:** the migration is additive (nullable columns + a CHECK widening + index) → forward-only with a
  trivial revert (drop columns/index, restore prior CHECK) if needed pre-launch. No data backfill to undo.
- **Flag / scaling-gate:** ship the `'offered'` handshake behind `COURIER_OFFER_HANDSHAKE_ENABLED` (default
  **off**). Off = current behavior (owner-direct `'accepted'`). On = offer→accept path + sweep. "Schema rich,
  runtime minimal": the columns/index land inert; the runtime offer path turns on by flag only when the
  courier-app accept/decline UI is ready. The crumb-recording (`payment_outcome`, trace columns) can ship
  unflagged — it is pure passive recording with no behavior change.
- **Guardrail (encodes the §4 discipline — M-3a precise rule):** the read/display vs read/decision line,
  stated mechanically so the rule is falsifiable:
  - **Allowed:** reading a signal row (`delivery_trace`/`order_sensor_events`/`customer_signals`) to
    **serialize it into an HTTP/WS response for a human to view** (display).
  - **Banned:** any signal-row value flowing into a **control-flow branch that mutates order/assignment
    status or writes a ledger row**, **or** that **emits an automated alert/penalty** (the verdict-gate by
    another name — and the courier-scoring vector counsel flagged in agent-health).
  - **Deterministic authority = a behavioral test:** the delivered/transition handlers' **outcome is a pure
    function of the courier's tapped `payment_outcome`/`cash` + server-authoritative `orders` columns,
    independent of any signal-row value** (mutate a signal row → identical outcome). The lint (signal columns
    in an `if`/`switch`/SQL `WHERE` of a state-mutating statement) is **advisory backup**. Honest limit:
    perfect static read/display separation is undecidable → the behavioral test + code-review rule are the
    real gate.
  - **Agent-health extension:** the rule also forbids any **automated courier-scoring/penalty layer derived
    from a crumb** at reconciliation — no such layer may land without its own Triadic Council (closes the
    scoring-creep seam counsel surfaced).
- **H-4 structural guardrail:** a test asserting **every `IN_DELIVERY`/`DELIVERED` order has an assignment
  row that passed through `'accepted'`** — fails red if any force-path reaches delivery without the handshake
  once `COURIER_OFFER_HANDSHAKE_ENABLED` is on.
- **R2-1 completion-parity guardrail:** a test asserting **every order that reaches `DELIVERED` has a
  `delivery_trace` row AND (when `paid_full`) a `courier_cash_ledger` `'hold'` row** — red against the
  current owner-proxy path (`dashboard.ts:444-462` writes neither), green once both routes call
  `completeDelivery`.
- **R2-3 no-strand guardrail:** a test asserting **after any `IN_DELIVERY→{CANCELLED,READY}` transition there
  is zero `courier_assignments` row for that order in an active status** (`offered/assigned/accepted/picked_up`)
  — red against the current no-show path (`signals.ts:234`), green after the central fold in `updateOrderStatus`.
- **R2-8 (M-1 rescope):** the cross-courier-predicate guardrail is scoped to **courier-context handlers only**
  (`routes/courier/*`, `request.user.sub`=courier): every `courier_assignments` mutation there MUST carry
  `AND courier_id = $authenticatedCourier`. Owner-context handlers (`routes/owner/*`, location-scoped by RLS +
  `locationId`) are explicitly carved out — they legitimately mutate without `courier_id=$me`
  (`dashboard.ts:256,372,445`). Precision, not a universal claim.
- 🔴 **C1/Q5 durable artifact (counsel — Stage-21 no-auto-deduct + anti-scoring-creep), authored NOW:**
  (a) a **failing pending-guardrail test** (`stage21-no-auto-deduct.invariant.test.ts`) asserting
  `docs/adr/ADR-stage21-reconciliation.md` exists and contains the markers `NO-AUTO-DEDUCT` **and**
  `NO-COURIER-SCORING` — **RED today**, green only when the Stage-21 author records the invariant (cannot be
  forgotten); (b) an **`eslint-plugin-local` rule** banning any `courier_cash_ledger` write with `type ≠
  'hold'` (a `'deduction'`/`'penalty'`) **and** any penalty/score write deriving from a `delivery_trace`/
  signal-row column, unless the Stage-21 ADR marker is present (the anti-scoring-creep guard, merging the
  agent-health seam); (c) a `docs/regressions/REGRESSION-LEDGER.md` row. R-8 (no-auto-deduct) + R-9
  (embedded-staff) collapse into ONE Stage-21 invariant: *reconciliation never auto-deducts a no-fault
  shortfall and never derives a courier score from a crumb; shortfalls are owner-reviewed friction; no such
  layer lands without its own Triadic Council.*
- **R2-7 retention boot-assert:** `assertDeliveryTraceSchedule()` (mirroring `assertAccessRequestSchedules`)
  runs after `fastify.listen()`; a missing GPS-anonymize cron schedule is a **visible** prod deploy failure
  (`process.exit(1)`), not a silent indefinite-retention drift.
- 🔴 **R3-1 OUTCOME-based retention guardrail (schedule-existence is NOT enough):** schedule-existence
  (`assertDeliveryTraceSchedule`) only catches a *missing* cron — it stays GREEN while a context-free sweep
  silently anonymizes 0 rows under `delivery_trace`'s tenant FORCE policy. Add a deterministic **efficacy**
  integration test (`delivery-trace-retention.efficacy.test.ts`, real PG): seed a `delivery_trace` row across
  **≥2 tenants** with `delivered_at = now() − (window + 1 day)` and non-null `gps_lat/lng`/`name_snapshot`/
  `price_snapshot`; invoke the sweep the way the worker does (operational pool, **no** `app.user_id`/tenant
  context set); assert the call **RETURNs ≥ seeded count** AND **zero `delivery_trace` rows older than
  `(window + grace)` still have non-null GPS** (`count(*) WHERE delivered_at < now()−window AND gps_lat IS NOT
  NULL  ⇒ 0`). 🔴 **R4-1 mandatory precondition — the test's operational caller MUST run under a NOBYPASSRLS
  role** (the proven P6 `provision-rls.test.ts` pattern): the operational pool role carries `BYPASSRLS` today
  (migration 070's own crux comment), and under `BYPASSRLS` a context-free *raw* `UPDATE` ALSO anonymizes
  every cross-tenant row → the test would be **GREEN against both** the round-2 raw UPDATE and the DEFINER fix,
  proving "rows got anonymized" but **not** "the DEFINER routing reached them" (non-discriminating). Only when
  the test's operational caller is NOBYPASSRLS does a context-free raw `UPDATE` genuinely see **0 rows → RED**,
  and the DEFINER fn (reaching rows via its *owner's* privilege, not the caller's) → **GREEN**. Without this
  precondition the red→green claim is not guaranteed. **Red** against the round-2 context-free `UPDATE`
  (0 cross-tenant rows under a NOBYPASSRLS caller + FORCE), **green** only via
  `SECURITY DEFINER anonymize_stale_delivery_trace`. The red-line is enforced by *measured cross-tenant
  efficacy under a NOBYPASSRLS caller*, not the presence of a cron row.
- **R3-2 abort no-throw guardrail (flag-ON):** with `COURIER_OFFER_HANDSHAKE_ENABLED=on`, offer→accept an
  order (stays `CONFIRMED`/`READY`), POST `/abort`, assert **200** (not 400), assignment `'cancelled'`, order
  status **unchanged** (NOT forced `READY`, NOT `DELIVERED`), `orders.courier_id IS NULL`, shift `'available'`,
  order re-offerable. Red against the "force `READY`" spec (throws 400 → rollback → assignment never freed),
  green after the status-conditional order-side action (§5).
- **R3-3 no-new-raw-cancel guardrail:** a `tools/eslint-plugin-local` / grep-gate asserting **no NEW raw
  `UPDATE orders SET status … 'CANCELLED'` from an `IN_DELIVERY`-guarded path outside `updateOrderStatus`**.
  The single existing site (`customer/orders.ts:300-304`) is the **named, frozen, grandfathered** exception
  (allow-listed by exact location, cash-reversal-coupled); any new occurrence is RED → every future
  `IN_DELIVERY→CANCELLED` writer is forced through the central fold. Consolidation of the existing site is
  deferred to R-16 (its own money-path Council).

---

## 10. Open / accepted risks

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R-1 | `courier_assignments` lacks `FORCE` RLS + uses raw `current_setting` policy | **FIX** in this migration (add FORCE, align to `app_member_location_ids()`) | DB/architecture |
| R-2 | Two assignment paths diverge (owner-direct skips handshake, force-drives `IN_DELIVERY`) | **FIX** — unify both onto `'offered'`; owner-direct stops pre-pickup `IN_DELIVERY` | API |
| R-3 | One-time customer who paid without inspecting vs a rare bad-actor owner on a content dispute; courier pocket-and-lie on a no-cash tail | **ACCEPT** (named in contract §C; tightened post-H-3). The distinguishing crumb is now **collectable** (`payment_outcome` persisted) and the **customer can see their own snapshot** (counsel C2). What remains: the system **records** what the courier claimed + where they were, but does not **prove** the claim false (by design — no verdict engine). Till-accountability removes the payoff for `paid_full` lies, so the residual lives **only** on the no-cash tail. Accepted for the niche. | Product |
| R-4 | "Cash = proof" silently assumed when card flips on (§D) | **DEFER-FLAG.** Card has no doorstep cash ⇒ prepaid completion = courier tap + GPS-proximity sanity, no new machinery. 🔴 Burden-of-proof (§C) does **not** generalize: card-scheme rules + Albanian consumer law may statutorily shift burden to the merchant via chargeback regardless of app policy. Make the seam explicit — completion logic must read `payment_method` and **not** assume cash. Tracked as the card-seam ADR, not built now. | Architecture |
| R-5 | Sweep latency (≤ 60 s) before an expired offer is reclaimed | **ACCEPT** — negligible against a 5-min offer window; owner manual reassign is always available immediately. | API |
| R-6 | `customer_signals`/velocity tables exist and *could* be wired to a gate later | **ACCEPT + GUARDRail** — they are already "advisory, NEVER auto-block" (`1780421100057:104`); the §9 lint keeps them passive. | Architecture |
| R-7 | pg-boss `order.timeout`-style queues silently no-op on this infra | **ALREADY ROUTED AROUND** — B2 uses a cron sweep, not a delayed job; do not introduce any new queue. | Ops |
| R-8+R-9 (merged) | **ONE Stage-21 invariant** (counsel C1/Q5 merge): reconciliation **never auto-deducts** a no-fault shortfall (robbery/short-pay/counting error) **and never derives a courier score/penalty** from a crumb; shortfalls are **owner-reviewed friction**; no such layer lands without its own Triadic Council. Subsumes the embedded-staff assumption (R-9) — *no one is auto-deducted regardless of employment status*, which makes employment-status moot for **harm** (the fairness-of-burden narrative stays a launch judgment). | **CARRIED CONSTRAINT → NEEDS-HUMAN, but now a DURABLE ARTIFACT (not prose):** a **red-on-disk failing guardrail** (`stage21-no-auto-deduct.invariant.test.ts` requires `ADR-stage21-reconciliation.md` with `NO-AUTO-DEDUCT`+`NO-COURIER-SCORING`) + an eslint ban on non-`'hold'` ledger writes / crumb-derived penalties (§9). deliver-v2 creates **no** deduction logic (no-op here — only the `'hold'`). The Stage-21 *mechanism* stays human-decided; the *guard* can no longer be skipped by forgetting. | Product + Stage-21 |
| R-11 | Owner-proxy `/deliver` (`dashboard.ts:408-481`) wrote no HOLD/`payment_outcome`/`delivery_trace` — the cash-as-proof primitive absent on the second completion path (R2-1) | **FIXED** — both paths unified through `lib/deliveryCompletion.ts::completeDelivery`; parity guardrail §9. | API |
| R-12 | C-2 revert gated behind the 5-min cancel window → en-route failure >5 min stuck `IN_DELIVERY` (R2-2) | **FIXED** — distinct `/abort` exit, no time gate; `accepted`→READY, `picked_up`→CANCELLED. | API |
| R-13 | Widened `IN_DELIVERY→{CANCELLED,READY}` stranded the no-show / owner-PATCH paths' active assignment (R2-3) | **FIXED** — terminalize folded into `updateOrderStatus`; no-strand guardrail §9. | API |
| R-14 | GPS retention was prose with no worker + 83-day overhang past the dispute window (R2-7/Q2b) | **FIXED** — 14d (7d dispute + 7d buffer) + `delivery-trace-retention.ts` worker + boot-assert. | DB/Ops |
| R-16 (R3-1) | The round-2 worker (mirror `access-request-retention`) anonymizes **0 rows** — `delivery_trace` is tenant-scoped FORCE (`USING app_member_location_ids()`), not `USING(true)`; a context-free pool sees no rows; schedule-existence guardrail stays green | **FIXED** — sweep routes through `SECURITY DEFINER anonymize_stale_delivery_trace($window)` (privileged role bypasses FORCE, pinned search_path, REVOKE/grant-mirror; the read_public_menu/app_is_shadow_location canon) + OUTCOME-based efficacy guardrail (zero non-null GPS past window across ≥2 tenants). §E-consistent: a narrowly-scoped privileged cross-tenant PII-anonymization path is the sanctioned maintenance mechanism, not a normal-path bypass. | DB/Ops |
| R-17 (R3-2) | `/abort` from `accepted` forced `updateOrderStatus(…,'READY')` on a pre-pickup order (`CONFIRMED`/`READY`) → `IllegalTransition`/`SameStatus` 400 → rollback → assignment never freed (flag-ON) | **FIXED** — terminalize binding unconditionally first; order-side action **conditional on the order's actual status** (only `IN_DELIVERY` takes a transition; pre-pickup = clear binding, no transition) → abort always frees the assignment, never throws on a no-op. | API |
| R-18 (R3-3) | The central-fold exhaustiveness claim is carried by a **duplicate** raw-UPDATE at `customer/orders.ts:300-304` (cash-reversal-coupled, omits history/WS) | **ACCEPT-RISK** (residual: pre-existing, customer-facing-only, post-condition holds via the duplicate) **+ DEFER-FLAG** (consolidate via a shared cash-reversal-aware fold under its **own money-path Council** — not folded into v2; rolling the cash-immutable bypass into the central fold for a LOW would expand a 🔴 money primitive) **+ guardrail** (no NEW raw `IN_DELIVERY→CANCELLED` UPDATE; existing site frozen/named). | API |
| R-15 | Customer order route returns masked courier name/phone/live-GPS during active delivery (R2-9, pre-existing) | **ACCEPT-RISK + DEFER-FLAG** — masked, active-order-only, **not introduced by v2**; revisit masked-PII-to-customer separately. §8 minimization claim scoped to v2-added crumbs. | Product/Privacy |
| R-10 | H-4 flag-OFF interim: legacy owner-direct force-`IN_DELIVERY` runs (no offer→accept handshake) until the flag flips | **ACCEPT-RISK (interim).** Bounded + no-trap (C-2 mirror-revert + partial-unique). Structural guarantee (every delivered order passed `'accepted'`) lands when `COURIER_OFFER_HANDSHAKE_ENABLED` turns on; the §9 H-4 guardrail enforces it then. | API |
