# Resolution — MVP Sensor-Seams Batch (Architect, RESOLVE step)

> Disposition of every Breaker finding (C1–L3) + the Counsel ETHICAL-STOP (ESTOP-1) + Counsel's
> non-blocking advice. Each row: **fix** (proposal/ADR updated) / **accept-risk** (+owner+justification) /
> **defer-flag** (MISSING — explicit) / **human-needed**. Every disposition is grounded in verified source
> (file:line), not hand-waved. Companion: `proposal.md` (hardened), `docs/adr/0007|0008|0009`.

Verified source re-touched this round: `apps/api/src/routes/orders.ts` (`:105` BEGIN, `:364` idem SELECT,
`:609` INSERT `'PENDING'`, `:655` idem INSERT), `apps/api/src/lib/orderStatusService.ts:89-117` (CONFIRMED
guarded UPDATE), `apps/api/src/workers/order-timeout-sweep.ts:67-71` (PENDING→CANCELLED, no restock),
`apps/api/src/routes/courier/shifts.ts:336-378` (sets `app.current_tenant`, NOT `app.user_id`),
`packages/db/migrations/1780421100042_courier-positions.ts:22-23` (RLS keyed on `app.current_tenant`, ENABLE
only), `…041_courier-assignments.ts:28-29` (same), `…027_delivery-trace.ts:24-25` (member-derived, FORCE),
`1780310071220_core-identity.ts:76-79` (`app_member_location_ids()` ← `app.user_id` ← memberships),
`1790000000029_idempotency-composite-pk.ts:11` (PK `(location_id, key)`).

---

## Critical

### C1 — Decrement-without-restock leaks stock on terminal paths → **FIX (re-architected, ADR-0007 v2)**

**Verified break:** order INSERTs as `'PENDING'` (`orders.ts:609`), not CONFIRMED; CONFIRMED is a separate
later transition (`orderStatusService.ts:89-94`); the per-minute sweep does a bare `UPDATE … status='CANCELLED'`
with **no restock** (`order-timeout-sweep.ts:67-71`); zero restock logic exists anywhere. Decrement-at-create
(the old ADR-0007) therefore burns a unit on every never-confirmed PENDING order → the whole daily cap is
exhaustible by orders that are never confirmed. The brief's "worst case = 1 portion" is false.

**Decision — Option A: decrement at CONFIRM, not at create.** Weighed:
- **(A) decrement at CONFIRM** — stock burns only on a real, owner-or-auto confirmed order. A PENDING order
  reserves nothing; timeout-CANCEL / owner-REJECT of a PENDING order leak nothing because nothing was taken.
  This is the *correct lifecycle place*: stock is a kitchen-commitment resource, and the commitment is the
  confirm, not the customer's tap. **No restock path needed for the dominant terminal states** (timeout,
  reject-before-confirm) because they never decremented.
- (B) decrement at create + restock-on-every-terminal-non-fulfilled — rejected: requires a *guaranteed*
  idempotent restock on three async paths (sweep, owner-reject, customer-cancel), each a new failure surface;
  a single missed/duplicated restock re-introduces the leak or double-restocks (oversell). More moving parts,
  more invariants, strictly more fragile than (A).
- (C) hybrid (soft-reserve at create, commit at confirm) — rejected for MVP: a reservation TTL is exactly the
  restock-compensation machinery of (B) wearing a different hat; over-engineered for ~30 orders/day.

**Why A composes with auto-confirm (§4 premise):** auto-confirm is just a programmatic CONFIRMED transition
through the SAME guarded UPDATE — the decrement rides that transition identically whether a human or the
auto-confirm timer fires it. The one residual leak path is **CONFIRMED → later REJECTED/CANCELLED** (owner
rejects *after* confirming, or a confirmed order is cancelled). For THOSE — and only those — a **restock
compensation** runs: a guarded `UPDATE products SET stock_remaining = stock_remaining + qty` keyed to the
order's items, executed **once**, idempotently, when an order leaves a post-CONFIRMED state to a terminal
non-fulfilled state. Idempotency is enforced by a `stock_committed boolean` flag on `orders` (decrement sets
it true; restock flips it false and only restocks `WHERE stock_committed = true`) so a re-fired transition
cannot double-restock and a never-confirmed order (flag false) is a restock no-op.

**Proof of no leak on ANY terminal path** (in ADR-0007 v2 race/lifecycle test matrix):
| Terminal path | Decremented? | Restocked? | Net |
|---|---|---|---|
| PENDING → timeout-CANCELLED | no (never confirmed) | n/a (flag false) | 0 leak |
| PENDING → owner-REJECTED | no | n/a | 0 leak |
| CONFIRMED → … → DELIVERED | yes | no (fulfilled) | correct sale |
| CONFIRMED → REJECTED/CANCELLED | yes | yes (once, flag-guarded) | 0 leak |
| double-fired terminal transition | — | guarded by `stock_committed` flip | no double-restock |

**Files updated:** ADR-0007 (re-titled "decrement at CONFIRM + flag-guarded restock"), proposal §2.1 / §1c /
§4.2 / §7. The race test (`orders.stock-race.spec.ts`) is rewritten to fire concurrent CONFIRMs (not creates)
and adds the four lifecycle rows above.

### C2 — Geofence sensor write fails RLS in the courier context, silently swallowed → **FIX (ADR-0009 v2)**

**Verified break:** the ping handler sets `app.current_tenant` (`shifts.ts:337`) and **never** `app.user_id`
(0 hits in the whole courier dir). `app_member_location_ids()` derives from `app.user_id` via `memberships`
(`core-identity.ts:76-79`); couriers are in `couriers`, not `memberships`. So a WITH CHECK on
`app_member_location_ids()` is **empty-set → every INSERT denied**, and the proposed best-effort SAVEPOINT
swallows the denial → zero geofence rows, ever, no error. `courier_positions` writes succeed precisely because
its RLS is `USING (location_id = current_setting('app.current_tenant')::uuid)`
(`courier-positions.ts:22-23`) — the courier idiom, not the member idiom.

**Decision — dual-context RLS policy keyed on BOTH idioms, FORCE.** `order_sensor_events` must be writable in
the courier ping context AND readable by owners/analytics. So its `tenant_isolation` policy USING + WITH CHECK
is the **disjunction** of the two tenant idioms:
```sql
CREATE POLICY tenant_isolation ON order_sensor_events
  USING (
    location_id IN (SELECT app_member_location_ids())
    OR location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid
  )
  WITH CHECK (
    location_id IN (SELECT app_member_location_ids())
    OR location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid
  );
```
Owner/analytics reads land via `app_member_location_ids()` (member idiom, the only one they set); the courier
ping write lands via `app.current_tenant` (the only one IT sets). `NULLIF(…, true)` + the `, true` missing-ok
flag means an unset variable is NULL (no row matches), never an error — so neither context can leak into the
other (an owner never sets `app.current_tenant`; a courier never sets `app.user_id`). FORCE stays on (the API
role is not BYPASSRLS). This is the same disjunction-of-idioms pattern, made explicit because this is the one
table written from both worlds.

Rejected alternative — "set `app.user_id` in the courier handler too": useless, because couriers aren't
members so `app_member_location_ids()` is still empty; it would not satisfy the WITH CHECK.

**DoD now includes a positive-presence test:** after simulating a boundary crossing in the ping handler's exact
context (`app.current_tenant` set, `app.user_id` unset), assert the `order_sensor_events` row **is present**
(not best-effort-swallowed) AND a cross-tenant SELECT returns 0. This is the test that catches the silent loss.

**Files updated:** ADR-0009 §1 (policy), §2 (note the dual-context requirement + the verified `app.user_id`
gap), proposal §3.1 `…071` RLS cell + §4.2 + §5.

---

## High

### H1 — Concurrent same-key replay double-decrements → **FIX (ADR-0007 v2, claim-first idempotency)**

**Verified break:** idem SELECT at `:364`, idem INSERT at `:655` (after the order, near COMMIT). Two
concurrent same-key txns both SELECT 0 rows (neither has committed), both proceed. The composite PK
`(location_id, key)` (`…029:11`) means the *second to COMMIT* errors on the PK — so the "protection" is a
500-crash, not the brief's claimed "short-circuit before decrement."

**Fix:** with the C1 re-architecture the create path no longer decrements (decrement moved to CONFIRM), which
already removes double-*decrement* at create. But to make idempotency itself correct (no double *order*, clean
replay), **claim the key FIRST**: move the `INSERT INTO idempotency_keys … ON CONFLICT (location_id, key) DO
NOTHING RETURNING key` to immediately after the request-hash compute, BEFORE the order INSERT. A concurrent
same-key pair: the first INSERT acquires the unique-index lock and proceeds; the second **blocks on the index**
until the first commits, then its `ON CONFLICT DO NOTHING` returns 0 rows → it does NOT proceed, it re-reads
the now-committed order and returns the stored 200-replay. Exactly one order, exactly one CONFIRM-time
decrement, clean 200 (not 500). The order_id is back-filled into the claimed key row via the same txn (the row
is claimed with a NULL order_id, then `UPDATE … SET order_id` before COMMIT — or claimed with the order_id
once the INSERT returns; ADR-0007 v2 pins the exact two-statement order).

**Files updated:** ADR-0007 v2 §3 (claim-first), proposal §4.1. Race test asserts a concurrent same-key pair →
exactly one 201 + one 200-replay (never a 500), exactly one decrement at confirm.

### H2 — "Migration-free" omits the intermediate/batch node → **FIX (ADR-0008 v2, honest re-scope)**

**Verified break:** ADR-0008 line 84 swears "no parent_id remap." But the FLAT MVP runtime authors **only
direct product→raw-ingredient** rows (no UI/runtime for intermediates). Activating a batch node later forces
re-pointing existing `recipe_components` rows from raw ingredients to the new intermediate — a row remap, the
exact ret-migration the brief fears.

**Disposition — honest re-scope, not a false universal claim.** The migration-free property is **TRUE for the
direct→derived reader swap** (manual `min(stock)` → `min(stock, floor(ingredient/qty))` over the *same* rows)
and is the load-bearing MVP claim. It is **NOT** claimed for *introducing a NEW intermediate node into an
existing flat recipe* — that is a deliberate, owner-initiated **re-modelling** (the owner decides "this sauce
is now a batch node"), and ADR-0008 v2 documents it as **the one backfill the seam needs**, with the exact
shape: insert the `ingredients(kind='intermediate')` row + re-point the affected products' `recipe_components`
+ author the intermediate's child recipe — an owner-driven data edit through the future BOM UI, NOT a schema
migration and NOT a silent surprise. The brief's "migration-free upgrade" = "manual→derived **reader** swap is
free"; it never promised "every future topology change is free," and ADR-0008 v2 says so in those words.

**Files updated:** ADR-0008 v2 "Proof" section (split the claim: reader-swap = free [proven]; node-introduction
= a named, owner-driven backfill [honestly documented]), proposal §2.2 + §7.

### H3 — No FK / no cascade on `parent_id` → orphan recipe rows on product delete → **FIX (ADR-0008 v2)**

**Verified break:** `parent_id` is bare `uuid NOT NULL` (no FK); the write-assertion guards INSERT, not the
parent's DELETE. Deleting a product orphans its `recipe_components` rows.

**Fix — a real referential guard for the un-FK'd polymorphic parent:** an `AFTER DELETE` trigger on `products`
(and on `ingredients`) that deletes the matching `recipe_components` rows
(`WHERE parent_kind='product' AND parent_id = OLD.id`), giving cascade semantics the polymorphic column can't
declare natively. This is a real DB guarantee on the delete side, mirroring what a native `ON DELETE CASCADE`
FK would do — the seam no longer "gives that up." It is inert-safe (zero rows at MVP) and lands in the same
seam migration. (Soft-delete: products are soft-deleted in the menu manager — the trigger is the hard-delete
backstop; the future derived reader additionally filters on the product's live state.)

**Files updated:** ADR-0008 v2 "Integrity of the missing FK" (add the AFTER DELETE trigger as guard #4 covering
the delete side), proposal §3.2 `…073` + §7.

### H4 — Unauthenticated, unthrottled funnel ingest floods + poisons the padding-creep brake → **FIX (proposal §4.3) + accept-risk (residual)**

**Verified context:** the order velocity throttle (`orders.ts:250-261`) guards order-create, NOT funnel; no
funnel rate-limit exists. The funnel feeds the §8.2 padding-creep counter-metric → a poisonable control input.

**Fix:**
1. **Per-IP rate-limit on the funnel ingest endpoint** (reuse the repo's existing rate-limiter; cap e.g.
   ~60 events/min/IP — generous for a real session, lethal to a flood). Still returns uniform 200/204 (anti-
   enumeration preserved) but drops over-cap events server-side.
2. **Bound the table:** the §6 90-day sweep DELETEs in **batches** (`LIMIT … ` loop) so it can't lock-contend
   with live writes; add `funnel_events(created_at)` index (already in `…070`).
3. **De-poison the signal:** the padding-creep counter-metric MUST compute abandon-rate over **distinct
   `session_ref`** (not raw row count) and is **advisory** input to a human/loop, never a direct autopilot
   actuator (brief §8.2 already says variability moves on it — ADR-0009 v2 names it advisory-with-human-review).
   Aggregating per-session blunts a single-IP flood (one session_ref ≈ one vote).

**Residual accept-risk:** a *distributed* botnet minting distinct session_refs across many IPs can still skew
the signal at scale. Owner: **Ops**. Justification: at cold-start volume (~30 orders/day) this is not the MVP
threat; the per-IP cap + per-session aggregation + human-review-before-actuation make it non-load-bearing for
the *seam*; revisit (proof-of-work / signed session_ref) only if the funnel ever directly actuates the loop.

**Files updated:** proposal §4.2/§4.3 (rate-limit + batch sweep), ADR-0009 v2 §4 (signal-norm: per-session,
advisory), proposal §7 (residual risk row).

### H5 — Set-once trigger on the hot orders UPDATE path: unmeasured overhead + privileged-write bypass → **FIX (proposal §1c) + accept-risk (privileged bypass)**

**Verified context:** the trigger is `BEFORE UPDATE … FOR EACH ROW` on `orders` — fires on every status flip
and the bulk sweep UPDATE (`order-timeout-sweep.ts:67`). Proposal §1c never measured it.

**Fix:**
1. **Gate the trigger body on the column being touched:** the plpgsql is a single `IF OLD.promised_window_lo_min
   IS NOT NULL AND NEW.* IS DISTINCT FROM OLD.* THEN RAISE` — a few comparisons, **no query**, so per-row cost
   is a handful of CPU instructions (sub-microsecond), not a round-trip. §1c now states this explicitly and
   notes the sweep's bulk UPDATE never touches the window columns so the `IS DISTINCT FROM` is always false
   (fast path). This is measured-by-reasoning (no I/O in the trigger) and pinned as a DoD micro-assertion
   (EXPLAIN/timing on a bulk sweep UPDATE shows no material delta).
2. **Privileged-write bypass — accepted, documented asymmetry.** A migration/superuser write (or
   `session_replication_role='replica'`) bypasses the BEFORE-UPDATE trigger. Owner: **Architect**.
   Justification: this is the *intended* escape hatch — the only legitimate correction of a mis-set immutable
   window is a deliberate, logged, privileged migration (consistent with ESTOP-1's "(b) human decision, written
   down"). App writes cannot bypass it; the immutability is hard for the app and intentionally overridable by a
   recorded human/migration. Proposal §7 + ADR-0009 v2 state the asymmetry instead of overselling "hard
   invariant."

**Files updated:** proposal §1c (trigger cost) + §7 (bypass asymmetry), ADR-0009 v2 §3.

---

## Medium

### M1 — Naive AVG over partial NULL timestamp set → biased sensor data day 1 → **FIX (ADR-0009 v2, NULL-handling contract)**

**Disposition:** specify the reconstruction NULL-contract so the literal-minimum implementer can't ship a
biased AVG. ADR-0009 v2 §"Reconstructable durations" states: every duration metric is computed **only over
orders that actually have BOTH endpoints non-NULL**, segmented by fulfilment type (delivery-by-courier vs
pickup vs cancelled-mid-flight kept separate, never pooled), and the dwell metric is explicitly **conditional
on a geofence row existing** (post-C2-fix, that's the courier-delivery subset only). The metric layer reports
**n (sample size) alongside each AVG** so a partial-population bias is visible, never silent. This is the
measurement-bias the brief §8.1 exists to fix; we fix it at the reconstruction contract, not just the schema.
**Files:** ADR-0009 v2 + proposal §5 (durations contract row).

### M2 — `session_ref` re-identifiable via time-correlation → **FIX (proposal §4.3) + Counsel #5**

**Disposition:** assert and design-in unlinkability. ADR-0009 v2 + proposal §4.3: (a) `session_ref` is a
client-minted opaque id that is **never written onto an order row** and **never logged on the order-create
path** (grep-gate: no `session_ref` reference in `orders.ts`); (b) the FE **rotates** the session_ref at order
submission (a new id post-order) so the pre-order funnel session and the order are not the same token; (c) the
unlinkability claim goes in the `/compliance` SoT + storefront privacy notice (Counsel #5). Residual
time-correlation (an analyst joining by `created_at` within seconds) is acknowledged as a weak attack requiring
DB-admin access to both tables; accepted, owner **Ops**, because the funnel carries no identity column to join
*to* — the correlation is timing-only and the privacy notice discloses aggregate funnel analytics.
**Files:** proposal §4.3, ADR-0009 v2, `/compliance` follow-up flagged.

### M3 — Geofence ON CONFLICT must be pinned (proposal §4.2 vs ADR-0009 §1 disagree) → **FIX**

**Disposition:** pin it in BOTH docs. The geofence INSERT is `INSERT … ON CONFLICT (order_id, event_type) DO
NOTHING` (the UNIQUE in `…071`), full stop — proposal §4.2 now repeats the ON CONFLICT clause verbatim so the
two documents no longer disagree, and the DoD asserts a second crossing is a no-op (exactly one row).
**Files:** proposal §4.2, ADR-0009 v2 §1 (already states it; now cross-referenced).

### M4 — Multi-row decrement lock-ordering → deadlock → **FIX (ADR-0007 v2)**

**Disposition:** decrement (now at CONFIRM) applies the per-product `UPDATE` in a **deterministic order sorted
by product_id**, so two overlapping multi-item orders can never acquire row locks in opposing order → no
40P01 deadlock. ADR-0007 v2 §2 states "iterate distinct products `ORDER BY product_id` before the guarded
UPDATE." DoD adds a two-product cross-order concurrency test asserting no deadlock.
**Files:** ADR-0007 v2 §2.

### M5 — Cited velocity throttle is phone-only, not phone+IP → **FIX (proposal §4.3)**

**Verified:** the gate is `if (phoneHash) { … }` only (`orders.ts:250-261`); `clientIpHash` is computed at
`:247` but never gates. A phone-rotating attacker is ungated.

**Disposition:** with C1 (decrement at CONFIRM) the create-time DoS-on-availability is **already neutralised**
— a flood of never-confirmed PENDING orders now decrements nothing, so the stock-burn the throttle was cited to
bound no longer exists at create. The throttle's remaining job is bounding PENDING-order spam (owner dashboard
noise + the one-tap-abort load), not stock. We still **add the IP half** (a parallel `if (clientIpHash)` gate
mirroring the phone block, using the already-computed `clientIpHash`) so phone-rotation is bounded by IP. This
is the brief's stated "velocity-ліміти (phone+IP)" finally made whole. Owner: **Architect**.
**Files:** proposal §4.3 (add IP gate), §7 (the C1+IP combination closes the "1 portion" claim honestly).

---

## Low

### L1 — Ingredient cycle enterable now; future tree-walk has no guard → **defer-flag (MISSING, guard must validate pre-existing rows)**

**Disposition:** the cycle data is enterable today (the seam allows `parent_kind='ingredient'`), the recursive
reader is out of scope. We **defer the cycle guard to the North-Star derived-reader phase** but record the
hard requirement (MISSING until then): the future `available_units()` reader MUST carry a depth-cap +
visited-set memo AND a one-time validation pass that rejects/flags pre-existing cycles (not just new INSERTs),
because data can be authored before the guard exists. ADR-0008 v2 records this as an explicit deferred-flag,
not a silent gap. Owner: **North-Star phase lead**.
**Files:** ADR-0008 v2 "Deferred (North-Star)" note, proposal §7.

### L2 — range-never-point collapses to a point at eta_cap clamp / lo==hi → **FIX (Counsel #1, ADR-0009 v2)**

**Disposition:** add the value-level floor (this IS Counsel non-blocking #1, the missing half of range-never-
point). `CHECK (promised_window_hi_min > promised_window_lo_min)` is too strict (a legit 5–10 is fine but the
clamp must not collapse to a point). Instead: a `locations.min_window_width_min int NOT NULL DEFAULT 5` floor;
the synthesis helper enforces `hi := max(hi, lo + min_window_width_min)` AFTER the `eta_cap` ceiling clamp, and
a `CHECK (promised_window_lo_min IS NULL OR promised_window_hi_min >= promised_window_lo_min + 1)` rejects a
literal point at the DB. The client Zod response schema rejects `lo == hi`. range-never-point is now enforced
at value-level + render-level, not just schema-shape.
**Files:** ADR-0009 v2 §4 (width floor), proposal §3.1 `…069` (`min_window_width_min`) + §5.

### L3 — delivery_trace baseline idempotency citation unverified → **FIX (re-verified) / accept**

**Verified:** `delivery_trace` IS `UNIQUE(order_id)` + `ON DELETE CASCADE` and the migration comment states the
DELIVERED handler writes it "ON CONFLICT DO NOTHING → idempotent/recoverable" (`…027:5-6,12`). The §1.2
baseline columns (`route_distance_m`, `expected_delivery_min`) are ADDED to this same row, so the baseline
write must be folded into the **same** ON-CONFLICT upsert at DELIVERED (`INSERT … ON CONFLICT (order_id) DO
UPDATE SET route_distance_m = EXCLUDED…, expected_delivery_min = EXCLUDED…` — or a guarded UPDATE on the
existing row). Proposal §4.1 now pins the baseline write as part of the existing idempotent DELIVERED upsert,
not a separate plain INSERT. Owner: **Architect**.
**Files:** proposal §4.1.

---

## Counsel ETHICAL-STOP

### ESTOP-1 — Immutable promised_window vs client honesty → **FIX (adopt the split — Counsel option (a))**

**Decision:** adopt **Counsel's split** — separate the two concepts the set-once trigger conflated. The
append-only-window-log steel-man (Counsel §4) is genuinely more elegant for the measurement *trajectory*, but
the **two-column split is the cheaper, lower-blast-radius MVP cut** that delivers BOTH ethics (frozen promise
+ live customer truth) without a new table on the hot read path. We take the split now and **record the
append-only log as the North-Star upgrade** (it subsumes the frozen column: the first row of the log IS the
frozen promise). Concretely:

1. **`orders.promised_window_{lo,hi}_min`** = the **promise-as-made at confirm**, frozen by the set-once
   trigger (measurement ground truth for §8). Unchanged from the original design.
2. **`orders.live_eta_{lo,hi}_min`** = the **live current estimate** the customer sees collapse through stages
   (§2.4 confirmed→cooking→picked_up→arriving). **MUTABLE** — explicitly NOT covered by the set-once trigger.
   This is the client truth channel. ADR-0009 v2 names it so §2.4's collapsing window no longer silently
   collides with §1.1's immutability on the same field.

The customer page reads `live_eta_*` (the truth as it evolves); the §8 metric reads `promised_window_*` (the
frozen first promise). A mis-set promise still shows a wrong *frozen* number in the historical record, but the
customer always sees the *current* truth via `live_eta_*` — the party range-never-point protects is no longer
frozen into a lie. The residual "historical record shows a wrong first promise" is the *correct* behaviour for
measurement (it catches the mis-set as a falsification data point) and is overridable only by a recorded
privileged migration (H5 asymmetry). **This dissolves the tension rather than choosing a side**, exactly as
Counsel recommended; no human-needed disposition remains for ESTOP-1.

**Files updated:** ADR-0009 v2 §3 (two columns, named live channel, trigger scoped to the frozen pair only),
proposal §2.3 + §5 (live channel named, §2.4 wiring), §7 (append-only log = recorded North-Star upgrade).

---

## Counsel non-blocking advice

| # | Advice | Disposition |
|---|---|---|
| 1 | window-width LOWER bound (forbid "1–2 min") | **FIX** — see L2: `min_window_width_min` floor + DB CHECK + Zod. The other half of range-never-point. |
| 2 | courier-metric norm written BEFORE the metric ships (normalized not raw; advisory not de-facto deactivation) | **FIX (norm written) / defer-flag (metric itself)** — the courier normalized-time metric is NOT in this batch (it's §8.3 North-Star). ADR-0009 v2 records the binding norm NOW so it can't ship without it: *the rating surfaces the normalized number only, is owner-advisory, and MUST NOT be the basis of an automated deactivation.* Owner: **North-Star phase lead.** |
| 3 | §2.1 dispatch nudge is courier-advisory, NOT an owner-visible compliance signal | **FIX** — proposal §4.3 / §5 state the nudge is courier-facing advisory; non-compliance is NOT recorded as an owner-visible signal. "Courier owns the moment" made true in lived experience, not just code. |
| 4 | OUT_OF_STOCK carries a cause-hint | **FIX** — ADR-0007 v2 already returns `{ code:'OUT_OF_STOCK', error:'Product <name> is out of stock' }` (humane cause, not a bare 422). Pinned as a DoD assertion (the customer sees the product name + reason). |
| 5 | funnel in the privacy notice + session_ref unlinkable to identity | **FIX** — see M2: unlinkability designed-in (no session_ref on orders, rotation at submit) + disclosed in `/compliance` SoT + storefront privacy notice. |

## Counsel open question (§5) — who decides WHERE in the honest band the promise sits; nothing measures the client cost of an OTP-biased band

**Disposition: human-needed (recorded, carried to North-Star autopilot design — NOT a seam blocker).** This is
a genuine power-allocation question (the owner holds the conservativeness knob; the funnel measures only the
*venue's* lost-cart cost, never the *customer's* late-arrival cost inside an "honest" band). It does not block
the seam/schema work. Recorded as an **open question with an owner (Product + North-Star lead)** in proposal
§7: before the autopilot loops make the asymmetry self-reinforcing, the design must add a customer-side cost
signal (e.g. late-within-band rate) so the band isn't centered solely on the venue's OTP target. Flagged for a
human decision at autopilot-design time; nothing to decide for THIS batch.

---

## Summary of dispositions

| Finding | Severity | Disposition | Where |
|---|---|---|---|
| C1 | CRIT | **FIX** — decrement at CONFIRM (Opt A) + flag-guarded restock on post-confirm terminal states | ADR-0007 v2, proposal §2.1/§4.2 |
| C2 | CRIT | **FIX** — dual-idiom RLS (`app_member_location_ids()` OR `app.current_tenant`), FORCE; presence-test in DoD | ADR-0009 v2, proposal §3.1/§4.2 |
| H1 | HIGH | **FIX** — claim idempotency key FIRST (ON CONFLICT DO NOTHING) before any write; clean 200-replay | ADR-0007 v2, proposal §4.1 |
| H2 | HIGH | **FIX** — honest re-scope: reader-swap migration-free [proven]; node-introduction = one named owner-driven backfill | ADR-0008 v2, proposal §2.2 |
| H3 | HIGH | **FIX** — AFTER DELETE trigger on products/ingredients deletes orphan recipe_components (cascade the polymorphic col can't) | ADR-0008 v2, proposal §3.2 |
| H4 | HIGH | **FIX** (per-IP rate-limit + batched sweep + per-session advisory signal) + **accept-risk** (distributed botnet, Ops) | proposal §4.2/§4.3, ADR-0009 v2 |
| H5 | HIGH | **FIX** (trigger body is I/O-free, cost pinned) + **accept-risk** (privileged-write bypass is the intended escape hatch, Architect) | proposal §1c/§7, ADR-0009 v2 |
| M1 | MED | **FIX** — NULL-handling contract: both-endpoints-non-NULL, segmented by fulfilment, report n, dwell conditional on geofence row | ADR-0009 v2, proposal §5 |
| M2 | MED | **FIX** — session_ref never on orders + rotate at submit + disclosed; residual time-correlation accept-risk (Ops) | proposal §4.3, ADR-0009 v2 |
| M3 | MED | **FIX** — pin ON CONFLICT (order_id,event_type) DO NOTHING in both docs | proposal §4.2, ADR-0009 v2 |
| M4 | MED | **FIX** — decrement UPDATEs ordered by product_id (deadlock-free) | ADR-0007 v2 |
| M5 | MED | **FIX** — add IP-half gate; C1 already neutralises the create-time stock DoS | proposal §4.3 |
| L1 | LOW | **defer-flag (MISSING)** — cycle guard deferred to North-Star reader; must validate pre-existing rows (owner: NS lead) | ADR-0008 v2, proposal §7 |
| L2 | LOW | **FIX** — min_window_width floor + DB CHECK + Zod (Counsel #1, the other half of range-never-point) | ADR-0009 v2, proposal §3.1/§5 |
| L3 | LOW | **FIX** — baseline folded into the existing idempotent DELIVERED upsert (re-verified UNIQUE+ON CONFLICT) | proposal §4.1 |
| ESTOP-1 | E-STOP | **FIX** — adopt the split: frozen `promised_window_*` + mutable `live_eta_*`; append-only log = recorded NS upgrade | ADR-0009 v2 §3, proposal §2.3/§5 |
| Counsel #1–5 | advisory | **FIX** (1,3,4,5) / **FIX-norm + defer metric** (2) | proposal + ADR-0009 v2 |
| Counsel open-Q §5 | advisory | **human-needed** — recorded open question, carried to autopilot design (owner: Product + NS lead); not a seam blocker | proposal §7 |

## Still needing a human decision

1. **Counsel open question §5** — where inside the honest band the promise sits, and the missing *customer-side*
   cost signal (late-within-band rate) to balance the owner's OTP knob. Recorded; to be decided at
   North-Star autopilot-design time, before the loops self-reinforce. Owner: Product + North-Star lead. **This
   is the only open human-decision; it does not block the MVP seam/schema batch.**

Everything else is resolved to fix / accept-risk(+owner) / defer-flag(MISSING). No finding is marked "resolved"
without a verified source basis above.

---

# RESOLVE round 2 — disposition of the NEW (regression-pass) findings

> Scope: the NEW findings introduced by the v2 fixes — Breaker "RE-ATTACK round 2" (R2-C1/R2-H1/R2-H2/R2-M1/
> R2-M2) + Counsel "RE-EXAMINE round 2" non-blocking notes (R2-a live-channel rejection, R2.1 width-floor on
> live recompute, R2.3 late-within-band metric). Every disposition verified against live source this round.
> Companion: `proposal.md` (v3), ADR-0007/0008/0009 **v3**.

Verified source re-touched this round: `apps/api/src/routes/customer/orders.ts:239-335` (the raw-UPDATE
post-dispatch cancel — `:289-293`, bypasses `updateOrderStatus`), `apps/api/src/lib/orderStatusService.ts:85-181`
(the guarded path + per-stage `*_at` stamp at `:106-116`), `apps/api/src/workers/order-timeout-sweep.ts:67-71`
(`WHERE status='PENDING'` only — never stock-committed), `apps/api/src/routes/owner/dashboard.ts:260`
(`status='READY'` reassign — not terminal), `apps/api/src/routes/owner/signals.ts:230` (`status_notes`, not
`status`), `apps/api/src/routes/courier/shifts.ts:336-378` (sets `app.current_tenant`; reads the courier's own
active assignment at `:365-369`), `apps/api/src/routes/orders.ts:364-381` (idem replay SELECT + the `DELETE …
WHERE id=NULL` fallthrough). Full raw-status-writer grep (`status\s*=\s*'(CANCELLED|REJECTED)'`) enumerated:
sweep (PENDING-only), service (the home), customer-cancel (the leak), reconciliation (read-only count).

## NEW CRITICAL

### R2-C1 — Restock leak via the raw customer-cancel path → **FIX (DB trigger — restock made UNBYPASSABLE)**

**Verified break (confirmed, not hypothetical):** `customer/orders.ts:289-293` is the customer-facing
`POST /orders/:orderId/cancel` (`:239`), gated on `order.status === 'IN_DELIVERY'` (`:275`) + a 5-min
post-dispatch window (`:283`), which flips status with a **raw `client.query` UPDATE** —
`UPDATE orders SET status='CANCELLED', cancelled_at=now(), cancellation_reason=$1 WHERE id=$2` — and **never
calls `updateOrderStatus`**. An IN_DELIVERY order is unambiguously post-CONFIRMED, so by ADR-0007 v2
`stock_committed=true` and a unit was decremented at confirm. The v2 service-method restock is by construction
blind to this path → 5 ordinary customer cancels burn 5 units = the identical permanent leak C1 describes, now
on a shipped customer feature. The v2 C1 "no leak on ANY terminal path" matrix is green on the audited writers
(`orderStatusService`, owner-reject) and **red on the un-audited raw one** — C1 was PARTIALLY closed.

**Full writer map verified (so the fix is provably complete):** five `UPDATE orders SET status` writers exist;
only `customer/orders.ts:289` hits a post-confirmed terminal transition. The sweep is `WHERE status='PENDING'`
(never stock-committed); `owner/dashboard.ts:260` sets `'READY'` (not terminal); `owner/signals.ts:230` sets
`status_notes` not `status`; `orderStatusService` is the home. So the leak is exactly one raw writer today —
but a service-method restock is blind to it AND to any future raw writer.

**Decision — Option A: a DB trigger, not "force all cancels through `updateOrderStatus`".** Weighed both as
instructed:
- **(A, chosen) `BEFORE UPDATE OF status` trigger `orders_restock_on_terminal` on `orders`** that, when
  `OLD.stock_committed=true AND NEW.status IN ('CANCELLED','REJECTED') AND OLD.status IS DISTINCT FROM
  NEW.status`, restocks `order_items` (sorted by product_id, NULL-stock no-op) and flips
  `NEW.stock_committed:=false` in the **same row write**. **A trigger cannot be bypassed by a raw UPDATE the
  way a service method can** — it fires regardless of which of the five (or any future) writer issues the
  status flip. Robust against the customer-cancel raw UPDATE and every future raw writer; the invariant lives
  where the data lives. Hot-path-safe: `BEFORE UPDATE OF status` means a pure timestamp/notes UPDATE never
  enters the body, and `stock_committed=false` short-circuits with no query.
- (B, rejected) refactor every raw status writer through `updateOrderStatus` — requires finding+rewriting
  every raw `UPDATE orders SET status` now AND forever; the customer-cancel txn also runs courier-assignment +
  shift-reset + cash-reversal (`customer/orders.ts:295-317`) that doesn't belong in the generic status
  service; one missed/future writer re-opens the leak. Strictly more fragile than a DB-enforced guard.

**Proof NO terminal path from a stock-committed order leaks:** the trigger fires on every `status` flip to
CANCELLED/REJECTED of a `stock_committed=true` row, independent of the writer — so the raw customer-cancel,
the owner-reject, and any future raw UPDATE all restock identically. DELIVERED is excluded (fulfilled);
PENDING/never-confirmed has `stock_committed=false` (no-op); double-fire is idempotent (the flag flips in the
same row write + `OLD.status IS DISTINCT FROM NEW.status`).

**Anti-cheat-green DoD (mandatory, per instruction):** the restock test MUST exercise the **leaking route** —
drive an order to IN_DELIVERY then hit `POST /orders/:orderId/cancel` (the raw path) and assert
`stock_remaining` is restored + `stock_committed=false`. A green test that only calls `updateOrderStatus` is a
cheat-green and does not cover the bypass. Pinned as ADR-0007 v3 DoD #3.

**Files updated:** ADR-0007 v3 §3 (DB-trigger restock + writer map + weighed alternatives) + DoD #3/#9,
proposal §1c + §3.1 `…066` + §7 (R2-C1 row).

## NEW HIGH

### R2-H1 — Intra-tenant order_id forgery on sensor events → **FIX (order-assignment scope on the write)**

**Verified break:** the C2 dual-context WITH CHECK validates `location_id` only; it never ties `order_id` to
the writing courier's assignment. A courier on shift at venue X (legitimate `app.current_tenant=X`) could
`INSERT order_sensor_events(order_id=O2, location_id=X, 'courier_geofence_enter')` for a colleague's order O2
at the same venue → forged dwell/road sensor data. Cross-tenant is bounded (JWT-pinned tenant); intra-tenant
courier-vs-courier is the concrete break (HIGH, not CRITICAL). This mirrors `courier_positions`' weakest
property (tenant-only) while carrying an `order_id` positions never had.

**Decision — FIX, not accept-risk.** I considered the accept-risk framing (geofence dwell is owner-ADVISORY,
intra-tenant forgery is low-value/detectable). **Rejected** because the brief §8 falsification fuel must stay
trustworthy at the *capture* layer — a poisonable sensor feeds the P1/P2/P7 falsification harness and the
North-Star normalized-courier metric (ADR-0009 §4c); a courier manufacturing another's performance data is
exactly the "falsification fuel staying trustworthy" the proposal exists to protect, and the fix is cheap
because **the write path already knows the courier's own assignment** (`shifts.ts:365-369` reads
`courier_assignments WHERE courier_id=<self>` for the GPS-consent gate). v3 extends that read to return the
assigned `order_id` and uses **that** for the geofence INSERT — never an order_id from the ping payload (the
ping body is `{lat,lng,accuracy_meters}` only; v3 forbids adding an order_id field). A courier can therefore
only ever stamp the order they are actually delivering = v1's intent restored. App-layer ownership enforcement
where the assignment is already in hand (cheaper than a `courier_assignments` join inside the WITH CHECK; the
RLS-subquery variant is documented as defence-in-depth if a future writer ever takes an external order_id).

**DoD:** order-assignment-scope test — courier C (assignment O1) cannot stamp colleague's O2 at the same venue
(the handler derives order_id from C's own assignment; no forged-order_id path exists). The existing
cross-tenant SELECT test does NOT cover this — the new test does.

**Files updated:** ADR-0009 v3 §2a (order-assignment scope) + DoD (scope test), proposal §4.2 + §5 (geofence
invariant) + §7 (R2-H1 row).

### R2-H2 — Claim-first idempotency txn-semantics unpinned (crash-poison / replay-body) → **FIX (single-txn `state` lifecycle)**

**Verified break:** v2 §4 hedged the claim's txn placement ("claimed with a NULL order_id, then `UPDATE … SET
order_id` before COMMIT — or claimed with the order_id once the INSERT returns"). R2-H2 showed: same-txn claim
gives no concurrency benefit AND separate-txn claim crash-poisons the key (a committed `order_id=NULL` claim
the existing replay at `orders.ts:375-381` can only resolve by `DELETE WHERE id=NULL` → two racing retries both
delete + both create = double order); and the replay-body contract (return the prior order body) is unspecified
in the restructure.

**Decision — pin ONE mechanism: single-txn claim + a `state {claimed→completed}` column.** This definitively
resolves the "mutually exclusive" objection: the claim, order INSERT, and `state='completed'` back-fill all
commit in ONE txn; the composite-PK unique index (`1790000000029:11`) is the serialization point — a concurrent
peer's `ON CONFLICT DO NOTHING` **blocks on the index lock until the owner txn commits**, then sees a
`completed` key with a valid `order_id` and re-enters the existing replay SELECT (`orders.ts:375-378`) → returns
the **full order body** (R2-H2 part 3 closed, not a bare 200). Prove the three required properties:
- **Exactly-one decrement under concurrency:** index-serialized claim + decrement-at-confirm (one winner per
  key; the loser replays).
- **No permanently-poisoned key on crash:** single-txn rollback removes the claim entirely (normally no
  surviving `claimed` row); an orphaned `claimed` row (separate-connection pathology) is reclaimable via a
  **guarded** `DELETE … WHERE state='claimed' AND claimed_at < threshold RETURNING` (exactly one retry wins the
  delete; the other sees 0 rows and re-reads) — no double-create, no forever-poisoned key.
- **Correct replay body:** `state='completed'` → return the prior order body; `state='claimed'` recent → 409
  IN_FLIGHT (client retries); the v2 unconditional `DELETE WHERE id=NULL` (the double-create vector) is
  removed.

H1's original double-*decrement* was already closed by decrement-moving-to-confirm; R2-H2 closes the residual
idempotency-correctness regression.

**Files updated:** ADR-0007 v3 §4 (single-txn `state` lifecycle + the replay state-machine) + DoD #4/#5,
proposal §3.1 `…066b` (the `state`/`claimed_at` ALTER) + §4.1 + §7 (R2-H2 row).

## NEW MEDIUM

### R2-M1 — `live_eta_*` split is schema-only (no writer) → **FIX (writer specified; width-floor on every recompute)**

**Verified break:** v2 named a mutable `live_eta_*` column but specified **no writer** beyond the confirm-seed
→ `live_eta == promised_window` forever → the customer reads the frozen promise from a different column and the
ESTOP-1 fix is cosmetic. §2.4 (the collapsing window) IS in this batch, so the writer must be specified now.

**Decision — FIX, make the customer truth channel actually live.** v3 §3a specifies the writer: `live_eta_*`
is recomputed at each stage transition, **co-located with the existing per-transition `*_at` stamp in
`orderStatusService.ts:106-116`** (no new worker, no new hot path) — PREPARING→remaining prep+travel;
READY→travel only; IN_DELIVERY/PICKED_UP→remaining travel off the latest ping; geofence→arriving band — all via
the **same synthesis helper** as the promised window, so it inherits the cap and floor. **The width-floor
`min_window_width_min` applies to EVERY recompute (Counsel R2.1), not just the initial synthesis** — the helper
does `hi := min(hi, eta_cap)` then `hi := max(hi, lo + min_window_width_min)`, so the band can never narrow
below the honest floor at the arriving stage (where pseudo-precision "1–2 min" is most tempting). The recompute
is best-effort within the transition (a failure degrades to the prior live band, never back to the frozen first
promise). ESTOP-1 is therefore RESOLVED in behaviour, not merely schema.

**Files updated:** ADR-0009 v3 §3a (the writer + width-floor on recompute) + DoD (live-eta-writer test),
proposal §2.3 + §7 (R2-M1 row).

### R2-M2 — AFTER-DELETE FOR-EACH-ROW trigger misses TRUNCATE → **FIX (statement-level companion) + scope honesty**

**Verified break:** Postgres `FOR EACH ROW` DELETE triggers do **not** fire on `TRUNCATE` (only statement-level
`AFTER TRUNCATE` triggers do). So v2's "exactly like a native `ON DELETE CASCADE` FK" was FALSE for the
TRUNCATE path — a `TRUNCATE products CASCADE` (a common test-data/tenant-purge shortcut; the repo runs bulk
test-data ops) would orphan `recipe_components` exactly as the original H3.

**Decision — FIX (statement-level companion trigger), and stop over-claiming FK-equivalence.** Considered
accept-risk (TRUNCATE is admin-only, not a runtime path) but a statement-level trigger is cheap and the repo
demonstrably runs bulk TRUNCATEs, so I closed it rather than accepting it. v3 adds AFTER TRUNCATE
`FOR EACH STATEMENT` triggers on products/ingredients deleting the matching `recipe_components`. The ADR now
states the **honest scope**: trigger-based RI is functionally equivalent to `ON DELETE CASCADE` for the DELETE
*and* TRUNCATE paths, but a declared FK additionally gives planner metadata + rejects orphan inserts; the
**soft-delete-without-hard-purge** path leaves recipes in place **by design** (a soft-deleted product is
restorable; the future derived reader filters on live state), so that is not an orphan-leak. The blanket
"exactly like a native FK" claim is removed.

**Files updated:** ADR-0008 v3 (TRUNCATE companion trigger + scope honesty) + DoD (orphan-TRUNCATE test),
proposal §3.2 `…073` + §7 (R2-M2 row).

## Counsel R2 non-blocking notes

| # | Note | Disposition |
|---|---|---|
| R2-a | Confirm-time stock rejection must surface on the LIVE order-view the customer is watching, not only a swallowable 422 | **FIX** — ADR-0009 v3 §3b: a CONFIRMED→rejected-for-stock event rides the same live order channel (`orderStatusService.ts:151` `messageBus.publish(orderChannel(orderId), …)`) with the humane `{code:'OUT_OF_STOCK', product:'<name>'}` cause-hint. proposal §2.3. |
| R2.1 | Width-floor (`min_window_width_min`) applies to EVERY `live_eta_*` recompute near delivery, not just the initial synthesis | **FIX** — folded into R2-M1: the same synthesis helper runs on every recompute, floor after cap. ADR-0009 v3 §3a. |
| R2.3 | NAME the **late-within-band** customer-cost metric NOW (collection only; centering decision deferred) so the autopilot can't be built OTP-skewed before the signal exists | **FIX (measurement built now) / defer (decision)** — ADR-0009 v3 §4b names `late_within_band_rate = delivered_at > promised_window_hi / live_eta_hi`, derivable from laid columns with the same NULL-contract (no new seam). The *centering decision* stays deferred to autopilot-design (open-Q §5). The two costs now reach autopilot-design as peers. proposal §5. |

## Round-2 dispositions summary

| Finding | Severity | Disposition | Where |
|---|---|---|---|
| R2-C1 | CRIT | **FIX** — restock as an UNBYPASSABLE `BEFORE UPDATE OF status` DB trigger; anti-cheat-green test drives the raw customer-cancel route | ADR-0007 v3 §3, proposal §1c/§3.1/§7 |
| R2-H1 | HIGH | **FIX** — geofence `order_id` derived from the courier's OWN assignment server-side, never payload; not accept-risk (falsification fuel must stay trustworthy) | ADR-0009 v3 §2a, proposal §4.2/§5/§7 |
| R2-H2 | HIGH | **FIX** — single-txn claim + `state {claimed→completed}`; index-serialized, crash-recoverable, full-body replay; v2 `DELETE WHERE id=NULL` double-create vector removed | ADR-0007 v3 §4, proposal §3.1/§4.1/§7 |
| R2-M1 | MED | **FIX** — `live_eta_*` writer specified (per-stage recompute via the synthesis helper), width-floor on every recompute; ESTOP-1 resolved in behaviour | ADR-0009 v3 §3a, proposal §2.3/§7 |
| R2-M2 | MED | **FIX** — AFTER TRUNCATE statement-level companion trigger; FK-equivalence claim narrowed to DELETE+TRUNCATE | ADR-0008 v3, proposal §3.2/§7 |
| Counsel R2-a | non-blocking | **FIX** — confirm-time rejection on the live channel with cause-hint | ADR-0009 v3 §3b |
| Counsel R2.1 | non-blocking | **FIX** — width-floor on every live recompute | ADR-0009 v3 §3a |
| Counsel R2.3 | non-blocking | **FIX (collect) / defer (decide)** — late-within-band metric named now | ADR-0009 v3 §4b |

## Round-2 — still needing a human decision

**None new.** The only standing human-needed item remains Counsel open-Q §5 (where inside the honest band the
promise sits — the *centering decision*), now sharpened so the *measurement* (late-within-band) is built now
and only the *decision* is deferred to autopilot-design (owner: Product + North-Star lead). It does not block
this seam batch.

## Round-2 verdict — ALL CRITICAL/HIGH resolved

- **R2-C1 (CRITICAL)** → FIX (DB-trigger restock, unbypassable, anti-cheat-green DoD). Verified the full
  raw-status-writer map: the trigger covers every one.
- **R2-H1 (HIGH)** → FIX (order-assignment scope, server-derived order_id).
- **R2-H2 (HIGH)** → FIX (single-txn `state` lifecycle; exactly-one decrement + crash-recoverable + correct
  replay body, all proven).

Every CRITICAL and HIGH from both rounds is now **fix** (none left as accept-risk for C/H). Remaining accepts
are tail-risk MED/LOW with named owners (distributed botnet — Ops; session_ref timing-correlation — Ops;
privileged-write bypass as the intended escape hatch — Architect). **Hard-exit: the seam batch is clear of
unresolved CRITICAL/HIGH findings.**

> SUPERSEDED in round 3: the round-2 hard-exit was premature. The R3 exit-check found the v3 restock TRIGGER
> still leaks (R3-C1) at a layer below the join (FORCE-RLS on `products` in the empty-context customer-cancel),
> plus two more restock/geofence holes. See "RESOLVE round 3" below. The strategic disposition there RETIRES
> the stock RUNTIME from this batch (column-seam only), so C1/R2-C1/R3-C1/R3-H1 no longer block it.

---

# RESOLVE round 3 — exit-check dispositions + the STRATEGIC stock-runtime call (A vs B)

> Scope: the four NEW findings from Breaker "RE-ATTACK round 3" (R3-C1 CRIT, R3-H1 HIGH, R3-H2 HIGH, R3-M1 MED),
> attacking the v3 mechanisms. PLUS the conductor's explicit strategic call: ship the stock decrement/restock
> RUNTIME now (option A) or ship only the inert `products.stock_remaining` column-SEAM and DEFER the runtime
> (option B). Every disposition verified against LIVE source this round. Companion: `proposal.md` (v4),
> ADR-0007 **v4**, ADR-0009 **v4**.

Verified source re-touched this round (all confirmed against working tree, not the ADR self-description):
- `apps/api/src/routes/customer/orders.ts:255-319` — the cancel handler opens a **raw `db.connect()`** (`:255`)
  and sets **only** `SET LOCAL app.settlement_reversal='true'` (`:297`); **0 hits** for `app.user_id` /
  `app.current_tenant` in the file. The status flip is a raw `UPDATE orders SET status='CANCELLED'` (`:289-293`).
- `packages/db/migrations/1780310072731_menu.ts:42-45` — `products` **ENABLE + FORCE ROW LEVEL SECURITY**;
  the only policy is `tenant_isolation USING ( location_id IN (SELECT app_member_location_ids()) )` (no WITH
  CHECK clause → USING governs the UPDATE row-visibility).
- `packages/db/migrations/1780310071220_core-identity.ts:75-79` — `app_member_location_ids()` is `SECURITY
  DEFINER` but its body reads `memberships WHERE user_id = app_current_user()` and `app_current_user()` =
  `NULLIF(current_setting('app.user_id', true),'')::uuid` (`:70-72`). Unset `app.user_id` ⇒ NULL ⇒ **empty set**.
- `packages/db/migrations/1780338982023_order_items_product_fk_set_null.ts:6-8` — `order_items.product_id …
  REFERENCES products(id) **ON DELETE SET NULL**`.
- `apps/api/src/routes/courier/shifts.ts:365-369` — assignment read is `SELECT 1 FROM courier_assignments WHERE
  courier_id=$1 AND status=ANY($2) **LIMIT 1**` (**no ORDER BY**).
- `packages/db/migrations/1780421100041_courier-assignments.ts:23-24` — **only** `UNIQUE(order_id)` + a
  non-unique `(courier_id, status)` index; **no** partial-unique restricting a courier to one active assignment.
- `docs/adr/0009-…:200,237` — floor AFTER cap: `hi := min(hi, eta_cap_min)` then `hi := max(hi, lo +
  min_window_width_min)`; `proposal.md:205` — `eta_cap_min DEFAULT 90`, `min_window_width_min DEFAULT 5`.

---

## ★ STRATEGIC DECISION — stock RUNTIME: **OPTION B (ship the column-SEAM only; DEFER the decrement/restock runtime)**

**Recommendation: B.** Ship ONLY the inert `products.stock_remaining` column (NULL = unlimited, zero runtime,
zero regression) in this batch. DEFER the atomic decrement-at-confirm + restock-on-terminal RUNTIME — together
with §3.2 atomicity, the `orders.stock_committed` flag, the restock trigger, claim-first idempotency's coupling
to the decrement, and the §4 per-unit DoS surface — to a **named follow-up: "Stock-runtime (decrement + restock)
follow-up"** with its own red→green race+leak proof against the REAL empty-context customer-cancel handler.

### Why B — the evidence is a three-round pattern, not a single miss

The decrement/restock area has broken in **THREE consecutive rounds, each a unit-leak via a DIFFERENT invisible
context boundary** that the prior fix did not model:

| Round | Mechanism shipped | How it leaked | The boundary the fix missed |
|---|---|---|---|
| C1 | decrement-at-**create** | PENDING-timeout/reject never restocks | the order **lifecycle after COMMIT** (PENDING ≠ sale) |
| R2-C1 | restock in **`updateOrderStatus`** | raw customer-cancel UPDATE bypasses the service method | the set of **raw status writers** (5, only 2 audited) |
| R3-C1 | restock in a **DB trigger** | `UPDATE products` hits 0 rows under FORCE-RLS in the cancel's empty `app.user_id` context | the **RLS firing-context** of the raw-pool cancel handler |

This is the brief's own anti-pattern made concrete: each fix moved the invariant one layer down and discovered
a new context the layer below does not satisfy. The customer-cancel handler is the recurring villain — it runs
in a **raw `db.connect()` pool connection with NO tenant context** (only `app.settlement_reversal`), which is
exactly the context that (a) bypasses the service method and (b) is denied by FORCE-RLS on `products`. A fourth
fix (e.g. `SECURITY DEFINER` on the trigger function, see R3-C1 disposition) would close R3-C1 — but the pattern
says: **the runtime's correctness depends on an authorization/lifecycle surface we keep mis-modeling, and that
surface deserves its own focused round with race+leak proofs, not a fourth in-flight patch riding a schema batch
whose other 13 findings are settled.**

### Why deferring loses almost nothing NOW (the §2.3 "working counter for limited specials" requirement)

The brief §2.3 manual checklist wants a counter for limited specials. **The binary `is_available` toggle is
already shipped and already covers the MVP need:** an owner running a limited special sets the product available
in the morning and toggles it OFF when it sells out (a one-tap action the daily-reset checklist already assumes).
`read_public_menu` filters `is_available=true`, so a sold-out special vanishes the moment the owner toggles —
the same UX outcome the numeric counter would produce at the `stock_remaining → 0` boundary, minus the automatic
trigger. For ~30 orders/day at cold-start, the manual toggle is operationally sufficient; the **numeric
auto-decrement is a convenience, not a correctness requirement**, for MVP. Deferring it costs the owner one
manual toggle per sold-out special — and BUYS not shipping a 🔴-money runtime that has leaked three rounds
running.

### What ships now (the irreversible part, at zero risk)

- `products.stock_remaining int` (NULL=unlimited) + `CHECK (stock_remaining IS NULL OR stock_remaining >= 0)`
  — migration `…067`, inert. **No runtime reads or writes it.** This is the "schema full, runtime later"
  doctrine (brief §3.3/§6) applied exactly: the irreversible column lands now with provably zero behavioural
  change (no code path references it), and the breakable runtime lands in the follow-up.
- **NOT shipped now (moved to the follow-up):** `orders.stock_committed` flag (`…066` cell), the decrement-at-
  confirm UPDATE (ADR-0007 v4 §2), the `orders_restock_on_terminal` trigger (§3), and the claim-first
  idempotency's *coupling to the decrement* (the idempotency `state` lifecycle itself is independently useful and
  stays — see note below).

### The §4 DoS-on-availability surface SHRINKS to the existing binary toggle (per instruction)

With NO per-unit decrement runtime, there is **no per-unit burn** to exhaust: a flood of orders cannot drive
`stock_remaining` down because nothing decrements it. The only availability lever an attacker has is the same
one the system already has — the binary `is_available` flag, which **only the owner can toggle** (no order path
writes it). So the §4 "найгірше = 1 змарнована порція" claim becomes trivially TRUE in this batch: the worst an
order flood can do is create PENDING noise (bounded by the phone+IP velocity throttle, M5), never burn
availability. The per-unit DoS surface (C1/R2-C1/R3-C1/M5's stock-burn half) is **removed from this batch's
threat model entirely** and re-enters only with the follow-up runtime, where it gets its own bound + proof.

### Note: idempotency `state` lifecycle (R2-H2 fix) stays; only its decrement-coupling defers

ADR-0007 v4 §4 (single-txn claim + `state {claimed→completed}`, migration `…066b`) is **independently correct
and useful** for "no double order on double-tap" regardless of stock. It does NOT depend on the decrement. It
stays in this batch (it was clean at the R3 exit — see R3 verdict #3). Only the *sentence* coupling it to "one
decrement at confirm" moves to the follow-up. So this batch keeps idempotency hardening and drops only the
stock-mutation runtime.

### Consequence for the R3 findings

Because B retires the stock runtime from this batch, **R3-C1, R3-H1, and the C1/R2-C1 lineage are DEFERRED WITH
THE RUNTIME** — they cannot occur with no decrement/restock code. They are NOT "accepted as live risks"; they
are **out of scope** for what ships, and each is recorded as a **named blocking pre-condition** the follow-up
MUST close before its runtime ships (with the verified fix already designed, below, so the follow-up starts
ahead). R3-H2 (geofence) and R3-M1 (live_eta floor) are NOT stock-related and ARE fixed in this batch.

---

## NEW CRITICAL

### R3-C1 — Restock trigger `UPDATE products` RLS-denied (FORCE-RLS, empty member context) on the customer-cancel route → **DEFERRED WITH THE RUNTIME (Option B); fix pre-designed for the follow-up**

**Verified break (confirmed, not the ADR's self-description):** the cancel handler (`customer/orders.ts:255-319`)
runs on a **raw `db.connect()`** connection (`:255`) and sets only `SET LOCAL app.settlement_reversal='true'`
(`:297`) — **never** `app.user_id` nor `app.current_tenant` (0 hits in the file). `products` is **FORCE RLS**
(`menu.ts:43`) with the sole writable policy `USING (location_id IN (SELECT app_member_location_ids()))`
(`:44-45`). `app_member_location_ids()`, though `SECURITY DEFINER`, derives its set from `app.user_id` via
`memberships` (`core-identity.ts:75-79`); unset `app.user_id` ⇒ NULL ⇒ **empty set** ⇒ the USING predicate is
false for every product row ⇒ the v3 trigger's `UPDATE products … +qty` matches **0 rows**. The trigger still
flips `NEW.stock_committed := false`, so it **consumes the idempotency guard while restocking nothing** — strictly
worse than R2-C1 (which left the flag true). 5 customer cancels = 5 permanently-leaked units on the exact raw
route the v3 trigger was introduced to cover.

**Disposition under B — DEFERRED WITH THE RUNTIME.** With no decrement/restock code in this batch, there is no
trigger and no leak. The finding cannot occur. It is recorded as a **blocking pre-condition** of the
"Stock-runtime follow-up", with the fix already designed and verified so the follow-up does not re-discover it:

> **Pre-designed fix for the follow-up (so it starts ahead):** make `orders_restock_on_terminal()` a
> **`SECURITY DEFINER`** function owned by the table owner, so its `UPDATE products` runs with the **owner's**
> rights and **bypasses RLS** — exactly the pattern `app_member_location_ids()` itself already uses
> (`core-identity.ts:77`). To prove it CANNOT be abused to cross tenants, the function derives the location from
> the **order row being updated** (`NEW.location_id`) and scopes the UPDATE `WHERE p.location_id = NEW.location_id`
> — it takes **NO caller input** (no `app.*` setting, no argument); a caller cannot point it at another tenant's
> products because the only location it ever touches is the one already on the order row it is firing for, and
> the order row's `location_id` is itself written under the order's own tenant constraints. SECURITY DEFINER here
> is *narrowing* (one fixed location derived from the row), not *widening* (no dynamic, caller-supplied scope).
> The follow-up's DoD MUST run the restock test against the **REAL empty-context handler** — drive an order to
> IN_DELIVERY then hit `POST /orders/:orderId/cancel` and assert `stock_remaining` actually MOVED back (the row
> changed) — and MUST NOT pre-set `app.user_id` or use a BYPASSRLS test role (that is the cheat-green that hides
> this exact bug). Owner: **Stock-runtime follow-up lead.**

**Files updated:** ADR-0007 → **v4** (Status: stock RUNTIME DEFERRED; the v3 §2/§3 decrement+restock retained as
the follow-up's pre-designed spec, with the SECURITY DEFINER + row-derived-location fix folded into §3 and the
anti-cheat-green DoD #3 sharpened to forbid a member-context harness); proposal §2.1/§3.1 (`…067` ships inert;
`…066` `stock_committed` cell + the restock trigger move to the follow-up) + §4 (DoS surface shrinks to the
binary toggle) + §7.

## NEW HIGH

### R3-H1 — `order_items.product_id ON DELETE SET NULL` severs the restock line for a since-deleted product → **DEFERRED WITH THE RUNTIME; accept-risk in the follow-up (justified)**

**Verified break:** `order_items.product_id … REFERENCES products(id) ON DELETE SET NULL`
(`1780338982023:6-8`). When an owner hard-deletes a product, every historical `order_items` row loses its
`product_id` (kept: `name_snapshot`/`price_snapshot`/`quantity`). The trigger's
`… JOIN … WHERE p.id = oi.product_id` then silently drops the NULL-product line → that line's unit is not
restocked, yet `stock_committed` is flipped false.

**Disposition under B — DEFERRED WITH THE RUNTIME; recommended disposition in the follow-up = ACCEPT-RISK,
justified.** With no restock runtime this batch, it cannot occur. For the follow-up I recommend **accept-risk,
not snapshot**, with this justification (so the follow-up does not over-build):

> A `product_id` is set NULL **only by the product's own hard-DELETE**. After that DELETE, the product *has no
> `stock_remaining` row to restock into* — the restock target no longer exists, so "the unit is leaked" is moot
> for that product (there is nothing to leak it back to). The only non-moot case is a **multi-item order mixing a
> since-deleted product with a still-live one**: the trigger restocks the live line correctly and skips the dead
> line — which is the correct outcome (the live product gets its unit back; the dead product has no counter).
> The flip of `stock_committed=false` is also correct: every line that *could* be restocked *was*. The residual
> "a since-deleted product's daily-cap unit is not returned" is **definitionally unobservable** (no row shows it).
> Therefore **accept-risk** (owner: Stock-runtime follow-up lead), documented; a snapshot `product_id` on
> `order_items` would add a column + write-path coupling to defend a state (restock into a deleted product) that
> has no observable effect. Snapshot is **rejected as over-engineering** for the follow-up unless the derived-BOM
> reader later needs the historical product linkage for a different reason.

**Files updated:** ADR-0007 v4 §3 (restock-line integrity note: `ON DELETE SET NULL` + the accept-risk rationale),
proposal §7 (R3-H1 row, deferred-with-runtime + follow-up accept-risk).

### R3-H2 — Geofence `order_id` via `LIMIT 1`-no-ORDER-BY over a multi-active (batch) courier binds to the WRONG order → **FIX NOW (singular-active-assignment guard) + HARD batch-phase flag**

**Verified break:** ADR-0009 v3 §2a derives the geofence `order_id` from `SELECT order_id FROM
courier_assignments WHERE courier_id=$c AND status=ANY($active) **LIMIT 1**` (mirroring `shifts.ts:365-369`,
which is `LIMIT 1` with **no ORDER BY**). `courier_assignments` has **only `UNIQUE(order_id)`**
(`1780421100041:23`) — no partial-unique restricting a courier to one active assignment — so a batched courier
(brief §49 `courier_sequence`, P3) can hold N active (`accepted`/`picked_up`) assignments, and `LIMIT 1` returns
an **arbitrary** one. The geofence (and therefore dwell) is stamped on a possibly-wrong order; `ON CONFLICT
DO NOTHING` then permanently locks the wrong binding. This is NOT stock-related, so it is **in scope and fixed
in this batch.**

**Decision — FIX NOW: make "the courier's active assignment" deterministic AND single, with a HARD flag that
the batch phase MUST bind geofence per-assignment before `courier_sequence` activates.** Two layers:

1. **MVP correctness (this batch):** the geofence `order_id` read becomes **deterministic and single-asserting**.
   v4 §2a pins the read as:
   ```sql
   SELECT order_id FROM courier_assignments
    WHERE courier_id = $c AND status = ANY($active::text[])
    ORDER BY picked_up_at NULLS LAST, accepted_at, order_id   -- deterministic, never arbitrary
    LIMIT 1;
   ```
   AND, because MVP doctrine is "one active assignment per courier", v4 adds a **partial-unique guard so that
   invariant is enforced, not assumed**:
   ```sql
   CREATE UNIQUE INDEX courier_one_active_assignment
     ON courier_assignments (courier_id)
     WHERE status IN ('accepted','picked_up');
   ```
   With the partial-unique in place, `LIMIT 1` over an at-most-one set is exact; the `ORDER BY` is belt-and-
   suspenders for the transition window. **This is the "fix now" the finding asks for: the active assignment is
   made singular at the DB level for MVP, so the geofence binding is unambiguous.**
2. **HARD batch-phase flag (recorded, blocking the P3 seam):** `courier_sequence` (the batch seam) CANNOT
   activate while the geofence read is "the courier's one assignment" — a batched courier legitimately holds
   several. **HARD REQUIREMENT (MISSING until P3):** before `courier_sequence` activates, the geofence binding
   MUST become **per-assignment** — the crossing is matched to a specific assignment by **proximity to that
   assignment's destination** (the order's delivery coordinates), not "the courier's only active order", AND the
   `courier_one_active_assignment` partial-unique MUST be dropped/relaxed in the same phase (it is incompatible
   with batching by construction). Recorded as a blocking pre-condition on the P3 batch phase. Owner:
   **North-Star / batch (P3) phase lead.**

**DoD (this batch):** (a) the partial-unique rejects a second active assignment for one courier (red→green);
(b) the geofence read returns the single active order deterministically; (c) a regression note that activating
`courier_sequence` without the per-assignment geofence rebind is a HARD-blocked precondition.

**Files updated:** ADR-0009 → **v4** §2a (deterministic ORDER BY + `courier_one_active_assignment` partial-unique
+ the HARD batch-phase rebind flag) + DoD; proposal §3.1 (`…071b` partial-unique on courier_assignments) + §4.2 +
§5 (geofence-binding invariant) + §7 (R3-H2 row + P3 blocking flag).

## NEW MEDIUM

### R3-M1 — `live_eta` width-floor applied AFTER the eta_cap clamp pushes `hi` above `eta_cap` → **FIX NOW (apply the absolute cap LAST / clamp `lo` so the floored window fits under cap)**

**Verified break:** ADR-0009 v3 §3a/§4 fixes the order as `hi := min(hi, eta_cap_min)` THEN `hi := max(hi, lo +
min_window_width_min)` (`:200,237`). With `eta_cap_min=90`, `min_window_width_min=5` (`proposal:205`) and `lo`
**never** clamped to the cap, a late recompute `lo=92,hi=95` yields: cap → `hi=min(95,90)=90`; floor →
`hi=max(90,97)=97`. Final `(92,97)` — `hi=97 > eta_cap=90`. The DB `CHECK (hi >= lo+1)` passes (no inversion),
so nothing rejects it; the "eta_cap absolute / hard external brake on padding-creep" (brief §1.4) is FALSE. Not
stock-related → **in scope, fixed in this batch.**

**Decision — FIX NOW: make `eta_cap` actually absolute by clamping `lo` first, then applying the floor, then a
final hard cap on `hi`.** The synthesis helper's clamp order becomes:

```
-- eta_cap_min is the HARD ceiling on the whole window. Clamp lo so the floored window still fits under cap:
lo := min(lo, eta_cap_min - min_window_width_min)   -- (1) make room for the floor under the cap
lo := max(lo, 0)                                     --     never negative
hi := max(hi, lo + min_window_width_min)             -- (2) width floor (honest-below)
hi := min(hi, eta_cap_min)                           -- (3) ABSOLUTE cap, applied LAST — wins over the floor
```

Now the `lo=92,hi=95` example: (1) `lo := min(92, 90-5=85) = 85`; (2) `hi := max(95, 90) = 90`; (3)
`hi := min(90, 90) = 90`. Final `(85,90)` — width 5 (floor honored), `hi=90` (cap honored, absolute), no
inversion (`90 > 85`). The cap is the hard ceiling; the floor is satisfied **underneath** it by lowering `lo`,
never by lifting `hi` past the cap. A genuinely-very-late order shows `(85–90)` and the §1.4 cap-hit advisory
fires cleanly on `hi == eta_cap_min` (now unambiguous — the cap is applied last so `hi == eta_cap` means a real
cap hit). The DB CHECK `hi >= lo+1` and the floor `min_window_width_min` both still hold.

**DoD:** a synthesis test with `lo > eta_cap - min_window_width_min` (e.g. `lo=92`) asserts final `hi ==
eta_cap_min` and `hi - lo >= min_window_width_min` and `hi <= eta_cap_min` — i.e. the cap is never breached and
the floor is never violated, jointly.

**Files updated:** ADR-0009 v4 §3a/§4 (clamp `lo` first → floor → absolute cap last; the helper pseudocode above)
+ DoD; proposal §5 (clamp-order note) + §7 (R3-M1 row).

---

## Round-3 dispositions summary

| Finding | Severity | Disposition | Where |
|---|---|---|---|
| R3-C1 | **CRIT** | **DEFERRED WITH THE RUNTIME (Option B)** — no decrement/restock ships this batch; SECURITY DEFINER + row-derived-location fix pre-designed + anti-cheat-green-against-real-handler DoD recorded as a blocking follow-up pre-condition | ADR-0007 v4, proposal §2.1/§3.1/§4/§7 |
| R3-H1 | HIGH | **DEFERRED WITH THE RUNTIME** — accept-risk recommended in the follow-up (a SET-NULL'd product has no counter to restock into; snapshot rejected as over-engineering) | ADR-0007 v4 §3, proposal §7 |
| R3-H2 | **HIGH** | **FIX NOW** — deterministic `ORDER BY` + `courier_one_active_assignment` partial-unique (active assignment made singular for MVP) + HARD per-assignment-rebind flag blocking the P3 batch seam | ADR-0009 v4 §2a, proposal §3.1/§4.2/§5/§7 |
| R3-M1 | MED | **FIX NOW** — clamp `lo` first, floor, then ABSOLUTE cap last → `eta_cap` truly absolute, floor still honored, no inversion | ADR-0009 v4 §3a/§4, proposal §5/§7 |

## Round-3 — still needing a human decision

1. **The A-vs-B stock-runtime call itself** — recommended **B** above, with full reasoning. This is an
   Architect recommendation; the human/conductor confirms the de-scope (ship column-seam only) vs. insisting on
   A (ship the runtime now with the R3-C1 SECURITY DEFINER fix). If the human picks A, R3-C1/R3-H1 flip from
   "deferred" to "fixed in batch" with the pre-designed SECURITY DEFINER restock + the accept-risk on H1, and the
   §4 per-unit DoS surface re-enters with the M5 phone+IP throttle as its bound.
2. **Carried, unchanged:** Counsel open-Q §5 (band-centering / late-within-band customer-cost signal) — autopilot
   -design time, not a seam blocker (owner: Product + North-Star lead).

## Round-3 verdict — CRITICAL/HIGH status after the A-vs-B call

Under the recommended **Option B**:
- **R3-C1 (CRITICAL)** → **out of scope of what ships** (no stock runtime); pre-designed fix + blocking DoD
  recorded for the named follow-up. Not a live risk in this batch.
- **R3-H1 (HIGH)** → out of scope of what ships; accept-risk recommended for the follow-up.
- **R3-H2 (HIGH)** → **FIXED in this batch** (deterministic single-active-assignment + partial-unique + P3 flag).
- **R3-M1 (MED)** → **FIXED in this batch** (absolute cap last).

**Therefore, for the batch that actually ships (Option B):** every CRITICAL/HIGH is either FIXED in-batch
(R3-H2) or removed-from-scope-with-the-runtime (R3-C1, R3-H1) with a named follow-up owner + pre-designed fix +
anti-cheat-green DoD. No unresolved CRITICAL/HIGH remains in the shipping scope. The recurring 🔴-money runtime
that broke three rounds running is no longer riding this schema batch — it lands in its own focused follow-up
where it gets the race+leak proof against the REAL empty-context handler it has needed all along.

**HARD-EXIT (conditional on the human confirming Option B):** the shipping batch (sensor columns + geofence
log + BOM seam + idempotency `state` + the inert `stock_remaining` column) is clear of unresolved CRITICAL/HIGH.
If the human instead picks Option A, R3-C1's SECURITY DEFINER fix + R3-H1 accept-risk must be implemented and
proven (the anti-cheat-green DoD #3 against the real `/cancel` handler is the gate) before hard-exit holds.
