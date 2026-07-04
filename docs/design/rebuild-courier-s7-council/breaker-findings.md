# S7-COURIER/DISPATCH Port — BREAKER FINDINGS (round 1)

> System-Breaker attack on `proposal.md` / `open-questions.md` / `threat-model.md`, verified against
> ground truth (`apps/api/src/routes/courier/*`, `owner/couriers.ts`, `plugins/auth.ts`,
> `lib/{dispatch,deliveryCompletion,bindingRelease,shiftService,courierAssignmentService}.ts`,
> `packages/db/migrations/*`, `rebuild/crates/api/src/auth/*`, the 085 draft). Read-only. No fixes.
> Each finding: `[SEVERITY] vector · finding · break-scenario/number · violated invariant`.

**Severity counts: CRIT 1 · HIGH 3 · MED 3 · LOW 1 · (4 verified-negatives).**

The dominant theme: the packet's Q6/§8 **broken-tenant-seat census is incomplete**. It correctly
flags settlements.ts / couriers`/live` / couriers`/details` / owner-settlement reads, but **misses two
more seat-broken surfaces that its own text affirmatively certifies as correct** — and post-B3 those
detonate the same way the flagged ones do.

---

## CRIT

### CRIT-1 · B-DATA / B-OPS (Q6 / S7-T10) — the DeliveryPage single-assignment read has a broken tenant seat the packet certified as correct
`courier/assignments.ts:110-116` — `GET /assignments/:id` (the courier's active-delivery `DeliveryPage`
read) does `client.connect()` → immediately `SELECT set_config('app.current_tenant',$1,true)` **with NO
`BEGIN`** → the `is_local=true` GUC is scoped to that statement's implicit auto-committing tx and is
**discarded before the next `client.query`** runs the enriched SELECT. This is the identical mechanism the
packet cites for `settlements.ts` and `couriers/live`. Its sibling `GET /me/assignments` (`:80`) **does**
`BEGIN` — so this is a **within-file seat drift**, exactly the class the packet flagged for `couriers.ts`
(`/couriers` has BEGIN, `/live` does not, Q-COURIERS-SEAT-DRIFT) but **there is no equivalent quirk row for
assignments.ts**, and §8 states outright: *"The shift/assignment/ping writes correctly `BEGIN` + `set_config`
… — carry the family."*

- **Post-B3 mechanism (verified):** `courier_assignments` policy is `location_id = NULLIF(current_setting
  ('app.current_tenant', true),'')::uuid OR location_id = ANY(app_member_location_ids())` (mig 073:47-49,
  FORCE at 051:7). With the seat discarded → courier branch = `NULL` (excluded), courier is not a member
  → `app_member_location_ids()` empty → **0 rows → `sendError(404)` for the courier's OWN active assignment**.
- **Break scenario:** the packet (§9, threat-model §4) explicitly treats the B3 (NOBYPASSRLS) flip as an
  **orthogonal, independently-reversible** event from the Node→Rust flip. The Playwright/parity oracle runs
  under today's BYPASSRLS pool (where the broken seat "works"), so the seat ships GREEN. When B3 later flips
  with **no S7 re-test**, **every courier mid-delivery gets 404 on `/assignments/:id`** → cannot load
  address / customer phone / instructions → cannot complete the delivery. This is a fleet-wide
  active-delivery outage on the core courier surface, triggered by a flip the packet says needs no S7
  coordination.
- **Violated invariant:** §11 DoD "a live NOBYPASSRLS probe asserts `app.current_tenant=activeLocationId`
  seated (in one real tx) on **every** courier … read"; threat-model §4 "every courier … access is correct
  independent of which pool role is live." The packet's actionable §8 census (which the porter implements)
  contradicts its own §11 by certifying assignments.ts clean.

---

## HIGH

### HIGH-1 · B-STATE / B-SEC (Q2 / S7-T5) — honest-dispatch has NO synthetic-courier exclusion; the packet asserts one that is not in the code
`lib/dispatch.ts:27-40` — the availability query filters `c.status='active' AND cs.status='available' AND
c.id NOT IN (active-binding set)`. It does **not** carry `AND c.email_hash <> SYNTHETIC_COURIER_EMAIL_HASH`.
That exclusion lives **only** in the owner roster query (`owner/couriers.ts:40`), not in the dispatch engine.
- The packet §4.2 and threat-model **S7-T5 mitigation** claim honest-dispatch "carries … the synthetic
  exclusion"; the §11 DoD only tests the **roster** exclusion ("the synthetic courier is excluded from the
  roster"). The dispatch path is untested and unguarded.
- **Break scenario:** on staging (`ALLOW_DEV_LOGIN` on) or any DB carrying a seeded synthetic courier
  (`/dev/seed-visual-state`) with `status='active'` and an `'available'` shift in a location, a real paid
  order transitioning to IN_DELIVERY calls `attemptHonestDispatch` → the synthetic row is a legal candidate
  → a **real order is bound to a non-human "courier"** (fake dispatch / silent orphan). The DoD's roster
  test passes while this ships.
- **Violated invariant:** Q2 🔴 / S7-T5 "no fake courier." "Carry verbatim" faithfully reproduces an
  **unguarded** dispatch engine while the packet documents a guard that does not exist there.

### HIGH-2 · B-DATA / B-OPS (Q6 / S7-T10) — the entire `courier/me.ts` file (4 in-scope reads) is bare-pool with no tenant seat; omitted from the census
`courier/me.ts` — `GET /me` (`:40-47`), `GET /me/audit-log` (`:97-104`), `GET /me/earnings` (`:181-223`),
`GET /me/history` (`:252-264`) are **all** bare `db.query` on the pool with **no `set_config('app.current_tenant')`
at all**. These routes are explicitly in S7 scope (§2.6, §2.7), yet §8's broken-seat census names settlements /
live / details / owner-settlement-reads and **never mentions me.ts**.
- **Post-B3 (verified against mig 077 rewrites):**
  - `/me` JOINs `courier_locations` (FORCE 051:5, missing-ok 077:80-82) → unseated → **0 rows → 404 on the
    courier's OWN profile.**
  - `/me/earnings` reads `courier_payouts` (missing-ok 077:83-85) + `courier_assignments` (missing-ok) →
    **silent "0 earned today / empty payouts"** — a money-display lie, not an error.
  - `/me/history` reads `courier_assignments`/`orders`/`customers` → **empty history.**
  - `/me/audit-log` reads `courier_audit_log` (missing-ok 077:71-73) → **empty.**
- The §8 "belt-and-suspenders `WHERE courier_id=$`" reasoning does **not** save these: FORCE-RLS filters by
  the tenant policy **first**, so the explicit `WHERE` sees an already-empty set. The packet's belt claim
  ("hold independent of which pool role is live") is false for a tenant-keyed FORCE policy with no seat.
- **Violated invariant:** §11 DoD "app.current_tenant seated on every courier read" — the census that
  implements it skips a whole file.

### HIGH-3 · B-MIGRATION (Q3 / Q7 / S7-T6/T7) — the settlement-safety argument presumes 085 is applied, but 085 is an un-applied draft and the flip is not gated on its application
§5.1 states the catch-up / idempotent / immutable / single-flight / watermark logic "**all live in**
`app_generate_settlements`." Ground truth: `1790000000085_settlements-catchup.ts` sits in
`docs/design/audit-fix-money/migration-drafts/` (header line 1-2: *"OPERATOR ACTION REQUIRED: place this file …
at packages/db/migrations/"*) — it is **NOT in `packages/db/migrations/`**. The live fn is **mig-078's
`>= p_period_start` version** with the SKIP-LOCKED-loss / phantom-count bugs 085 exists to close.
- The §11 DoD gates the flip on *"the 085 watermark **verified** before any settlement apply"* — verifying a
  literal, **not** on *085 applied*.
- **Break scenario:** the S7 port flips and a 2 AM settlement-cron (or `/regenerate`) fires during the
  rebuild window before an operator lands 085. The port is a faithful "thin caller" → it calls the **old
  lossy fn** → the C2 money-loss paths the packet's own safety argument assumes are closed are **live**. The
  packet's premise ("the money engine is Postgres, the port just preserves it") silently assumes the
  Postgres it preserves is the fixed one.
- **Violated invariant:** Q3 🔴 settlement idempotency / no-loss (M-2). The packet scopes 085 *out* of S7
  ("S7 does not author/apply 085") without making its **application** a hard pre-flip gate.

---

## MED

### MED-1 · B-CONSIST (Q5 / D1 / S7-T12) — the packet's own D1 fix reintroduces a shift-state corruption on an overnight delivery
D1's disposition prescribes "deterministic single-row selection **matching `openShift`**." `openShift`
(`shiftService.ts:18`) filters `DATE(started_at)=CURRENT_DATE`. A shift that went `on_delivery` at 23:00 and
is still active at 01:00 has `started_at` on **yesterday's** date → a "today's row" selector returns nothing.
- **Break scenario:** `/shifts/transition` then reads `currentStatus='offline'`, and for `to='available'`
  INSERTs a **new** shift (shifts.ts:262-268) while the on_delivery shift is still live → duplicate active
  shifts and the `CANNOT_GO_OFFLINE_WITH_ACTIVE_ORDER` guard reads the wrong row — i.e. exactly the S7-T12
  shift-state corruption D1 is meant to eliminate.
- The correct deterministic reference is `/me/shift`'s **status-filter** (`status IN ('available',
  'on_delivery') ORDER BY started_at DESC LIMIT 1`, shifts.ts:26-31), which is date-agnostic; the packet
  chose the wrong sibling to mirror.
- **Violated invariant:** deterministic single-active-shift selection (Q5 🔴 D1). Low-frequency trigger
  (delivery crossing midnight) → MED, not HIGH.

### MED-2 · B-DATA (Q5 / D1) — the D1 arbitrary-row defect also lives in `/me/shift/end`, which the packet's fix scope excludes
`shifts.ts:122-126` — `/me/shift/end` selects `SELECT id,status … WHERE courier_id=$ AND location_id=$ AND
status IN ('available','on_delivery') FOR UPDATE` with **no `ORDER BY` / `LIMIT`**. If duplicate active shift
rows exist (creatable via the D1 transition-INSERT path), `rows[0]` is arbitrary — the same class as D1.
The packet's D1 disposition names only `/shifts/transition`; this sibling and the (also-unfiltered)
`/me/shift/end` are not enumerated for the fix, so a "carry the rest verbatim" reading re-ships it.
- **Violated invariant:** deterministic shift selection (Q5).

### MED-3 · B-SEC (Q3 / S7-T10) — the settlement-read failure MODE differs by route; the packet's single "ERRORS post-B3" framing under-describes it
The packet (S7-T10, Q-SETTLE-SEAT) says the `settlement_items` policy "hard-ERRORS, not just 0-rows."
Verified true for the **items** query (`settlements.ts:79-85` → `settlement_items`, bare `current_setting`
mig 045:21, **not** rewritten by 077 → error). But the **list** route `/me/payouts` (`settlements.ts:28-46`)
reads `courier_payouts`, which **077:83-85 rewrote to missing-ok** → **0 rows, silent empty, NOT an error.**
So under the packet's own carry, `/me/payouts` silently shows "no payouts" while `/me/payouts/:id` 500s — a
split failure mode a single "it errors" test will not catch, and a silent-empty payout list is a worse
courier-trust outcome than a hard error.
- **Violated invariant:** Q3 🔴 "courier read == owner-approved amount" (silent-empty ≠ the approved rows).

---

## LOW

### LOW-1 · B-SCALE / B-CONSIST (Q2) — honest-dispatch is a lock-free check-then-act; race-safety comes from the frozen DB constraint, not the app ordering the packet credits
`lib/dispatch.ts:18-52` reads `bound` (no `FOR UPDATE`) and `availRes` (no lock) then INSERTs; the caller
(`orders.ts:891-897`) reads the order **without `FOR UPDATE`** (verified: plain membership-JOIN SELECT under
`withTenant`). Two concurrent IN_DELIVERY transitions on one order both see `bound=0` and pick the **same**
courier (deterministic `ORDER BY … LIMIT 1`). The double-INSERT is prevented **only** by the mig-073 partial
uniques `courier_assignments_order_active_uniq` + `courier_one_active_assignment`, surfacing as a **500** on
the losing tx (not a graceful `already_assigned`). The packet §4.2 attributes the no-double-bind to the
app-level "find-then-advance ordering"; the real guard is the DB constraint. LOW because the schema is frozen
(constraints carry) and the path is low-QPS — but the packet mis-credits the mechanism, so a future
"optimization" of the ordering that trusts the prose would regress it.

---

## Verified-negatives (record for the council — the packet is CORRECT here)

- **Q1 claims-shape / kid-drift (priority target #1): NO divergence found.** Node courier body =
  `{role, activeLocationId, jti, sub, kid, iat, exp}` (jwt.ts:50-59 spread + `.setIssuedAt/.setExpirationTime`
  + legacy.ts:165 `.strict()`) == Rust `CourierClaims` (claims.rs:87-103, `deny_unknown_fields`, exactly 7
  fields). Both write `kid` to the body AND header (jwt.ts:56 / jwt.rs:139,215). The S2 verifier's strict
  parse accepts a same-stack and cross-stack courier token in both directions. **Q1(a) "reuse the S2 minter,
  never a second impl" is sound; no cross-verify break.**
- **Q1c session-liveness (priority #2): real and per-request.** `courierSessionValid` (plugins/auth.ts:24-30)
  runs inside `verifyAuth` (`:44-92`) on every `/me`, `/assignments`, `/shifts` route (preValidation/preHandler/
  onRequest hooks). Deactivate + suspend revoke sessions (`couriers.ts:114-120`), password-change revokes
  (`me.ts:161-164`) → `revoked_at` catches them → next REST 401. **The carry closes the S6 WS-T8-on-REST
  gap; dropping it is the 14-day-tail the packet fears. No gap found in the existing bind.**
- **Q2 actor-gate (priority #3): complete on all mutations.** `AND courier_id=$ AND status=$ FOR UPDATE`
  present on accept/offered (assignments.ts:144-145), reject (192-195), picked-up (253-256), delivered
  (319-326), cancel (433-437), abort (498-502), decline (545-548), and the legacy service
  (courierAssignmentService.ts:21-27, 47-52). **No missing predicate → no cross-courier hijack via a dropped
  gate.** (The Q2 exposure is HIGH-1, the synthetic gap — not the actor-gate.)
- **Q3 courier PII / role-projection (priority #4): no leak.** Courier `/me/payouts[/:id]` selects only safe
  columns; items omit `order_id`/`assignment_id` (settlements.ts:79-89) vs owner items exposing them
  (owner/settlements.ts:91-92); no owner-only field (`paid_at`, `approved_by`) reaches the courier DTO. The
  085 watermark literal appears in **exactly three** places (`:66,:133,:148`) — the packet's "bump all three"
  count is accurate.

---

**Escalation call:** CRIT-1 (assignments-GET broken seat, packet-certified clean) is the held critical; it
composes with HIGH-2 (me.ts census gap) into one root cause — **the Q6/§8 broken-seat census is incomplete
and its "carry the family" assertions actively mislabel two seat-broken surfaces as correct.** HIGH-1
(synthetic dispatch) is a factual gap in a 🔴 no-fake-courier claim. HIGH-3 (085 not applied) is the money
timing landmine the packet's own safety argument silently presumes away.

council seats: breaker, counsel
🟡 DRAFT — findings for the live council
