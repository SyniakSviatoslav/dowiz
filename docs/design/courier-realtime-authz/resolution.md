# RESOLVE — `courier-realtime-authz` (Triadic Council, Round 1)

**Role:** System Architect (DeliveryOS) · **Date:** 2026-06-29
**Inputs:** `breaker-findings.md` (Round 1) + `counsel-opinion.md`
**Outputs updated:** `proposal.md` (§2, §7, §9, §10, §11) + `docs/adr/0013-courier-realtime-authz.md`
**Re-verified in code before resolving:** `websocket.ts:33-52,185-253`, `message-bus.ts:36-65,168-181`,
`order-messages.ts:13-28,55-72,127-134`, `courier/assignments.ts:456,511`, `owner/dashboard.ts:362,422`,
`lib/bindingRelease.ts`, `packages/db/src/index.ts:17-42` (operational pool `max=20`, restricted role,
Supavisor txn-mode multiplexed), `registry.ts:47` (`orderChannel`).

**Exit bar:** 0 unresolved CRITICAL / HIGH. **Met (pending re-attack)** — see scorecard.

---

## Scorecard

| # | Sev | Finding (one line) | Disposition | Mechanism |
|---|-----|--------------------|-------------|-----------|
| C1 | CRIT | Eviction fires on wrong paths; victim paths emit no `order:<O>` signal | **FIX (redesign)** | Fan-out-time binding revalidation (self-healing; no per-path event needed) |
| C2 | CRIT | `binding_changed` has no `courierId`; room can't target the evictee | **FIX (redesign)** | Revalidate per-member against DB (knows `sub` from RoomMember); never target by payload |
| H1 | HIGH | Reconnect-storm vs. starvation-prone operational pool; fail-closed → fleet-deny | **FIX** | Tri-state authz (deny vs. DB-unavailable→retryable), short-TTL cache, jittered reconnect, bounded authz concurrency |
| H2 | HIGH | R1 id-conflation: FE subscribes `order:<assignmentId>` → gate denies every courier | **FIX (atomic)** | Gate + FE `order:${task.orderId}` ship in ONE deploy; positive-control E2E drives the REAL `/courier/delivery/:id` route |
| M1 | MED | REST predicate runs on plain pool, NO tenant context → ADR RLS claim is fiction | **FIX (correct claim)** | State plainly: `courier_id=$sub` binding predicate is the boundary on this surface; not RLS |
| M2 | MED | Mid-stream owner-reassign uncovered (delegated to broken R2) | **FIX (subsumed by C1 redesign)** | Fan-out revalidation covers owner-reassign A→B |
| L1 | LOW | Observability watches DENY; the real leak emits no deny | **FIX** | Add an **eviction** counter (the leak-closing event) + label DENYs (probe vs. over-deny) |
| L2 | LOW | Fail-closed-by-throw closes the socket → reconnect loop amplifies H1 | **FIX** | `courierCanAccessRoom` catches internally, returns tri-state, never throws to outer handler |
| Counsel R1 | — | Watch-condition: authz before FE id-fix denies real couriers | **HONORED** | Same as H2 (atomic + real-route E2E) |
| Counsel §3a | — | B7/N1 deferred with no severity/window | **HONORED** | Tracked with severity + target window (below) |
| Counsel §3c | — | Single-instance bus-eviction is a latent scaling coupling | **RESOLVED + noted** | Redesign is per-instance self-healing → coupling REMOVED; noted in ADR |
| Counsel §5 | — | Does ex-courier device still hold customer GPS/address/chat? | **DISPOSITIONED** | In-scope: FE purges in-memory/DOM customer PII on eviction. Persistent-storage audit = follow-up |

---

## C1 + C2 — the redesign (the whole ballgame)

### Why the old §7 mechanism is dead
- **C2 is dispositive:** the emitted event is `{ type:'binding_changed', orderId }` (assignments.ts:456,511)
  — **no `courierId`**. A `Set<RoomMember>` keyed by socket cannot be told *which* member to evict from a
  payload that names no courier. The only payload-driven choices were "evict everyone" (kicks the
  legitimate replacement) or "re-query per member" (which is exactly the redesign — so admit it).
- **C1 is dispositive:** the victim paths (owner-reassign A→B `dashboard.ts:362/422`, `/decline` `:524`,
  `/reject` `:178`, offer-sweep) emit **nothing on `order:<O>`** that names a leaving courier. Plumbing a
  `courierId`-bearing control event onto every victim path is fragile (any path we forget = a silent leak)
  and still needs the room to target by identity.

**Conclusion:** an event-driven, payload-targeted eviction is the wrong shape. We do not try to fix the
event; we stop depending on it for correctness.

### Chosen mechanism — **fan-out-time binding revalidation (self-healing), short-TTL authz cache**

> **Concept:** capability re-check on the relay path. A courier's authority to *keep receiving*
> `order:<O>` is re-derived from the live binding, not from a one-time admission. This is
> self-healing on **every** victim path because it never asks "who left?" — it asks, per frame,
> per member, "does this member still hold a binding?" The DB knows `courier_id`; the RoomMember
> carries `user.sub` — so the evictee *is* targetable, via the binding state, with no `courierId`
> in any event (kills C2) and no per-victim-path plumbing (kills C1).

Two relay surfaces both gain the guard (both fan to courier members of `order:<O>`):
1. the **bus room handler** (`websocket.ts:36-45`) — `order.message`, status deltas;
2. the **GPS relay loop** (`websocket.ts:220-253`) — `client_location` / `client_location_stop`.
   (Breaker NON-finding: GPS is local-Set-only, not bus-fanned — so it must be guarded *separately*,
   in its own loop, not only in the bus handler.)

**Before relaying a frame to a `role==='courier'` member of an `order:<O>` room:**
- Consult a process-local **authz cache** keyed `(<orderId>, <courierSub>)` → `{ allowed, expiresAt }`,
  TTL **≈ 10 s**.
- **Cache hit (fresh):** use it. **Allowed** → relay. **Denied** → **evict** (remove from the room `Set`,
  send `{type:'error', error:'binding_revoked'}` once, do not relay).
- **Cache miss / stale:** run the binding point-read (`courierHasBinding`, the same shared predicate as
  the subscribe gate), then:
  - query → 0 rows (definite negative) → cache `allowed:false` → **evict**;
  - query → ≥1 row → cache `allowed:true` → relay;
  - query → **error/timeout** (DB-unavailable, *not* "no binding") → **do NOT evict** (the member was
    already admitted at subscribe); set a short retry (`expiresAt = now + 2 s`) and relay this frame.
    *Revocation fails safe* for an already-admitted member; admission stays fail-closed (below).

**Self-healing guarantee:** worst-case leak window after *any* binding terminalization (owner-reassign,
decline, reject, sweep, cancel, abort, deliver) = **≤ TTL (~10 s)**, on **every** path, with **zero**
per-path event plumbing. M2 (mid-stream owner-reassign) is now covered by construction.

**Optional accelerator (additive, NOT required for correctness):** when a `binding_changed` /
`order.status→terminal` frame is received on `order:<O>`, **bust the cache** for that order
(`cache.delete(orderId)`) so the *next* frame re-reads and evicts within broadcast-latency instead of
waiting up to TTL. Add a one-line `binding_changed` (no `courierId`) on the victim paths that lack it
(owner-reassign, decline, reject, sweep) purely to *accelerate* — never as the correctness mechanism.
A missed/forgotten event degrades the window to TTL, never to "until disconnect."

### Cost — quantified on the message hot path (the breaker's demand)
- The TTL cache makes the check **NOT a DB read per frame.** GPS streams ~1 frame/s during active
  delivery → with TTL≈10 s, that's **1 point-read per (socket,order) per 10 s**, regardless of frame rate.
- Steady DB load from revalidation: `(active courier-in-order sockets) / TTL`.
  - MVP (500 couriers, ~40 % mid-delivery = 200): **~20 reads/s**.
  - 10× (5000 couriers, 2000 mid-delivery): **~200 reads/s** — sub-ms indexed point-reads on
    `courier_assignments(order_id, courier_id)`, behind Supavisor txn-mode multiplexing over the
    `max=20` operational pool. Same order of magnitude as the H1 reconnect burst, and the **same cache**
    absorbs both (the subscribe gate, the fan-out guard, and REST all share one `(orderId,sub)` cache).
- Per-frame non-DB cost: one `Map` lookup per member — negligible.
- **Connection budget:** unchanged in the base decision (reuses the operational pool; no new pool, no
  worker/analytics/migration delta). The bounded-concurrency semaphore (H1) caps concurrent authz reads;
  a **dedicated 5-conn authz read pool** is the documented scaling-gate (below), not the default.

---

## H1 — reconnect storm + fail-closed semantics

**Tri-state authz (the core of the fix).** `courierHasBinding` distinguishes three outcomes, never throws:
| Outcome | Meaning | Subscribe gate | Fan-out revalidation |
|---|---|---|---|
| `ALLOW` | ≥1 live binding row | admit | relay |
| `DENY` | query OK, 0 rows | refuse (`Forbidden room`) — fail-closed on a *definite* negative | **evict** |
| `UNAVAILABLE` | DB error/timeout/pool-exhausted | **retryable soft error** `{error:'authz_unavailable', retryable:true}` — NOT a permanent deny | **do NOT evict** (retry in ~2 s) |

This is the key asymmetry the breaker demanded: a transient pool blip during a deploy stampede yields
*retryable* errors, **not** fleet-wide `Forbidden room`. Admission is fail-closed on a real negative;
the *revocation* path fails safe so a blip can't mass-evict legitimate mid-delivery couriers.

**Burst mitigation (defense in depth, cheapest-first):**
1. **Short-TTL authz cache** (the same `(orderId,sub)`/`(courier:self)` cache) — a courier reconnecting to
   the order they were just on hits a warm entry *within the process lifetime*; absorbs WiFi↔LTE flaps.
2. **Jittered reconnect (FE):** spread WS reconnect over a random 0–5 s window so a deploy doesn't create
   a synchronized thundering herd (turns the ~700–1000 reads/s spike into a smeared ~150–300 reads/s).
3. **Bounded authz concurrency:** a small semaphore (≈10) caps concurrent authz reads against the
   operational pool; excess subscribes queue (a few-hundred-ms delayed `subscribed`) rather than
   exhausting the pool and tripping the documented starvation mode.
4. **Scaling-gate (deferred, flagged):** a **dedicated 5-connection authz read pool** isolates authz
   point-reads from order/menu operational traffic. Quantify: 5 conns × sub-ms point-reads ≫ 1000 reads/s
   with headroom. **Not** in the base PR (YAGNI); adopt only if staging load or `EXPLAIN`/pool metrics
   show operational-pool contention from authz. Documented in ADR as the next lever. (+5 to the API
   connection budget if taken.)

**Cold-cache caveat (named, accepted):** a deploy drops the process → cache is cold → the *first*
reconnect wave is all misses. Items 2+3 (jitter + semaphore) bound that wave; item 4 is the lever if it
proves insufficient under real load. Accepted-risk with owner = Architect, re-evaluate after staging
load test.

---

## H2 — R1 atomicity (one indivisible deploy)

The WS+REST authz gate and the FE room-key fix are **one atomic unit**, not "FIX in PR" as a footnote:
- FE: `DeliveryPage.tsx:121` must subscribe `order:${task.orderId}` (the real order id), not
  `order:${id}` (the route param = **assignment** id, `CourierRoutes.tsx:88`). This requires
  `toTaskShape` (assignments.ts:24) to expose `orderId` on the task object (currently `id = row.id`).
  The order-messages REST calls (`/orders/:orderId/messages`) must likewise use `task.orderId`.
- **Hard gate (counsel R1 → enforced):** the positive-control E2E (proposal §11 item 4) MUST **navigate
  the real `/courier/delivery/:id` FE route** and assert the bound courier receives per-transition
  deltas. A synthetic hand-crafted room is BANNED for this control — it would mask the exact denial R1
  describes (authz green while a real courier is locked out). Added to §11 as a non-negotiable.
- **Deploy discipline:** do NOT merge/deploy the API gate without the FE change in the same release.
  Since FE and API are independent build artifacts, the E2E-against-real-route is the gate that makes a
  split impossible to ship green.

---

## M1 — REST predicate has NO tenant context (correct the claim)

**Verified:** `order-messages.ts` issues every query via plain `db.query` — no `client.connect()`, no
`BEGIN`, no `set_config('app.current_tenant', …)`. The ADR line "REST predicate runs under the existing
`app.current_tenant` context — verify it is inside the tenant-set" is **fiction for this route** and is
**corrected**.

**Decision (do NOT introduce a transaction/tenant-set here):** wrapping the route in `withTenant` would
change its whole stateless-query model and is scope-creep on a security-tightening PR. Instead, state
plainly: **isolation on this surface is the explicit `courier_id = $sub AND order_id = $orderId AND
status IN (...)` binding predicate — not RLS.** This is defense-in-app, and it is sound regardless of the
operational role's BYPASSRLS status (unconfirmed; memory note "verify:rls likely BYPASSRLS artifact").
The route demonstrably functions today without `app.current_tenant`, which itself confirms isolation on
this surface is app-level, not RLS-level. Corrected in ADR §"Compliance with red lines" and proposal §5.

**Follow-up (tracked):** "establish `app.current_tenant` on `order-messages` via `withTenant`" — only if
the operational role is *confirmed* non-BYPASSRLS and these tables are FORCE-RLS. Severity LOW (current
predicate is sufficient), owner Architect, window: next hardening sprint.

---

## Counsel dispositions

- **§2 R1 watch-condition:** honored as H2 — real-route positive-control E2E is now a hard gate.
- **§3a deferral horizon — B7 & N1 filed with severity + window (not silent):**
  | Ticket | Severity | Surface / actor | Target window | Owner |
  |---|---|---|---|---|
  | **N1** customer JWT order-scope on REST (`/orders/:id/messages` customer branch is order-scoped; the REST sibling of an already-scoped WS room — a *confirmed-live* customer-PII gap) | **HIGH** | order-messages REST / customer | **within 2 weeks** (next sprint) | Architect → human confirm |
  | **B7** owner settlement-regenerate cross-tenant | **MEDIUM** | settlement module / owner | **within 1 month** | Architect → human confirm |
  Recorded in ADR "Deferred" with severity + window so deferral is a *scheduling* decision, not a quiet
  permanent acceptance.
- **§3b DENY-counter dual reading:** honored in L1 — the DENY metric is labeled to separate
  attacker-probing from R1-over-deny, and an **eviction** counter is added for the actual leak class.
- **§3c single-instance bus coupling:** the redesign **removes** the coupling. Eviction is now
  per-instance DB-revalidation (each API instance independently re-checks its own room members against
  the shared DB and self-heals). The optional bus accelerator is also per-instance (each instance LISTENs
  and busts its own cache; PG LISTEN/NOTIFY is cross-instance). Multi-instance is correct by construction;
  the TTL floor holds per-instance regardless. Stated explicitly in the ADR.
- **§5 open question (device-retained customer PII):** **explicit disposition.**
  - **In-scope (this PR):** "stop-sending" is insufficient for dignity; on eviction / `binding_revoked`
    the **FE MUST purge the rendered customer PII from memory/DOM** — last-known GPS marker, address, and
    chat thread — and navigate away from the delivery view. Cheap, closes the live-session half of §5.
  - **Follow-up (tracked, LOW, next sprint, owner Architect):** audit *persistent* client storage
    (service-worker cache, localStorage, IndexedDB) for retained customer PII after binding end. Rationale
    for splitting: the live-session purge is a one-handler FE change shippable now; a storage audit is a
    separate surface with its own proof and would dilute this PR's blast radius. Not silently dropped.

---

## Re-disposition of accepted risks (proposal §10)

- **R2** "FIX via bus-driven eviction" → **REPLACED** by fan-out-time revalidation (self-healing). The
  false premise (victim paths emit `binding_changed` with `courierId`) is removed.
- **R3** "subscribe-time only; revoked-mid-stream accepted" → **NO LONGER ACCEPTED** — revalidation now
  evicts within ≤TTL on all paths. R3 is closed, not laundered.
- **M2** owner-reassign mid-run → **covered** (not an accepted edge).

---

## Exit-bar confirmation

- CRITICAL: C1 **FIX**, C2 **FIX** → **0 unresolved.**
- HIGH: H1 **FIX**, H2 **FIX** → **0 unresolved.**
- MEDIUM/LOW: M1 FIX, M2 FIX (subsumed), L1 FIX, L2 FIX.
- Counsel: 0 ETHICAL-STOP; R1 watch-condition enforced as a hard gate; all advice honored or tracked
  with severity+window.

**0 unresolved CRITICAL/HIGH — exit bar met, pending Breaker re-attack on the redesigned mechanism**
(the re-attack target is the TTL-window leak bound, the tri-state UNAVAILABLE fail-safe-on-revocation,
the cold-cache reconnect wave, and the GPS-loop guard parity with the bus-handler guard).

---

# RESOLVE — Round 2 (Triadic Council re-attack)

**Date:** 2026-06-29 · **Inputs:** `breaker-findings.md` (Round 2 — NEW-A..NEW-H) + `counsel-opinion.md`
(Revision 2). **Re-verified in code before resolving:** `courier/assignments.ts:78-191` (the existing
tenant-context pattern — every read sets `set_config('app.current_tenant', $locationId, true)` in a tx);
`packages/db/migrations/1790000000073…:36-47` (`courier_assignments` **FORCE ROW LEVEL SECURITY**, policy
`USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid)`);
`1790000000069…:19-20` (role → effectively NOBYPASSRLS on this branch).

**Round-1 verdict re-affirmed:** C1/C2 remain genuinely RESOLVED by the fan-out-revalidation redesign —
**not regressed**. H2/L1/L2 resolved-in-design. The two NEW HIGHs are fixes to the *fix*, on the same 🔴
lines. Below: per-finding mechanism + disposition.

## Round-2 scorecard

| # | Sev | Finding (one line) | Disposition | Mechanism |
|---|-----|--------------------|-------------|-----------|
| NEW-A | HIGH | `UNAVAILABLE→don't-evict` breaks the ≤TTL bound under sustained pool degradation (co-located with the H1 deploy storm) — leak becomes "until DB recovers" | **FIX** | **Fail-safe eviction ceiling**: evict an already-admitted member after `N=3` consecutive `UNAVAILABLE` **or** a hard `~60 s` wall since first-UNAVAILABLE, even under `UNAVAILABLE`. Ceiling is **load-bearing**. |
| NEW-B | HIGH | Predicate sound ONLY under BYPASSRLS; this branch removes it (NOBYPASSRLS) → FORCE-RLS policy returns 0 rows → **deny-all-couriers** fleet-wide | **FIX (reverse M1)** | **Set tenant context on BOTH authz read sites**: `set_config('app.current_tenant', activeLocationId, true)` in a tx (the exact `assignments.ts:79` convention), so the FORCE-RLS policy passes. NOBYPASSRLS-safe regardless of merge order vs. privilege-hardening. |
| NEW-C | MED | Async revalidate grafted onto SYNC fan-out loops → relay-then-revalidate leaks the trigger frame each TTL boundary | **FIX** | **Relay-only-on-fresh-ALLOW** (sync `Map` hit). Miss/stale/UNAVAILABLE → **withhold the frame from that member** + async re-read; member resumes on the next frame after the cache refreshes, or is evicted. Never relay-then-revalidate. |
| NEW-D | MED | Subscribe gate is both "must be live/uncached" AND "cache-backed"; sliding TTL + reconnect-flap → revoked entry warm forever | **FIX** | **Fixed (absolute) TTL, no refresh-on-access.** Subscribe READS the fixed-TTL cache (absorbs H1 burst); the fixed wall guarantees re-evaluation within ≤TTL on every surface; a flap cannot keep a revoked entry warm. |
| NEW-E | MED | Same nontrivial revalidate+evict logic hand-duplicated across two structurally different sites → drift re-creates the Round-1 "guarded on X, open on Y" leak | **FIX** | **One shared `guardedCourierRelay(member, frame, room)` helper**; both the bus handler and the GPS loop fan out ONLY through it. Plus a structural drift guardrail (no raw `.ws.send` to a courier member outside the helper). |
| NEW-F | LOW | Process-local cache has no size bound/sweep → unbounded heap between deploys | **FIX** | **Bounded LRU + periodic sweep**: cap entries (~50 k) drop-oldest, plus a `setInterval` sweep dropping past-`expiresAt` entries. Heap bounded independent of order volume. |
| NEW-G | LOW | "FE purge closes the live-session half" overstated; an *offered-then-declined* courier already fetched the customer **delivery address via REST** — eviction can't recall it | **ACCEPT-RISK + tracked tighten** | Reframe: real boundary = **server-side stop-sending**; FE purge is **best-effort** (honest online client only); already-delivered PII is **unrecoverable**. Tracked follow-up: coarsen the *offered*-state REST payload (zone, not exact address) pre-accept. Owner Architect → human/owner. |
| NEW-H | LOW | Cold-cache "sub-ms read" bound is circular (assumes the healthy pool that fails under starvation); the structural fix (dedicated authz pool) is deferred → degradation feeds NEW-A | **RECONCILED + scaling-gate w/ trigger** | NEW-A's **ceiling decouples the confidentiality bound from pool health** (≤60 s even under sustained UNAVAILABLE) → degradation no longer feeds an *unbounded* leak. Availability cost stays accepted-risk; dedicated 5-conn authz pool promoted to a scaling-gate with a measurable trigger. |

---

## NEW-A — fail-safe eviction ceiling (the load-bearing fix)

**The contradiction (breaker-correct):** the ≤TTL bound — the *sole* basis for accepting a known leak on
a 🔴 privacy line — and `UNAVAILABLE→don't-evict` are mutually exclusive. Under a sustained pool/DB
degradation (the documented "public-menu pool starvation" mode, **the exact moment H1's deploy storm
saturates the pool**), every revalidation of a *displaced* courier returns `UNAVAILABLE` → 0 evictions →
the displaced courier keeps receiving customer GPS (~1/s) + chat for the whole 30–60 s+ incident, fleet-
wide for every courier displaced during the window. Claimed 10 s; actual ≈ DB-recovery time (unbounded).

**Resolution — make the ceiling load-bearing, not optional.** Per (orderId, courierSub) member track
`consecutiveUnavailable` and `firstUnavailableAt`. On fan-out revalidation:
- `ALLOW` → relay; **reset** the streak (`consecutiveUnavailable=0`, clear `firstUnavailableAt`).
- `DENY` → evict immediately (`binding_revoked`) — unchanged.
- `UNAVAILABLE` → **withhold this frame** from the member (NEW-C) and:
  - `consecutiveUnavailable += 1`; set `firstUnavailableAt` if unset;
  - if `consecutiveUnavailable >= N (=3)` **OR** `now − firstUnavailableAt > CEILING (~60 s)` →
    **EVICT the member anyway** (`binding_revoked`, reason=`authz_unavailable_ceiling`), even though the
    DB never returned a definite negative.

**The honest bound (replaces "≤TTL on every path"):**
> **≤ TTL (~10 s) under DB-availability; ≤ ceiling (~60 s) even under sustained DB-UNAVAILABLE.**

**Why this is the correct asymmetry (counsel-endorsed):** `UNAVAILABLE` cannot distinguish a blip-affected
*legitimate* courier from a just-*revoked* one — so the ceiling will, during a >60 s incident, bounce a
legitimate mid-delivery courier too. That cost is **recoverable** (the courier re-subscribes; the subscribe
gate fails *safe-retryable* on UNAVAILABLE, jittered, so the bounce is a soft reconnect, not a permanent
deny). The customer-PII leak it re-bounds is **not** recoverable (location data, once streamed to a
deauthorized device, is gone). For customer location the trade favors the customer — the right asymmetry.
The ceiling is now the load-bearing confidentiality backstop; it is **not** an "optional accelerator."

**Cross-fix composition (closes the H1×NEW-A trap):** the deploy storm saturates the pool → revalidations
return UNAVAILABLE → without the ceiling, dozens of displaced couriers leak fleet-wide for the incident.
With the ceiling, that same fleet-wide displacement is bounded to ≤60 s regardless of how long the pool
stays degraded. NEW-H's "degradation feeds the leak" is thereby de-fanged at the confidentiality layer.

## NEW-B — set tenant context on BOTH authz reads (reverse the M1 "don't-wrap" decision)

**The M1 "correction" was itself false.** Round-1 said isolation here is "the `courier_id=$sub` predicate,
not RLS … sound regardless of BYPASSRLS." Verified inverted:
- `courier_assignments` is **FORCE RLS**, policy `location_id = current_setting('app.current_tenant')`
  (`1790000000073…:43-47`). Both new read sites — `courierCanAccessRoom` (WS, `fastify.db`) and the REST
  binding predicate (`order-messages.ts:22`, plain `db.query`) — set **no** tenant context.
- Under **NOBYPASSRLS** (which `feat/mvp-sensor-seams` introduces), `current_setting('app.current_tenant',
  true)` is empty → `NULLIF→NULL` → `location_id = NULL` → **0 rows for every order** → DENY → every courier
  is `Forbidden room` + 404 on every thread. **Fleet-wide outage**, not a leak.
- "Functions today" proves the *opposite* of "sound regardless": it only works because the role is
  **BYPASSRLS at runtime**. The predicate is sound **iff BYPASSRLS** — the exact inverse of the claim.

**Resolution — wrap BOTH authz reads in the established tenant-context pattern.** The convention already
exists on this very table: `assignments.ts:79/111/139/190` each do `set_config('app.current_tenant',
$locationId, true)` in a tx before reading `courier_assignments`. The Round-1 "withTenant = scope-creep"
call was **wrong** — it is the table's own house style.
- **`courierHasBinding(client, orderId, sub, scope)`** takes a **connected client with tenant context
  already set** (not `fastify.db` directly). Its two callers wrap it:
  - **WS** (`courierCanAccessRoom`): `const c = await fastify.db.connect(); try { await c.query("SELECT
    set_config('app.current_tenant',$1,true)", [activeLocationId]); …predicate… } finally { c.release() }`.
  - **REST** (`order-messages.ts`): the GET/POST/mark-read predicate reads move **into** a
    `set_config`-scoped client (the route's stateless-query model gains a short read tx — the cost NEW-B
    forces and B3 requires).
- **Tenant value = the courier's `activeLocationId`** (RS256 token claim) — exactly what `assignments.ts`
  uses as `locationId`. The binding row's `location_id` equals the courier's active location for any live
  run, so the policy admits it; the `courier_id=$sub AND order_id=$orderId AND status IN (...)` predicate
  then narrows to *this* courier's binding. **Defense-in-depth, not either/or:** RLS scopes to the tenant,
  the predicate scopes to the binding.

**B3 reconciliation / sequencing (the key ask):** because both reads now **always** set tenant context,
the gate is correct under **both** postures and therefore **order-independent** vs. the privilege-hardening:
- Under BYPASSRLS (current prod): `set_config` is harmless; the predicate still filters by courier/order.
- Under NOBYPASSRLS (post-hardening): `set_config` makes the FORCE-RLS policy pass.
- **No "land before/after B3" dependency** — this PR is NOBYPASSRLS-safe whichever lands first. That is the
  whole point of always-wrap: it removes the cross-cutting outage the Round-1 design baked in.
- **Accepted edge (flagged):** tenant = `activeLocationId`. A courier holding a binding whose `location_id
  ≠ activeLocationId` (stale token / mid-shift location switch with a still-active binding) would be
  RLS-denied their own order under NOBYPASSRLS. This mirrors the `assignments.ts` behaviour exactly (same
  tenant source), is moot under BYPASSRLS, and is a rare multi-location edge — **watch-item**, owner
  Architect; the binding-location should track the active location for any live run.

The M1 claim is **corrected** in proposal §5 + ADR "Compliance": tenant context **IS** set; the predicate
is sound under both BYPASSRLS and NOBYPASSRLS; RLS is **not** waved off — it is the outer layer.

## NEW-C — relay-only-on-fresh-ALLOW (resolve the sync/async mismatch)

Both fan-out loops are synchronous (`websocket.ts:36-45` bus handler; `:231-235`/`:246-250` GPS). A
per-member `await` before each relay would serialize fan-out and add DB latency; relay-then-revalidate
leaks the boundary frame. **Resolution — make the decision synchronous and fail-closed for the current
frame:**
- The cache lookup is a **sync `Map` read**. A frame is relayed to a courier member **only on a
  fresh-cache ALLOW** (the overwhelming majority — GPS 1/s, fixed TTL 10 s → ~9/10 frames are fresh hits).
- **Cache miss / stale / UNAVAILABLE → the frame is WITHHELD from that member** (not relayed), and an
  out-of-band async re-read is kicked off (deduped per (orderId,sub) so a burst triggers one read). The
  member resumes receiving on the **next** frame once the cache is fresh-ALLOW again, or is evicted on
  DENY/ceiling. The trigger frame is **never** relayed to an unconfirmed member.
- This is "revalidate-before-relay" realized as **relay-only-on-known-allow**: the hot path stays sync, the
  DB read gates *future* frames and never retroactively justifies the current one. A displaced courier
  receives **zero** frames from the moment their entry goes stale — including the boundary frame.
- **Cost to a legitimate courier:** at most one withheld GPS frame per TTL boundary during the sub-ms
  re-read (the next 1-Hz frame carries the latest position — self-correcting); a withheld chat frame is
  recoverable via REST history on the next fresh frame. Negligible vs. the leak it closes.

**§11 DoD pin:** the eviction E2E (item 6) MUST assert the displaced courier receives **ZERO** frames
*including the reassignment-trigger / TTL-boundary frame* on BOTH surfaces — run RED against a
relay-then-revalidate stub to prove the test bites the boundary-frame leak.

## NEW-D — fixed (absolute) TTL; correct the "do not cache" contradiction

§6/§2's "re-evaluates live each time … another reason not to cache" contradicts the H1 cache. Resolution:
- The authz cache uses a **FIXED absolute TTL** (`expiresAt = createdAt + TTL`), **never refreshed on
  access**. A flapping courier (WiFi↔LTE reconnect every <10 s) cannot keep a revoked entry warm — the
  hard wall forces a re-read within ≤TTL regardless of access frequency.
- **Subscribe READS the fixed-TTL cache** (so the reconnect storm is absorbed — H1). A stale ALLOW
  re-admitting a just-revoked courier is bounded: the **same fixed-TTL entry** the fan-out guard reads
  expires at the same wall → re-read → DENY → evict within ≤TTL. So subscribe-on-stale-ALLOW is **≤TTL
  bounded by the fan-out guard**, not a new unbounded hole.
- **Corrected wording (proposal §2/§6):** drop "not to cache / live each time." The revocation bound on
  *every* surface (subscribe, bus fan-out, GPS fan-out, REST) is **≤TTL via the fixed-TTL re-read +
  fan-out eviction** (and ≤ceiling under sustained UNAVAILABLE, NEW-A). The cache is correctness-preserving
  *because* the TTL is fixed and short, not despite caching.

## NEW-E — one shared guarded-relay helper + drift guardrail

Round-1's root cause was "two fan-out paths, one guarded." Resolution: a single
`guardedCourierRelay(member, frame, room)` encapsulating the **entire** decision (fixed-TTL `Map` lookup →
fresh-ALLOW relay / fresh-DENY evict / miss-stale-UNAVAILABLE withhold + async re-read + ceiling counters +
`binding_revoked` emit). **Both** the bus room handler and the GPS relay loop fan out to courier members
**only** through it — neither hand-rolls cache/evict logic. Backstops:
- **Structural drift guardrail** (`tools/eslint-plugin-local` rule or a grep test): inside the courier
  fan-out sites, a raw `member.ws.send(...)` to a `role==='courier'` member outside `guardedCourierRelay`
  is a build error. This makes "guarded on X, open on Y" un-shippable, not just test-covered.
- **§11 item 6 covers BOTH surfaces** (already pinned) — the behavioural backstop.

## NEW-F / NEW-G / NEW-H — LOW dispositions

- **NEW-F (FIX):** the authz cache is a **bounded LRU** (cap ~50 k, drop-oldest) **+** a periodic
  `setInterval` sweep (~30–60 s) dropping past-`expiresAt` entries. Heap bounded independent of order
  volume / deploy cadence. Specified in proposal §7 + ADR.
- **NEW-G (ACCEPT-RISK + tracked tighten):** state plainly — the FE purge is **best-effort** (enforceable
  only on an honest, online, unmodified client); the **authoritative** privacy boundary is server-side
  **stop-sending**. Already-delivered customer PII is **unrecoverable** — specifically, an
  *offered-then-declined* courier has already fetched the customer **delivery address via REST**
  (`/courier/assignments/:id`) onto the device; eviction cannot recall it. CPII is **re-labeled**: "live-
  session best-effort purge + accept that already-delivered PII is unrecoverable" — **not** "closes the
  live-session half." Tracked follow-up (LOW, owner Architect → human/owner): **coarsen the offered-state
  REST payload** (expose pickup/zone/distance to decide, withhold exact street/unit until accept) so a
  declining courier never holds the precise address. Accept-risk-with-owner until then.
- **NEW-H (RECONCILED + scaling-gate with a trigger):** the cold-cache "sub-ms read" bound *is* circular —
  but **NEW-A's ceiling removes the confidentiality dependency on pool health**: even when the pool starves
  and reads time out (`UNAVAILABLE`), displaced couriers are evicted within ≤60 s. So degradation now costs
  **availability** (couriers' feeds soft-degrade / bounce during an at-scale cold-cache deploy —
  recoverable), **not an unbounded confidentiality leak**. The dedicated **5-conn authz read pool** is
  promoted from "maybe" to a **scaling-gate with a measurable trigger**: adopt if staging load test shows
  operational-pool contention from authz **OR** the production ceiling-eviction (`authz_unavailable_ceiling`)
  rate during deploys exceeds threshold (honest signal that couriers are being bounced). Owner Architect,
  +5 to the API connection budget if taken, re-eval post load-test.

---

## Counsel (Revision 2) dispositions

- **B(1) ≤10 s window — not a STOP:** honored; the window is now stated honestly (≤TTL / ≤ceiling) and the
  *involuntary* owner-reassign path gets the **default-on cache-bust** (counsel's "symmetric-dignity" nudge
  promoted from optional to SHOULD on that one path; self-decline/sweep stay at TTL).
- **B(2) persistent-storage purge — honest LOW:** kept deferred; ticket re-labeled "LOW — confirm-empty"
  with the grep-triage as step 1 (counsel C evidence: `sw.js` excludes `/api/`+`/ws/`; `DeliveryPage.tsx`
  holds customer data in React state only). NEW-G adds the *offered REST address* caveat counsel's audit
  did not cover (that PII is device-resident regardless of SW/localStorage).
- **B(3) "≤TTL on every path" overstated — corrected:** done in NEW-A; the spec now names the
  DB-unavailable exception AND caps it with the hard ceiling counsel offered (taken as mandate, not offer).
- **0 ETHICAL-STOP** prior or new (counsel Revision 2). The NEW-A ceiling tightens, not loosens, the 🔴 line.

---

## Round-2 exit-bar confirmation

- **CRITICAL:** C1/C2 remain RESOLVED (not regressed) → **0 unresolved.**
- **HIGH:** NEW-A **FIX** (ceiling, load-bearing) · NEW-B **FIX** (tenant context on both reads,
  order-independent vs. B3) → **0 unresolved.**
- **MEDIUM:** NEW-C FIX (relay-only-on-fresh-ALLOW) · NEW-D FIX (fixed TTL) · NEW-E FIX (shared helper +
  drift guardrail).
- **LOW:** NEW-F FIX (bounded LRU + sweep) · NEW-G ACCEPT-RISK + tracked tighten · NEW-H reconciled +
  scaling-gate trigger.

**0 unresolved CRITICAL/HIGH — exit bar met, pending Breaker Round-3 re-attack.** The honest residual the
re-attack should target: the ceiling's `N=3 / ~60 s` constants (tune under load), the relay-only-on-fresh
withheld-frame UX for legitimate couriers at TTL boundaries, the `activeLocationId`-as-tenant multi-location
edge, and the offered-REST-address accept-risk (NEW-G) pending the coarsen-payload follow-up.

---

# RESOLVE — Round 3 (Triadic Council, FINAL convergence)

**Date:** 2026-06-29 · **Inputs:** Breaker Round-3 (residuals R3-1..R3-4) + Counsel Round-3.
**Breaker R3 verdict:** 🔴 confidentiality/tenant-isolation exit bar **MET** — 0 CRITICAL, 0 HIGH security,
0 ETHICAL-STOP. R3-1..R3-4 are **NO-DESIGN-CHANGE** residuals: back-of-envelope honesty (a hard-exit
criterion), an impl-hazard merge-gate, a guardrail-coverage gap, and a tuning constant. All four closed
below; no security finding re-opened.
**Re-verified in code before resolving:** `courier/assignments.ts:80-81` (correct `BEGIN`→`set_config(…,true)`
→…→`COMMIT` shape) vs. `:111` (bare `set_config(…,true)` with **NO** `BEGIN` — autocommit kills the local
setting before the SELECT); `websocket.ts:36-44` (bus room handler fans `payload` to **every** room member
ROLE-AGNOSTICALLY — `for (const m of members) … m.ws.send(payload)`, no `role==='courier'` test).

## Round-3 scorecard

| # | Class | Residual (one line) | Disposition | Owner |
|---|-------|---------------------|-------------|-------|
| R3-1 | back-of-envelope honesty (hard-exit) | §2 "sub-ms multiplexed point-reads" is false — NEW-B makes each authz read a connection-PINNING tx, not a Supavisor-txn-mode multiplexed query | **FIX (honesty) + ACCEPT-RISK (availability)** | Architect |
| R3-2 | impl-hazard (merge-gate) | tenant ctx must be a real `BEGIN…COMMIT` (`:80-81`), NOT bare `set_config(…,true)` (`:111`) — the latter dies before SELECT → silent NEW-B deny-all under NOBYPASSRLS | **FIX (merge-gate + DoD pin)** | impl PR |
| R3-3 | guardrail gap (merge-gate) | NEW-E drift rule keyed on `role==='courier'` MISSES the role-agnostic bus handler; there are THREE raw courier-send sites, not two | **FIX (re-key guardrail + 3-site helper)** | impl PR |
| R3-4 | tuning (DoD) | `N=3` at ~1 Hz GPS ⇒ ~3 s effective ceiling → bounces legit couriers on a brief blip | **FIX (re-tune; wall-dominant)** | impl PR / Architect |

## R3-1 — back-of-envelope re-derived HONESTLY (proposal §2 rewritten)

**The false claim, retracted.** §2 leaned on "sub-ms indexed point-reads behind Supavisor txn-mode
multiplexing." NEW-B forces tenant context, so each authz read is **not** one multiplexed statement — it is a
**connection-PINNING multi-statement transaction**: `connect → BEGIN → set_config('app.current_tenant',
activeLocationId, true) → SELECT … → COMMIT → release`. Under Supavisor **txn-mode** a `BEGIN…COMMIT` pins a
server connection for the whole transaction, so the multiplexing benefit the estimate assumed is **largely
lost**. The binding constraint is therefore the **`max=20` operational pool** (shared with all order/menu
request handlers) **+ the ≈10 authz-concurrency semaphore**, not point-read latency.

**Honest budget:**
- **Steady-state — fine.** The fixed-TTL `(orderId,sub)` cache (shared by subscribe gate + fan-out guard +
  REST) yields ~1 pinning-tx per (socket,order) per TTL: ~20 tx/s MVP, ~200 tx/s at 10×. At ~2-4 ms/tx
  (4 RTT + checkout) the concurrency demand is `rate × T_tx ≈ 200 × 0.003 ≈ <1` pool conn. Comfortable.
- **The real pressure — cold-cache deploy reconnect storm.** A deploy drops every WS socket with a cold
  cache; ~40 % mid-delivery couriers re-subscribe, all as misses → **~700-1000 authz pinning-txns/s burst**.
  If `T_tx` stays ~3 ms the semaphore-capped (≈10) demand is satisfiable; but under the *documented pool
  starvation* the burst can trigger, `T_tx` balloons to tens/hundreds of ms → required concurrency
  `rate × T_tx` blows past both the semaphore and the 20-conn pool → surplus authz txns **time out →
  `UNAVAILABLE`**. This is now stated as the connection-budget pressure, not hidden behind "sub-ms."
- **Cumulative API connection picture:** API request handlers + WS-authz txns + worker + analytics +
  migrations each draw on their pools; this PR adds only WS-authz txn load to the **API** operational pool
  (worker/analytics/migration budgets untouched — no migration, no worker change).

**Why it stays OFF the confidentiality line (the load-bearing reconciliation).** NEW-A's fail-safe eviction
ceiling fires from **in-memory state ONLY** (`consecutiveUnavailable` counter + the `~60 s` wall-clock since
`firstUnavailableAt`) — **eviction needs ZERO DB reads.** So even if the burst starves the pool and *every*
authz pinning-tx times out, a displaced courier is still evicted within ≤ ceiling from purely local state.
The cold-cache wave therefore costs **availability** (subscribes soft-retry; admitted members bounce at the
≤60 s ceiling — recoverable), **NOT an unbounded confidentiality leak.** Confirmed: the in-memory ceiling
keeps the burst off the 🔴 line.

**Disposition:**
- **FIX (honesty):** proposal §2 + ADR "Negative/cost" rewritten to the connection-pinning-tx framing.
- **ACCEPT-RISK (availability), owner Architect:** the deploy-storm authz-tx pool pressure (couriers bounce /
  soft-retry under a starved pool). Recorded in §10 H1c/R3-1.
- **Scaling-gate PROMOTED** from "optional" to a **defined trigger** (owner Architect): adopt the dedicated
  **≈5-conn authz read pool** (+5 API connection budget) when a **staging load test shows authz-tx
  contention** — authz-tx pool-checkout wait > ~50 ms p95 OR operational-pool saturation attributable to
  authz under the deploy-storm replay — **OR** the prod `authz_unavailable_ceiling` eviction rate during
  deploys exceeds threshold. No longer a maybe; it has a measurable firing condition.

## R3-2 — mandate the real `BEGIN…COMMIT` tx (impl-hazard merge-gate)

**Verified inverted-trap.** `assignments.ts:80-81` is the **correct** shape: `BEGIN` → `set_config(…,true)`
→ SELECT → `COMMIT`. `assignments.ts:111` is the **broken** shape: bare `set_config('app.current_tenant',
$loc, true)` with **NO** `BEGIN`. Because `is_local=true` is **transaction-local** and a pooled client with
no explicit `BEGIN` runs each `client.query` in its **own implicit autocommit tx**, the setting **dies when
the `set_config` statement's implicit tx commits**, BEFORE the SELECT runs in a fresh implicit tx with
`app.current_tenant` empty again. Under NOBYPASSRLS this is `NULLIF→NULL` → 0 rows → **silent NEW-B
deny-all**. The `:111` site is non-broken **today only because of BYPASSRLS** — it detonates the moment the
privilege-hardening lands.

**Disposition — FIX (merge-gate + DoD pin):**
- `courierHasBinding` MUST be invoked on a client whose tenant context was set **inside a real
  `BEGIN…COMMIT` transaction** (the `:80-81` shape). The bare-`set_config`-without-`BEGIN` (`:111`) shape is
  **banned** for the new authz reads. Corrected in proposal §5 + ADR Compliance (the earlier citation of
  `:111` as "the convention" is removed; `:80-81` is the convention).
- **§11 item-8 pinned to run UNDER NOBYPASSRLS**, RED against **two** stubs: (a) no-`set_config` (no tenant
  context), and (b) `set_config`-without-`BEGIN` (the `:111` shape) — both produce deny-all under
  NOBYPASSRLS, proving the gate would have silently broken. A unit/contract test additionally asserts the
  predicate sees a non-empty `app.current_tenant` at SELECT time.

## R3-3 — re-key the drift guardrail to cover ALL THREE courier-send sites (merge-gate)

**Verified gap.** The NEW-E guardrail was specified as "raw `member.ws.send` to a `role==='courier'` member
outside the helper = build error." But `websocket.ts:36-44` (the bus room handler) fans `order.message` +
status to room members **ROLE-AGNOSTICALLY** — `for (const m of members) { if (OPEN) m.ws.send(payload); }`
with **no** `role==='courier'` test. A guardrail keyed on a `role==='courier'` send-shape would **not match**
this site → the exact Round-1 "guarded on X, open on Y" leak re-appears through the un-matched bus handler.
And there are **THREE** raw courier-send sites, not two: (1) the role-agnostic bus handler, (2)
`client_location` (GPS loop), (3) `client_location_stop` (GPS loop).

**Disposition — FIX (re-key + 3-site helper):**
- The shared `guardedCourierRelay` helper wraps **all three** sites; each `order:<O>` fan-out to a
  courier-joinable room routes members through it.
- The drift guardrail is **re-keyed on the fan-out SITE** (the bus handler + both GPS relays), **NOT** on a
  `role==='courier'` test: any raw `member.ws.send` over a courier-joinable room, in any of the three sites,
  outside the helper = build error. Updated in proposal §7 + §11 item-6 + ADR.
- **§11 item-6 behaviourally backstops** all three: post-reassignment ZERO frames on the bus handler
  (message/status trigger frame) AND on `client_location` AND `client_location_stop`.

## R3-4 — re-tune the ceiling so the ~60 s wall dominates (DoD-tunable)

**Verified mis-tune.** With `N=3` and a ~1 Hz GPS stream, 3 consecutive `UNAVAILABLE` arrive in **~3 s**, so
the intended ~60 s fail-safe degenerates to a **3 s** effective ceiling — a brief mid-delivery DB blip would
bounce a **legitimate** courier after 3 s. The count-bound, not the wall, was dominating — backwards.

**Disposition — FIX (DoD-tunable):** make the **~60 s wall-clock** (`now − firstUnavailableAt`, which is
**frame-rate-independent**) the **dominant** bound. Demote the consecutive-count `N` to a secondary safety
that must NOT fire before the wall under normal frame rates: `N ≥ CEILING × max_frame_rate` (≈60 at 1 Hz),
or drop `N` and bound on the wall alone. **Target (DoD-tunable):** effective ceiling ≈ 60 s, NOT ~3 s; final
`CEILING`+`N` recorded after the staging load test, with the invariant "`N` never fires before the wall at
the observed peak frame rate." A unit test asserts the wall fires first at ~1 Hz (a 3 s legit-bounce = RED).
Updated in proposal §7 + §11 unit block + §10 (new R3-4 row) + ADR fail-semantics.

## Counsel Round-3

- **0 ETHICAL-STOP.** R3-1..R3-4 tighten honesty/coverage/tuning; none loosens the 🔴 line. The R3-4
  re-tune *reduces* wrongful bouncing of legitimate couriers (a dignity improvement) while preserving the
  ≤60 s confidentiality wall — strictly better on both axes.
- No new watch-condition; the prior watch-items (offered-REST address NEW-G, multi-location tenant edge)
  remain owned and tracked (below).

## Round-3 exit-bar confirmation

- **CRITICAL/HIGH:** none re-opened; R3-1..R3-4 are honesty/impl/guardrail/tuning, not security findings →
  **0 unresolved CRITICAL/HIGH.**
- **Back-of-envelope:** now converges **honestly** — connection-pinning-tx framing, real pool pressure
  named, in-memory ceiling confirmed keeping the burst off the confidentiality line, scaling-gate trigger
  defined. **Hard-exit criterion MET.**
- **0 ETHICAL-STOP.**
- **All residuals owned** (table above + §10).

---

## STOP-DESIGN-B — hardened plan

> Design FROZEN. This is the package the fix PR implements; every DoD item is proven **RED first**, then
> green, each with a `docs/regressions/REGRESSION-LEDGER.md` row. NO production code in this design change.

### 1. Hardened plan summary — the final authz model

- **`courierCanAccessRoom(courierSub, activeLocationId, room)`** (mirrors `ownerCanAccessRoom`) + a shared
  **`courierHasBinding(client, orderId, sub, scope)`** predicate, **tenant-scoped**:
  - `courier:<sub>` → `room === courier:<sub>` OR `room.startsWith(courier:<sub>:)` (self-namespaced).
  - `location:*` → **DENY** (couriers have no legitimate location room).
  - `order:<orderId>` (WS + REST read) → EXISTS `courier_assignments(orderId, sub)` with
    `status IN ('offered','assigned','accepted','picked_up')` — `offered` included for the handshake.
  - `order:<orderId>` (REST send) → same but `status IN ('assigned','accepted','picked_up')` (offered may
    read, not yet emit `cu_*`/`cc_*`). Subsumes the old `hasCourier()` "some courier" into "**this** courier".
  - Anything else / malformed / empty id → **DENY**.
- **Tenant context (NEW-B + R3-2):** every authz read runs inside a real `BEGIN → set_config(
  'app.current_tenant', activeLocationId, true) → SELECT → COMMIT` tx (the `assignments.ts:80-81` shape — the
  `:111` bare-`set_config` shape is BANNED). Defense-in-depth: RLS scopes to the tenant, the
  `courier_id=$sub` predicate scopes to the binding. NOBYPASSRLS-safe, order-independent vs. B3.
- **Stale-binding revocation = fan-out-time revalidation** (NOT event-eviction): re-derive each courier
  member's authority from the live binding on the relay path.
  - **Fixed (absolute) ~10 s TTL authz cache**, keyed `(orderId, courierSub)`, **never refreshed on access**
    (a reconnect-flap can't keep a revoked entry warm); **bounded LRU (~50 k) + periodic sweep** (heap
    bounded). Shared by subscribe + fan-out + REST.
  - **Relay-only-on-fresh-ALLOW** (sync `Map` read): miss/stale/`UNAVAILABLE` → **withhold** the frame from
    that member + async re-read; never relay-then-revalidate; the trigger/TTL-boundary frame is never relayed
    to an unconfirmed member.
  - **`DENY` → evict** (`binding_revoked`, not socket-close). **Fail-safe eviction ceiling (in-memory, no DB
    read):** under sustained `UNAVAILABLE`, evict after the **~60 s wall (dominant)** OR a high secondary `N`
    (R3-4: `N` never fires before the wall at ~1 Hz). Honest bound: **≤ TTL (~10 s) DB-available; ≤ ceiling
    (~60 s) under sustained DB-UNAVAILABLE.**
  - **Default-on cache-bust** on the involuntary owner-reassign path (broadcast-latency window); self-decline/
    sweep stay at TTL.
- **Tri-state authz** (`ALLOW`/`DENY`/`UNAVAILABLE`, never throws): subscribe-`UNAVAILABLE` → *retryable*
  soft error (not fleet `Forbidden`); admission fail-closed on a real negative.
- **ONE shared `guardedCourierRelay(member, frame, room)` over ALL THREE raw courier-send sites** (R3-3):
  the role-AGNOSTIC bus room handler (`websocket.ts:36-44`) + `client_location` + `client_location_stop`.
  Drift guardrail keyed on the fan-out **site** (not `role==='courier'`).
- **Atomic FE id-fix (R1):** FE subscribes `order:${task.orderId}` (real order id, not the assignment-id
  route param); positive-control E2E drives the REAL `/courier/delivery/:id` route (synthetic room BANNED).

### 2. Accepted-risks — WITH owners

| Risk | Disposition | Owner |
|------|-------------|-------|
| **R3-1 authz-tx pool pressure (availability):** cold-cache deploy storm → ~700-1000 connection-pinning authz txns/s on the `max=20` pool; under starvation `T_tx` balloons → `UNAVAILABLE`. | **ACCEPT-RISK (availability).** Confidentiality is OFF this line — NEW-A's ceiling evicts from in-memory state (no DB read) within ≤60 s. Mitigations: jittered reconnect + ≈10 semaphore. **Dedicated ≈5-conn authz pool = defined scaling-gate trigger** (staging authz-tx contention OR prod ceiling-eviction rate > threshold; +5 API budget). Re-eval post staging load test. | **Architect** |
| **NEW-G offered-courier REST-fetched address retention:** the offer-handshake exposes the exact delivery address via REST before accept; a declining courier retains it; eviction can't recall already-delivered PII. | **ACCEPT-RISK + tracked tighten:** authoritative boundary = server-side stop-sending; FE purge is best-effort; already-delivered PII unrecoverable. Follow-up: coarsen the *offered*-state REST payload (pickup/zone/distance to decide; withhold exact street/unit until accept). LOW. | **Architect → human/owner** |
| **R3-4 ceiling tuning:** final `CEILING`/`N` constants pending load data. | **ACCEPT (DoD-tunable):** ship with wall-dominant (~60 s), `N` set so it never fires before the wall at peak frame rate; record finals post staging load test. | **Architect** |
| **activeLocationId-as-tenant multi-location edge:** a binding whose `location_id ≠ activeLocationId` (stale token / mid-shift switch) is RLS-denied under NOBYPASSRLS. | **ACCEPT (watch-item):** rare, moot under BYPASSRLS, mirrors `assignments.ts` exactly; binding-location should track active location for any live run. | **Architect** |

### 3. Threat-model → red→green DoD (port into the fix PR; each proven RED first + a LEDGER row)

| # | Test (threat) | RED-first proof it bites |
|---|---------------|--------------------------|
| 1 | Cross-TENANT WS denial: courier of B subscribes `order:<A's order>` → `Forbidden room` + **ZERO** msgs across A's transitions (status/message/GPS). | RED against current code (leak / `subscribed`). |
| 2 | Cross-COURIER intra-tenant WS denial: courier X (no binding) on `order:<Y's order>` → `Forbidden` + ZERO msgs. | RED (the gap the owner-only QA missed). |
| 3 | `location:` denial: courier subscribes `location:<own>:dashboard` / `location:<any>` → `Forbidden`. | RED against current code (no check). |
| 4 | Positive control (REAL `/courier/delivery/:id` route — synthetic room BANNED): bound courier sees the run + each per-transition delta; **offered**-state control: offered courier can read + GET messages, cannot POST `cu_*`. | RED if the FE id-conflation (R1) ships without the gate (real courier locked out). |
| 5 | REST N3: courier X `GET`/`POST .../<Y's order>/messages` → 404/409; bound courier → 200. | RED against current location-wide `hasCourier`. |
| 6 | Revocation/eviction (C1+C2+NEW-C+**R3-3 three sites**): owner reassigns O A→B → within ≤TTL A receives ZERO further frames **incl. the reassignment-trigger / TTL-boundary frame**, on the **bus handler** AND `client_location` AND `client_location_stop`; A got `binding_revoked` (not socket-close); B keeps receiving. **Drift guardrail** (3-site, fan-out-site-keyed). | RED against a *relay-then-revalidate* stub (boundary-frame leak) AND against a role-keyed guardrail that misses the bus handler. |
| 7 | Fail-mode + **ceiling** (H1/L2 + NEW-A + **R3-4**): subscribe-`UNAVAILABLE` → retryable, no `ws.close`; revalidation-`UNAVAILABLE` does NOT evict on first occurrences but DOES at the **~60 s wall**; **the wall fires before `N` at ~1 Hz** (a 3 s legit-bounce = RED). | RED against a never-evict-on-UNAVAILABLE stub AND against `N=3` (3 s bounce). |
| 8 | **NOBYPASSRLS-safety — run UNDER NOBYPASSRLS** (NEW-B + **R3-2**): role forced NOBYPASSRLS + FORCE-RLS → bound courier still ALLOW/200. | RED against **two** stubs: no-`set_config`, AND `set_config`-without-`BEGIN` (the `:111` shape) — both deny-all. |

Unit (fast, deterministic): `courierHasBinding` table-test (self/other/location/offered/assigned/accepted/
picked_up/unbound/terminal/malformed) + tri-state (ALLOW/DENY/UNAVAILABLE, never throw) + the
`guardedCourierRelay` ceiling state machine (wall-dominant; ALLOW resets) + fixed-TTL (no slide) +
tenant-context-at-SELECT (non-empty `app.current_tenant`, RED against no-`BEGIN`).

### 4. Companion in-PR items + deferred tickets

**In-PR (atomic with the gate):**
- **R1 — atomic FE orderId:** FE subscribes `order:${task.orderId}`; `toTaskShape` exposes `orderId`;
  order-messages REST calls use `task.orderId`. Same deploy as the gate (E2E-against-real-route makes a split
  un-shippable-green).
- **FE rendered-PII purge (best-effort):** on `binding_revoked`, FE purges rendered customer GPS/address/chat
  from memory/DOM + navigates away. Best-effort (honest online client only); authoritative boundary =
  server-side stop-sending; already-delivered PII unrecoverable (NEW-G).

**Deferred tickets (filed WITH severity + window — counsel §3a):**
- **N1** customer JWT order-scope on REST (`/orders/:id/messages` customer branch) — **HIGH**, **within 2
  weeks (next sprint)**, owner Architect → human. (Confirmed-live customer-PII gap; REST sibling of the
  already-scoped customer WS room.)
- **B7** owner settlement-regenerate cross-tenant — **MEDIUM**, **within 1 month**, owner Architect → human.
- **NEW-G** coarsen offered-state REST payload (zone not exact address pre-accept) — LOW, owner
  Architect → human/owner.
- **CPII-2** persistent client-storage audit (SW cache / localStorage / IndexedDB) — "LOW — confirm-empty"
  (grep-triage first), next sprint, owner Architect.

---

## Round-3 hard-exit — explicit confirmation

- **0 CRITICAL / 0 HIGH** (none re-opened; R3-1..R3-4 are honesty/impl/guardrail/tuning, not new security
  findings).
- **0 ETHICAL-STOP** (the R3-4 re-tune is strictly better — less wrongful bouncing AND the ≤60 s wall held).
- **Back-of-envelope converges HONESTLY** — connection-pinning-tx framing, real `max=20`-pool pressure named,
  in-memory ceiling confirmed keeping the burst off the confidentiality line, dedicated authz pool promoted
  to a triggered scaling-gate.
- **All residuals owned** (R3-1 → Architect; R3-2/R3-3 → impl PR merge-gates; R3-4 → impl PR / Architect;
  NEW-G + multi-location edge → Architect/owner; N1/B7/CPII-2 deferred with windows).

**STOP-DESIGN-B: design FROZEN, ready for the fix PR's red→green DoD.**
