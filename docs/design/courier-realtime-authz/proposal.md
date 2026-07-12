# Design Proposal — `courier-realtime-authz`

**Status:** PROPOSED (design-time — Triadic Council). NO production code in this PR.
**Author:** System Architect (DeliveryOS)
**Companion ADR:** `docs/adr/0013-courier-realtime-authz.md`
**Surfaces:** `apps/api/src/websocket.ts` (WS subscribe), `apps/api/src/routes/order-messages.ts` (REST threads).
**Does NOT touch ADRs it would contradict** — extends the owner-authz pattern (`ownerCanAccessRoom`,
websocket.ts:82-109) and the offer-handshake binding model (ADR-deliver-v2-cash-as-proof, dispatch.ts).

---

## 1. Problem + non-goals

### Problem (verified in code)

`apps/api/src/websocket.ts:185-195` — the **courier** `subscribe` branch validates only the room
**prefix** (`courier:` / `location:` / `order:`) and that a `courier:` room equals `courier:${sub}`.
For `location:<id>` and `order:<id>` it performs **NO authorization**. The owner branch (`:175-184`)
calls `ownerCanAccessRoom(ownerId, room)`; the courier branch has no equivalent.

Consequence: **any authenticated courier can `subscribe` to any tenant's `order:<id>`** (and any
`location:…` room) and receive that tenant's live stream — `order.created` (server.ts:589),
`assignment.created` (dispatch.ts:54), status deltas, `order.message` (order-messages.ts:106),
customer GPS relays (websocket.ts:220-253). This is a **live cross-tenant data leak that needs no
RLS flip** — the WS fan-out path never consults RLS; the in-memory room is the only gate, and that
gate is open for couriers.

A sibling gap exists at REST: `order-messages.ts:55-72` (and the GET history `:126-134`, mark-read
`:157-165`) scope the courier to **location-wide membership**, NOT to an active assignment row. So a
courier reads/sends on **orders not assigned to them** within their own shop (intra-tenant
over-broad). The `hasCourier()` guard (`:21-28`) only proves *some* courier is assigned, not *this*
courier — a courier can fire `cu_*` / `cc_*` presets on a colleague's order. (= finding **N3**.)

### Root cause

Couriers were added to the WS allow-list by prefix without porting the owner's per-row authz
predicate. The token carries `sub` (= `courier_assignments.courier_id` = `couriers.id`, confirmed by
assignments.ts using `request.user.sub` as `ca.courier_id`) and `activeLocationId`, so the data to
authorize is present; only the check is missing.

### Non-goals

- **Not** re-architecting the WS transport, room model, or the messageBus.
- **Not** moving WS authz into RLS (the fan-out path is in-memory; RLS does not apply there).
- **Not** fixing B7 (owner settlement-regenerate cross-tenant) or N1 (customer JWT order-scope on
  REST) here — RELATED, different surfaces/actors, tracked as separate PRs (§10).
- **Not** introducing a feature flag that defaults to the insecure path — this is a security fix,
  shipped default-secure (§9).

---

## 2. Back-of-envelope

**Where authz runs:** ONCE per `subscribe` (cold path), never per delivered message (the hot path is
the in-memory `Set` fan-out in `subscribeToRoom`'s handler, untouched).

**Subscribe rate (fleet):**
- A courier opens `courier:<sub>` ~once per app session, and `order:<orderId>` ~once per task
  (open DeliveryPage) + reconnects.
- Plausible MVP scale: 100 locations × ~5 active couriers = 500 couriers; a busy courier runs
  ~3-5 deliveries/hr → ≈ 2 500 task-opens/hr across the fleet ≈ **<1 subscribe/sec steady**, with
  reconnect bursts (WiFi→LTE handoff) maybe ×10 for a few seconds → **~10/sec peak**.
- Even at 10× growth (5 000 couriers) → ~10/sec steady, ~100/sec burst. Trivial.

**Per-subscribe authz cost — HONEST shape (Breaker R3-1; the earlier "sub-ms multiplexed point-read"
framing was optimistic and is RETRACTED).** NEW-B forces tenant context, so each authz read is **not** a
single Supavisor-txn-mode multiplexed query — it is a **connection-PINNING multi-statement transaction**:
`connect → BEGIN → set_config('app.current_tenant', activeLocationId, true) → SELECT … → COMMIT → release`
(the `assignments.ts:80-81` convention — a real tx, **not** the bare `set_config(…,true)` of
`assignments.ts:111`, which under autocommit loses the local setting before the SELECT; see R3-2 / §11).
Consequences for the budget:
- The `SELECT` itself is still a sub-ms indexed point-read on `courier_assignments(order_id, courier_id)`
  (≤3 rows/order, buffer-cache resident) — but it is wrapped in **~4 round-trips** (checkout, BEGIN,
  set_config, SELECT, COMMIT) and **holds one operational-pool connection for the whole tx**, not for one
  multiplexed statement. Under Supavisor **txn-mode** pooling a `BEGIN…COMMIT` pins a server connection for
  the entire transaction, so the multiplexing benefit the old §2 leaned on is **largely lost** for these
  reads. The binding constraint is therefore the **`max=20` operational pool** (shared with all order/menu
  request handlers) **+ the bounded-authz-concurrency semaphore (≈10)** — not raw point-read latency.
- The **fixed-TTL authz cache (§6/§7)** still absorbs repeats so steady-state is ~1 pinning-tx per
  (socket,order) per TTL, **not** per frame.

**Fan-out-time revalidation cost (the message hot path — added after Breaker C1+C2, see §7):**
the TTL≈10 s `(orderId,sub)` cache makes the guard **NOT a DB read per frame.** GPS streams ~1 frame/s
during active delivery ⇒ ~1 pinning-tx per (socket,order) per 10 s. Steady revalidation DB load =
`(active courier-in-order sockets) / TTL`: MVP ~20 tx/s (200 mid-delivery), 10× ~200 tx/s (2000
mid-delivery). At ~2-4 ms wall per pinning tx (4 RTT on a co-located DB + pool checkout), steady-state
concurrency demand is `rate × T_tx` ≈ 200 × 0.003 ≈ **<1 conn of the pool** — comfortable. The **same**
`(orderId,sub)` cache backs the subscribe gate, the fan-out guard, and REST, so the reads do not stack.
The pressure is **not** steady-state; it is the cold-cache burst (next paragraph).

**Reconnect-storm burst shape (Breaker H1 + R3-1 honesty — the real pressure point):** a deploy/restart
drops every WS socket at once with a **cold** authz cache; ~40 % mid-delivery couriers re-subscribe
`order:<id>` inside the reconnect window, ALL as cache misses. At 10× (5000 couriers) that is ~2000
subscribes in ~2-3 s ≈ **700-1000 authz *pinning-txns*/s burst** on the `max=20` operational pool — which
has a documented starvation incident, AND which is now being asked for connection-pinning txns, not
multiplexed point-reads. The honest concurrency demand: at 700-1000 tx/s, IF each tx stays ~3 ms the
semaphore-capped (≈10) demand is satisfiable; but under the very starvation the burst can trigger, `T_tx`
balloons to tens/hundreds of ms → required concurrency `rate × T_tx` blows past both the semaphore and the
20-conn pool → the surplus authz txns **time out → `UNAVAILABLE`**. Mitigations (§7 fail-mode + below):
(1) tri-state authz so a transient pool exhaustion → *retryable* soft error, **not** fleet `Forbidden`;
(2) **jittered FE reconnect** (random 0-5 s) smears the spike to ~150-300 tx/s; (3) the **bounded authz
concurrency** semaphore (≈10) queues excess subscribes rather than exhausting the pool (excess waits, not
fails); (4) **scaling-gate — PROMOTED from "optional" to a defined trigger (Breaker R3-1 / NEW-H):** a
**dedicated small (≈5-conn) authz read pool** isolates authz pinning-txns from order/menu traffic. **Trigger
(measurable):** adopt when a **staging load test shows authz-tx contention** — authz-tx pool-checkout wait
> ~50 ms p95 **OR** operational-pool saturation attributable to authz under the deploy-storm replay — **OR**
when the production `authz_unavailable_ceiling` eviction rate during deploys exceeds threshold (the honest
signal that couriers are being bounced by NEW-A's ceiling). Owner Architect; +5 to the API connection budget
if taken.

**Why the burst stays OFF the confidentiality line (the load-bearing point — R3-1).** NEW-A's fail-safe
eviction ceiling fires from **in-memory state only** (`consecutiveUnavailable` counter + the `~60 s`
wall-clock since `firstUnavailableAt`) — **eviction needs ZERO DB reads.** So even if the burst starves the
pool and *every* authz pinning-tx times out (`UNAVAILABLE`), a displaced courier is still evicted within
≤ ceiling from purely local state. The cold-cache reconnect wave therefore costs **availability**
(subscribes soft-retry; admitted members bounce at the ≤60 s ceiling — all recoverable), **NOT an unbounded
confidentiality leak.** This is recorded as an explicit **ACCEPT-RISK (availability), owner Architect**
(§10 H1c/R3-1), re-evaluated after the staging load test that also tests the scaling-gate trigger above.

**Connection budget (honest):** base decision uses `fastify.db` (operational pool, `max=20`) exactly as
`ownerCanAccessRoom` already does — **no new pool, no new role in the base PR.** The additive load is the
bounded subscribe/revalidation **pinning-txns** above (each holding a pool conn for its tx, not a single
multiplexed statement), capped by the ≈10 concurrency semaphore. The cumulative API connection picture the
budget must hold: **API request handlers + WS authz txns + worker + analytics + migrations** all draw on
their respective pools; this PR adds only the WS-authz txn load to the **API** operational pool (worker /
analytics / migration budgets untouched — no migration, no worker change). The dedicated ≈5-conn authz pool
(+5 to the API budget) is the **trigger-gated** scaling-gate above, not part of the base PR.

**Index decision:** `order_id` index alone makes the predicate a point-read (≤3 rows). A composite
`courier_assignments(order_id, courier_id)` is **optional** — add forward-only ONLY if `EXPLAIN` on
staging shows the FK index is missing or a seq scan appears. Likely **NONE** (§5).

---

## 3. Options (≥2) with tradeoffs

The shared decision is: keep `courier:<sub>` self-only; **deny all `location:`** for couriers
(`dashboardChannel`=`location:<id>:dashboard` is the OWNER feed; `courierChannel`=`location:<id>:couriers`
is not subscribed by the courier FE — grep confirms the courier FE uses only `courier:<sub>` and
`order:<orderId>`). The options differ on **`order:<id>`**.

### Option A — Assignment-row-scoped order rooms ("binding-scoped least-privilege")

`order:<id>` allowed **iff** an assignment row exists for `(orderId, courierSub)` in status
`('offered','assigned','accepted','picked_up')`.

- **Concept:** least-privilege, row-level capability — the same shape as `ownerCanAccessRoom`'s
  order branch, but keyed on the courier's binding instead of membership.
- **Honors the offer-handshake (the KEY edge):** an `offered` row is included, so a courier who has
  been *offered* (not yet accepted) an order can open `order:<id>` to decide accept/decline. The
  courier learns of the offer via a push on `courier:<sub>` (dispatch.ts:55 / handshake), THEN opens
  the order room — by which time the `offered` row already exists. No pre-offer subscribe need.
- **+** Tightest blast radius: a courier sees ONLY their own runs. Closes both the cross-tenant leak
  AND the intra-tenant colleague leak in one predicate.
- **+** Self-revoking: once the binding terminalizes (delivered / reoffered), the NEXT subscribe is
  denied (the live run is over — correct).
- **−** One DB query per order-subscribe (quantified trivial in §2).
- **−** Needs the stale-membership eviction companion (§7) for the reassignment race.

### Option B — Location-scoped order rooms ("tenant-coarse authz")

`order:<id>` allowed **iff** `order.location_id === token.activeLocationId`.

- **Concept:** tenant-coarse authorization — the boundary is the shop, not the assignment.
- **+** Simplest; `activeLocationId` is in the token, so a single `SELECT location_id FROM orders`
  (or even a cached order→location map) suffices; no offer-edge reasoning.
- **−** **COARSER — over-broad.** Any courier on shift at location L can watch EVERY order at L,
  including colleagues' deliveries, the customer's live GPS, and the customer↔courier message
  thread. That is an intra-tenant privacy leak (customer location + PII-adjacent chat exposed to a
  courier with no business reason). It closes the cross-tenant hole but leaves N3's class open.
- **−** Does not match the principle that a courier's authority IS their binding.
- **Reject:** trades correctness for a marginal simplification; re-opens the colleague leak.

### Option C — Hybrid (assignment-scoped + location fallback for a pre-offer window)

`order:<id>` = Option A, plus a location-scoped fallback during a notional "pre-offer browse" window.

- **Concept:** capability-with-fallback.
- **−** There is **no pre-offer subscribe need** — the dispatch model is push-then-open, so by the
  time a courier opens an order room an `offered` (or further) row exists. The fallback authorizes
  nothing that A doesn't already, while re-introducing B's over-broad surface.
- **Reject:** collapses to A with extra attack surface; over-engineering.

---

## 4. Decision + rationale

**Adopt Option A.** Introduce `courierCanAccessRoom(courierSub, activeLocationId, room): Promise<boolean>`,
mirroring `ownerCanAccessRoom`, and wire it into the courier branch of the WS subscribe handler:

```
courier:<sub>         → room === `courier:<sub>`  OR  room.startsWith(`courier:<sub>:`)   (self-namespaced; covers shiftChannel(sub))
location:*            → DENY (couriers have no legitimate location room)
order:<orderId>       → EXISTS courier_assignments WHERE order_id=orderId AND courier_id=sub
                         AND status IN ('offered','assigned','accepted','picked_up')
anything else         → DENY
```

Apply the **same binding predicate at REST** (`order-messages.ts`), replacing the location-wide
membership check for couriers (fix **N3**):
- **GET history / mark-read:** courier allowed iff a binding row exists in
  `('offered','assigned','accepted','picked_up')` for `(orderId, sub)`.
- **POST send:** courier allowed iff a binding row exists in `('assigned','accepted','picked_up')`
  for `(orderId, sub)` — an *offered* (not-yet-accepted) courier may READ to decide but may not yet
  emit `cu_*`/`cc_*` presets. This subsumes and tightens the existing `hasCourier()` "some courier"
  check into a "**this** courier" check.

Both the WS and REST paths call ONE shared helper (e.g. `courierHasBinding(db, orderId, sub, scope)`)
so the predicate has a single source of truth and the guardrail tests one function.

**Rationale:** A is the only option that closes BOTH the cross-tenant leak and the intra-tenant
colleague leak, matches the owner pattern (consistency, reviewability), and is the cheapest correct
predicate (§2). Boring & proven > novelty; the data to authorize already sits in the token + an
existing indexed table.

---

## 5. Data / migrations

**Migrations required: NONE (verify).** Pure authz code. The predicate reads `courier_assignments`
by `(order_id, courier_id, status)`; `order_id` is an FK and already indexed, with ≤3 rows per order
→ a point-read.

- **Forward-only, optional:** IF staging `EXPLAIN (ANALYZE)` on the predicate shows the FK index
  absent or a seq scan, add `CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_courier_assignments_order_courier
  ON courier_assignments (order_id, courier_id);` as a standalone forward-only migration. Atomic,
  additive, no data backfill, RLS-irrelevant (read path). Do not pre-emptively add it (YAGNI).
- No new columns, no integer-money surface, no RLS table created → no `ENABLE/FORCE` change.
- **RLS posture — tenant context IS set on both authz reads (corrected again, Breaker NEW-B; the Round-1
  M1 "don't-wrap" decision was WRONG).** `courier_assignments` is **FORCE ROW LEVEL SECURITY** with policy
  `USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid)`
  (`packages/db/migrations/1790000000073…:43-47`). The new authz read sites — `courierCanAccessRoom` (WS,
  `fastify.db`) and the REST binding predicate (`order-messages.ts:22`) — set **no** tenant context as
  originally drafted. **Under `NOBYPASSRLS` (which `feat/mvp-sensor-seams` itself introduces via the
  pg-privilege-hardening), `current_setting` is empty → `NULLIF→NULL` → 0 rows for every order →
  DENY-ALL-couriers fleet-wide** (realtime + messaging outage, not a leak). "Functions today" proves the
  role is **BYPASSRLS at runtime**, so the predicate is sound **iff BYPASSRLS** — the inverse of the
  Round-1 claim "sound regardless."
  - **FIX:** both authz reads now **set `set_config('app.current_tenant', activeLocationId, true)` inside a
    real `BEGIN…COMMIT` transaction** — the **correct** convention shape on this table
    (`courier/assignments.ts:80-81`: `BEGIN` → `set_config(...,true)` → SELECT → `COMMIT`).
    `courierHasBinding(client, orderId, sub, scope)` takes a **tenant-scoped connected client**, not
    `fastify.db` directly. This makes the policy pass; the `courier_id=$sub AND order_id=$orderId AND
    status IN (...)` predicate then narrows to *this* courier's binding. **Defense-in-depth:** RLS scopes
    to the tenant (location), the predicate scopes to the binding — neither is "the boundary instead of
    the other."
  - **🔴 IMPL-HAZARD MERGE-GATE (Breaker R3-2 — do NOT copy the `assignments.ts:111` shape).** The tenant
    context MUST use a real `BEGIN…COMMIT` transaction (the `:80-81` shape). The bare
    `set_config('app.current_tenant', $loc, true)` WITHOUT a wrapping `BEGIN` (the `assignments.ts:111`
    shape) is **wrong**: `set_config(…, is_local=true)` is local to the *current transaction*, and a pooled
    client with no explicit `BEGIN` runs each `client.query` in its **own implicit (autocommit) tx** — so
    the setting **dies when the `set_config` statement's implicit tx ends**, BEFORE the SELECT runs in a
    fresh implicit tx with `app.current_tenant` empty again. Under NOBYPASSRLS this **silently re-introduces
    the NEW-B deny-all** (`NULLIF→NULL` → 0 rows → every courier `Forbidden`/404). It "works" only under
    BYPASSRLS — a latent trap that detonates the instant the privilege-hardening lands. The `:111` site is
    itself only non-broken today because of BYPASSRLS. **§11 item-8 is pinned to run under NOBYPASSRLS and
    is RED against a no-`BEGIN`/no-`set_config` stub** so this exact shape cannot ship green.
  - **B3 sequencing:** because the reads **always** set tenant context, the gate is correct under BOTH
    BYPASSRLS (set_config harmless; predicate still filters) and NOBYPASSRLS (policy passes) → **no
    land-before/after dependency** on the privilege-hardening. This PR is NOBYPASSRLS-safe whichever lands
    first. (Round-1's refusal-to-wrap baked in a cross-cutting outage; this removes it.)
  - **Accepted edge (watch-item, owner Architect):** tenant = `activeLocationId` (token claim, same source
    as `assignments.ts`). A courier holding a binding whose `location_id ≠ activeLocationId` (stale token /
    mid-shift location switch) would be RLS-denied their own order under NOBYPASSRLS — a rare multi-location
    edge, moot under BYPASSRLS, mirroring `assignments.ts` exactly. The binding-location should track the
    active location for any live run.

---

## 6. Consistency + idempotency

- **Subscribe is idempotent:** re-subscribing to the same room re-adds the member to the `Set`
  (no duplicate fan-out — the dedup is by `Set` identity) and re-evaluates authz against the
  **fixed-TTL authz cache** (NEW-D below). So a courier whose binding was revoked is denied within
  **≤TTL** on the next subscribe/reconnect — bounded by the same fixed-TTL re-read the fan-out guard uses.
- **Cache TTL semantics — FIXED (absolute), never sliding (Breaker NEW-D).** The authz cache entry is
  `{allowed, expiresAt = createdAt + TTL}`, **never refreshed on access**. This is load-bearing: a
  flapping courier (WiFi↔LTE reconnect every <10 s) **cannot** keep a revoked entry warm — the hard wall
  forces a re-read within ≤TTL regardless of access frequency. **Subscribe READS the cache** (so a deploy
  reconnect storm is absorbed — §2/H1); a stale ALLOW re-admitting a just-revoked courier is **≤TTL
  bounded** because the *same fixed-TTL entry* the fan-out guard reads expires at the same wall → re-read
  → DENY → evict. The earlier "another reason not to cache / live each time" framing is **retracted**: the
  cache is correctness-preserving *because* the TTL is fixed and short — the revocation bound on **every**
  surface (subscribe / bus fan-out / GPS fan-out / REST) is ≤TTL (and ≤ceiling under sustained
  UNAVAILABLE, §7).
- **Server is authoritative:** authz keys on `token.sub` / `token.activeLocationId` (RS256-verified),
  never on a client-asserted identity. The room string is untrusted input, parsed defensively
  (empty/`undefined` id → DENY, mirroring `ownerCanAccessRoom`).
- **Offer→accept transition consistency:** the `('offered'…'picked_up')` set spans the whole live
  lifecycle, so a courier does not lose the order room at the accept/pickup boundaries (no flicker on
  legitimate state changes).

---

## 7. Failures + degradation

- **Authz query fails (DB blip / timeout) → fail CLOSED.** `courierCanAccessRoom` wraps the query in
  `try/catch` and returns `false` on error (exactly like `ownerCanAccessRoom`:105-108) → the subscribe
  is refused with `{type:'error',error:'Forbidden room'}`. A failure can NEVER widen access.
- **No cascade:** a denied/failed subscribe does not drop existing rooms or close the socket; the
  courier FE shows its reconnecting / soft state and retries. The query is bounded by the pool's
  statement timeout; on timeout → deny.
- **Stale-membership / revoked-binding eviction — fan-out-time revalidation (REDESIGNED after Breaker R1, C1+C2; see `resolution.md`).**
  Authz is checked at subscribe time *and re-checked on the relay path*, because the in-memory room
  persists past a binding. The Round-1 design (evict on a `courierId`-bearing `binding_changed`) was
  **scrapped**: the emitted event is `{type:'binding_changed', orderId}` with **no `courierId`** (a
  `Set<RoomMember>` cannot target the evictee from it — C2), and the **victim paths emit nothing on
  `order:<O>`** that names a leaving courier (owner-reassign `dashboard.ts:362/422`, `/decline`,
  `/reject`, offer-sweep — C1). Event-driven, payload-targeted eviction is the wrong shape.
  - **ONE shared helper `guardedCourierRelay(member, frame, room)` over ALL THREE raw courier-send sites
    (Breaker NEW-E + R3-3 — there are THREE, not two).** The entire decision (fixed-TTL `Map` lookup →
    relay / evict / withhold + async re-read + ceiling counters + `binding_revoked` emit) lives in a single
    helper. The three raw `member.ws.send(...)` sites that reach a courier member are:
    (1) the **bus room handler** (`websocket.ts:36-44`) — `order.message` + status deltas, which fan to
    room members **ROLE-AGNOSTICALLY** (`for (const m of members) … m.ws.send(payload)` — *no*
    `role==='courier'` test; couriers receive because they are members, not because of a role check);
    (2) the GPS relay loop `client_location`; (3) the GPS relay loop `client_location_stop`. All three are
    in `order:<O>` fan-out and all three must route courier members through `guardedCourierRelay`.
  - **Drift guardrail — re-keyed (R3-3; the role-keyed form MISSES the bus handler).** Keying the guardrail
    on "send to a `role==='courier'` member" would **not catch the bus handler**, which sends
    role-agnostically — the exact shape that re-creates the Round-1 "guarded on X, open on Y" leak. So the
    structural drift guardrail (eslint-local rule / grep test) instead asserts: **within the courier
    `order:<O>` fan-out sites, every `member.ws.send(...)` (or `ws.send` over a room-member iteration) goes
    through `guardedCourierRelay` — covering all THREE sites, role-agnostic.** A raw member send to a room
    that couriers can join, outside the helper, is a build error. §11 item-6 behaviourally backstops it
    across all three sites.
  - **Relay-ONLY-on-fresh-ALLOW (Breaker NEW-C — never relay-then-revalidate).** The cache lookup is a
    **synchronous `Map` read**, so the hot fan-out loops stay sync. A frame is relayed to a courier member
    **only on a fresh-cache ALLOW** (GPS 1/s, fixed TTL 10 s → ~9/10 frames are fresh hits). On
    **miss / stale / UNAVAILABLE the frame is WITHHELD from that member** (NOT relayed) and an out-of-band
    async re-read is kicked off (deduped per `(orderId,sub)`); the member resumes on the **next** frame
    once the cache is fresh-ALLOW, or is evicted on DENY/ceiling. The trigger / TTL-boundary frame is
    **never** relayed to an unconfirmed member → a displaced courier receives **zero** frames from the
    moment their entry goes stale. Cost to a legitimate courier: at most one withheld GPS frame per TTL
    boundary during the sub-ms re-read (the next 1-Hz frame self-corrects); a withheld chat frame is
    recoverable via REST history.
  - **Authority source — live binding, tenant-scoped:** the per-member re-read derives authority from the
    **live binding** via the shared `courierHasBinding` predicate run on a **tenant-context client**
    (`set_config('app.current_tenant', activeLocationId, true)` — Breaker NEW-B / §5). The member carries
    `user.sub`; the DB knows `courier_id` — the evictee IS targetable via binding state, no `courierId`
    event needed.
  - **NOT a DB read per frame, bounded heap (Breaker NEW-F):** the process-local authz cache keyed
    `(orderId, courierSub)` → `{allowed, expiresAt, consecutiveUnavailable, firstUnavailableAt}` is a
    **bounded LRU (cap ~50 k, drop-oldest)** plus a periodic `setInterval` sweep (~30–60 s) dropping
    past-`expiresAt` entries — heap bounded independent of order volume / deploy cadence (terminal-order
    entries no longer accrete forever). GPS at ~1 frame/s ⇒ ~1 read per (socket,order) per TTL.
  - **Eviction:** revalidation `DENY` (query OK, 0 rows) → remove the member from the room `Set`, send
    `{type:'error', error:'binding_revoked'}` once, stop relaying.
  - **Self-healing on ALL victim paths** with zero per-path plumbing; worst-case leak window after any
    terminalization (owner-reassign / decline / reject / sweep / cancel / abort / deliver) =
    **≤ TTL (~10 s) under DB-availability; ≤ ceiling (~60 s) even under sustained DB-UNAVAILABLE** (§7
    fail-semantics, Breaker NEW-A).
  - **Cache-bust accelerator — DEFAULT-ON for the involuntary owner-reassign path (counsel Rev-2 B1).** On
    a `binding_changed` / terminal-status frame for `order:<O>`, bust the cache for that order so the next
    frame evicts within broadcast-latency. For the **owner-reassign** path specifically (the courier did
    NOT choose to leave — the least-defensible continued-GPS) the one-line `binding_changed` emit + cache
    bust is a **SHOULD (default-on)**, not optional. Self-decline/sweep stay at TTL. A missed event degrades
    the window to TTL/ceiling, never to "until disconnect."
  - **Multi-instance safe:** revalidation is per-instance against the shared DB (each API instance
    self-heals its own members); the cache-bust accelerator is also per-instance (PG LISTEN/NOTIFY is
    cross-instance). No single-instance eviction coupling.
- **Fail semantics — tri-state, never throw (Breaker H1/L2):** `courierHasBinding` returns
  `ALLOW` / `DENY` / `UNAVAILABLE` and **never throws to the outer `ws.on('message')` handler** (which
  would `ws.close(1008)` and feed a reconnect loop — L2). `UNAVAILABLE` (DB error/timeout/pool-exhausted)
  is **not** "no binding": on the **subscribe** gate it yields a *retryable* soft error
  (`{error:'authz_unavailable', retryable:true}`), NOT a permanent `Forbidden room` — a pool blip during
  a deploy stampede must not fleet-wide-deny.
- **Fail-safe eviction ceiling (Breaker NEW-A — load-bearing, NOT optional).** On **revalidation** of an
  already-admitted member, `UNAVAILABLE` does **NOT** evict on the first occurrences (the member was already
  admitted; a single blip mustn't mass-evict legitimate mid-delivery couriers) — BUT a *sustained*
  `UNAVAILABLE` is **bounded by a hard ceiling**, because otherwise the ≤TTL confidentiality bound (the
  sole basis for accepting a known leak on a 🔴 line) collapses to "until DB recovers" exactly when the
  H1 deploy storm saturates the pool. Mechanism, per `(orderId, courierSub)` member:
  - `ALLOW` → relay; **reset** `consecutiveUnavailable=0`, clear `firstUnavailableAt`.
  - `DENY` → evict immediately (`binding_revoked`).
  - `UNAVAILABLE` → **withhold the frame** (NEW-C); `consecutiveUnavailable += 1`; set `firstUnavailableAt`
    if unset; if `now − firstUnavailableAt > CEILING (~60 s)` **(the dominant bound)** **OR**
    `consecutiveUnavailable >= N` **(secondary safety, N re-tuned — R3-4)** → **EVICT anyway**
    (`binding_revoked`, reason `authz_unavailable_ceiling`).
  - **🔴 Ceiling re-tune (Breaker R3-4 — DoD-tunable).** The Round-2 `N=3` was wrong against a ~1 Hz GPS
    stream: 3 consecutive `UNAVAILABLE` arrive in **~3 s**, so a brief mid-delivery DB blip would bounce a
    **legitimate** courier after 3 s — far below the intended ~60 s. **Fix the tuning so the ~60 s
    wall-clock is the DOMINANT bound, not the count.** The wall-clock (`now − firstUnavailableAt`) is
    **frame-rate-independent** and is the honest confidentiality bound; the consecutive-count `N` is
    demoted to a **secondary safety** and must be set high enough that it does **not** fire before the wall
    under normal frame rates — i.e. `N ≥ CEILING × max_frame_rate` (at ~1 Hz GPS and a 60 s wall, `N ≥ ~60`)
    — or dropped entirely in favour of the wall-clock alone. Keeping `N` only guards pathologically fast
    streams; it must never be the bound a 1 Hz stream hits first. **Target (DoD-tunable):** effective
    ceiling ≈ 60 s (the wall), NOT ~3 s; record final `CEILING` + `N` (or "wall-only") after the staging
    load test, with the constraint `N` never fires before the wall at the observed peak frame rate.
  - **Honest bound:** `≤ TTL (~10 s)` under DB-availability; `≤ ceiling (~60 s, wall-dominant)` even under
    sustained DB-UNAVAILABLE. The ceiling decouples the confidentiality bound from pool health (de-fangs
    NEW-H), and fires from **in-memory state only** (no DB read needed to evict — R3-1).
  - **Correct asymmetry (counsel-endorsed):** the ceiling will, during a >60 s incident, also bounce a
    *legitimate* courier (UNAVAILABLE can't tell the two populations apart). That cost is **recoverable**
    (re-subscribe; the subscribe gate fails *safe-retryable*, jittered — a soft reconnect, not a permanent
    deny). The customer-PII leak it re-bounds is **not** recoverable. For customer location data the trade
    favors the customer. Admission stays fail-closed on a real negative; revocation fails safe *up to the
    ceiling*, then fails closed.

---

## 8. Security + tenant isolation (the core)

- **Cross-tenant:** a courier of tenant B can no longer subscribe to tenant A's `order:`/`location:`
  rooms (binding predicate fails; `location:` denied outright).
- **Intra-tenant (colleague):** a courier with no binding on order O cannot watch O — closes the
  customer-GPS / message-thread leak between couriers of the same shop.
- **REST parity (N3):** the same binding predicate gates `order-messages` GET/POST/read, so the WS
  and REST capability surfaces are identical (no "blocked on WS, open on REST" asymmetry).
- **No PII in denials:** the error is a flat `Forbidden room` (no order/tenant detail). REST denials
  return `404 NOT_FOUND` (existing pattern — non-enumerable; does not confirm the order exists).
- **Token authority:** `sub`/`activeLocationId` from the RS256 JWT only; zero cookies; room string is
  untrusted. No secret/PII added to logs (denied subscribes log role+sub+room, already present).

---

## 9. Operability

- **Health:** unchanged — authz is subscribe-time; no new health surface. Degraded-vs-down is
  inherited from the operational pool (a pool outage fails subscribes closed, does not flap health).
- **Observability (<1 min) — two signals (Breaker L1):** the subscribe-DENY counter alone watches the
  WRONG event — the C1 stale-room leak is a *successful past subscribe* that keeps receiving (no deny,
  no new subscribe). So:
  1. **Eviction counter** (the leak-closing event), **labeled by reason** (Breaker NEW-A): `binding_revoked`
     (real DENY = revocation/reassignment) vs. `authz_unavailable_ceiling` (the ≤60 s fail-safe ceiling firing
     under sustained DB-UNAVAILABLE). A `binding_revoked` rate is the proof revalidation is firing; a *spike of
     `authz_unavailable_ceiling`* is a **DB/pool-incident** signal (couriers being bounced), NOT a revocation
     storm, and is the trigger to consider the dedicated authz pool (§2 scaling-gate / NEW-H). Distinguishing
     the two readings is what makes the eviction counter actionable rather than misleading.
  2. **DENY counter, labeled** (role, sub, room-prefix, reason) — and labeled to **distinguish
     attacker-probing from R1 over-deny** (legitimate couriers locked out): a DENY spike on `order:`
     rooms a courier *should* hold is the H2 regression, not an attack (counsel §3b). Alert on both
     readings, or you mistake your own users being locked out for an attack.
  The existing `console.warn` lines give a baseline; the two metrics are the upgrade.
- **Rollback:** pure code on a feature branch — revert the commit. No schema to unwind.
- **Flag / scaling-gate:** **NO insecure-default flag.** This is a security fix; ship default-secure.
  If a kill-switch is wanted for the REST tightening (in case a courier FE relied on location-wide
  reads), its "off" position MUST still be a *secure* variant (e.g. assignment-scoped read but the
  legacy `hasCourier` send) — never the original hole. Validate on staging with a 2-courier shop
  before prod (multi-courier accept/reassign/reconnect flow).

---

## 10. Open / accepted risks

| # | Risk | Disposition | Owner |
|---|------|-------------|-------|
| R1 | **FE id-conflation (Breaker H2 — ATOMIC):** DeliveryPage:121 subscribes `order:${id}` where `:id` is the route param = **assignment** id (CourierRoutes.tsx:88). Binding authz keys on `order_id` → the gate denies every legitimate courier unless the FE ships `order:${task.orderId}` in the **SAME atomic deploy** (requires `toTaskShape` to expose `orderId`; assignments.ts:24 currently sets `id=row.id`). The gate + FE room-key fix are one indivisible unit. | **FIX in PR — atomic.** The §11 positive-control E2E MUST drive the REAL `/courier/delivery/:id` route (no synthetic room) so a split cannot ship green. | impl PR |
| R2 | **Stale membership on reassignment** — courier A keeps O's stream after reoffer. | **FIX in PR — REDESIGNED (Breaker C1+C2):** the Round-1 `courierId`-bearing bus-eviction is scrapped (event carries no courierId; victim paths emit nothing on `order:<O>`). Replaced by **fan-out-time binding revalidation** (§7) — self-healing on all victim paths, ≤TTL window, no per-path plumbing. | impl PR |
| R3 | Binding revoked mid-stream not evicted (was subscribe-time only). | **CLOSED — no longer accepted.** Fan-out revalidation evicts within ≤TTL on ALL paths incl. mid-stream owner-reassign (Breaker M2). | Architect |
| N3 | order-messages REST courier over-broad scope. | **FIX in THIS PR** (§4) — same predicate, same blast-radius class. | impl PR |
| H1c / R3-1 | **Cold-cache reconnect wave → authz-tx pool pressure (AVAILABILITY):** a deploy empties the authz cache → the first reconnect wave is all misses, each now a **connection-pinning tenant-context tx** (not a multiplexed point-read) on the `max=20` operational pool (~700-1000 tx/s burst); under starvation `T_tx` balloons → surplus authz txns time out (`UNAVAILABLE`). | **ACCEPT-RISK (availability), owner Architect.** Bounded by jittered reconnect + ≈10 authz-concurrency semaphore (§2/§7). **Confidentiality is OFF this line:** NEW-A's ceiling evicts displaced couriers from **in-memory state only (no DB read)** within ≤60 s even if every authz tx times out — so the cost is couriers bouncing/soft-retrying (recoverable), not a leak. The **dedicated ≈5-conn authz pool is now a defined scaling-gate trigger** (staging load shows authz-tx contention OR prod `authz_unavailable_ceiling` rate > threshold), not "optional." Re-eval post staging load test. | Architect |
| NEW-A | **≤TTL bound false under sustained DB-UNAVAILABLE** (co-located with the H1 deploy storm): `UNAVAILABLE→don't-evict` → displaced courier keeps customer GPS+chat for the whole incident, fleet-wide. | **FIX — fail-safe eviction ceiling (load-bearing, in-memory, no DB read).** Evict an admitted member after a hard `~60 s` wall since first-`UNAVAILABLE` (**dominant bound**) OR a high secondary count `N` (R3-4 re-tune: `N` never fires before the wall at peak frame rate — at ~1 Hz GPS, `N ≥ ~60` or wall-only). Honest bound: **≤TTL under DB-availability; ≤ceiling (~60 s, wall-dominant) under sustained DB-UNAVAILABLE** (§7). | impl PR |
| R3-4 | **Ceiling mis-tuned:** `N=3` at ~1 Hz GPS ⇒ ~3 s effective ceiling → bounces legit mid-delivery couriers on a brief blip. | **FIX — re-tune (DoD-tunable):** make the ~60 s wall-clock the dominant bound; demote/raise `N` so it never fires before the wall under normal frame rates; record final `CEILING`+`N` post staging load test. | impl PR / Architect |
| NEW-B | **Predicate sound ONLY under BYPASSRLS; this branch removes it** → FORCE-RLS policy 0 rows → deny-all-couriers fleet-wide. | **FIX — set tenant context on BOTH authz reads** (`set_config('app.current_tenant', activeLocationId, true)`, the `assignments.ts` convention); NOBYPASSRLS-safe, order-independent vs. B3 privilege-hardening (§5). | impl PR |
| CPII | **Device-retained customer PII (Counsel §5 / Breaker NEW-G):** eviction stops *future* frames; an ex-courier's app still renders/holds last-known GPS/address/chat — and an **offered-then-declined** courier already fetched the customer **delivery address via REST** (`/courier/assignments/:id`) onto the device. | **FIX (best-effort live session) + ACCEPT-RISK (already-delivered):** on `binding_revoked` the FE purges rendered customer GPS/address/chat from memory/DOM + navigates away — but this is **best-effort** (honest online client only); the authoritative boundary is **server-side stop-sending**. Already-delivered PII (incl. the offered-courier's fetched address) is **unrecoverable** — re-label from "closes the live-session half." | impl PR + owner accept |
| NEW-G | **Offered-courier delivery-address exposure:** the offer-handshake exposes the exact delivery address via REST *before* accept; a declining courier retains it. | **ACCEPT-RISK with owner + tracked tighten:** follow-up to coarsen the *offered*-state REST payload (pickup/zone/distance to decide; withhold exact street/unit until accept). Severity LOW; owner Architect → human/owner. | Architect → human/owner |
| CPII-2 | Customer PII in *persistent* client storage (SW cache / localStorage / IndexedDB) after binding end. | **DEFER — tracked, "LOW — confirm-empty."** First step = the grep-triage (counsel C evidence: `sw.js` excludes `/api/`+`/ws/`; `DeliveryPage.tsx` holds customer data in React state only). Window: next sprint. Owner Architect. | Architect |
| B7 | Owner **settlement regenerate** cross-tenant. | **DEFER — separate PR.** Severity **MEDIUM**; surface=settlement module, actor=owner. **Target window: within 1 month.** Filed (not silent). | Architect → human |
| N1 | Customer JWT order-scope on REST (`/orders/:id/messages` — REST sibling of the already-scoped customer WS room websocket.ts:169-174; a *confirmed-live* customer-PII gap being knowingly left open). | **DEFER — separate PR.** Severity **HIGH**; actor=customer, surface=order-messages REST. **Target window: within 2 weeks (next sprint).** Filed with deadline so deferral is *scheduling*, not permanent acceptance (counsel §3a). | Architect → human |
| M1f | ~~order-messages REST has no `app.current_tenant`; isolation is the `courier_id=sub` predicate, not RLS.~~ **SUPERSEDED by NEW-B.** The Round-1 "accepted, sound regardless of BYPASSRLS" disposition was WRONG — under NOBYPASSRLS the FORCE-RLS policy denies all couriers. | **REVERSED → FIX:** both authz reads now set `app.current_tenant = activeLocationId` (§5). Tenant context is no longer a "tracked follow-up" — it is required for soundness on this branch. | impl PR |
| NEW-tenant-edge | tenant = `activeLocationId`; a binding whose `location_id ≠ activeLocationId` (stale token / mid-shift location switch) is RLS-denied under NOBYPASSRLS. | **Accepted edge (watch-item).** Rare multi-location case, moot under BYPASSRLS, mirrors `assignments.ts` exactly. Binding-location should track active location for any live run. | Architect |

**Why N3 is in-PR but B7/N1 are not:** N3 is the *same vulnerability* (courier over-broad scope) on
the *same threat model* as the WS gap and shares the exact predicate — fixing one without the other
leaves a trivially-pivotable hole (block WS, read REST). B7 and N1 are different actors/surfaces; each
deserves its own focused proof and would dilute this PR's blast radius — but both are now filed with a
**severity and a target window** (counsel §3a), so the deferral is a scheduling decision, not a quiet
permanent acceptance.

---

## 11. Guardrail plan (red→green)

The prior "cross-tenant realtime QA 6/6 green" (`e2e/tests/cross-tenant-realtime-qa.spec.ts`) missed
this because its isolation block tested only **customer-A→order-B** and **owner-fake-room** — it never
subscribed a SECOND courier or a cross-tenant courier. The fix is a deterministic isolation net.

**Unit (fast, deterministic):** `courierCanAccessRoom` / `courierHasBinding` table-test:
`courier:<self>` ✓, `courier:<other>` ✗, `courier:<self>:shift` ✓, `location:<any>` ✗,
`order:<bound>` ✓ for each of offered/assigned/accepted/picked_up, `order:<unbound>` ✗,
`order:<terminal:delivered>` ✗, malformed/empty id ✗. **Tri-state (Breaker H1/L2):** DB OK+0 rows → `DENY`,
DB error/timeout → `UNAVAILABLE` (never throws). **Ceiling (NEW-A + R3-4 tuning):** the
`guardedCourierRelay` state machine — `now − firstUnavailableAt > CEILING (~60 s, dominant)` OR the
secondary count `N` → evict; an intervening `ALLOW` resets the streak; **assert the wall fires before `N`
at a ~1 Hz frame rate** (the R3-4 mis-tune: a 3 s bounce of a legit courier on a brief blip must be RED).
**Fixed-TTL (NEW-D):** an entry's `expiresAt` does NOT advance on access (no sliding). **Tenant context
(NEW-B + R3-2):** `courierHasBinding` is invoked with a client whose tenant context was set **inside a real
`BEGIN…COMMIT` tx** (the `assignments.ts:80-81` shape), not the bare `set_config(…,true)`-without-`BEGIN`
(`:111`) shape — a unit/contract test asserts the predicate sees a non-empty `app.current_tenant` at SELECT
time (RED against a no-`BEGIN` stub, where the setting has already died).

**E2E (real staging WS + REST)** — new spec `e2e/tests/courier-realtime-authz.spec.ts`, plus a courier
block appended to the existing cross-tenant spec:
1. **Cross-TENANT WS:** courier of location B subscribes `order:<A's order>` → expect `Forbidden room`,
   collector proves **ZERO** messages across A's transitions (status/message/GPS).
2. **Cross-COURIER intra-tenant WS:** courier X (no binding) subscribes `order:<order bound to courier
   Y>` → `Forbidden room` + zero messages. (The gap the owner-only QA missed.)
3. **location: denial:** courier subscribes `location:<own>:dashboard` / `location:<any>` → `Forbidden`.
4. **Positive control — MUST drive the REAL `/courier/delivery/:id` FE route (Breaker H2 / counsel R1, hard gate).**
   Navigate the actual courier UI for a bound courier and assert the run is visible + receives each
   per-transition delta. A synthetic hand-crafted room is **BANNED** for this control — it would mask the
   R1 id-conflation denial (authz green while a real courier is locked out). Include an **offered-state**
   positive control (handshake): offered courier can read `order:` + GET messages, cannot POST `cu_*`.
5. **REST N3:** courier X `GET /orders/<Y's order>/messages` → 404; `POST .../messages` with `cu_*` →
   404/409. Bound courier → 200.
6. **Revocation / eviction (Breaker C1+C2 — the redesign's proof; NEW-C boundary-frame pin):** courier A
   bound to O subscribes `order:<O>` and is receiving frames → owner **reassigns O to B** (and, in sibling
   cases, A declines/rejects / sweep expires) → within ≤TTL the collector proves A receives **ZERO** further
   `order.message` / status / `client_location` frames — **explicitly including the reassignment-trigger /
   TTL-boundary frame** (run RED against a *relay-then-revalidate* stub to prove the boundary-frame leak is
   caught, not just steady-state) — **and** A's socket got `binding_revoked` (NOT a socket close). B (the
   new bindee) keeps receiving. Cover **BOTH** fan-out surfaces: the bus handler (message/status) and the
   GPS relay loop (`client_location` AND `client_location_stop`). **NEW-E + R3-3 drift guardrail (THREE
   sites):** a structural test/lint asserts all **three** raw courier-send sites — (1) the role-AGNOSTIC bus
   room handler (`websocket.ts:36-44`), (2) `client_location`, (3) `client_location_stop` — fan out to
   courier-joinable rooms ONLY via the shared `guardedCourierRelay` helper. The guardrail is keyed on the
   fan-out **site**, NOT on a `role==='courier'` test (which would miss the role-agnostic bus handler — the
   exact gap that re-creates the leak). A raw `member.ws.send` over a courier-joinable room outside the
   helper = build error. Behaviourally assert eviction holds on all three (reassignment trigger frame on the
   bus handler; `client_location` AND `client_location_stop` on the GPS loop).
7. **Fail-mode + ceiling (Breaker H1/L2 + NEW-A):** assert `UNAVAILABLE` (DB error) on a **subscribe**
   yields a *retryable* error, NOT a permanent `Forbidden room`, and does NOT `ws.close()` the socket (no
   reconnect loop); assert revalidation `UNAVAILABLE` does NOT evict an already-admitted member **on the
   first occurrences**, BUT **DOES evict at the ceiling** (`>=N consecutive UNAVAILABLE` OR `>~60 s` →
   `binding_revoked` reason `authz_unavailable_ceiling`). This pins the honest bound "≤ceiling under
   sustained DB-UNAVAILABLE" — run RED against a never-evict-on-UNAVAILABLE stub (leak persists past the
   ceiling) to prove the test bites.
8. **NOBYPASSRLS-safety (Breaker NEW-B + R3-2) — the E2E MUST run under NOBYPASSRLS.** With the operational
   role forced **NOBYPASSRLS** (set in the test's DB role/connection, not assumed) and `courier_assignments`
   FORCE-RLS, a *bound* courier still gets ALLOW on WS `order:<O>` + 200 on REST threads (proves tenant
   context is genuinely live at SELECT time). Run RED against **two** broken stubs to prove the test bites:
   (a) a **no-`set_config`** predicate (no tenant context at all → every courier denied / 404 = the
   deny-all outage); **(b) a `set_config(…,true)`-WITHOUT-`BEGIN` predicate (the `assignments.ts:111`
   shape)** — under NOBYPASSRLS the local setting dies before the SELECT → same deny-all. Both reds confirm
   the gate would have silently broken under the privilege-hardening unless the real `BEGIN…COMMIT` tx shape
   is used. Order-independent of B3.

**Process:** run each red FIRST against current code (expect leak / 200 / `subscribed`) to prove the
test bites, then green after the fix. Add a `docs/regressions/REGRESSION-LEDGER.md` row. Do not weaken
any existing gate; no skip/.only/inflated-timeout/commented assertion (test-integrity rules).
