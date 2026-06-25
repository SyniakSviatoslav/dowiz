# ADR 0009: Sensor Bus — Non-Status Event Log + Frozen `promised_window` + live `live_eta`

**Status:** PROPOSED (design-time — Triadic Council). Implements brief §1.1 (+ §1.3/§1.4 capture surfaces).
**Version:** v4 — hardened after Breaker R3-H2/R3-M1 (geofence `order_id` made DETERMINISTIC + SINGULAR: a
`courier_one_active_assignment` partial-unique + deterministic `ORDER BY` close the `LIMIT 1`-no-ORDER-BY /
multi-active-batch-assignment ambiguity, with a HARD per-assignment-rebind flag blocking the P3 batch seam; and
the `eta_cap` made truly ABSOLUTE — clamp `lo` first, floor, then cap LAST, so the width-floor can no longer
lift `hi` past the cap).
v3 — hardened after Breaker R2-H1/R2-M1/R2-M2 + Counsel R2 (order-assignment scope on the
geofence write, not just `location_id`; the `live_eta_*` WRITER specified with the width-floor on every
recompute; the AFTER-DELETE TRUNCATE gap honestly bounded; confirm-time stock rejection on the live channel;
the late-within-band customer-cost metric named).
v2 — hardened after Breaker C2/H5/M1/M3/L2 + Counsel ESTOP-1 (dual-context RLS for the courier-
context sensor write; the promised/live split; width floor; trigger-cost + privileged-bypass honesty; NULL
reconstruction contract). **Resolution:** `…/resolution.md` C2/H5/M1/M2/M3/L2/ESTOP-1/R2-H1/R2-M1/R2-M2 + Counsel #1–5 + R2.
**Supersedes:** nothing · **Extends:** `order_status_history` (`1780338982015_order_history.ts`), the
per-transition timestamp columns on `orders` (`orderStatusService.ts:89-117`), and the courier ping stream
(`courier_positions`, ADR-GEO-SEAMS).
**Companion design:** `docs/design/mvp-sensor-seams/proposal.md` §2.3, §3.1, §5.

## Context

Brief §1.1 wants an append-only event trail (`confirmed_at`, `courier_geofence_enter` [exactly once],
`picked_up`, `delivered_at`, optional `geofence_enter_customer`) plus an **immutable** customer-shown
`promised_window` on the order, so prep/road/dwell durations are reconstructable and the historical promise
is the ground truth for the P1/P2/P7 falsification tests (brief §8) — none of which is retroactively
recoverable.

Grounded reality (corrects the brief):
- **Status timestamps already exist and are written**: `confirmed_at`, `preparing_at`, `ready_at`,
  `in_delivery_at`, `picked_up_at`, `delivered_at` on `orders` (`orderStatusService.ts:10-17,89-117`). So
  `confirmed_at`/`picked_up`/`delivered_at` are **DONE**.
- **`order_status_history` is a STATUS log**, keyed on `to_status order_status NOT NULL`
  (`1780338982015_order_history.ts:9-10`). It **cannot** carry `courier_geofence_enter` — that is not a
  status, and the state machine `assertTransition` (`orderStatusService.ts:75`) enforces the enum.
- **No venue geofence exists** — `locations` has `lat`/`lng` but no radius/polygon; ETA is advisory haversine
  with no per-ping router (ADR-GEO-SEAMS).
- `order_status_history` is append-only **by convention only** — no trigger blocks UPDATE/DELETE; it is
  written best-effort in a SAVEPOINT (`orderStatusService.ts:128-139`).

## Decision

### 1. Non-status sensor events → a NEW append-only table (NOT `order_status_history`)

```sql
CREATE TABLE order_sensor_events (
  id          bigserial PRIMARY KEY,
  order_id    uuid NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
  location_id uuid NOT NULL,
  event_type  text NOT NULL CHECK (event_type IN ('courier_geofence_enter','geofence_enter_customer')),
  payload     jsonb NOT NULL DEFAULT '{}'::jsonb,
  created_at  timestamptz NOT NULL DEFAULT now(),
  UNIQUE (order_id, event_type)            -- "geofence_enter рівно раз" enforced at the DB
);
CREATE INDEX order_sensor_events_order_idx ON order_sensor_events (order_id);

ALTER TABLE order_sensor_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE order_sensor_events FORCE  ROW LEVEL SECURITY;
-- DUAL-CONTEXT policy (Breaker C2): this is the ONE sensor table written from the COURIER
-- ping handler (which sets ONLY app.current_tenant — shifts.ts:337, never app.user_id) AND
-- read by OWNER/analytics (which set ONLY app.user_id → app_member_location_ids()). A
-- member-only WITH CHECK is the empty set in the courier context → every geofence INSERT is
-- DENIED and the best-effort SAVEPOINT silently swallows it (geofence/dwell captured NEVER).
-- The policy is therefore the DISJUNCTION of both tenant idioms. NULLIF(...,true) makes an
-- unset variable NULL (no row matches) instead of an error, so neither context leaks into the
-- other (an owner never sets app.current_tenant; a courier never sets app.user_id).
CREATE POLICY tenant_isolation ON order_sensor_events
  USING (
    location_id IN (SELECT app_member_location_ids())
    OR location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid
  )
  WITH CHECK (
    location_id IN (SELECT app_member_location_ids())
    OR location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid
  );
REVOKE ALL ON order_sensor_events FROM anon, authenticated, service_role;
-- + GRANT SELECT,INSERT,UPDATE,DELETE TO deliveryos_api_user (guarded by pg_roles EXISTS),
--   mirroring courier_positions' grant so the ping handler's role can INSERT.
```

Rationale: status-transition timestamps live on `orders` (done) + `order_status_history` (the status audit
log). Sensor crossings are a DIFFERENT shape — a new table is the correct cut, and `UNIQUE(order_id,
event_type)` + `INSERT … ON CONFLICT DO NOTHING` makes "exactly once" a DB invariant, idempotently, even if
the ping handler re-fires the crossing. The dual-context policy is the explicit, verified fix for the C2
silent-loss: it is the same disjunction-of-idioms made necessary because `order_sensor_events` is the one
table written from the courier world and read from the owner world.

### 2. Geofence detection — derived, no router (brief §1.2)

Add `locations.geofence_radius_m int NOT NULL DEFAULT 150`. In the IN_DELIVERY courier-ping handler
(`shifts.ts:336-378`, which has `app.current_tenant` set — so the §1 dual-context WITH CHECK now passes),
compute `haversine(ping.lat,ping.lng, location.lat,location.lng) <= geofence_radius_m` (O(1), data already in
hand, the same `distanceKm` primitive used at `orders.ts:540`). On first crossing →
`INSERT INTO order_sensor_events (order_id, location_id, event_type) VALUES (…, 'courier_geofence_enter')
ON CONFLICT (order_id, event_type) DO NOTHING` (M3 — the ON CONFLICT target is pinned; a re-crossing under
GPS jitter is a no-op, never a 23505). **Best-effort in a SAVEPOINT** (mirror `orderStatusService.ts:128-139`)
— a failed sensor insert must NEVER fail the position update (brief §0.1 observe-don't-control; proposal §4.2).
The SAVEPOINT is now a *real* fallback for a genuine failure, not a swallower of a guaranteed RLS denial (C2
fixed at the policy). **DoD requires a positive-presence test** (the geofence row IS present after a crossing
in the courier context), so a regression back to silent-loss is caught.

### 2a. The `order_id` MUST be the courier's OWN active assignment — order-assignment scope (Breaker R2-H1)

**The C2 dual-context RLS validates `location_id`, NOT `order_id`-ownership.** A row passes the WITH CHECK iff
`order_sensor_events.location_id = app.current_tenant` — so any courier on shift at venue X could stamp a
`courier_geofence_enter` for **another** courier's order at X (forging dwell/road sensor data on a colleague's
delivery). The `order_id` is the natural scoping key and the RLS throws it away (it mirrors
`courier_positions`' weakest property — tenant-only — while carrying an `order_id` that positions never had).

**Fix — the write path supplies the `order_id` from the courier's OWN assignment; it is never courier- or
payload-supplied.** The ping handler already reads the courier's active assignment at `shifts.ts:365-369`
(`SELECT 1 FROM courier_assignments WHERE courier_id = $1 AND status = ANY(active) LIMIT 1`). v3 changes that
read to also return the assigned `order_id`, and the geofence INSERT uses **that** order_id — never an
attacker-chosen one from the ping payload (the ping body is `{lat,lng,accuracy_meters}` only; it carries no
order_id, and v3 forbids adding one):

> **R3-H2 (HIGH, verified) — "the courier's active assignment" is NOT singular, and the read is non-deterministic.**
> The v3 read mirrored `shifts.ts:365-369`: `… status = ANY($active) **LIMIT 1**` with **no ORDER BY**.
> `courier_assignments` has **only `UNIQUE(order_id)`** (`1780421100041:23`) — no partial-unique restricting a
> courier to one active assignment — so a batched courier (brief §49 `courier_sequence`, P3; ACTIVE =
> `['accepted','picked_up']`, `courier-gps.ts:9`) can hold N active assignments, and `LIMIT 1` returns an
> ARBITRARY one → the geofence/dwell is stamped on a possibly-WRONG order, then locked by `ON CONFLICT DO
> NOTHING`. v4 makes the active assignment **singular at the DB** for MVP and the read **deterministic**, with a
> HARD flag that the batch phase must rebind geofence per-assignment before `courier_sequence` activates.

```sql
-- v4 (R3-H2): enforce "one active assignment per courier" for MVP at the DB level, so the
-- read below is over an at-most-one set (the geofence binding is then unambiguous):
CREATE UNIQUE INDEX courier_one_active_assignment
  ON courier_assignments (courier_id)
  WHERE status IN ('accepted','picked_up');

-- already-present assignment read, extended to yield the order_id the courier owns RIGHT NOW,
-- now DETERMINISTIC (belt-and-suspenders for the brief transition window):
SELECT order_id FROM courier_assignments
 WHERE courier_id = $courier AND status = ANY($active::text[])
 ORDER BY picked_up_at NULLS LAST, accepted_at, order_id   -- never arbitrary
 LIMIT 1;                                                  -- exact: partial-unique makes the set ≤ 1
-- geofence INSERT uses THAT order_id; nothing else can be stamped:
INSERT INTO order_sensor_events (order_id, location_id, event_type)
VALUES ($ownAssignmentOrderId, $loc, 'courier_geofence_enter')
ON CONFLICT (order_id, event_type) DO NOTHING;
```

> **HARD batch-phase flag (MISSING until P3 — blocks `courier_sequence`).** `courier_one_active_assignment` is
> incompatible with batching by construction — a batched courier legitimately holds several active assignments.
> Before the P3 `courier_sequence` seam activates, TWO things MUST happen together: (1) drop/relax the
> partial-unique; (2) rebind the geofence to a **specific assignment by proximity to that assignment's
> destination** (the order's delivery coordinates), NOT "the courier's only active order". Activating
> `courier_sequence` without the per-assignment geofence rebind is a HARD-BLOCKED precondition. Owner:
> **North-Star / batch (P3) phase lead.**

Because the `order_id` is derived from `courier_assignments WHERE courier_id = <self>` over an at-most-one active
set (partial-unique) read deterministically, a courier can only ever stamp the order they are actually delivering
— exactly v1's intent ("the courier stamps a geofence on the order they are delivering"), restored AND made
unambiguous under MVP's single-active-assignment invariant. This is an **app-layer ownership enforcement** on the write path that
already knows the assignment; it does not need a WITH CHECK subquery (which would have to join
`courier_assignments` from inside the RLS policy — more expensive and the same effect). **Defence-in-depth
option (documented, not required for MVP):** the WITH CHECK *could* additionally assert
`order_id IN (SELECT order_id FROM courier_assignments WHERE courier_id = current_setting('app.courier_id')…)`
if a future write path ever takes an external order_id; for MVP the single ping-handler writer deriving its own
assignment is sufficient and cheaper. **DoD adds an order-assignment-scope test** (a courier stamping a
*colleague's* order_id at the same venue is rejected/impossible because the write derives the order_id from the
courier's own assignment, never from input).

### 3. Frozen `promised_window` + MUTABLE `live_eta` — the split (Counsel ESTOP-1)

The set-once trigger conflated two concepts: the **promise-as-made** (frozen, §8 measurement fuel) and the
**live current estimate** (§2.4's collapsing window — the client truth channel). Freezing one field for both
silently makes §2.4 honesty fight §1.1 immutability, and freezes the customer into a possibly-wrong number
with no repair path (the exact party range-never-point protects). **v2 splits them** (Counsel's recommendation
(a); the append-only-log steel-man is the recorded North-Star upgrade — its first row IS this frozen column).

```sql
-- Frozen: the promise as made at confirm — measurement ground truth for §8 (P1/P2/P7).
ALTER TABLE orders ADD COLUMN promised_window_lo_min int;
ALTER TABLE orders ADD COLUMN promised_window_hi_min int;
-- MUTABLE: the live current estimate the customer sees collapse through stages (§2.4
-- confirmed→cooking→picked_up→arriving). The client truth channel — NOT frozen.
ALTER TABLE orders ADD COLUMN live_eta_lo_min int;
ALTER TABLE orders ADD COLUMN live_eta_hi_min int;
-- range-never-point at VALUE level (Breaker L2 / Counsel #1): reject a literal point.
ALTER TABLE orders ADD CONSTRAINT orders_promised_window_is_range
  CHECK (promised_window_lo_min IS NULL
         OR promised_window_hi_min >= promised_window_lo_min + 1);
ALTER TABLE orders ADD CONSTRAINT orders_live_eta_is_range
  CHECK (live_eta_lo_min IS NULL OR live_eta_hi_min >= live_eta_lo_min + 1);

-- The trigger guards ONLY the frozen pair — live_eta is intentionally mutable.
CREATE OR REPLACE FUNCTION orders_promised_window_set_once() RETURNS trigger AS $$
BEGIN
  IF OLD.promised_window_lo_min IS NOT NULL
     AND (NEW.promised_window_lo_min IS DISTINCT FROM OLD.promised_window_lo_min
       OR NEW.promised_window_hi_min IS DISTINCT FROM OLD.promised_window_hi_min) THEN
    RAISE EXCEPTION 'promised_window is immutable once set (order %)', OLD.id;
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER orders_promised_window_set_once_trg
  BEFORE UPDATE ON orders
  FOR EACH ROW EXECUTE FUNCTION orders_promised_window_set_once();
```

`promised_window_*` is written **once** in the CONFIRMED branch of `orderStatusService.ts:90-94` (the same
guarded UPDATE that sets `confirmed_at = now()`); `live_eta_*` is seeded equal to it at confirm and then
**updated** as the order collapses through stages. **The customer page reads `live_eta_*` (the truth as it
evolves); the §8 metric reads `promised_window_*` (the frozen first promise).** A mis-set first promise stays
visible in the historical record (correct — it's a falsification data point) but the customer always sees the
current truth via `live_eta_*`. The party range-never-point protects is no longer frozen into a lie.

### 3a. The `live_eta_*` WRITER — specified, not just the schema (Breaker R2-M1)

**v2 named a mutable column but no writer, so `live_eta_*` would be byte-identical to the frozen
`promised_window_*` forever (live==frozen) and the ESTOP-1 fix would be cosmetic.** v3 specifies exactly which
transitions recompute it, where, and under which floor. The recompute is **co-located with the existing
per-transition stamp in `orderStatusService.ts:106-116`** (the `else` branch that already writes
`preparing_at/ready_at/in_delivery_at/picked_up_at`) — no new worker, no new hot path:

| Stage transition (where `*_at` is already stamped) | `live_eta_*` recompute |
|---|---|
| CONFIRMED | seed `live_eta_* := promised_window_*` (the first promise, in the §2 confirm UPDATE) |
| PREPARING | recompute from `remaining_prep + travel_estimate` off the new `preparing_at` baseline |
| READY | prep done → `live_eta_* := travel_estimate` only (the window tightens) |
| IN_DELIVERY / PICKED_UP | recompute from `remaining_travel` (haversine ETA off the latest courier ping) |
| (geofence_enter — §2) | `live_eta_*` collapses toward the arriving band (the dwell-start signal) |

Each recompute calls **the SAME synthesis helper** that produces the promised window, so it inherits the
width-floor and cap by construction. **The width-floor `min_window_width_min` (Counsel #1 / Breaker L2)
applies to EVERY `live_eta_*` recompute, not just the initial synthesis** (Counsel R2.1) — so as the live window
tightens near delivery (where pseudo-precision "1–2 min" is most tempting), it can never narrow below the honest
floor.

> **R3-M1 (verified) — clamp order: the cap must be ABSOLUTE, applied LAST.** v3 ordered the clamps
> `hi := min(hi, eta_cap_min)` THEN `hi := max(hi, lo + min_window_width_min)` — but `lo` was never capped, so a
> late recompute (`lo=92, hi=95`, `eta_cap=90`, floor=5) gave `hi := min(95,90)=90` then `hi := max(90,97)=97` →
> `hi=97 > eta_cap=90`. The floor, applied last, defeated the cap. v4 fixes the order so the floor is satisfied
> **underneath** the cap by lowering `lo`, never by lifting `hi` past the cap:

```
-- eta_cap_min is the HARD ceiling on the WHOLE window. Make room for the floor under it, then cap last:
lo := min(lo, eta_cap_min - min_window_width_min)   -- (1) clamp lo so a floored window still fits under cap
lo := max(lo, 0)                                     --     never negative
hi := max(hi, lo + min_window_width_min)             -- (2) width floor (honest-below)
hi := min(hi, eta_cap_min)                           -- (3) ABSOLUTE cap, LAST — wins over the floor
```

Worked: `lo=92,hi=95` → (1) `lo=min(92,85)=85`; (2) `hi=max(95,90)=90`; (3) `hi=min(90,90)=90` → `(85,90)`,
width 5 (floor honored), `hi=90` (cap honored, absolute), no inversion. A genuinely-very-late order shows
`(85–90)` and the §1.4 cap-hit advisory fires cleanly on `hi == eta_cap_min` (now unambiguous — cap applied
last). The DB `CHECK (live_eta_hi_min >= live_eta_lo_min + 1)` is the last-line guarantee. **The live channel is
the customer-honesty channel; the floor is applied on its UPDATE path, not only the confirm synthesis** —
closing the R2.1 caveat that the honest-below guarantee weakened at the arriving stage, AND the R3-M1 caveat that
the cap was not actually absolute.

> The recompute is **best-effort within the transition** (a `live_eta` recompute failure must not roll back a
> status change — it is observation/estimate, not order state, unlike the frozen `promised_window` which IS
> order state coupled to the confirm). If a recompute is skipped, the customer simply keeps the prior live
> band (degrades to "slightly stale truth", never to "frozen first promise" — the previous live value, not the
> confirm value). ESTOP-1 is therefore RESOLVED in behaviour, not merely in schema (R2-M1 closed).

### 3b. Confirm-time stock rejection surfaces on the live customer channel (Counsel R2-a)

The C1 re-architecture moved the OUT_OF_STOCK refusal from create-time to **confirm-time** — i.e. after the
customer has committed and is watching the live order page. A bare API 422 (which the FE may swallow) is the
wrong dignity surface for a customer already waiting. **v3 pins:** a CONFIRMED→rejected-for-stock event rides
the **same live order channel** the customer is already watching (the `live_eta_*` / order-status broadcast,
`orderStatusService.ts:151` `messageBus.publish(orderChannel(orderId), …)`) carrying the humane cause-hint
`{ code:'OUT_OF_STOCK', product:'<name>' }` — the customer sees "Product X is out of stock" on the live view,
not only a 422 the FE may drop. This is non-blocking dignity, not a red line, but it is now specified for the
confirm-time path, not just the create-time path.

**H5 — trigger cost + privileged-bypass honesty (not oversold as a "hard invariant"):**
- The trigger body is a handful of comparisons with **no query / no I/O** → sub-microsecond per row; the
  timeout sweep's bulk `UPDATE … status='CANCELLED'` (`order-timeout-sweep.ts:67`) never touches the frozen
  columns so the `IS DISTINCT FROM` is always false (fast path, no RAISE). Pinned as a DoD micro-assertion
  (timing on a bulk sweep UPDATE shows no material delta). It is hot-table-safe.
- **App writes cannot bypass** the trigger; a **migration/superuser write (or `session_replication_role=
  'replica'`) intentionally can** — that is the *one* legitimate, logged, privileged correction of a mis-set
  promise (consistent with ESTOP-1's recorded-human-decision (b)). v2 states this asymmetry rather than
  claiming an unqualified "hard invariant."

### 4. Invariants this ADR enforces (not merely documents)

- **range-never-point — BOTH bounds (§0.4, Breaker L2 / Counsel #1):** the schema has ONLY `_lo_min`/`_hi_min`
  (no point column), AND value-level: `min_window_width_min` (new `locations` col, DEFAULT 5) is the **floor**.
  **Clamp order (R3-M1): `lo := min(lo, eta_cap_min - min_window_width_min)` first, then the floor
  `hi := max(hi, lo + min_window_width_min)`, then the ABSOLUTE cap `hi := min(hi, eta_cap_min)` LAST** — so the
  floor is satisfied UNDER the cap by lowering `lo`, never by lifting `hi` past the cap (`eta_cap` is a hard
  ceiling, §1.4). The DB `CHECK (… hi >= lo + 1)` (§3) rejects a literal point. So a band can never collapse to
  a point at the cap clamp or at `lo == hi`, can never exceed `eta_cap_min`, and "1–2 min" pseudo-precision is
  forbidden by the floor. The client ETA
  response Zod type is `{lo:int, hi:int}` and rejects `lo == hi`. Range-never-point is enforced at schema-shape
  + value-level + render-level — the complete contract (honest below, useful above).
- **agents-declare-privately (§3.1):** the customer order-status read selects the synthesised window columns,
  NOT the raw per-agent inputs (`prep_time_minutes`, courier timing). Those stay internal; the system emits
  ONE client number-range.
- **eta_cap absolute (§1.4):** the synthesis helper clamps `hi_min` to `locations.eta_cap_min`; hitting the
  cap raises an owner `customer_signals`-style advisory (NOT silent — brief §1.4). The cap is a hard external
  brake on padding-creep.

### 4b. Reconstruction NULL-contract (Breaker M1) — bias-free by construction

Every reconstructed duration is computed **only over orders with BOTH endpoints non-NULL**, **segmented by
fulfilment type** (delivery-by-courier vs pickup vs cancelled-mid-flight kept separate, never pooled into one
AVG), and the dwell metric (`picked_up_at − geofence_ts`) is **conditional on a geofence row existing**
(post-C2-fix, the courier-delivery subset only — never silently averaged as if universal). Each AVG reports
its **sample size `n`** so a partial/self-selected population is visible, never hidden. This fixes the
measurement-bias the brief §8.1 exists to fix at the reconstruction layer (which v1 left unspecified, inviting
a biased naive AVG).

**Late-within-band customer-cost metric — NAMED NOW (collection only; decision deferred) — Counsel R2.3.**
The funnel (`…070`) actively measures the *venue's* cost (lost carts). Its symmetric *customer-side* cost — the
**late-within-band rate** — must be named as a first-class reconstruction output of THIS contract so the two
costs reach autopilot-design time **as peers**, not one-wired-one-hypothetical (else the autopilot is built
OTP-skewed before anyone decides it should be — the §5 self-reinforcing asymmetry). It needs **no new seam** —
it is just another both-endpoints-non-NULL duration over columns this batch already lays:

```
late_within_band_rate =
  count(orders WHERE delivered_at > promised_window_hi_min  -- vs the frozen promise (trust spent)
                  OR delivered_at > live_eta_hi_min)        -- vs the last live band the customer saw
  / count(delivered orders)        -- segmented by fulfilment type, reporting n, same NULL-contract as above
```

This is **collection only**: it makes the customer-side signal exist from day 1 alongside the funnel. The
**centering decision** — where inside the honest band the synthesized promise sits, and how to weigh this
signal against the owner's OTP conservativeness knob — is a *runtime policy* that lives in the (out-of-scope)
synthesis helper and is **deferred to autopilot-design time as a recorded human decision** (proposal §7,
resolution open-Q §5). v3 separates them: the **measurement** is built now (free); the **decision** is deferred
(human). Per Counsel R2.3, this is the difference between deferring the question fairly and deferring it in a
way that quietly answers it.

### 4c. Signal/metric norms — written NOW so they can't drift (Breaker H4 + Counsel #2/#3/#5)

- **Funnel padding-creep counter-metric (§8.2):** computed over **distinct `session_ref`** (abandon-rate per
  session, not raw row count) so a single-IP flood is one session ≈ one vote; it is **advisory** input to a
  human/loop, **never a direct autopilot actuator**. The funnel ingest endpoint is per-IP rate-limited
  (proposal §4.3). `session_ref` is **never written onto an order** and the FE rotates it at order submission
  (unlinkability designed-in — Counsel #5; disclosed in `/compliance` + the storefront privacy notice).
- **Courier normalized-time metric (§8.3, North-Star — NOT in this batch; norm recorded so it can't ship
  without it, Counsel #2):** the rating surfaces the **normalized** (road-distance-fair) number only, never raw
  time; it is **owner-advisory and MUST NOT be the basis of an automated deactivation**. Owner: North-Star
  phase lead.
- **§2.1 dispatch nudge (Counsel #3):** courier-facing **advisory** only; non-compliance is **NOT** recorded as
  an owner-visible compliance signal. "Courier owns the moment" true in lived experience, not just code.

### 5. `order_status_history` hardening (optional, flagged)

It is append-only by convention. Hardening it with a REVOKE-UPDATE/DELETE or an immutability trigger is
**out of this batch's necessity** (the new fuel lives in `order_sensor_events` + the set-once window). Flagged
for a future hygiene pass; not blocking. *(Human/Council decision — see proposal §10.)*

## Proof / DoD

- **Geofence PRESENCE test (Breaker C2 — the key new test)**: in the courier ping handler's exact context
  (`app.current_tenant` set, `app.user_id` UNSET), simulate a boundary crossing → assert the
  `order_sensor_events('courier_geofence_enter')` row **IS present** (not best-effort-swallowed). Red before the
  dual-context policy, green after.
- **Geofence-once test (M3)**: two pings inside the radius → exactly one row (`UNIQUE` + ON CONFLICT
  (order_id,event_type) DO NOTHING). A ping failure does not fail the position update.
- **Cross-context RLS test**: an owner (member idiom) SELECTs the geofence rows for their location AND a
  cross-tenant SELECT returns 0; a courier context cannot read another tenant's rows. Both idioms isolated.
- **Order-assignment-scope test (Breaker R2-H1)**: courier C (assignment on O1) at venue X attempts to stamp a
  geofence on O2 (a colleague's order at X) → impossible, because the ping handler derives the order_id from
  `courier_assignments WHERE courier_id = C` (yields O1, never O2) and the ping body carries no order_id. Assert
  the write lands on O1 only; a forged-order_id path does not exist in the handler.
- **Set-once test**: confirm an order (`promised_window_*` written) → a second UPDATE changing it RAISES; an
  UPDATE not touching it succeeds. **Live-mutable test**: updating `live_eta_*` after confirm SUCCEEDS (it is
  NOT frozen). Red→green guardrail + regression-ledger row.
- **Live-eta WRITER test (Breaker R2-M1)**: drive an order CONFIRMED→PREPARING→READY→IN_DELIVERY → assert
  `live_eta_*` is recomputed at each stage (NOT byte-identical to the frozen `promised_window_*` once it has
  diverged) AND that each recompute obeys the `min_window_width_min` floor (a recompute that would narrow below
  the floor is widened). Proves the live channel is actually live, not cosmetic.
- **Width-floor / range test (L2)**: a synthesis that would emit `lo == hi` is widened to `lo + min_window_
  width_min`; a direct INSERT of `lo == hi` violates the `CHECK`. The client Zod schema rejects `lo == hi`.
  Assert the floor is applied on the `live_eta_*` recompute path too, not only the initial synthesis.
- **Reconstructable durations test (M1)**: assert prep/road/dwell are derivable AND that the AVG is computed
  only over both-endpoints-non-NULL orders, segmented by fulfilment, reporting `n` (no naive pooled AVG).
- **Late-within-band collection test (Counsel R2.3)**: assert the `late_within_band_rate` reconstruction
  (`delivered_at` vs `promised_window_hi_min` / `live_eta_hi_min`) is derivable from the laid columns with the
  same NULL-contract — the customer-side cost signal EXISTS from day 1 (collection only; centering decision
  deferred, no actuation).
- **RLS test**: cross-tenant SELECT on `order_sensor_events`/`funnel_events` returns 0 rows.

## Consequences

- The P1/P2/P7 falsification fuel (brief §8) is captured from order #1 with zero history, retroactively
  unrecoverable cost paid now — AND the geofence/dwell fuel actually LANDS (C2 dual-context RLS), instead of
  being silently denied + swallowed (the v1 silent-zero).
- The customer keeps the live truth (`live_eta_*`) while the metric keeps the frozen first promise
  (`promised_window_*`) — ESTOP-1 dissolved, not chosen against the customer. The append-only window-log
  (Counsel §4 steel-man) is the recorded North-Star upgrade (its first row IS the frozen column).
- Zero new external dependency / network call → no new circuit-breaker surface (ADR-GEO-SEAMS posture intact).
- All capture is non-blocking except the order's own `promised_window` write (which IS order state, coupled to
  the confirm it rides on) — every true sensor (geofence, funnel, baseline) cannot fail an order.
- The set-once trigger is hard for app writes, intentionally bypassable by a logged migration/superuser write
  (the one correction escape hatch — H5); it is I/O-free so it is hot-orders-UPDATE-safe.
- Forward-only; the new table/trigger are guarded (`IF NOT EXISTS` / `DROP … IF EXISTS`) for a retried
  release_command.
