# ADR — `deliver` v2 (Cash-as-Proof) + Courier Accept/Decline

- **Status:** Proposed (design-time; no production code in this change)
- **Date:** 2026-06-28
- **Supersedes:** the Stage-20 correlation/verdict engine (never built — see Grounding)
- **Extends:** Stage-18 (courier assignment), Stage-19 (GPS/geofence), Stage-21 (cash HOLD ledger)
- **Related:** ADR-0007/0008/0009 (sensor-bus), proposal `docs/design/deliver-v2-cash-as-proof/proposal.md`

## Context

Delivery confirmation must be **a timestamp recorded by a human tap, not an automated verdict**. The
courier is restaurant staff (fraud = direct owner risk), the customer is a local repeat (wants the food),
the restaurant is local (reputation damage is immediate). So we replace technical verification with
**economic embedding + reputation + burden-of-proof-on-the-accuser**.

**Grounding (verified against live source):** the dangerous Stage-20 surface the contract says to "remove"
**does not exist in production code** — `proof_photo`, `delivery_flags`, the correlation engine, friction-UX,
and heartbeat-as-completion were never built (grep `apps/api/src` + `packages/db/migrations`; matches are
docs-only). The riskiest part is therefore *avoided by a discipline rule, not deleted*. What exists and is
already aligned: `payment_outcome` enum (`1780310044710:16`, value is **`paid_full`**), `delivery_trace`
(immutable, RLS FORCE, `1790000000027`), `courier_cash_ledger` `'hold'` **atomic** with DELIVERED
(`1790000000028` + `assignments.ts:329-361`), and `customer_signals` already marked *"NEVER used for
auto-block"* (`1780421100057:104`). The PENDING timeout is a **timestamp + cron sweep**
(`order-timeout-sweep.ts`), **not** a pg-boss delayed job — and app roles cannot create pg-boss queues
(`assignments.ts:372-378`).

## Decision

1. **Completion = one courier tap carrying `payment_outcome` + cash; no gate, no correlation.** The tap is
   the completion AND the timestamp. The delivered body carries a first-class
   `payment_outcome ∈ {paid_full, refused_goods, refused_payment, customer_cancelled_on_door}` (server-
   authoritative; `cash_collected ⟺ paid_full ∧ cash_amount===total`) and an integer-`nonnegative`
   `cash_amount`; the handler **persists `payment_outcome`** to `orders` and `delivery_trace` in the
   DELIVERED tx (H-3). `paid_full` writes a `courier_cash_ledger` `'hold'` row in the **same transaction**
   as the status-guarded order→`DELIVERED` UPDATE (Stage-21, unchanged). The security primitive is
   **collected-cash accountability (till-accountability)** — recording delivery creates a real debt for cash
   the courier **already physically holds**, reconciled at shift close like any cashier's till; lying
   "delivered+collected" costs money. This is **NOT posted personal surety** — the courier never fronts
   their own capital (prose corrected so no reader mistakes it for a bond, and so it cannot seed a future
   courier-penalty framing).
   - **No-cash tail terminal (H-1):** `refused_goods`/`refused_payment`/`customer_cancelled_on_door` →
     order **`CANCELLED`** (the customer never sees "Delivered" for refused food), assignment `'cancelled'`,
     **no** ledger hold, trace crumb written. Requires the new `IN_DELIVERY→CANCELLED` machine edge — reuses
     the existing terminal, **no `order_status` enum value add**.
   - **`paid_partial` is FORBIDDEN as a delivered outcome (H-2 / counsel C3):** the ledger is full-or-nothing;
     forbidding it removes the one silent-courier-debt trap. A customer short on cash is a `refused_payment`
     tail (→`CANCELLED`). **(Amended — RESOLVE round 5, D-R1):** the forbidding mechanism is **enum-omission →
     400**, not a named `422 PARTIAL_NOT_SUPPORTED`: the delivered-body Zod `payment_outcome` enum lists only
     `{paid_full, refused_goods, refused_payment, customer_cancelled_on_door}`, so `.strict()` rejects
     `paid_partial`/`pending` at the edge before the handler — leaner, single-source-of-truth (the enum *is* the
     allowed set), uniform on both money paths once D3 lands. `CASH_AMOUNT_MISMATCH` (422) remains the one
     explicit completion error.
   - 🔴 **No-partial-handover carried rule (R2-4 / counsel Q1) — what makes forbidding honest:** *full cash in
     hand before the food changes hands; short on cash → no goods, tap `refused_payment` (→CANCELLED, food
     returns).* The completion UI offers only `paid_full`/the no-cash tails (no partial-amount affordance) →
     partial collection is **operationally prevented**, not merely enum-rejected. Without this rule the silent
     debt relocates to the doorstep; with it, the `refused_payment` record is *true*.
   - 🔴 **One shared completion path (R2-1):** both the courier `delivered` handler **and** the owner-proxy
     `/deliver` handler call a single `lib/deliveryCompletion.ts::completeDelivery(client,…)` — the only writer
     of the `'hold'` + `payment_outcome` + `delivery_trace` crumb. The owner-proxy path previously wrote **none**
     (`dashboard.ts:444-462`); unifying makes the cash-as-proof primitive structurally guaranteed on **every**
     `delivered`, with a §9 completion-parity guardrail.
2. **Crumbs stay, the gate goes.** Record passively into the append-only `delivery_trace`:
   delivered-timestamp, GPS proximity at delivery, `payment_outcome` + amount, and an immutable order
   snapshot (name/price). These are **never evaluated, never thresholded, never raise a flag, never block.**
3. **Never build the verdict engine.** No signal→{autoConfirm|friction|block} logic, ever. Encoded as a
   repo guardrail (a test/lint asserting no transition branches on a signal row).
4. **Courier accept/decline (§A):** owner assigns → `courier_assignments.status='offered'` with
   `offered_expires_at = now()+5min`. Accept → `'accepted'` → picked_up → delivered. Decline or 5-min sweep
   timeout → order returns assignable. 🔴 **Timeout/decline never touch the customer's order** — only the
   courier binding is rolled back.
   - **C-1 (re-offer must be physically possible):** the live `courier_assignments_order_uniq` is a **FULL**
     unique on `order_id` → one row per order forever → blocks every re-offer. **Replace it with a PARTIAL
     unique on ACTIVE states** (`WHERE status IN ('offered','assigned','accepted','picked_up')`); terminal
     rows no longer block. This is the central red-line fix — without it the whole §A loop cannot create its
     second row (today: 0% redispatch for any order that ever had a row).
   - **C-3 (reassign must be deterministic):** re-offer/reassign = a status-guarded **terminalize-then-insert**
     (UPDATE the active row → terminal, rowcount=1 wins / 0 → 409, **then** INSERT the new `'offered'` row),
     NOT a fresh unguarded INSERT. Single rowcount authority; the concurrent decline contends the same row.
   - **C-2 (no `IN_DELIVERY` trap):** new machine edges `IN_DELIVERY → {CANCELLED, READY}`. Courier `cancel`
     of an owner-direct order **reverts the order mirror in the same tx** (status-guarded
     `→READY, courier_id=NULL`, routed through `updateOrderStatus`). With the handshake flag ON the owner-direct
     path no longer force-drives `IN_DELIVERY` before pickup, so an order can **never** be `DELIVERED`/
     `IN_DELIVERY` without a real `accepted` assignment (H-4, enforced by a guardrail test).
   - 🔴 **C-2 round-2 closures:** (R2-3) the active-assignment terminalize + shift-free is **folded into
     `updateOrderStatus`** for every `IN_DELIVERY→{CANCELLED,READY}` (one central invariant — *no order leaves
     `IN_DELIVERY` without its active binding terminalized in the same tx*), closing the no-show
     (`signals.ts:234`) and owner-PATCH (`orders.ts:779`) strands the global edge-widening opened. (R2-2) the
     5-min `CANCEL_AFTER_DISPATCH_WINDOW_MS` gate is for **accept-regret only**; a NEW un-gated `/abort`
     en-route exit recovers the >5-min stuck case. 🔴 **R3-2:** `/abort`'s order-side action is **conditional
     on the order's actual status, never a forced transition** — it terminalizes the binding *unconditionally
     first* (abort always frees the assignment), then calls `updateOrderStatus` **only** from `IN_DELIVERY`
     (`picked_up`→`CANCELLED`, legacy-force `accepted`→`READY`); a flag-ON `accepted` order (still
     `CONFIRMED`/`READY`, never advanced) takes the **no-transition** branch (clear `courier_id`, stay
     re-offerable) — so abort can never throw `IllegalTransition`/`SameStatus` and roll back. 🔴 **R4-3:** that
     no-transition branch must converge with the decline path — in the same tx it **publishes a binding-change
     broadcast** (id-only; so owner/customer realtime is not stale) **and re-enqueues to
     `courier_dispatch_queue`** (the same auto-re-offer mechanism the decline path uses), so the order is
     genuinely re-offered, not merely eligible. **(Amended — RESOLVE round 5, D-R3):** the shipped broadcast is
     a `{ type:'binding_changed', orderId }` event on `orderChannel` (id-only, claim-check-clean), not an
     `order.status` delta — ratified; DEFER-FLAG that the owner + customer **FE must handle `binding_changed`**
     (re-fetch the order) for reconvergence. Relevant only once `COURIER_OFFER_HANDSHAKE_ENABLED` flips. **R4-4:** the `IN_DELIVERY ∧ accepted → READY` legacy branch
     also `UPDATE orders SET courier_id=NULL` (the central fold does not touch the mirror) so a re-offerable
     READY order does not point at the departed courier. (R2-5)
     a revert-to-`READY` emits a `READY` event, not `ORDER_CANCELLED`. (R2-6) the owner-reassign revert routes
     through `updateOrderStatus` (history + WS), not a raw UPDATE.
5. **§G resolutions:** (1) offered/accepted live on **`courier_assignments.status`**, NOT the customer-facing
   `order_status` enum (no enum churn; order stays `CONFIRMED`/`READY` until pickup). (2) Unify on
   **`paid_full`** ("paid_cash" is a docs typo). (3) **Do not create `delivery_flags`** — it does not exist
   and Stage-21 reconciliation runs off `courier_cash_ledger`.
6. **Mechanism choices:** offered-timeout = **timestamp column + reuse of the `order-timeout-sweep` cron**
   (NOT a new pg-boss queue — infeasible on this infra). Offer sub-state = **assignment-row**, not an
   `order_status` enum value.
7. **Unify the two assignment paths.** The owner-direct path (`owner/dashboard.ts:302-320`) currently INSERTs
   `'accepted'` and force-drives the order to `IN_DELIVERY` with no handshake — bring it onto `'offered'`,
   advancing the order to `IN_DELIVERY` only at pickup.
8. **Card seam (§D) stays explicit and unbuilt.** `payment_method` enum is `('cash')` only. Completion logic
   must read `payment_method` and must **not** bake in "cash = proof"; burden-of-proof does **not** generalize
   to card (scheme rules + Albanian consumer law may statutorily shift burden to the merchant via chargeback).

## Red lines preserved

Human-authority (one tap, zero auto-verdict) · no state is a trap (offer-timeout/decline/no-cash never block
courier OR customer) · friction-not-verdict (only friction = shift reconciliation) · crumbs passive · status-
guarded transitions (rowcount>0) · N-safe (MessageBus only) · claim-check (queue/bus = id only) · money integer
`CHECK(>=0)` + `.int().nonnegative()` at the edge (M-2) · RLS `SET LOCAL app.user_id`/`app.current_tenant` +
**FORCE** (incl. the R-1 fix bringing `courier_assignments` to FORCE) · zero cookies · `crypto.randomUUID()` ·
Zod `.strict()` · parameterized SQL.

**Isolation honesty (M-1):** `FORCE` + `app_member_location_ids()` closes the **owner/BYPASSRLS** bypass
**only**, and is **location-scoped** — it does **NOT** close the cross-courier-same-location IDOR the offer
handshake introduces. That vector is closed **solely** by the inline `AND courier_id = $authenticatedCourier`
predicate on every offered mutation (app-code discipline + a §9 guardrail test), **not** by the DB. (Earlier
prose over-credited FORCE; corrected.)

## Carried constraints (not built here — recorded for the human + downstream stages)

- 🔴 **R-8 + R-9 merged → ONE Stage-21 invariant, now a DURABLE ARTIFACT (counsel C1/Q5):** reconciliation
  **never auto-deducts** a no-fault shortfall (robbery / short-pay / counting error) **and never derives a
  courier score/penalty** from a crumb; shortfalls are **owner-reviewed friction**; no such layer lands
  without its own Triadic Council. This subsumes the embedded-staff assumption (R-9): no one is auto-deducted
  regardless of employment status, so employment-status is moot for *harm* (the fairness-of-burden narrative
  remains a launch judgment, NEEDS-HUMAN). **Materialized NOW** (not prose): a **red-on-disk failing
  guardrail** (`stage21-no-auto-deduct.invariant.test.ts` requires `docs/adr/ADR-stage21-reconciliation.md`
  with markers `NO-AUTO-DEDUCT` + `NO-COURIER-SCORING`) + an `eslint-plugin-local` ban on non-`'hold'` ledger
  writes / crumb-derived penalties + a regression-ledger row (§9). deliver-v2 creates no deduction logic
  (no-op here — only the `'hold'`); the guard can no longer be skipped by forgetting.
- **Customer evidence (counsel C2 + Q4):** the customer gets read-access to their **own** immutable order
  snapshot (items, integer price, `delivered_at`) **and their own recorded `payment_outcome` +
  `cancellation_reason`** (humane-rendered) — so the *accused* (a customer a lying courier records as refuser)
  can see and contest the record (Q4, inverse of C2). Their own data, NOT the courier GPS crumbs. The
  owner-only `delivery_trace` crumbs (`gps`, `name_snapshot`, `price_snapshot`) carry a **declared purpose**
  (dispute-adjudication evidence) + a **purpose-derived retention bound = 14 days (7-day §C dispute window +
  7-day off-platform settlement buffer), then GPS+name+price anonymized to NULL** (anonymize-not-delete; the
  non-PII facts — total, `delivered_at`, distance, `payment_outcome` — retained). 🔴 **Enforced by a real
  worker** `workers/delivery-trace-retention.ts` (advisory-lock cron + boot-assert) — R2-7, no longer prose.
  🔴 **R3-1 — the sweep must actually reach the rows:** `delivery_trace` is **tenant-scoped FORCE**
  (`USING (location_id IN (SELECT app_member_location_ids()))`, `1790000000027:22-25`), **not**
  `access_requests`' `USING(true)` — so a context-free operational-pool `UPDATE` would match **0 rows** and
  the schedule-existence boot-assert could not detect it. The sweep therefore runs through a **`SECURITY
  DEFINER` `anonymize_stale_delivery_trace(interval)`** (owned by the privileged migration role → bypasses
  FORCE, pinned `search_path`, `REVOKE … FROM PUBLIC` + grant-mirror — the **read_public_menu /
  app_is_shadow_location** canon; `1790000000070:33-34`), with **no per-tenant loop and no reliance on the
  *operational* role's `BYPASSRLS`** attribute. 🔴 **R4 precise mechanism + stated assumption:** `SECURITY
  DEFINER` alone does **not** bypass `FORCE` (FORCE exists precisely to subject the table OWNER to RLS); the
  sweep reaches all-tenant rows **only because the function's OWNER role — the migration `postgres`/admin —
  carries `BYPASSRLS`/superuser.** Explicit assumption: *migrations are run by a privileged
  (BYPASSRLS/superuser) owner* (the standard Supabase/Fly deploy). If migrations are ever run by a NOBYPASSRLS
  non-superuser role, the sweep silently anonymizes 0 rows again. 🔴 **R4-2 dispute-window floor:** the fn
  clamps `p_window` to `GREATEST(p_window, '7 days')` so a mis-set `DELIVERY_TRACE_GPS_RETENTION` can never
  anonymize evidence inside the dispute window. Guardrail strengthened from schedule-existence to
  **OUTCOME-based efficacy** (a real-PG test asserting zero `delivery_trace` rows past the window retain
  non-null GPS across ≥2 tenants); 🔴 **R4-1: the efficacy test runs its operational caller under a NOBYPASSRLS
  role** (the proven `provision-rls.test.ts` pattern) — otherwise the operational role's `BYPASSRLS` makes a
  context-free *raw* `UPDATE` ALSO anonymize cross-tenant rows and the test cannot discriminate the regression
  from the fix → green against both. Under a NOBYPASSRLS caller a raw UPDATE sees 0 rows → RED, the DEFINER fn
  → GREEN. §E-consistent: a narrowly-scoped privileged cross-tenant PII-anonymization path is the sanctioned
  maintenance mechanism, not a normal-path RLS bypass.

## Consequences

**Positive:** additive delta (partial-unique swap + 6 trace columns + `payment_outcome` persist + `'offered'`
handshake + 2 machine edges); reuses shipped Stage-21 + sweep machinery; zero new pools/queues; structurally
cannot trap the customer once the handshake flag is on (offer state on a different row + partial-unique +
guarded reassign + `IN_DELIVERY→{CANCELLED,READY}` edges); no PII photo, no doorstep friction.

**Negative / accepted:** R-3 paid-without-inspecting + courier pocket-and-lie on a no-cash tail — accepted/
niche, tightened (the crumb is now collectable and the customer can see their own snapshot, but the system
records-not-proves). R-4 card seam deferred with an explicit "cash≠proof for card" warning. R-1
(`courier_assignments` missing FORCE) fixed in the migration — but FORCE does NOT close cross-courier IDOR
(M-1; predicate does). R-10 flag-OFF interim runs the legacy owner-direct force path (bounded, no-trap via
C-2 revert) until the handshake flag flips. Sweep adds ≤60 s reclaim latency (negligible vs 5-min window).

**Round-2 closures (regression round):** both completion paths unified through one `completeDelivery` (R2-1,
money red-line on the owner-proxy path); un-gated `/abort` en-route exit (R2-2); assignment-terminalize folded
into `updateOrderStatus` so the widened edge can never strand a binding (R2-3); no-partial-handover carried
rule (R2-4); revert emits the correct event (R2-5); reassign revert routed through the machine (R2-6); GPS
retention aligned to 14d + enforced by a real worker (R2-7); customer sees their own recorded outcome (Q4).

**Round-3 closures (convergence round):** (R3-1, HIGH) the retention sweep runs through a privileged
`SECURITY DEFINER` fn that bypasses `delivery_trace`'s tenant FORCE policy (a context-free pool would
anonymize 0 rows) + an OUTCOME-based efficacy guardrail (zero non-null GPS past window across ≥2 tenants);
(R3-2, MED) `/abort`'s order-side action is conditional on the order's actual status — terminalize-first +
transition-only-from-`IN_DELIVERY` — so a flag-ON pre-pickup abort never throws/rolls back; (R3-3, LOW)
**ACCEPT-RISK + DEFER-FLAG** — the central-fold exhaustiveness is carried by the one pre-existing
cash-reversal-coupled duplicate (`customer/orders.ts:300-304`); consolidating it would expand a 🔴 money
primitive for a LOW gain, so it is deferred to its own money-path Council (R-18) and frozen by a guardrail
banning any NEW raw `IN_DELIVERY→CANCELLED` UPDATE. **converged: 0 open CRITICAL / 0 open HIGH.**

**NEEDS-HUMAN before launch:** the merged **R-8+R-9 Stage-21 invariant** (author `ADR-stage21-reconciliation.md`
with `NO-AUTO-DEDUCT`+`NO-COURIER-SCORING` — the failing guardrail is already red-on-disk; the human records
the rule + the embedded-staff employment judgment), and flipping `COURIER_OFFER_HANDSHAKE_ENABLED` only when the
accept/decline + `/abort` courier UI ships.

## Migration (forward-only, additive, RLS FORCE, integer)

Add `'offered'`/`'offered_expired'` to `courier_assignments.status` CHECK + `offered_at`/`offered_expires_at`
columns + a `WHERE status='offered'` partial index for the sweep; extend `courier_one_active_assignment` to
include `'offered'`. 🔴 **C-1: drop the FULL `courier_assignments_order_uniq` and replace with a PARTIAL
unique `courier_assignments_order_active_uniq ON (order_id) WHERE status IN
('offered','assigned','accepted','picked_up')`** so terminal rows never block a re-offer (the central
no-trap fix). Add **`FORCE ROW LEVEL SECURITY`** to `courier_assignments`. **(Amended — RESOLVE round 5,
D-R2):** the policy is **two-context**, not `app_member_location_ids()` alone — `location_id =
current_setting('app.current_tenant')` (the courier session; couriers live in `courier_locations`, NOT org
memberships, so a member-only policy would break all courier access under FORCE) **OR** `location_id = ANY
(app_member_location_ids())` (the owner). This is a grounding correction, not a weakening: cross-courier IDOR
on the new offer surface is closed by the **M-1 predicate** (`AND courier_id=$me` on every courier mutation's
locking SELECT), not by the policy. **Domain (not a migration):** widen `order-machine.ts` `IN_DELIVERY → ['DELIVERED',
'CANCELLED', 'READY']` (H-1 tail terminal + C-2 revert). Add to `delivery_trace`: `payment_outcome`,
`cash_amount integer CHECK(>=0)`,
`gps_lat/lng`, `name_snapshot jsonb`, `price_snapshot integer CHECK(>=0)`. All nullable/additive — no backfill,
trivially revertible pre-launch. **No `order_status` enum value adds** (avoids irreversible Postgres enum churn).
🔴 **R3-1: also create `SECURITY DEFINER FUNCTION anonymize_stale_delivery_trace(p_window interval) RETURNS
integer` (`VOLATILE`, `SET search_path = public`, body NULLs gps/name/price past `delivered_at < now()−window`
and `RETURN count`), `REVOKE ALL … FROM PUBLIC` + grant-mirror EXECUTE** — the retention worker calls this
(not a context-free `UPDATE`) so it reaches all tenants' stale rows past `delivery_trace`'s tenant FORCE
policy. 🔴 The bypass works **via the function OWNER's `BYPASSRLS`/superuser** (NOT `SECURITY DEFINER` alone —
FORCE subjects the owner to RLS otherwise); assumes migrations run as a privileged owner. 🔴 **R4-2:** the fn
floors `p_window` at `GREATEST(p_window, '7 days')` (dispute window) so a mis-set env cannot over-anonymize
early. Same canon as `read_public_menu`/`app_is_shadow_location`.

## Flag / rollout

`COURIER_OFFER_HANDSHAKE_ENABLED` (default off) gates the offer→accept runtime; schema lands inert ("schema
rich, runtime minimal"). Crumb-recording ships unflagged (pure passive recording, no behavior change). Turn the
handshake on only when the courier-app accept/decline UI is ready.

## Addendum — dispatch-exhausted grace-cancel routed through the machine (2026-07-02)

- **Status:** Accepted (design-time). Ships WITH the code (merge-gated per Counsel).
- **Context:** the offer-sweep Pass-4 grace-cancel (`courier-offer-sweep.ts`, flag-off
  `DISPATCH_OWNER_GRACE_ENABLED`) did a **raw** `UPDATE orders SET status='CANCELLED'` — R3-3 violation
  blocking prod. Root cause: `order-machine.ts` owns no `CONFIRMED/PREPARING/READY→CANCELLED` edge, so
  `updateOrderStatus`/`assertTransition` could not express a no-courier terminal for a pre-IN_DELIVERY order.
  Full analysis + breaker/counsel rounds: `docs/design/deliver-v2-offer-sweep-cancel/`.
- **Decision (Option A + coupling-fix — keeps the machine the SINGLE transition authority):**
  1. **Widen `order-machine.ts` `TRANSITIONS`** — add `CANCELLED` to `CONFIRMED`, `PREPARING`, `READY` (the
     same additive-edge pattern this ADR used for `IN_DELIVERY→{CANCELLED,READY}`). These are **SYSTEM-only**
     terminal edges. Pinned by an **exhaustive `assertTransition` test** so the widening is conscious and
     future drift fails red.
  2. **Owner-exposure closed at the route layer** — the two owner PATCH sites that pipe request `newStatus`
     into `updateOrderStatus` (`routes/orders.ts`, `owner/dashboard.ts::transitionOrder`) call a shared
     `assertOwnerTargetAllowed(from,to)` that rejects owner-requested `CANCELLED` from
     `{CONFIRMED,PREPARING,READY}` → `403 CANCEL_NOT_PERMITTED`. Machine = *what is possible*; route = *who is
     allowed*. Existing owner cancels (`PENDING→CANCELLED`, no-show `IN_DELIVERY→CANCELLED`) preserved.
  3. **Pass-4 routed through `updateOrderStatus`** (system actor) inside the worker tx — no new raw UPDATE, no
     new export, `RAW_CANCEL_ALLOW` unchanged (R3-3 satisfied by a real funnel, not laundering — Option B, a
     colocated `cancelUndispatchableOrder` export, was WITHDRAWN because it was owner-callable + guardrail-blind).
  4. **`ORDER_CANCELLED` fan-out is a POST-commit caller responsibility** — the worker publishes
     `BUS_CHANNELS.ORDER_CANCELLED` after COMMIT (mirrors `signals.ts`) so `lifecycle-handlers` resolves dwell
     alerts + `boss.cancel`s pending `notify.dispatch.*` escalation jobs (else a grace-cancelled order keeps
     open dwell alerts and fires a contradictory escalation after the cancel).
  5. **R2-3 fold extended** — `updateOrderStatus` terminalizes any active assignment on **any**
     `newStatus==='CANCELLED'` (idempotent; previously IN_DELIVERY-only), so a widened edge can never strand a
     binding. Cash-safe: terminalizing writes no `'hold'` (the ledger row is written only by `completeDelivery`
     at DELIVERED).
- 🔴 **STOP-REFUND-BEFORE-GRACE (pre-registered ETHICAL-STOP, attaches to the grace-cancel enablement
  council):** `DISPATCH_OWNER_GRACE_ENABLED` and prepaid (`PAYMENTS_CRYPTO_ENABLED`/`PREPAID`) must **NOT be
  co-enabled** until a paid-prepaid grace-cancel writes a `refund_due` obligation (or is proven impossible by
  state). Grace-cancel routes through `updateOrderStatus`, not `completeDelivery`, so it emits no `refund_due`
  — a paid customer could be silently cancelled with no refund-of-record. Friction, not veto: it pauses
  co-enablement pending a recorded human decision; either flag alone, and the dark code, are unaffected.
  Owner: payments council + grace-cancel council jointly.
- **Cash-as-proof preserved:** no completion path changed; no ledger/trace write added anywhere; the grace
  path (no courier by precondition, re-checked under lock) creates no hold.
- **Deferred:** exposing the widened edge to **owners** (cancel-a-preparing-order) is a product decision owned
  by the grace-cancel STOP-ETHICS council; until then the route guard keeps it SYSTEM-only.
