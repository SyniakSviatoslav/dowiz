# ADR 0013: Courier Realtime Authorization — Binding-Scoped Room + Thread Access

**Status:** ACCEPTED — DESIGN FROZEN (STOP-DESIGN-B). Triadic Council Round-3 converged: 0 CRITICAL/HIGH,
0 ETHICAL-STOP, back-of-envelope converges honestly, all residuals owned. Ready to hand to the fix PR
(red→green DoD below). NO production code in this design change.
**Date:** 2026-06-29 (Round-3 final)
**Companion design:** `docs/design/courier-realtime-authz/proposal.md`
**Supersedes:** nothing · **Extends:** the owner WS-authz pattern (`ownerCanAccessRoom`,
`apps/api/src/websocket.ts:82-109`) and the offer-handshake binding model
(ADR-deliver-v2-cash-as-proof, `apps/api/src/lib/dispatch.ts`).
**Security class:** 🔴 tenant-isolation / authz (cross-tenant + intra-tenant data leak).

## Context

The WS `subscribe` courier branch (`websocket.ts:185-195`) authorizes only by room **prefix** and
`courier:<sub>` self-match; `location:<id>` and `order:<id>` get **no** check, unlike the owner branch
which calls `ownerCanAccessRoom`. Any authenticated courier can therefore subscribe to any tenant's
`order:`/`location:` room and receive its live stream (order/status/message/GPS) — a cross-tenant leak
that needs no RLS flip, because the WS in-memory fan-out never consults RLS. A sibling REST gap
(`order-messages.ts:55-72`) scopes couriers to **location-wide membership**, not to an assignment, so a
courier reads/sends on colleagues' orders within their own shop (finding N3).

The courier JWT carries `sub` (= `courier_assignments.courier_id` = `couriers.id`) and
`activeLocationId`; the binding table `courier_assignments` carries the lifecycle statuses
`offered → assigned → accepted → picked_up` (+ terminal). The data to authorize exists; only the
predicate is missing.

## Decision

Introduce `courierCanAccessRoom(courierSub, activeLocationId, room)` mirroring `ownerCanAccessRoom`, and
a shared `courierHasBinding(db, orderId, sub, scope)` predicate used by BOTH the WS subscribe handler
and the `order-messages` REST routes:

- `courier:<sub>` → `room === courier:<sub>` OR `room.startsWith(courier:<sub>:)` (self-namespaced; covers `shiftChannel`).
- `location:*` → **DENY** (no legitimate courier location room; `dashboardChannel` is the owner feed).
- `order:<orderId>` (WS + REST read) → EXISTS `courier_assignments` for `(orderId, sub)` with
  `status IN ('offered','assigned','accepted','picked_up')` — **`offered` included** so the
  offer-handshake courier can view the order to decide accept/decline.
- `order:<orderId>` (REST message **send**) → same, but `status IN ('assigned','accepted','picked_up')`
  (an offered-only courier may read, not yet emit `cu_*`/`cc_*`); subsumes the old `hasCourier()`
  "some courier" check into "**this** courier".
- Anything else / malformed / empty id → **DENY**. Authz failure (DB error/timeout) → **fail CLOSED**.

This is **Option A — binding-scoped least-privilege**, chosen over Option B (location-scoped, coarser —
leaves the intra-tenant colleague leak open) and Option C (hybrid — collapses to A with extra surface).

In the same **atomic** PR: (1) FE must subscribe `order:${task.orderId}` (the real order id), resolving
the id-conflation in DeliveryPage (route `:id` is the **assignment** id; proposal R1 / Breaker H2). The
positive-control E2E MUST drive the REAL `/courier/delivery/:id` route — a synthetic room would mask the
denial. (2) Revoke a stale binding via **fan-out-time revalidation**, not event-eviction (see below).

**Stale-binding revocation — fan-out-time revalidation (REDESIGNED after Breaker R1 C1+C2).** The
Round-1 plan (evict on a `courierId`-bearing `binding_changed`) is **scrapped**: the emitted event is
`{type:'binding_changed', orderId}` with **no `courierId`** (verified `assignments.ts:456,511`), so a
`Set<RoomMember>` cannot target the evictee from the payload (C2); and the **victim paths emit nothing on
`order:<O>`** (owner-reassign `dashboard.ts:362/422`, `/decline`, `/reject`, offer-sweep — C1).
Instead: **before relaying a frame to a courier member of `order:<O>`** (on BOTH fan-out surfaces — the
bus room handler AND the local-Set GPS relay loop), re-derive the member's authority from the **live
binding** (the member carries `user.sub`; the DB knows `courier_id` — the evictee is targetable via
binding state, no `courierId` event needed) via the shared `courierHasBinding` predicate on a
**tenant-scoped client** (above).
- **One shared `guardedCourierRelay` helper over ALL THREE raw courier-send sites (Breaker NEW-E + R3-3):**
  the three sites that send to a courier member are (1) the **role-AGNOSTIC** bus room handler
  (`websocket.ts:36-44` — sends to every room member, no `role==='courier'` test), (2) `client_location`,
  (3) `client_location_stop` (GPS loop). All three fan out to courier-joinable rooms ONLY through the
  helper. The drift guardrail is keyed on the fan-out **site**, NOT on `role==='courier'` (which would miss
  the role-agnostic bus handler — the exact gap that re-creates the leak); a raw `member.ws.send` over a
  courier-joinable room outside the helper is a build error. Kills the Round-1 "guarded on X, open on Y"
  root cause structurally, not just by test.
- **Relay-only-on-fresh-ALLOW (Breaker NEW-C — never relay-then-revalidate):** the cache lookup is a sync
  `Map` read; a frame is relayed to a courier member ONLY on a fresh-ALLOW. Miss/stale/UNAVAILABLE →
  **withhold the frame from that member** + async re-read; the member resumes on the next fresh frame or is
  evicted. The trigger / TTL-boundary frame is never relayed to an unconfirmed member.
- **Fixed-TTL, bounded cache (Breaker NEW-D/F):** keyed `(orderId, courierSub)`, TTL ≈ 10 s **absolute, no
  refresh-on-access** (a reconnect-flap cannot keep a revoked entry warm); a **bounded LRU (~50 k) + periodic
  sweep** keeps the heap bounded (terminal-order entries don't accrete). ~1 point-read per (socket,order)
  per TTL, not per frame.
- `DENY` → evict + `{error:'binding_revoked'}`. **Self-healing on every victim path**, zero per-path
  plumbing. A **default-on** cache-bust on the involuntary **owner-reassign** path (one-line `binding_changed`)
  shrinks that path's window to broadcast-latency (counsel Rev-2); self-decline/sweep stay at TTL.

**Fail semantics — tri-state + fail-safe ceiling, never throw (Breaker H1/L2 + NEW-A).** `courierHasBinding`
returns `ALLOW`/`DENY`/`UNAVAILABLE` and never throws to the outer handler (a throw → `ws.close(1008)` →
reconnect loop). `UNAVAILABLE` (DB error/timeout) is NOT "no binding": subscribe → *retryable* soft error
(not permanent `Forbidden`, so a pool blip can't fleet-deny). Revalidation of an already-admitted member
does NOT evict on the first `UNAVAILABLE`s — **but a fail-safe ceiling evicts after a `~60 s` wall since
first-`UNAVAILABLE` (the DOMINANT bound) OR a high secondary count `N`, even under UNAVAILABLE** (Breaker
NEW-A, **load-bearing**). The ceiling fires from **in-memory state only — no DB read needed to evict**
(Breaker R3-1), so it holds even when the pool is fully starved. Without it, `UNAVAILABLE→don't-evict`
collapses the ≤TTL bound to "until DB recovers" exactly when the H1 deploy storm saturates the pool.
**🔴 Ceiling tuning (Breaker R3-4, DoD-tunable):** the Round-2 `N=3` was wrong — at ~1 Hz GPS, 3 consecutive
`UNAVAILABLE` arrive in ~3 s, bouncing a *legitimate* courier on a brief blip. The **~60 s wall-clock**
(frame-rate-independent) is the dominant/honest bound; `N` is demoted to a secondary safety set so it never
fires before the wall at peak frame rate (`N ≥ CEILING × max_frame_rate`, e.g. `N ≥ ~60` at 1 Hz, or
wall-only). Final `CEILING`+`N` recorded after the staging load test. **Honest bound: ≤ TTL (~10 s) under
DB-availability; ≤ ceiling (~60 s, wall-dominant) even under sustained DB-UNAVAILABLE.** The ceiling will,
during a >60 s incident, also bounce a legitimate courier (recoverable re-subscribe) — for customer location
data that trade favors the customer (correct asymmetry). Admission stays fail-closed on a real negative;
revocation fails safe *up to the ceiling*, then fails closed.

## Consequences

**Positive:** closes the cross-tenant AND intra-tenant colleague leak with one predicate; WS+REST
capability surfaces become identical; matches the owner pattern (reviewable); self-revoking on the next
subscribe; cheap (one indexed point-read per subscribe — subscribe is a cold path, §2 of the proposal).

**Negative / cost (honest — Breaker R3-1; the "sub-ms multiplexed point-read" framing is RETRACTED).**
Each authz read is a **connection-PINNING multi-statement transaction**
(`connect → BEGIN → set_config → SELECT → COMMIT → release`, the `assignments.ts:80-81` shape), **not** a
single Supavisor-txn-mode multiplexed query: under txn-mode pooling a `BEGIN…COMMIT` pins a server
connection for the whole tx, so the multiplexing benefit is largely lost and the **binding constraint is
the `max=20` operational pool + the ≈10 authz-concurrency semaphore**, not point-read latency. Steady-state
is fine — the fixed-TTL `(orderId,sub)` cache (shared by subscribe + fan-out + REST) yields ~1 pinning-tx
per (socket,order) per TTL (~20 tx/s MVP, ~200 tx/s at 10×, <1 conn of demand). The real pressure is the
**cold-cache deploy reconnect-storm** (~700-1000 pinning-txns/s burst): under the documented pool starvation
`T_tx` balloons and surplus authz txns time out (`UNAVAILABLE`). This is recorded as an explicit
**ACCEPT-RISK (availability), owner Architect** — because **NEW-A's fail-safe ceiling fires from in-memory
state (no DB read) and so decouples the confidentiality bound from pool health**: even if every authz tx
times out, displaced couriers are evicted within ≤60 s, so a degraded pool costs *availability* (couriers
bounce/soft-retry, recoverable), **not an unbounded leak**. The **dedicated ≈5-conn authz read pool is
PROMOTED from "optional" to a defined scaling-gate trigger** (+5 to the API connection budget): adopt when a
**staging load test shows authz-tx contention** (authz-tx pool-checkout wait > ~50 ms p95 OR operational-pool
saturation attributable to authz) **OR** the prod `authz_unavailable_ceiling` eviction rate during deploys
exceeds threshold (Breaker NEW-H / R3-1). The REST tightening could break a courier FE that relied on
location-wide reads — validated on a 2-courier staging shop before prod.

**Scaling coupling (counsel §3c — resolved by the redesign).** The Round-1 single-instance bus-eviction
coupling is **removed**: fan-out revalidation is per-instance against the shared DB (each API instance
self-heals its own room members), and the optional cache-bust accelerator is per-instance over
cross-instance PG LISTEN/NOTIFY. Multi-instance is correct by construction; the TTL floor holds
per-instance regardless of bus delivery.

**Migrations:** NONE expected (predicate is a point-read on the indexed `order_id`). Optional
forward-only composite index `courier_assignments(order_id, courier_id)` ONLY if staging `EXPLAIN`
shows a seq scan. No new RLS table, no integer-money surface.

**Deferred (separate PRs, tracked WITH severity + window — counsel §3a):**
- **N1** customer JWT order-scope on REST — **HIGH** (the REST sibling of the already-scoped customer WS
  room; a confirmed-live customer-PII gap). **Target: within 2 weeks (next sprint).** Owner Architect → human.
- **B7** owner settlement-regenerate cross-tenant — **MEDIUM**. **Target: within 1 month.** Owner Architect → human.
- **Device-retained customer PII (counsel §5):** in-PR the FE **best-effort** purges rendered customer
  GPS/address/chat from memory/DOM on `binding_revoked` (live session); a **persistent**-storage audit
  (SW cache / localStorage / IndexedDB) is deferred — "LOW — confirm-empty" (grep-triage first), next
  sprint, owner Architect.
- **Offered-courier delivery-address exposure (Breaker NEW-G):** the offer-handshake exposes the exact
  delivery address via REST before accept; a declining courier retains it (eviction cannot recall
  already-delivered PII). **ACCEPT-RISK with owner** + tracked tighten: coarsen the offered-state REST
  payload (pickup/zone/distance to decide; withhold exact street/unit until accept). LOW, owner
  Architect → human/owner.
Different actors/surfaces; bundling would dilute blast radius. Filed with deadlines so deferral is a
scheduling decision, not a quiet permanent acceptance.

**Guardrail (red→green, blocks merge):** unit table-test of `courierCanAccessRoom` (self/other/location/
offered/assigned/picked_up/unbound/terminal/malformed) + tri-state (ALLOW/DENY/UNAVAILABLE, never throw) +
the `guardedCourierRelay` state machine (ceiling: ~60 s wall **dominant** OR high secondary `N` → evict,
the wall fires before `N` at ~1 Hz — Breaker R3-4; ALLOW resets) + fixed-TTL (no slide on access) + the
**drift guardrail keyed on the fan-out SITE across all THREE raw courier-send sites** (the role-AGNOSTIC bus
handler + `client_location` + `client_location_stop`; raw `member.ws.send` over a courier-joinable room
outside the shared helper = build error — Breaker NEW-E + R3-3; NOT keyed on `role==='courier'`, which would
miss the bus handler). Plus an E2E isolation net proving: cross-tenant WS denial; cross-courier intra-tenant
WS denial; `location:` denial; a bound + offered positive control (**driving the REAL
`/courier/delivery/:id` route — synthetic room BANNED**, Breaker H2); REST N3 denial; **post-reassignment
fan-out eviction within ≤TTL on the bus handler AND both GPS relays (`client_location` +
`client_location_stop`), ZERO frames incl. the trigger/TTL-boundary frame (Breaker C1+C2+NEW-C+R3-3 —
`binding_revoked`, not socket-close)**; the fail-mode + **ceiling** (UNAVAILABLE subscribe → retryable, no
`ws.close`; revalidation-UNAVAILABLE does NOT evict on first occurrences but **DOES at the ~60 s wall**,
Breaker NEW-A); and **NOBYPASSRLS-safety run UNDER NOBYPASSRLS** (role forced NOBYPASSRLS + FORCE-RLS → a
bound courier still ALLOW/200, run red against **both** a no-`set_config` predicate AND a
`set_config`-without-`BEGIN` stub = deny-all-couriers, Breaker NEW-B + R3-2) — each proven red against
current code first. Ledger row required.
Closes the blind spot the prior owner-only "cross-tenant QA 6/6 green" left open.

## Compliance with red lines

- 🔴 tenant-isolation: tightened, fail-closed, server-authoritative (RS256 `sub`/`activeLocationId`).
- 🔴 no insecure-default flag: shipped default-secure; any kill-switch's "off" is still a secure variant.
- 🔴 RLS — **re-corrected (Breaker NEW-B; the Round-1 M1 "don't-wrap" call was WRONG).** WS fan-out is
  in-memory (RLS N/A on the relay) BUT the **authz reads** on BOTH surfaces (`courierCanAccessRoom` on
  `fastify.db`, and the `order-messages` binding predicate) read `courier_assignments`, which is **FORCE
  ROW LEVEL SECURITY** with policy `location_id = current_setting('app.current_tenant')`
  (`1790000000073…:43-47`). The Round-1 "sound regardless of BYPASSRLS" claim was **inverted**: under
  **NOBYPASSRLS** (which `feat/mvp-sensor-seams` introduces via the pg-privilege-hardening), the empty
  setting → `NULLIF→NULL` → **0 rows → deny-all-couriers fleet-wide**. **FIX:** both authz reads now set
  `set_config('app.current_tenant', activeLocationId, true)` **inside a real `BEGIN…COMMIT` transaction**
  — the **correct** convention shape on `courier_assignments` (`courier/assignments.ts:80-81`).
  `courierHasBinding` takes a **tenant-scoped client**, not `fastify.db` directly. **Defense-in-depth:**
  RLS scopes to the tenant (location), the `courier_id=$sub` predicate scopes to the binding. Because the
  reads **always** set tenant context, the gate is correct under BOTH BYPASSRLS and NOBYPASSRLS →
  **order-independent vs. B3 / the privilege-hardening** (no land-before/after dependency).
  **🔴 Impl-hazard merge-gate (Breaker R3-2):** the tenant context MUST use `BEGIN…COMMIT`, **NOT** the
  bare `set_config(…,true)`-without-`BEGIN` of `assignments.ts:111` — `is_local=true` is transaction-local,
  and a pooled client with no `BEGIN` runs each query in its own implicit autocommit tx, so the setting
  **dies before the SELECT** (deny-all under NOBYPASSRLS; the `:111` site is non-broken today only because
  of BYPASSRLS). §11 item-8 runs under NOBYPASSRLS and is RED against both a no-`set_config` and a
  no-`BEGIN` stub. Accepted edge: tenant = `activeLocationId`; a binding at a non-active location (stale
  token / mid-shift switch) is RLS-denied under NOBYPASSRLS — rare multi-location case, mirrors
  `assignments.ts`, watch-item.
- 🔴 PII: denials carry no order/tenant detail; no PII/secret added to logs. On binding revocation the FE
  **best-effort** purges rendered customer GPS/address/chat (counsel §5) — the **authoritative** boundary
  is server-side stop-sending; already-delivered PII (incl. an *offered-then-declined* courier's
  REST-fetched delivery address) is **unrecoverable** (Breaker NEW-G — accept-risk + tracked coarsen of
  the offered-state REST payload).
