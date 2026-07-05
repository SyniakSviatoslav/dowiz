# Breaker Findings — MVP Sensor-Bus + Manual-Bridges + North-Star Seams

> Adversarial review of `proposal.md` + ADR-0007/0008/0009. Assumes the implementer does the
> literal minimum the proposal+ADRs describe. Every finding is grounded in verified source
> (file:line) or a back-of-envelope number. NO fixes — the Architect fixes; this names how it
> breaks and which invariant is violated.

Verified source touched: `apps/api/src/routes/orders.ts`,
`apps/api/src/lib/orderStatusService.ts`, `apps/api/src/workers/order-timeout-sweep.ts`,
`apps/api/src/routes/courier/shifts.ts`, `apps/api/src/lib/courier-gps.ts`,
`packages/db/migrations/1780310071220_core-identity.ts`, `…1780421100042_courier-positions.ts`.

---

## CRITICAL

### C1 — B-DATA / B-CONSIST · Decrement-without-restock leaks stock on every cancelled/timed-out/rejected order → the EXACT DoS-on-availability the brief §4 tries to bound, made permanent

**Break scenario (verified, not hypothetical):**
- An order is INSERTed as **`status='PENDING'`** (`orders.ts:609`), **not** CONFIRMED. ADR-0007 puts
  the decrement at create time (`orders.ts:408`→before the INSERT), i.e. stock is burned at **PENDING**,
  before the order is real.
- The lifecycle is: PENDING → owner **manually** confirms, **OR** the `OrderTimeoutSweepWorker` runs every
  minute and does `UPDATE orders SET status='CANCELLED' … WHERE status='PENDING' AND timeout_at < now()`
  (`order-timeout-sweep.ts:67-71`). This sweep does a **bare status flip — it does NOT restock**.
- `grep` over `apps/api/src/` confirms **zero** restock / `stock_remaining +` logic exists anywhere, and
  no owner reject/cancel path restocks either (`apps/api/src/routes/owner/orders.ts` has no `stock`/`restock`).

**Number:** product with `stock_remaining = 5`. Five orders arrive, decrement to 0. Nobody confirms; after
`confirm_timeout_min` the sweep CANCELS all five and **does not give the 5 units back**. The special now
reads sold-out **forever** (until the owner's daily reset), with **zero** real sales. One attacker who can
place 5 PENDING orders has burned the entire special. The brief §4 claims "найгірше = 1 змарнована порція"
(worst case = 1 wasted portion); with auto-decrement at PENDING + no restock, the worst case is **the whole
daily cap, indefinitely**, achievable by orders that are never even confirmed.

**Violated invariant:** brief §0.1 *observe-don't-control on the shared resource* — control here is supposed
to PREVENT oversell, not silently destroy availability. ADR-0007's own "MVCC + ROLLBACK is the compensation"
claim (line 64-65) is **only true inside the create txn**; it is provably false for the CANCELLED-after-commit
path, which is the common path (PENDING is the default landing state and the sweep is the safety net that
runs every minute). The decrement is committed with the order and **survives** the later CANCELLED transition.

**Why ADR-0007 misses it:** the ADR reasons only about the create transaction and the idempotency replay. It
never models the order LIFECYCLE after COMMIT. Proposal §1a line 62 ("only active when stock_remaining is
non-NULL") and the DoD-gate ("stock decrement without oversell") both silently assume decrement-at-create ==
sale, but a PENDING order is not a sale and the dominant terminal states (timeout-CANCELLED, owner-REJECTED)
leak the unit.

---

### C2 — B-FAIL / B-SEC · Geofence capture is silently dead on arrival: the courier ping handler's RLS context cannot satisfy `order_sensor_events`'s WITH CHECK, and the SAVEPOINT swallows the denial → `courier_geofence_enter` is captured NEVER, with zero error surfaced

**Break scenario (verified):**
- ADR-0009 §2 says the geofence INSERT runs in "the IN_DELIVERY courier-ping handler, best-effort in a
  SAVEPOINT." The real ping handler (`courier/shifts.ts:336-376`) opens its txn with
  `SELECT set_config('app.current_tenant', $1, true)` — the **legacy** tenant idiom — and **never** sets
  `app.user_id` (`grep app.user_id apps/api/src/routes/courier/shifts.ts` → 0 hits).
- The proposed `order_sensor_events` table (proposal §3.1 `…071`, ADR-0009 §1) is RLS **FORCE** with
  `tenant_isolation … WITH CHECK (location_id IN (SELECT app_member_location_ids()))`.
  `app_member_location_ids()` derives from `app.user_id` (`1780310071220_core-identity.ts:72,76`) by joining
  `memberships`. In the ping handler `app.user_id` is unset → `app_member_location_ids()` returns the empty
  set. Even if it were set, a courier is in `couriers`, **not** `memberships`, so the membership join is empty.
- Therefore **every** geofence INSERT fails the WITH CHECK (RLS denial). ADR-0009 §2 then mandates "best-effort
  in a SAVEPOINT … a failed sensor insert must NEVER fail the position update" — so the denial is **caught and
  rolled back to savepoint and discarded**. Net result: positions keep flowing, and **not a single
  `courier_geofence_enter` row is ever written.** No error, no metric, no alarm.

**Violated invariant:** brief §1.1 acceptance "geofence_enter рівно раз; відновлювані тривалості prep/road/
**dwell**." Dwell (geofence→picked_up) becomes **永远 NULL** — the P1/P2/P7 falsification fuel (brief §8) the
whole proposal exists to capture is silently zero from day 1, retroactively unrecoverable (the exact failure
the brief calls "ретро не відновити"). The "best-effort SAVEPOINT" pattern, copied from
`orderStatusService.ts:128-139` where the context IS correct, here converts a guaranteed-failing write into a
guaranteed-silent loss.

---

## HIGH

### H1 — B-CONSIST · Idempotency does NOT short-circuit before the decrement for a CONCURRENT same-key retry; ADR-0007's "no double-decrement" proof only covers the SEQUENTIAL replay

**Break scenario:** ADR-0007 §3 and proposal §2.1 claim the idempotency key "short-circuits BEFORE the
decrement → never double-decrements." Verified: the key SELECT is at `orders.ts:364-381` and the key INSERT
is at `orders.ts:655-659` — i.e. the key is **stored AFTER the decrement, near COMMIT**. The two requests
race like this:
- Req A: BEGIN, idempotency SELECT → **0 rows** (key not yet inserted), proceeds, decrements stock, …still
  in-flight, has NOT committed.
- Req B (same key, concurrent): BEGIN on a **separate connection / separate snapshot**, idempotency SELECT →
  **also 0 rows** (A hasn't committed its key INSERT, and READ COMMITTED can't see A's uncommitted row),
  proceeds, **decrements stock a second time.**
- Both reach the key INSERT (`orders.ts:655`); the second to commit hits the `idempotency_keys` PK/unique and
  errors → that txn rolls back (so its decrement is undone). **BUT**: if `idempotency_keys.key` is not unique
  per-tenant or the conflict is not raised, both commit → **double order + double decrement.** Even in the
  benign case, the protection is "one of them crashes on a unique violation," not "the second short-circuits" —
  the brief's stated mechanism ("short-circuits before the decrement") is **false under concurrency**.

**Number:** double-tap / retry-on-slow-network fires two POSTs ~50ms apart with the same key; the create txn
holds for up to 4.5s (`orders.ts:112`). The race window is the entire txn duration — orders of magnitude
larger than the 50ms gap, so the concurrent-same-key case is **likely**, not exotic, on a flaky mobile network.

**Violated invariant:** brief §3.2 "ідемпотентність" + ADR-0007's central correctness claim. The Council must
prove `idempotency_keys` has a real UNIQUE on `(key, location_id)` AND that the second txn's failure mode is a
clean 200-replay (not a 500), or the "idempotent, no double-decrement" guarantee does not hold concurrently.

### H2 — B-DATA / 🔴 ret-migration · ADR-0008's "migration-free manual→derived" proof omits the batch-node (`is_batch_made`/`kind='intermediate'`) intermediate, which has ZERO rows at MVP — its later activation forces exactly the backfill the seam claims to avoid

**Break scenario:** ADR-0008 proves migration-freedom only for the **direct** case: product → `min(ingredient
stock / qty)`. But the schema ships `ingredients.kind='intermediate'`, `is_batch_made`, and a `recipe_components`
row with `parent_kind='ingredient'` (an intermediate's own recipe). At MVP the runtime is FLAT/manual, so **no
owner authors intermediate nodes** (there is no UI/runtime for them — they're inert). When the North-Star
derived reader lands and an owner wants a batch sauce (sold retail AND consumed by 3 dishes), they must:
(a) create the `ingredients` intermediate row, (b) **re-point** every existing product's `recipe_components`
row from the raw ingredients to the new intermediate node, (c) author the intermediate's child recipe. Step (b)
is a **row UPDATE/remap of pre-existing recipe_components rows** — precisely the "no `parent_id` remap" the ADR
swears (line 84) never happens. The proof holds only if recipes were authored against intermediates *from day
1*, which the inert FLAT MVP makes impossible.

**Violated invariant:** brief §6.2/§7 "апгрейд manual→derived міграційно-вільний." The migration-free claim is
proven for a topology (everything is a direct leaf) that the MVP runtime structurally cannot populate with the
intermediate nodes the derived phase needs. The remap is a data migration, not a reader swap.

### H3 — B-DATA · No real FK on `recipe_components.parent_id` + `ON DELETE CASCADE` absent → orphan recipe rows when a product is deleted; the derived reader will compute availability from a dangling parent or silently drop the product's recipe

**Break scenario:** `recipe_components.parent_id` is a bare `uuid NOT NULL` (ADR-0008 DDL line 46). When an
owner deletes a product (products have a real lifecycle — soft/hard delete via the menu manager), the
`recipe_components` rows with `parent_kind='product', parent_id=<deleted>` are **not** cascaded (no FK, no
trigger). They dangle. The "app-layer assertion on write" (ADR-0008 line 98) guards INSERT, not the parent's
DELETE. The future derived reader joining `recipe_components → ingredients` will either (a) include orphan rows
in a tenant's BOM aggregates, or (b) if it joins back to `products`, silently skip them. With a real FK the DB
guarantees referential integrity on delete; the polymorphic seam gives that up **and** provides no
delete-side compensation.

**Violated invariant:** the integrity a real FK provides (no orphan parent). ADR-0008 §"Integrity of the
missing parent_id FK" lists three holders (CHECK, write-assertion, inertness) — **none** covers parent-row
DELETE. Accepted-risk disposition (proposal §7) says "the derived reader adds the integrity join then," but a
join cannot resurrect already-orphaned rows or detect which deletion they belonged to.

### H4 — B-SCALE / B-OPS · The public `funnel_events` ingest is unauthenticated and uncapped → a single attacker floods the heaviest-writer table, skews the padding-creep counter-metric, and the 90-day sweep is the only defense (which runs on a table the attacker controls the size of)

**Break scenario:** proposal §4.2/§4.3 says funnel ingest is "fire-and-forget on a separate request path" with
a "uniform 200/204 regardless of validity" and "no auth" (it's the public storefront funnel). There is **no
rate-limit named** on this endpoint (the velocity throttle at `orders.ts:250` guards order-create, not funnel).
An attacker scripts `checkout_abandon` events with arbitrary `shown_eta_lo/hi_min`:
- **Volume:** the proposal's own estimate is 300–1500 funnel rows/day/location *legitimately*. An unthrottled
  script does that per second. At 1k inserts/s for an hour = 3.6M junk rows in one table for one tenant — past
  the proposal's own "~1M rows → revisit partitioning" threshold (§7) in 17 minutes.
- **Signal poisoning:** brief §8.2 makes funnel abandon-rate the **counter-metric** that brakes padding-creep
  (the loop is allowed to move `variability(σ)` based on it). Injected `checkout_abandon` rows with fake long
  ETAs feed a **falsified counterfactual** straight into the falsification harness the proposal exists to
  protect. The attacker doesn't just DoS — they steer the future autopilot's reliability brake.
- **Sweep cost:** the 90-day DELETE (proposal §6) on a multi-million-row table is itself a long-running,
  lock/IO-heavy operation; the proposal does not bound batch size, so the sweep can contend with live writes.

**Violated invariant:** brief §1.3 "анонімно … нуль PII понад потрібне" addresses PII but not enumeration/flood;
§8.2's padding-creep brake requires a **trustworthy** abandon signal. No auth + no rate-limit + a signal that
feeds a control loop = a poisonable sensor. (B-SEC overlaps: the uniform-200 hides existence but does nothing
against volumetric abuse.)

### H5 — B-FAIL · The set-once `promised_window` BEFORE-UPDATE trigger fires on EVERY orders UPDATE (a hot path), and any unrelated UPDATE that round-trips the row through the ORM with the same value re-written is safe only if the app never touches those columns — a fragile invariant on the busiest table

**Break scenario:** ADR-0009 §3 puts a `BEFORE UPDATE … FOR EACH ROW` trigger on **`orders`** — the hottest
mutable table (every status transition, every timeout flip, every sweep CANCELLED UPDATE runs through it). Two
problems:
1. **Overhead/lock surprise on the hot path:** a row-level trigger executes a plpgsql block on *every* orders
   UPDATE, including the timeout sweep's bulk `UPDATE orders SET status='CANCELLED' … RETURNING` (which can hit
   many rows at once, `order-timeout-sweep.ts:67`). The proposal's §1c latency analysis covers only the
   decrement, never this trigger's per-UPDATE cost on the order lifecycle.
2. **Bypass / false sense of immutability:** the trigger is `IS DISTINCT FROM` — it only blocks a *changed*
   value. The immutability is "only as strong as the trigger," and the proposal itself (proposal §5, line 240
   / ADR-0009 §5) admits `order_status_history` hardening is deferred. A migration or superuser write
   (release_command runs as the migration role) bypasses BEFORE-UPDATE triggers if `session_replication_role =
   replica`, and any future code path that legitimately needs to correct a mis-set window (proposal §7 admits
   product may disagree) has no escape hatch but to drop the trigger. The "hard DB invariant" is real for app
   writes and illusory for privileged writes — flag the asymmetry.

**Violated invariant:** none broken outright, but the proposal oversells "DB-enforced, not convention"
(proposal §5 line 240) without analyzing the trigger's blast radius on the hot orders UPDATE path or the
privileged-write bypass. HIGH because it's added to a 🔴 hot table with no latency/lock measurement.

---

## MEDIUM

### M1 — B-CONSIST · Reconstructable durations are biased from day 1 for pickup / no-courier / cancelled-mid-flight orders — a naive AVG over `delivered_at − in_delivery_at` divides by a partial, self-selected set

**Break scenario:** ADR-0009's "reconstructable durations test" asserts prep = ready−preparing, road =
delivered−in_delivery, dwell = picked_up−geofence. But: pickup-type orders never get `in_delivery_at`/
`delivered_at` via a courier; orders cancelled mid-flight have a NULL terminal timestamp; orders with no
courier assigned never reach IN_DELIVERY. Plus C2 makes the geofence ts always NULL → **dwell is always
uncomputable**, not just for edge orders. A day-1 analytics AVG that ignores NULL silently averages only the
happy-path delivered-by-courier subset → **biased sensor data from the very first week**, which is exactly the
measurement-bias the brief §1.2/§8.1 says this work exists to *fix*. The schema captures timestamps; nobody has
specified the NULL-handling contract for the reconstruction, so the literal-minimum implementer writes a biased
AVG.

**Violated invariant:** brief §8.1 "нормалізувати на складність, не сирий час" / "зміщення виміру." The bias is
reintroduced at the reconstruction layer the proposal leaves unspecified.

### M2 — B-SEC · `funnel_events.session_ref` is re-identifiable: a `session_ref` that later places an order can be correlated to that order's customer (phone/IP), de-anonymizing the "anonymous" funnel

**Break scenario:** proposal §4.3 calls `session_ref` "an opaque client-minted session id … No PII." But the
same browser session that emits `menu_view`/`checkout_start` funnel rows is the session that submits the order;
if the FE reuses the session id (or the order-create path logs it, or timestamps line up within seconds), an
analyst with both tables joins `funnel_events.session_ref`/`created_at` to the order's `customer_id`/phone_hash
by time-correlation. "Anonymous, session-scoped" holds only if the session_ref is **never** observable on an
order and the timing can't be correlated — neither is asserted. The proposal claims anonymity as a property it
doesn't enforce.

**Violated invariant:** brief §1.3 "анонімно, session-scoped, нуль PII понад потрібне." Anonymity is asserted,
not designed-in (no unlinkability argument, no rotation of session_ref at order time).

### M3 — B-SCALE / B-FAIL · Geofence INSERT under rapid boundary-crossing pings: even after C2 is fixed, the `INSERT … ON CONFLICT DO NOTHING` must be confirmed — a NAIVE `INSERT` (no ON CONFLICT) would 23505-error inside the ping txn and, if not caught precisely, poison the SAVEPOINT or bubble into the position update

**Break scenario:** courier pings ~1/10s (`DeliveryPage.tsx` `COURIER_GPS_POST_INTERVAL_MS`, server gate
1/10s). A courier hovering on the geofence boundary (GPS jitter) crosses in/out repeatedly. ADR-0009 §1 *claims*
`ON CONFLICT DO NOTHING`, but proposal §4.2's row says only "best-effort in a SAVEPOINT." If the implementer
writes a plain `INSERT` (the literal §4.2 text doesn't repeat ON CONFLICT), the second crossing raises 23505 on
`UNIQUE(order_id, event_type)`; the savepoint catch must be a *precise* rollback-to-savepoint or the whole ping
txn aborts and the position update is lost. MEDIUM (not HIGH) because ADR-0009 §1 does state ON CONFLICT — but
the two documents disagree and the acceptance test must pin it.

**Violated invariant:** brief §1.1 "geofence_enter рівно раз" + §0.1 "sensor never fails the order/position."
Contingent on the implementer following ADR-0009 §1 over proposal §4.2.

### M4 — B-SCALE · Deadlock risk: adding per-product `UPDATE … RETURNING` decrements inside the order txn introduces row-lock-ordering across multiple product rows; two overlapping multi-item orders that lock products in different orders can deadlock on the hot path

**Break scenario:** ADR-0007 decrements "per DISTINCT product" inside the txn. Order X = {pizza, cola}, Order Y
= {cola, pizza}. If the implementer iterates the cart in insertion order (not a sorted order), X locks pizza
then waits on cola; Y locks cola then waits on pizza → **deadlock**; Postgres kills one with 40P01. The
proposal's §1c latency analysis and ADR-0007's race-correctness argument both reason about a *single* contended
row; neither addresses multi-row lock ordering. The decrement must be applied in a **deterministic sorted
order** (e.g. by product id) to be deadlock-free — unspecified, so the literal-minimum implementer iterates
`items` order and ships the deadlock.

**Violated invariant:** none stated, but it widens the failure surface on the 🔴 order path and can surface as
random 500s under concurrent multi-item carts. Worsens the pool-wedge class the 4.5s timeout (`orders.ts:112`)
already guards (a deadlock-victim retry re-enters the held-connection path).

### M5 — B-ANTIPATTERN · The §4 velocity throttle the proposal leans on (5 orders / 15 min) is keyed on PHONE only; cash-on-delivery + no OTP means a phone-rotating attacker is ungated → C1's stock-burn is not even velocity-bounded

**Break scenario:** proposal §4.3 says "velocity throttle (`orders.ts:250-269`)" bounds the no-OTP DoS. Verified:
the hard gate is `if (phoneHash) { … THROTTLE_MAX_ORDERS = 5 }` (`orders.ts:250-261`) — keyed on `phoneHash`,
**not** `clientIpHash` (which is computed at `:247` but never gates). An attacker submitting cash orders with a
fresh fake phone each time (no OTP verifies the phone is real — §4 defers OTP) is **never throttled**. Combined
with C1 (decrement at PENDING, no restock), one scripted attacker exhausts a 5-unit special in 5 requests, each
with a different fake phone, faster than the per-minute timeout sweep and far faster than an owner's one-tap
abort can react. The proposal's "найгірше = 1 порція" rests on a throttle that the attack trivially sidesteps.

**Violated invariant:** brief §4 "velocity-ліміти (phone+IP)" — the proposal cites phone+IP but the code gate
is phone-only; the IP half is unbuilt, so the cited mitigation is half the size claimed.

---

## LOW

### L1 — B-DATA · `recipe_components` allows ingredient→ingredient cycles now (DAG constraint explicitly deferred); a manually-entered cycle is inert today but becomes an infinite-recursion / non-terminating reader when the derived tree-walk lands

**Break scenario:** brief §6.2 defers "DAG без циклів, memoize, кап глибини." The seam allows
`parent_kind='ingredient'` rows, so an owner (or a buggy import) can author A consumes B, B consumes A **today**,
with nothing to reject it. It's inert at MVP, but the cycle is a **persisted landmine**: the future recursive
`available_units()` reader (ADR-0008 line 81-83, "recursing through parent_kind='ingredient'") has no cycle
guard yet and will infinite-loop / stack-overflow on that row. LOW because it's inert now and the derived reader
is out of scope — but the data can be entered before the guard exists, so the guard must validate pre-existing
rows, not just new ones.

**Violated invariant:** brief §6.2 (DAG/cap-depth deferred but the *data* that violates it is enterable now).

### L2 — B-CONSIST · Range-never-point can collapse to a point at the `eta_cap_min` clamp or when `lo == hi`; the schema having two columns does not prevent the client from rendering "5 min"

**Break scenario:** proposal §5 / ADR-0009 §4 claim range-never-point is "structurally unrepresentable" because
the schema has only `_lo`/`_hi`. But nothing forbids `lo == hi` (a 5-unit window written as lo=5,hi=5), and the
`eta_cap_min` clamp (`hi := min(hi, eta_cap)`) can drive `hi` down to equal `lo`. The client then renders
"5 min" — a point — from a perfectly valid two-column row. The invariant is enforced at the schema shape but
NOT at the value level (`CHECK (hi > lo)` is absent) nor at the render contract. The proposal even admits
"(Council to confirm the client response Zod schema rejects a single-number ETA)" — i.e. the enforcement is a
TODO, not a fact. LOW because cosmetic, but it directly contradicts the §0.4 "навіть «1–2 хв»" promise.

**Violated invariant:** brief §0.4 range-never-point. The schema-shape argument is necessary but not sufficient;
value-level (`hi > lo`) and render-level enforcement are unspecified.

### L3 — B-OPS · `delivery_trace` baseline idempotency is asserted ("already ON CONFLICT DO NOTHING") but grep finds the ON CONFLICT writers in `notifications/workers`, not a verified DELIVERED-handler trace write — the proposal's §4.1 citation is unverified

**Break scenario:** proposal §4.1 claims "`delivery_trace` already UNIQUE(order_id) + the DELIVERED handler
uses ON CONFLICT DO NOTHING." A grep for `delivery_trace` + `ON CONFLICT` did not surface a DELIVERED-handler
upsert (the ON CONFLICT hits found are in `notifications/workers/index.ts`). If the baseline write
(§1.2 route_distance/expected_delivery at delivered_at) is a plain INSERT/UPDATE, a re-fired DELIVERED
transition (or the sweep) could double-write or 23505. LOW because likely a stale citation, but the
idempotency claim for the baseline write is **not** verified by the cited mechanism — Council should confirm
the actual DELIVERED-handler trace write path.

**Violated invariant:** proposal §4.1 idempotency claim (citation unverified, not necessarily false).

---

## Severity summary

| # | Severity | Vector | One-line |
|---|---|---|---|
| C1 | CRITICAL | B-DATA/B-CONSIST | Decrement at PENDING + no restock on CANCELLED/REJECTED/timeout → whole daily cap burned by never-confirmed orders |
| C2 | CRITICAL | B-FAIL/B-SEC | Geofence INSERT can't satisfy RLS in the courier-ping context; SAVEPOINT swallows it → geofence/dwell captured NEVER, silently |
| H1 | HIGH | B-CONSIST | Concurrent same-key retry double-decrements; idempotency key stored AFTER decrement, near COMMIT |
| H2 | HIGH | B-DATA (🔴 ret-mig) | "Migration-free" proof omits batch/intermediate nodes (zero rows at MVP) → their activation forces a recipe_components remap |
| H3 | HIGH | B-DATA | No FK / no cascade on parent_id → orphan recipe rows on product delete |
| H4 | HIGH | B-SCALE/B-SEC | Unauthenticated, unthrottled funnel ingest → flood + poison the padding-creep counter-metric |
| H5 | HIGH | B-FAIL | Set-once trigger on the hot orders UPDATE path: unmeasured overhead + privileged-write bypass |
| M1 | MED | B-CONSIST | Naive AVG over partial NULL timestamp set → biased sensor data day 1 (pickup/no-courier/+dwell-always-NULL) |
| M2 | MED | B-SEC | session_ref re-identifiable via time-correlation to the order's customer |
| M3 | MED | B-SCALE/B-FAIL | Geofence ON-CONFLICT must be pinned; proposal §4.2 vs ADR-0009 §1 disagree |
| M4 | MED | B-SCALE | Multi-row decrement lock-ordering → deadlock on overlapping multi-item carts |
| M5 | MED | B-ANTIPATTERN | Cited velocity throttle is phone-only, not phone+IP; phone-rotating attacker ungated |
| L1 | LOW | B-DATA | Cycle enterable now; future tree-walk reader has no cycle guard |
| L2 | LOW | B-CONSIST | range-never-point collapses to a point at eta_cap clamp / lo==hi (no `hi>lo` CHECK) |
| L3 | LOW | B-OPS | delivery_trace baseline idempotency citation unverified |

**Load-bearing irreversible decisions most at risk:** C1 (the 🔴 money/availability decrement — breaks the
brief's own DoS bound), C2 (the 🔄 sensor fuel the proposal exists to capture is silently zero), H2/H3 (the
🔴 ret-migration the BOM seam is built to avoid is re-introduced by the omitted intermediate node and the
missing FK).

---

# RE-ATTACK round 2 — regression pass on the round-1 FIXES

> Scope: did the C1/C2/H1/ESTOP-1/H2/H3 fixes (resolution.md + ADR-0007/0008/0009 **v2** + proposal.md
> hardened) introduce NEW holes, and do they actually close the originals? Every finding below is grounded in
> verified live source (file:line), not the design docs' self-description. NO fixes.
>
> Verified this round: `apps/api/src/routes/customer/orders.ts:239-335` (customer-cancel raw UPDATE),
> `apps/api/src/lib/orderStatusService.ts:89-181` (the ONLY guarded path), `apps/api/src/routes/owner/dashboard.ts`,
> `apps/api/src/routes/courier/assignments.ts:406-422`, `apps/api/src/routes/owner/signals.ts:226-228`,
> `apps/api/src/workers/order-timeout-sweep.ts:67-71`, `apps/api/src/routes/courier/shifts.ts:330-379`,
> `packages/db/migrations/1780421100042_courier-positions.ts:20-24`, `…1780310074262_orders.ts:62-98`
> (idempotency_keys DDL: `order_id uuid REFERENCES orders` nullable, `request_hash text NOT NULL`),
> `apps/api/src/routes/orders.ts:364-381,656` (replay + key INSERT).

## NEW CRITICAL

### R2-C1 — B-DATA/B-CONSIST · The C1 restock matrix has a HOLE: the customer post-dispatch cancel (`POST /orders/:orderId/cancel`) cancels an IN_DELIVERY (post-CONFIRMED, `stock_committed=true`) order via a RAW direct UPDATE that bypasses `updateOrderStatus` entirely → it never restocks → the exact leak C1 claims is closed is STILL OPEN on a customer-triggerable path

**The round-1 fix's load-bearing assumption is false in source.** ADR-0007 v2 §3 (line 93-94) and resolution.md C1 both state the restock "share[s] the same `orderStatusService` guarded-UPDATE machinery" — i.e. the fix assumes EVERY `CONFIRMED→REJECTED/CANCELLED` transition flows through `updateOrderStatus`, where the flag-guarded restock would be wired. **It does not.**

Verified bypass (`apps/api/src/routes/customer/orders.ts:289-293`):
```sql
UPDATE orders
SET status = 'CANCELLED', cancelled_at = now(), cancellation_reason = $1
WHERE id = $2
```
This is the customer-facing `POST /orders/:orderId/cancel` (`:239`). It guards `order.status === 'IN_DELIVERY'` (`:275`) and a 5-min post-dispatch window (`:283`), then flips status to CANCELLED with a **raw `client.query` UPDATE** — it does NOT call `updateOrderStatus`, the only place the v2 restock is specified to live. An IN_DELIVERY order is unambiguously post-CONFIRMED (CONFIRMED→PREPARING→READY→IN_DELIVERY), so by the v2 design `stock_committed = true` and a unit was decremented at confirm. This cancel restocks **nothing**.

**Break scenario / number:** product `stock_remaining = 5`. Five orders are placed, confirmed (auto-confirm or owner), dispatched (IN_DELIVERY). Stock is now 0 — correct so far (5 real commitments). Within the 5-minute post-dispatch window each customer taps "cancel" (`reason ≥ 5 chars`, the only gate). Five raw UPDATEs flip them to CANCELLED; the courier shift is freed; **`stock_remaining` stays 0**. Net: zero sales, special reads sold-out until the owner's daily reset — **the identical permanent-leak C1 describes, now reachable by ordinary customers exercising a shipped feature, not even an attacker.** The C1 matrix row "CONFIRMED → REJECTED/CANCELLED → restocked (once, flag-guarded)" silently assumes a code path this transition does not take.

**Violated invariant:** brief §0.1 observe-don't-control on the shared resource + ADR-0007 v2's own "no leak on ANY terminal path" claim (DoD matrix line 137). The matrix is proven for the `updateOrderStatus` paths (owner-reject `dashboard.ts:201`, no_show `signals.ts:228`) and FALSE for the raw-UPDATE customer-cancel path. C1 is **NOT genuinely closed** — the fix closed the dominant PENDING-timeout/PENDING-reject paths (correctly, via decrement-at-confirm) but left a post-confirmed terminal path that does not route through the guarded machinery. The round-1 C1 verdict must be downgraded from CLOSED to PARTIALLY-closed-with-a-live-leak.

> NOTE the asymmetry that makes this worse than it looks: the v2 design moved BURDEN onto the restock path while only auditing TWO writers. There are at least three order-status writers in source (`updateOrderStatus`, the timeout sweep's raw bulk UPDATE, and this customer-cancel raw UPDATE) plus `owner/dashboard.ts` reassign logic; any restock wired only into `updateOrderStatus` is by construction blind to the raw-UPDATE cancel. The DoD's "CONFIRMED→CANCELLED restocks exactly once" test (ADR-0007 v2 DoD #2) will PASS if it drives the transition through `updateOrderStatus` and **will never exercise the `/orders/:orderId/cancel` route** — a green test that does not cover the leaking path. This is a cheat-green risk, not just a miss.

## NEW HIGH

### R2-H1 — B-SEC/B-CONSIST · The C2 dual-context RLS disjunction widens the courier write to TENANT-only with NO order-assignment scope: any context with `app.current_tenant = X` can INSERT an `order_sensor_events` row for ANY order_id at location X — including an order the courier is not assigned to — because the WITH CHECK validates `location_id`, never `order_id`-ownership

**The disjunction is exactly as wide as the Breaker warned, and it matches `courier_positions`' weakest property, not its app-layer scoping.** ADR-0009 v2 §1 WITH CHECK is `location_id IN (app_member_location_ids()) OR location_id = current_setting('app.current_tenant')`. The courier ping handler sets `app.current_tenant = request.user.activeLocationId` (`shifts.ts:337`, value from the courier's own JWT). So the write is scoped to the **tenant**, and the RLS checks only that `order_sensor_events.location_id` equals that tenant. It does **not** verify the `order_id` belongs to an assignment the courier holds.

Compare to the cited model: `courier_positions`' policy (`1780421100042_courier-positions.ts:22-23`) is `USING (location_id = current_setting('app.current_tenant')::uuid)` — tenant-only, and crucially **ENABLE, not FORCE**, and it carries no `order_id` at all (a position is courier+location). The assignment-scope for positions is enforced in the APP layer (`shifts.ts:365-370` checks `onActiveDelivery` before INSERT). The proposed `order_sensor_events` row, by contrast, DOES carry an `order_id`, and nothing in the policy nor (as designed) the handler restricts which order_id a courier may stamp.

**Break scenario:** courier C is on shift at location X (legitimately holds a JWT with `activeLocationId = X`). Orders O1 (C's own assignment) and O2 (a *different* courier's assignment at the same venue) are both IN_DELIVERY. With `app.current_tenant = X` set, C's session (or a compromised/replayed courier token, or a malformed ping payload that carries an attacker-chosen `order_id`) can `INSERT INTO order_sensor_events (order_id=O2, location_id=X, 'courier_geofence_enter')` and the WITH CHECK PASSES — O2 is at location X. C just forged a geofence-enter (and therefore a dwell start) on another courier's order. The §8 dwell/road reconstruction for O2 is now poisoned by a fabricated timestamp; if a future courier normalized-time metric (ADR-0009 §4c, North-Star) ever reads dwell, one courier can manufacture another's performance data.

This is strictly WIDER than v1's intent: v1 wanted "the courier stamps a geofence on the order they are delivering." The disjunction delivers "any session at the tenant stamps a geofence on any order at the tenant." The `order_id` is the natural scoping key and the policy throws it away. **Cross-TENANT** leak is bounded (a courier's `app.current_tenant` is pinned to their JWT's `activeLocationId`, so they cannot set it to another tenant), so this is intra-tenant, courier-vs-courier — HIGH, not CRITICAL. But it is a real authorization regression introduced by the fix: the dual-context policy bought C2's geofence-presence at the cost of an order-ownership check that neither idiom enforces.

**Read side:** the read disjunction is safe for owners — an owner reads via `app_member_location_ids()` (only their locations) and never sets `app.current_tenant`, so they cannot read another tenant's sensor events. The hole is write-side and intra-tenant.

**Violated invariant:** brief §1.1 "geofence_enter рівно раз" carries an implicit "for the courier actually delivering this order"; the dual-context WITH CHECK reduces that to "for any order at the tenant." The DoD's cross-tenant SELECT test (ADR-0009 DoD line 205) checks tenant isolation and will pass — it does NOT test order-assignment scope, so the hole ships green.

### R2-H2 — B-CONSIST · Claim-first idempotency POISONS the key on the FK/back-fill ordering, and the crash-between-claim-and-commit case bricks the key forever — the claim row is committed with `order_id = NULL` (FK allows it) but a later txn rollback leaves a NULL-order claim that the existing replay path (`orders.ts:375-381`) cannot resolve

**The fix changes the replay contract and the failure mode in ways the existing replay code does not handle.** Verified facts:
- `idempotency_keys.order_id` is `uuid REFERENCES orders(id)` **nullable** (`1780310074262_orders.ts:65`). `request_hash` is `NOT NULL` (`:64`).
- The existing replay (`orders.ts:369-381`): on a found key it reads `SELECT … FROM orders WHERE id = row.order_id`; if **0 rows** (order_id was NULL or points to a vanished order) it `DELETE`s the key (`:380`) and falls through to create a NEW order.

Now apply claim-first (ADR-0007 v2 §4: "claim with a NULL order_id, then `UPDATE … SET order_id` before COMMIT"):

1. **Crash/rollback after claim, before order INSERT commits (intra-txn):** ADR-0007 v2 says the claim and the order share one txn ("back-fill `order_id` onto the claimed row before COMMIT"). If they are in ONE txn, a rollback undoes BOTH the claim and the order — so the key is NOT poisoned, BUT then claim-first provides **zero** concurrency benefit over the old SELECT-then-INSERT, because a concurrent peer's `ON CONFLICT DO NOTHING` only sees the claim row once it COMMITS, and at that moment the order is committed too. The "second peer blocks on the unique index then re-reads the committed order" story REQUIRES the claim to be visible-but-order-not-yet — which only happens if the claim is in a SEPARATE committed txn. The resolution wants both: "claimed first" (separate visibility) AND "rolled back together" (same txn). **These are mutually exclusive.** ADR-0007 v2 §4 line 125-126 even hedges ("or claimed with the order_id once the INSERT returns; ADR-0007 v2 pins the exact two-statement order") — i.e. the exact ordering that determines whether the bug exists is UNPINNED, left to the implementer.

2. **If the implementer commits the claim separately (to get real claim-first concurrency):** a crash between the claim-COMMIT and the order-COMMIT leaves a row `(location_id, key, request_hash, order_id=NULL)` **committed**. A legitimate retry now hits `existingKey.rowCount > 0` (`:369`), passes the hash check, runs `SELECT … FROM orders WHERE id = NULL` → 0 rows → `DELETE`s the key (`:380`) → creates a fresh order. That is *recoverable* (good) but it means claim-first does NOT actually short-circuit a retry-after-crash; it falls back to the old delete-and-recreate, so the "exactly one order" guarantee depends on the crash window being empty. Worse: if TWO retries race in this NULL-order state, both `SELECT WHERE id=NULL`→0, both `DELETE`, both create → **double order** — the exact failure claim-first was supposed to remove, resurfacing in the crash-recovery path.

3. **Concurrent peer return value:** the existing replay returns the **full order body** (`SELECT id, status, subtotal, total, created_at, timeout_at` — `:375`, 200). ADR-0007 v2 §4 says the blocked peer "re-reads the now-committed order and returns the stored 200-replay." But with `ON CONFLICT DO NOTHING RETURNING key` the peer gets **0 rows back from the INSERT and no order_id** — it must then re-run the `:365` SELECT to find the winner's `order_id`, then the `:375` SELECT for the body. That re-SELECT is NOT in the claim-first code path the ADR describes; if the implementer follows the ADR literally (claim → 0 rows → "return 200-replay") without re-entering the existing `:365-378` block, the peer returns a **bare 200 with no body** (or worse, proceeds as if it won). The round-1 replay semantics (return the prior order body) are not guaranteed to survive the restructure — the ADR asserts the outcome but the wiring to the existing replay SELECT is unspecified.

**Violated invariant:** brief §3.2 idempotency + the resolution's "clean 200-replay (not 500)" claim. H1 (sequential double-decrement) IS now moot because the create path no longer decrements (the decrement moved to confirm — genuinely closes the H1 *double-decrement*). But the idempotency *correctness under crash + the replay body contract* are newly fragile, and the one decision that determines whether the key gets poisoned (same-txn vs separate-txn claim) is explicitly left unpinned. HIGH because a crash-poisoned or NULL-order key on the money path is a real consistency hole, even though the original double-decrement is closed.

## NEW MEDIUM

### R2-M1 — B-CONSIST · ESTOP-1 split creates TWO sources of "the time" with NO specified writer for `live_eta_*` after confirm → the live channel is dead-equal-to-frozen, so the customer sees the FROZEN promise anyway (the split solves nothing in MVP), OR if a writer is added it can diverge from the frozen promise with no reconciliation rule

**The split is schema-only; the behavior that makes it meaningful is unwired.** ADR-0009 v2 §3 line 134: "`live_eta_*` is seeded equal to it at confirm and then **updated** as the order collapses through stages." Grep of the proposal/ADRs for a `live_eta` WRITER beyond the confirm-seed finds none — §2.4 "collapsing window" is described but no migration, worker, or status-transition hook is specified to actually UPDATE `live_eta_*` at PREPARING/READY/IN_DELIVERY. The literal-minimum implementer seeds `live_eta = promised_window` at confirm and never updates it.

Consequence: in MVP the customer reads `live_eta_*` which is byte-identical to the frozen `promised_window_*` forever. The ESTOP-1 tension the split claims to "dissolve" — customer frozen into a possibly-wrong number with no repair path — is **NOT dissolved**; the customer still sees the frozen first promise, just read from a different column. The split bought a mutable column that nothing mutates. If, conversely, a §2.4 writer IS added, two questions are unanswered: (a) which channel drives the §2.4 proactive-shift notification (resolution says "customer reads live_eta" and "§8 reads promised_window" but the notification trigger source — frozen vs live — is unstated), and (b) when live diverges (live says 50-60, frozen says 30-45) there is no rule that the live channel also obeys range-never-point's `min_window_width` floor or the `eta_cap` — the `CHECK (hi >= lo+1)` exists on both pairs (ADR-0009 §3 line 113-114) but the width-floor synthesis (`hi := max(hi, lo+min_window_width)`) is specified only "AFTER the eta_cap ceiling clamp" in the synthesis helper, and it is unstated whether the live-channel update re-runs that helper. So range-never-point's value-level guarantee (L2) is proven for the synthesis path but not asserted for the live-channel UPDATE path.

**Violated invariant:** none broken outright in MVP (because live==frozen, the customer sees a valid range). MEDIUM because the split is sold as "dissolving the tension" but in the shipped MVP it is inert (live channel unwired) — so ESTOP-1's actual customer-honesty concern is deferred, not resolved, and the "customer always sees the current truth" claim (resolution ESTOP-1, proposal §6 risk row line 333) is FALSE until a live_eta writer ships. The resolution should mark ESTOP-1 as DEFERRED-via-inert-column, not RESOLVED.

### R2-M2 — B-DATA · The H3 AFTER-DELETE cascade trigger is `FOR EACH ROW` → it does NOT fire on `TRUNCATE` and does not cover a soft-delete that later hard-purges via a bulk path; orphan `recipe_components` are still reachable

**A row-level trigger is not an FK, and the gap is a real Postgres semantic, not a hypothetical.** ADR-0008 v2 §"Integrity" guard #4 (lines 134-139) is `CREATE TRIGGER … AFTER DELETE ON products FOR EACH ROW`. Postgres `FOR EACH ROW` DELETE triggers **do not fire on `TRUNCATE`** (TRUNCATE only fires statement-level `AFTER TRUNCATE` triggers, which this is not). A native `ON DELETE CASCADE` FK *is* honored on TRUNCATE-with-CASCADE; the trigger is not — so the ADR's claim "exactly like a native `ON DELETE CASCADE` FK would do" (line 141-142) is FALSE for the TRUNCATE path.

**Break scenario:** any future test-data reset, tenant-purge, or maintenance script that `TRUNCATE products CASCADE` (a common operational shortcut, and the repo already does bulk test-data ops per MEMORY) removes products but the trigger never fires → `recipe_components` rows with `parent_kind='product', parent_id=<truncated>` orphan exactly as in the original H3. The future derived reader joins through them. Additionally: the products soft-delete path (menu manager) leaves recipe_components in place by design (resolution notes "the trigger is the hard-delete backstop") — so as long as products are soft-deleted, the trigger never runs and orphans-on-soft-delete accumulate until/unless a hard purge happens; the cleanup is coupled to a hard DELETE that may never come.

**Violated invariant:** the referential integrity a real FK provides. H3 is **partially** closed (the single-row hard DELETE path is now covered, which is the common owner action) but the TRUNCATE and soft-delete-without-purge paths remain orphan-producing. The ADR over-claims FK-equivalence. MEDIUM (down from the original HIGH) because the common path is now covered, but the equivalence claim is false and the orphan class is not eliminated.

## Regression verdict — round-1 findings re-checked against the v2 fixes

| R1 # | R1 sev | Fix claim | Round-2 verdict (verified) |
|---|---|---|---|
| **C1** | CRIT | decrement@confirm + flag-guarded restock | **PARTIALLY CLOSED — new R2-C1 (CRIT).** PENDING-timeout/PENDING-reject leak genuinely closed (decrement-at-confirm is correct, verified vs `order-timeout-sweep.ts:67` which only flips PENDING). BUT the customer post-dispatch cancel (`customer/orders.ts:289`) is a raw UPDATE that bypasses `updateOrderStatus` → CONFIRMED→CANCELLED leaks. NOT fully closed. |
| **C2** | CRIT | dual-context RLS disjunction, FORCE | **CLOSED for silent-loss; OPENS new R2-H1 (HIGH).** The geofence INSERT will now satisfy WITH CHECK via `app.current_tenant` (verified the courier handler sets it `:337`) → the silent-zero is genuinely fixed. BUT the disjunction is tenant-only on the write side with no order-assignment scope → intra-tenant courier-vs-courier forgery. Cross-tenant is bounded (JWT-pinned tenant). |
| **H1** | HIGH | claim-first idempotency | **double-DECREMENT genuinely closed** (create path no longer decrements). **Idempotency correctness NOT cleanly closed — new R2-H2 (HIGH):** same-txn-vs-separate-txn claim is unpinned (the two are mutually exclusive for the stated guarantee), crash leaves a NULL-order key, replay-body contract unspecified in the restructure. |
| **H2** | HIGH | honest re-scope | **GENUINELY CLOSED.** ADR-0008 v2 honestly documents node-introduction as a named owner-driven backfill (lines 95-104), no longer claims universal migration-freedom. The contradiction is resolved by honesty, which is the correct disposition. No new hole. |
| **H3** | HIGH | AFTER DELETE cascade trigger | **PARTIALLY CLOSED — new R2-M2 (MED).** Single-row hard-DELETE covered; TRUNCATE + soft-delete-without-purge still orphan. FK-equivalence claim is false. |
| **H4** | HIGH | per-IP rate-limit + batch sweep + per-session advisory | Plausibly closed at design level (per-IP cap + distinct-session aggregation + advisory-not-actuator). Residual distributed-botnet is honestly accept-risk'd (Ops). No re-attack regression; not re-verified against a built rate-limiter (none exists yet — design-time). |
| **H5** | HIGH | I/O-free trigger + privileged-bypass honesty | Closed at design level — the trigger body is genuinely query-free (verified the plpgsql in ADR-0009 §3 is pure comparisons); bypass asymmetry honestly stated. No new hole. |
| **M1** | MED | NULL-handling reconstruction contract | Closed at contract level (segmented, both-endpoints-non-NULL, report n). Depends on implementer honoring it; no re-attack regression. |
| **M3/M4/M5/L2/L3** | MED/LOW | pinned ON CONFLICT / sorted lock order / IP gate / width floor / folded upsert | All addressed at design level; no new holes found. M4 sorted-by-product_id is the correct deadlock fix. L2 width-floor closes the point-collapse. |
| **ESTOP-1** | E-STOP | frozen + live split | **Schema-closed, behavior-DEFERRED — new R2-M1 (MED).** The split is real in schema but `live_eta_*` has no specified writer post-confirm → live==frozen in MVP → the customer-honesty concern is inert, not dissolved. Should read DEFERRED, not RESOLVED. |

## NEW-finding severity summary (round 2)

| # | Severity | Vector | One-line | Closes/regresses |
|---|---|---|---|---|
| R2-C1 | **CRITICAL** | B-DATA/B-CONSIST | Customer post-dispatch cancel (`customer/orders.ts:289`) raw-UPDATEs CONFIRMED→CANCELLED, bypasses `updateOrderStatus`, never restocks → C1 leak still live | C1 not fully closed |
| R2-H1 | **HIGH** | B-SEC/B-CONSIST | C2 disjunction is tenant-only with no order-assignment scope → intra-tenant courier-vs-courier geofence/dwell forgery | C2 fix's new hole |
| R2-H2 | **HIGH** | B-CONSIST | Claim-first: same-txn-vs-separate-txn unpinned (mutually exclusive guarantee), crash→NULL-order poisoned key, replay-body contract unspecified | H1 idempotency-correctness regression |
| R2-M1 | MED | B-CONSIST | ESTOP-1 split is schema-only; `live_eta_*` has no post-confirm writer → live==frozen → customer-honesty inert, not "dissolved" | ESTOP-1 deferred not resolved |
| R2-M2 | MED | B-DATA | AFTER-DELETE FOR-EACH-ROW trigger doesn't fire on TRUNCATE / soft-delete → orphans still reachable; FK-equivalence claim false | H3 partially closed |

**Load-bearing regressions (as instructed, the two to weigh hardest):**
1. **R2-C1 (the restock-leak matrix):** the C1 fix's entire no-leak proof rests on "every terminal transition flows through `updateOrderStatus`," and source has a customer-facing raw-UPDATE cancel that does not. The matrix is green on the audited writers and red on the un-audited one. C1 is **not** closed.
2. **R2-H1 (the tenant-isolation hole):** the dual-context disjunction trades C2's silent-loss for an order-ownership gap — write authority is `location_id`-scoped, never `order_id`-scoped, matching `courier_positions`' weakest property while carrying an `order_id` that positions never had. Intra-tenant courier-vs-courier forgery is the concrete break.

---

# RE-ATTACK round 3 — exit check on the v3 fixes (restock TRIGGER + geofence assignment-scope + claim `state` lifecycle + live_eta floor)

> NARROW scope: did the round-2 v3 fixes (resolution.md "RESOLVE round 2" + ADR-0007/0009 v3) introduce a NEW
> CRITICAL/HIGH, or is this at hard-exit? Every finding below is grounded in verified LIVE source (file:line),
> not the ADR's self-description. NO fixes. Verified this round:
> `packages/db/migrations/1780310074262_orders.ts:50-58` (order_items DDL: `product_id uuid REFERENCES products` **nullable**, col is `quantity` not `qty`, `order_id … ON DELETE CASCADE`),
> `…1780338982023_order_items_product_fk_set_null.ts:6-8` (**`product_id … ON DELETE SET NULL`**),
> `…1780310072731_menu.ts:42-45` (`products` **FORCE RLS**, USING `app_member_location_ids()`, no WITH CHECK),
> `…1780338741329_public-menu-rls.ts:10-11` (`public_select` on products = SELECT-only USING(true)),
> `…1780310071220_core-identity.ts:76-77` (`app_member_location_ids()` SECURITY DEFINER, reads `app.user_id`),
> `apps/api/src/routes/customer/orders.ts:255-319` (cancel handler: raw `db.connect()`, sets ONLY `app.settlement_reversal`, **never** `app.user_id`/`app.current_tenant`),
> `apps/api/src/lib/orderStatusService.ts:89-139` (CONFIRMED/DELIVERED/else branches; REJECTED/CANCELLED fall through with NO `*_at` stamp),
> `apps/api/src/routes/courier/shifts.ts:365-369` + `apps/api/src/lib/courier-gps.ts:9` (assignment read `LIMIT 1`, **no ORDER BY**; ACTIVE = `['accepted','picked_up']`),
> `…1780421100041_courier-assignments.ts:23-24` (`UNIQUE(order_id)` only — **no** partial-unique on courier active-status → a courier CAN hold many active assignments),
> `docs/design/mvp-sensor-seams/brief.md:49` (`courier_sequence` = the batch-sequence seam, P3),
> ADR-0009 v3:200/236-237 (floor AFTER cap: `hi:=min(hi,eta_cap)` then `hi:=max(hi,lo+min_window_width)`),
> proposal.md:205 (`eta_cap_min DEFAULT 90`, `min_window_width_min DEFAULT 5`).

## NEW CRITICAL

### R3-C1 — B-SEC/B-DATA · The restock trigger's `UPDATE products` is BLOCKED by `products` FORCE-RLS in the customer-cancel context (no `app.user_id` set) → the trigger runs, restocks ZERO rows, silently → R2-C1's "unbypassable DB trigger" leaks on the exact route it was built to cover

**This is the #1 attack target (trigger access to order_items quantities) — and the real gap is one layer deeper than the join: it is RLS, on `products`, in the firing context.**

ADR-0007 v3 §3 moves restock into `orders_restock_on_terminal()` precisely so the raw customer-cancel (R2-C1) is covered "regardless of which writer issues the status flip." Verified, the trigger body (ADR-0007 v3:109-115) issues `UPDATE products p SET stock_remaining = stock_remaining + oi.qty FROM (SELECT … FROM order_items WHERE order_id = NEW.id …) oi WHERE p.id = oi.product_id AND p.location_id = NEW.location_id`. A plpgsql trigger function with no `SECURITY DEFINER` clause runs **SECURITY INVOKER** — under the calling session's role AND its `app.*` settings.

Now walk the leaking route the fix exists for (`customer/orders.ts:255-319`):
- The cancel handler opens a **raw pool connection** (`db.connect()`, `:255`) and sets **only** `SET LOCAL app.settlement_reversal='true'` (`:297`). It **never** sets `app.user_id` (0 hits in the file) nor `app.current_tenant`.
- `products` is **FORCE ROW LEVEL SECURITY** (`menu.ts:43`). FORCE means even the table owner / API role is subject to RLS. The only writable policy is `tenant_isolation USING (location_id IN (SELECT app_member_location_ids()))` (`menu.ts:44-45`); the `public_select` policy is **SELECT-only** (`public-menu-rls.ts:10`), so it does not admit the UPDATE. For an UPDATE the USING expression is the row-visibility filter.
- `app_member_location_ids()` (SECURITY DEFINER, `core-identity.ts:76-77`) derives its set from `memberships` keyed on `current_setting('app.user_id')`. In the cancel context `app.user_id` is **unset** → the function returns the **empty set** → the USING predicate is false for every product row → the trigger's `UPDATE products` matches **0 rows**.

**Break scenario / number (demonstrable, not hypothetical):** product `stock_remaining = 5`; 5 orders confirmed→IN_DELIVERY (decremented to 0, `stock_committed=true` each). Each customer taps `POST /orders/:orderId/cancel` within the 5-min window. The trigger fires (status flips CANCELLED, `OLD.stock_committed=true`, `IS DISTINCT FROM` true) — it enters the body, runs `UPDATE products … +qty` **which hits 0 rows under FORCE-RLS**, then sets `NEW.stock_committed := false`. Result: status=CANCELLED, **`stock_committed` is now false (the idempotency guard is spent)**, and **`stock_remaining` was never incremented**. The unit is leaked AND the flag that would let a later corrective restock re-fire is cleared. **5 customer cancels = 5 permanently-leaked units = the identical C1/R2-C1 permanent leak, on the precise raw route the v3 trigger was introduced to close.** The "DB trigger cannot be bypassed by any writer" claim (ADR-0007 v3:95-96, Consequences:298) is true for the *trigger firing* and false for the *restock landing*: FORCE-RLS on `products` silently zeroes the write in the one context that lacks the member idiom.

Worse than R2-C1: R2-C1 was a *missing* restock (flag stays true, theoretically re-restockable). R3-C1 *consumes the idempotency flag while restocking nothing* — so even a future correct restock path keyed on `stock_committed=true` now finds it false. The anti-cheat-green DoD #3 (drive the real `/cancel` route, assert `stock_remaining` back to 5) is exactly the test that **goes RED** here — IF it runs against FORCE-RLS with the real handler's empty context. If DoD #3 is written against a test harness that pre-sets `app.user_id` (or a BYPASSRLS test role), it cheat-greens past the very bug.

**Violated invariant:** brief §0.1 observe-don't-control on the shared resource + ADR-0007 v3's "no unit leaks on ANY terminal path" (matrix line 259) + "restock is UNBYPASSABLE" (Consequences). The owner-REJECT path (`dashboard.ts`, member context, `app.user_id` set) restocks fine; the customer-cancel path (no member context) does not — the trigger's correctness is **silently context-dependent**, and the SECURITY INVOKER + FORCE-RLS interaction is unanalysed in ADR-0007 v3.

## NEW HIGH

### R3-H1 — B-DATA · `order_items.product_id` is `ON DELETE SET NULL` → a confirmed order whose product was later deleted restocks NOTHING (and the trigger's join silently drops the line), re-leaking the unit on cancel; the trigger's "right products/qtys" assumption fails on the deleted-product path

**The #1-bullet join question (does a `BEFORE UPDATE ON orders` trigger reach the per-product quantities) is answerable YES for the happy path — but the FK semantics break it on a real path.** The columns line up: `order_items.quantity` exists (`orders.ts:56`, the trigger correctly aliases `sum(quantity) qty`) and the `FROM (subquery) … WHERE order_id = NEW.id` correlates to the row being updated — so the trigger CAN read the line items. BUT:

- `order_items.product_id` is **`ON DELETE SET NULL`** (`1780338982023:6-8`), and the column is nullable (`orders.ts:53`). When an owner deletes a product (a real lifecycle action; the menu manager hard-deletes), every `order_items` row that referenced it gets `product_id = NULL` — the historical order keeps `name_snapshot`/`price_snapshot`/`quantity` but **loses the product pointer**.
- The trigger groups `SELECT product_id, sum(quantity) … GROUP BY product_id` and joins `WHERE p.id = oi.product_id`. A NULL `product_id` row **never matches any `p.id`** → that line's quantity is **silently dropped** from the restock. The order decremented a real unit at confirm (the product existed then); after the product's deletion, a cancel restocks **nothing for that line**. The unit leaks, no error.

**Break scenario:** owner runs a flash special (product P, `stock_remaining=10`), 3 orders confirm+dispatch (P down to 7, three orders `stock_committed=true`). Owner deletes P from the menu (special over). `order_items.product_id → NULL` for those 3 lines. A customer then cancels within the window: trigger fires, the subquery yields a NULL-product group that joins to no `products` row → 0 rows updated for that line → `stock_committed` flipped false → **unit leaked**. (Note: if P was deleted, "leaked stock on P" is moot for P — but the same SET-NULL applies to ANY shared product across a multi-item order: order = {deleted P, live Q}; cancelling restocks Q but the trigger still flips `stock_committed=false` having only partially restocked, and the matrix's "restocked once" is now "restocked the surviving lines only".) The trigger's matrix row "CONFIRMED→CANCELLED → restocked (once, flag-guarded) → 0 leak" is FALSE for any order containing a since-deleted product line.

**Violated invariant:** ADR-0007 v3's no-leak matrix + the trigger's implicit "the order's line items + quantities tell us what to give back" — `ON DELETE SET NULL` severs exactly that linkage, and the trigger has no fallback (no snapshot product_id, no `name_snapshot`-keyed reconciliation). HIGH not CRITICAL because it requires a product deletion between confirm and cancel (narrower than R3-C1's every-cancel break), but it is a real path on a live menu-management feature and the trigger silently under-restocks.

## NEW HIGH

### R3-H2 — B-CONSIST · The geofence `order_id` is read with `LIMIT 1` and NO `ORDER BY` over a courier who can hold MULTIPLE active assignments (the batch seam) → the geofence/dwell binds to a NON-DETERMINISTIC / wrong order; R2-H1's "the courier's OWN assignment" is not singular

**R2-H1 (resolved as "derive order_id from the courier's own assignment") assumed the courier has exactly one active assignment. Source contradicts the assumption.** ADR-0009 v3 §2a:113-115 specifies `SELECT order_id FROM courier_assignments WHERE courier_id = $courier AND status = ANY($active::text[]) LIMIT 1` — a copy of the existing GPS-gate read at `shifts.ts:365-369`, which is `… LIMIT 1` with **no ORDER BY**.

- `courier_assignments` has **only `UNIQUE(order_id)`** (`1780421100041:23`) and a non-unique `(courier_id, status)` index (`:24`). There is **no** partial-unique index restricting a courier to one active (`accepted`/`picked_up`) assignment. So a courier CAN simultaneously hold N active assignments.
- The brief itself names this: `courier_sequence` is "шов під батч-послідовність P3" (`brief.md:49`) — batching is an explicit, designed seam, and ACTIVE = `['accepted','picked_up']` (`courier-gps.ts:9`) admits a picked-up-but-not-delivered batch.

**Break scenario:** courier C does a 2-order batch (O1, O2, both `picked_up`). C approaches O1's venue/customer; the ping handler computes a geofence crossing and needs an `order_id`. `SELECT … LIMIT 1` (no ORDER BY) returns an **arbitrary** row — Postgres may return O2. The `courier_geofence_enter` for O1's physical crossing is stamped on **O2** (`ON CONFLICT(order_id,event_type) DO NOTHING` then permanently locks O2's geofence to the wrong timestamp; O1 may never get one, or gets one at O2's crossing). Dwell/road reconstruction for both orders is now wrong: O1's dwell is NULL or late, O2's is fabricated. This is the *same falsification-fuel-poisoning* R2-H1 was upgraded-to-FIX to prevent ("falsification fuel must stay trustworthy at the capture layer", resolution R2-H1) — except the poisoning is now self-inflicted by non-determinism, not a malicious colleague. The geofence is also "exactly once per (order,event)" but it is once on the WRONG order.

**Violated invariant:** brief §1.1 "geofence_enter рівно раз" carries "for the order this crossing belongs to"; `LIMIT 1`-no-ORDER-BY over a multi-active-assignment courier binds it to an arbitrary order. The R2-H1 DoD ("courier stamps O1 only") passes only when the courier has a single assignment — it does NOT exercise the batch case, so the hole ships green. HIGH: it silently corrupts the §8 P1/P2/P7 measurement fuel the whole proposal exists to capture, on a designed (P3 batch) path.

## NEW MEDIUM

### R3-M1 — B-CONSIST · `live_eta` width-floor applied AFTER the eta_cap clamp pushes `hi` ABOVE `eta_cap` → the "eta_cap absolute" invariant is violated whenever `lo > eta_cap − min_window_width` (defaults: `lo > 85`); floor and cap can disagree, floor wins

**The #4 floor/cap-cross question: they do not cross to `lo > hi` (no inversion), but the floor defeats the cap.** ADR-0009 v3:200/237 fixes the order as `hi := min(hi, eta_cap_min)` THEN `hi := max(hi, lo + min_window_width_min)`. With defaults `eta_cap_min=90`, `min_window_width_min=5` (proposal:205):

- `lo` is **never clamped to the cap** (only `hi` is, per §4:245 "clamps `hi_min` to eta_cap_min"). So `lo` can exceed `eta_cap − 5`. Example: a late IN_DELIVERY recompute yields `lo=92, hi=95`. Cap clamp: `hi := min(95, 90) = 90`. Floor: `hi := max(90, 92+5=97) = 97`. Final `(lo=92, hi=97)` — `hi=97 > eta_cap=90`. **The cap is silently exceeded.** No `lo > hi` inversion (97 > 92), and the DB `CHECK (hi >= lo+1)` passes — so nothing rejects it; the customer is shown a window above the "absolute" cap, and the §1.4 cap-hit owner advisory may or may not fire (the value WAS clamped then un-clamped).

**Break scenario:** any order genuinely running long (`lo ≥ 86` after a real travel estimate) renders a customer band whose upper bound breaches `eta_cap_min` — exactly the padding-creep ceiling §1.4 calls "a hard external brake." It is a narrow numeric corner (requires `lo` near the cap, i.e. a very late order), and it is honest-ish (a late order SHOULD show a late number) — so MEDIUM, not HIGH. But it falsifies the stated invariant "eta_cap absolute / hard external brake on padding-creep" (§4:245-247): the floor, applied last, can lift `hi` past the cap, and the cap-hit advisory's trigger condition (`hi == eta_cap` post-clamp) is now ambiguous. No `lo > hi` crossing exists; the failure is cap-breach, not inversion.

## Round-3 exit verdict

The four v3 mechanisms were attacked individually. Result: **NOT at hard-exit — two NEW issues at CRITICAL/HIGH on the restock mechanism, one NEW HIGH on the geofence binding, one NEW MEDIUM on the floor.**

1. **Restock trigger (ADR-0007 v3) — NEW CRITICAL + NEW HIGH.**
   - Idempotency/double-restock: **clean** — `OLD.stock_committed=true` + same-row `NEW.stock_committed:=false` + `OLD.status IS DISTINCT FROM NEW.status` makes double-restock and re-open-leak impossible *when the UPDATE lands*. No double-restock finding.
   - Coexistence with the set-once `promised_window` trigger: **clean** — they are different triggers; both are `BEFORE UPDATE`, restock is `OF status`-gated, set-once fires on any UPDATE but its body only RAISEs on a frozen-column change. CANCELLED/REJECTED transitions don't touch `promised_window_*`, so the set-once trigger is a no-op pass-through; the two do not interfere or suppress each other. Trigger fire-order is alphabetical (`orders_promised_window_set_once_trg` < `orders_restock_on_terminal_trg`) and order-independent here. No conflict finding.
   - Terminal set / DELIVERED walk: **clean** — DELIVERED is excluded by `status IN ('CANCELLED','REJECTED')`; it routes through `orderStatusService.ts:100-104` which never sets those statuses → no restock on a fulfilled sale. Correct.
   - **order_items quantities access: BROKEN two ways — R3-C1 (CRITICAL, FORCE-RLS on `products` zeroes the UPDATE in the customer-cancel context) and R3-H1 (HIGH, `ON DELETE SET NULL` severs `product_id` so deleted-product lines silently drop).** The trigger CAN read `order_items.quantity` (the join is sound mechanically), but the restock WRITE to `products` fails RLS in the leaking context, and the product linkage is destroyable. This is the predicted-most-likely gap, confirmed deeper than the join.

2. **Geofence order_id from the courier's own assignment (ADR-0009 v3 §2a) — NEW HIGH (R3-H2):** "the courier's assignment" is NOT singular — `LIMIT 1`-no-ORDER-BY over a courier with multiple active (batch) assignments binds the geofence to an arbitrary/wrong order. Not unambiguous at ping time.

3. **Claim-state reclaim (ADR-0007 v3 §4) — CLEAN (no new C/H).** `state {claimed→completed}` single-txn: the unique index serializes; the guarded `DELETE … WHERE state='claimed' AND claimed_at < threshold RETURNING` lets exactly one retry win (loser sees 0 rows, re-reads) → no double-create. `T = CLAIM_STALE_MS` IS defined (≈30s, ADR:209/214). `completed` is set in the same txn as the order INSERT before COMMIT (`§4:181-184`) → a committed order always has `state='completed'`; a crash rolls back claim+order together → no "order committed but completed unset" path. The one residual ("orphaned separate connection" surviving `claimed` row) is honestly bounded and reclaim-guarded. No finding.

4. **live_eta writer + width-floor (ADR-0009 v3 §3a) — NEW MEDIUM (R3-M1):** no recompute bypasses the floor (all stages route through the same synthesis helper, §3a:197-200 — good). But floor-after-cap lets `hi` exceed `eta_cap` (the cap is not absolute); no `lo > hi` inversion (the DB CHECK holds). Narrow numeric corner → MEDIUM.

**Hard-exit conditions NOT met.** New blocking findings introduced by the v3 fixes:
- **R3-C1 (CRITICAL)** — restock trigger's `UPDATE products` is RLS-denied (FORCE-RLS, empty member context) on the customer-cancel route → the v3 "unbypassable restock" leaks ZERO-rows-silently on the exact path it was built to fix, and consumes the `stock_committed` idempotency flag while restocking nothing. The single most load-bearing failure: it re-opens C1/R2-C1 at the layer below the join, in the context the DoD must test against the REAL handler (not a member-context harness) or it cheat-greens.
- **R3-H1 (HIGH)** — `ON DELETE SET NULL` on `order_items.product_id` makes the trigger silently under-restock any order containing a since-deleted product.
- **R3-H2 (HIGH)** — geofence `order_id` non-deterministic for a batched courier (`LIMIT 1`, no ORDER BY, multi-active assignments allowed).

| # | Severity | Vector | One-line | Mechanism attacked |
|---|---|---|---|---|
| R3-C1 | **CRITICAL** | B-SEC/B-DATA | Restock `UPDATE products` RLS-denied (FORCE-RLS, no `app.user_id`) in customer-cancel ctx → 0 rows, flag consumed, unit leaked | restock trigger (#1) |
| R3-H1 | **HIGH** | B-DATA | `order_items.product_id ON DELETE SET NULL` → deleted-product lines silently drop from restock | restock trigger (#1) |
| R3-H2 | **HIGH** | B-CONSIST | Geofence `order_id` via `LIMIT 1`-no-ORDER-BY over multi-active (batch) assignments → binds to wrong order | geofence scope (#2) |
| R3-M1 | MED | B-CONSIST | live_eta floor-after-cap pushes `hi` above `eta_cap` (no `lo>hi` inversion) → "eta_cap absolute" false | live_eta floor (#4) |

**Clean (no new C/H), as verified above:** claim-state reclaim lifecycle (#3) — T defined, no double-create, no committed-but-uncompleted path; restock idempotency/double-restock/DELIVERED-exclusion; set-once ↔ restock trigger coexistence.
