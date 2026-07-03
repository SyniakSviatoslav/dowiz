# Breaker Findings — `courier-realtime-authz`

**Breaker:** System Breaker DeliveryOS · **Round:** 1 · **Date:** 2026-06-29
**Target:** `docs/design/courier-realtime-authz/proposal.md` + `docs/adr/0013-courier-realtime-authz.md`
**Verified against:** `apps/api/src/websocket.ts`, `routes/order-messages.ts`,
`routes/courier/assignments.ts`, `lib/dispatch.ts`, `lib/bindingRelease.ts`,
`workers/courier-offer-sweep.ts`, `routes/owner/dashboard.ts`, `lib/registry.ts`,
`packages/platform/src/message-bus.ts`, `apps/web/src/pages/courier/{TasksPage,DeliveryPage}.tsx`.

Verdict: **the cross-tenant subscribe hole is correctly diagnosed and Option A closes it. But the
R2 stale-membership eviction — the companion the proposal admits is REQUIRED (§7, R2: "FIX in PR") —
is built on a factually false premise and does not fire on the paths that create a victim. A
binding-scoped subscribe gate WITHOUT working eviction leaves the intra-tenant colleague leak fully
open the moment an order is reassigned/declined/expired. That is a surviving 🔴 hole.**

---

## CRITICAL

### C1 · B-FAIL / B-CONSIST — R2 eviction fires on the wrong paths; victim paths emit no `binding_changed`
The proposal §7 states the reassignment paths "already publish `binding_changed` on `orderChannel(O)`
(assignments.ts:456,511)" and lists `reject / abort / decline / reoffer — assignments.ts:178/404/472/524`.
**This is false.** Verified in code, `binding_changed` on `orderChannel(orderId)` is emitted at
**exactly two** sites — `assignments.ts:456` (`/cancel`) and `:511` (`/abort`). Those are the courier's
**own voluntary exits**: the only stale member is the courier who is leaving anyway, and the replacement
courier is not yet in the room. The eviction protects against essentially nothing.

The paths that create a genuine victim emit **no** `orderChannel binding_changed`:
- **Owner reassign A→B** (`owner/dashboard.ts:257-336`): terminalizes A's binding, then publishes only
  `courier:<B>` `task_offered` + `dashboardChannel` `offer_sent`/`offer_declined`. In the handshake
  branch the order is not advanced, so even `order.status` is silent. **Zero signal on `order:<O>`.**
- **`/decline`** (`assignments.ts:524-554`): row → `offered_expired`; publishes only `dashboardChannel`
  `offer_declined`.
- **`/reject`** (`assignments.ts:178-229`): publishes `BUS_CHANNELS.ORDER_CONFIRMED`, not `orderChannel`.
- **offer-sweep expiry** (`workers/courier-offer-sweep.ts:55`): publishes only `dashboardChannel`
  `offer_expired`.

**Break scenario (the decline race the brief calls out):** owner offers O to courier A (handshake) →
A gets `task_offered` on `courier:A` → A opens DeliveryPage → subscribes `order:<O>` → predicate sees
the `offered` row → **ALLOWED**, A joins the in-memory `Set`. A taps Decline (`/decline`) → row →
`offered_expired`, **no `orderChannel` event**. Order re-offered to B; B accepts, picks up, exchanges
`cc_*`/`cu_*` messages, customer streams GPS. A is **still in `order:<O>`** (websocket.ts:50 `Set`
never pruned) and receives every `order.message` (order-messages.ts:106) and every customer GPS relay
(websocket.ts:231-235 forwards `client_location` to **all** `role==='courier'` members of the room).
Leak persists until A's socket closes.

**Violated invariant:** binding-scoped least-privilege / intra-tenant (colleague) isolation on the
realtime fan-out. The in-memory room is "the only gate" (proposal §1) and it is never closed for the
displaced courier. The PR ships the subscribe gate believing R2 backstops the persistence; R2 does not
exist on the victim paths → the colleague leak the design claims to close (§8) stays open after any
reassignment/decline/expiry.

### C2 · B-SEC — `binding_changed` payload carries no `courierId`; the room model cannot target the evictee
The proposal §7 specifies eviction "on a `binding_released`/`binding_changed` control event **carrying
the released `courierId`**, it evicts that courier's member(s)." The actual event is
`{ type:'binding_changed', orderId }` (assignments.ts:456,511) — **no `courierId`**. The WS room handler
(websocket.ts:36-45) is a generic `JSON.stringify` forwarder with no control-event branch; the room is
`Set<RoomMember>` keyed by socket, and `RoomMember.user.sub` is the only courier identity available.

So even on the two paths where the event fires, the handler **cannot know which member to evict** from
the payload. The only payload-driven options are (a) evict **all** couriers from `order:<O>` — which
would also kick a legitimately-bound replacement who already re-subscribed, a self-inflicted denial, or
(b) re-query the DB per member on every `binding_changed` — unquantified in §2's back-of-envelope.
The design's stated mechanism is not implementable against the event it depends on.

**Violated invariant:** the §7 mitigation is internally inconsistent with the emitted contract; the
"keep the leak window to broadcast latency (sub-second)" guarantee is unfounded — the window is
"until the socket disconnects."

---

## HIGH

### H1 · B-SCALE — deploy reconnect storm vs. the shared operational pool (documented prior starvation)
§2 dismisses load as "<1 subscribe/sec steady… ~100/sec burst… trivial" and conflates steady
task-opens with a mass reconnect. A **deploy/restart drops every WS socket at once** — all active
couriers reconnect inside a few seconds, and each mid-delivery courier re-subscribes `order:<id>`,
firing one synchronous authz read on `fastify.db` (the **operational pool**, same one
`ownerCanAccessRoom` uses).

Back-of-envelope at the proposal's own 10× figure (5000 couriers), ~40% mid-delivery = **~2000
order-subscribes in a ~2-3 s reconnect window ≈ 700-1000 authz reads/sec burst**, on top of normal
order/menu read traffic, **on a pool with a documented starvation incident** ("public-menu pool
starvation": demo menu blinked empty = HTTP 500 from operational-pool conn-timeout). Because the gate
is **fail-closed** (§7), a transient pool exhaustion during the post-deploy stampede converts directly
into fleet-wide `Forbidden room` — couriers' order rooms go dark exactly when a deploy just happened.

**Violated invariant:** B-SCALE back-of-envelope must hold at the real burst shape (simultaneous
reconnect ≠ 10× steady). The "no cache needed / negligible" claim ignores the known shared-pool
saturation history; fail-closed turns a pool blip into a realtime outage.

### H2 · B-OPS / Regression — R1 id-conflation: FE subscribes `order:<assignmentId>`; the gate will deny every legitimate courier
Verified: `DeliveryPage.tsx:121` subscribes `room: \`order:${id}\`` where `id = useParams()` (`:30`).
The route is `delivery/:id` (`CourierRoutes.tsx:88`); the param is the **assignment id** — DeliveryPage
fetches `/courier/assignments/${id}` (`:93`), which is keyed on `ca.id` (assignments.ts:114), and
TasksPage navigates with the same `id` it passed to `/courier/assignments/${id}/accept`. So today
couriers subscribe `order:<assignmentId>` — a phantom room — which is also **why the current
cross-tenant leak is reachable only by a hand-crafted client, not the FE**.

Binding-scoped authz keys on `order_id`; `order:<assignmentId>` has no matching row →
**`courierCanAccessRoom` returns false → `Forbidden room` for every real courier**. The fix is only
correct if the FE ships `order:${task.orderId}` in the **same atomic deploy**. The proposal flags this
(R1, "FIX in PR") but understates the coupling: if the WS authz reaches prod before/without the FE
change (independent build artifacts), the entire fleet loses its order room.

**Violated invariant:** a security tightening must not silently break the legitimate flow; the gate and
the FE room-key change are a single indivisible unit, not "FIX in PR" as a footnote.

---

## MEDIUM

### M1 · B-SEC / factual — §5/ADR claim the REST predicate "runs under the existing `app.current_tenant` context"; it does not
`order-messages.ts` issues **every** query via the plain pool (`db.query`, lines 13, 22, 56, 96, 127,
136, 158, 167) — **no `client.connect()`, no `BEGIN`, no `set_config('app.current_tenant', …)`**.
There is no tenant context on this surface. The proposal §5 / ADR "Compliance with red lines" instruct
the implementer to "verify it is added inside, not before, the tenant-set on any transactional path" —
a verification premised on a transaction/tenant-set that this route does not have.

**Break:** if the operational role is **not** `BYPASSRLS` and `courier_assignments` is `FORCE`-RLS, the
new `EXISTS` predicate runs with no `app.current_tenant` → returns zero rows → **denies all couriers**
(over-deny). If it is `BYPASSRLS`, the predicate sees all tenants' rows and relies entirely on the
`courier_id = sub` filter for isolation — workable, but the design's stated RLS posture for this surface
is fiction. Memory note "verify:rls fails (likely BYPASSRLS env artifact)" makes the role's RLS status
unconfirmed — the implementer is told to verify the wrong thing.

**Violated invariant:** B-CONSIST/B-SEC — the authz predicate's correctness depends on a tenant context
the design asserts exists but the code does not establish.

### M2 · B-CONSIST — R3 "subscribe-time only" + broken R2 = mid-stream owner-reassign is the uncovered case
§10 R3 accepts subscribe-time-only revalidation "for non-reassignment terminal cases
(delivered/cancelled)… covered by R2 for reassignment." But R2 is non-functional on the reassignment
paths (C1/C2). The **owner reassign A→B mid-run** is neither a terminal-for-A case nor R2-covered: A's
binding is `cancelled`/`offered_expired` but A keeps the live room. So the one case R3 explicitly
delegates to R2 is precisely the one R2 misses. The accepted-risk table launders a live leak as
"covered."

**Violated invariant:** revocation must take effect; a revoked binding still streaming is a
consistency/isolation breach, not an accepted edge.

---

## LOW

### L1 · B-OPS — observability watches the wrong event; the actual leak emits no deny
§9 proposes a counter on **courier authz DENY** for "<1 min" visibility. The C1 stale-room leak is a
**successful past subscribe** that keeps receiving — it generates **no deny log and no new subscribe**.
The proposed metric cannot surface the actual leak; a spike of DENYs would instead be the H2 regression
(legitimate couriers denied), inverting the signal's meaning.

### L2 · B-FAIL — fail-closed-by-throw churns the socket, feeding H1
If `courierCanAccessRoom` throws (rather than catching → `false`), the outer `ws.on('message')`
try/catch does `ws.close(1008, 'Invalid message')` (websocket.ts:256-259) — fail-closed (good) but it
**drops the whole socket**, not a soft per-room deny. A transient DB error during a storm → socket
close → reconnect → another authz read → reconnect loop that amplifies H1. The design assumes a clean
`return false`; the surrounding handler escalates a throw to a disconnect.

---

## Verified NON-findings (attacks that did not land)
- **No location-wide fallback on `order:`.** Option A's predicate is pure `(order_id, courier_id,
  status)` with no location fallback (Option C rejected); `location:*` is denied outright. The
  `order.location_id === activeLocationId` weakening was genuinely not adopted. ✓
- **`activeLocationId` staleness is irrelevant to `order:` rooms.** The order predicate never reads
  `activeLocationId`, so a stale/multi-location token cannot widen or wrongly deny order-room access. ✓
- **No missed courier-reachable `location:` channel.** `courierChannel` (`location:<id>:couriers`) is
  published by shifts/courier-events but **subscribed only by the owner** `DashboardPage.tsx:163`; the
  courier FE subscribes only `courier:<sub>` (TasksPage) and `order:<id>` (DeliveryPage). Denying
  `location:*` for couriers breaks no legitimate courier flow. ✓
- **REST send-gate is sound; REST self-heals on decline.** POST requires `('assigned','accepted',
  'picked_up')` → an `offered`-only or declined (`offered_expired`) courier is denied on the next call;
  REST re-checks per request, so the stale-access problem is **WS-only** (which is exactly why C1 is the
  whole ballgame). ✓
- **Bus is PG LISTEN/NOTIFY (cross-instance).** `binding_changed` would reach all API instances, so
  per-instance eviction is topologically possible — the blocker is the missing `courierId` (C2), not
  the transport. (Note: customer GPS relay at websocket.ts:231 is **local-Set only**, not bus-fanned —
  pre-existing, out of scope, but it means the GPS leak in C1 requires victim + customer on the same
  instance.) ✓

## Regression risk to legitimate flows (summary)
- Denying `location:*`: **no regression** (couriers never subscribe it).
- Binding-scoped `order:`: **regression unless H2 (FE `order:${task.orderId}`) ships atomically** — else
  every courier is denied their own order room.
- Fail-closed: a pool blip (H1) denies the fleet; a throw (L2) disconnects the fleet.

---

# Breaker Findings — Round 2 (RE-ATTACK on the revision)

**Breaker:** System Breaker DeliveryOS · **Round:** 2 · **Date:** 2026-06-29
**Target:** revised `proposal.md` (§2,§7,§9,§10,§11) + `resolution.md` + `docs/adr/0013` (fan-out-time
revalidation + 10 s TTL authz cache + tri-state ALLOW/DENY/UNAVAILABLE + atomic FE deploy).
**Verified against:** `apps/api/src/websocket.ts:33-52,165-253` (sync bus handler + sync GPS relay loops),
`routes/order-messages.ts:12-72` (`getOrder`/`hasCourier` via plain `db.query`, no tenant set),
`routes/courier/assignments.ts:456,511,553` (publish sites), `packages/db/migrations/1790000000073…:43-47`
+ `1780421100041…:26-29` (`courier_assignments` FORCE-RLS, policy keyed on `app.current_tenant`),
`1790000000069_provision-grants.ts:19-20` ("role effectively NOBYPASSRLS"), `1780691681296…:8,21`.

**Verdict: exit bar NOT met.** Two new HIGH findings block (NEW-A, NEW-B). The Round-1 CRITICALs
(C1/C2) ARE genuinely resolved by the redesign, and H2/L1/L2 are resolved-in-design — but the H1 fix
(tri-state) and the M1 "correction" each open a fresh hole on the same 🔴 lines they were meant to close.

## Regression re-verification of Round-1 findings
- **C1 — RESOLVED (design).** Re-verified victim paths: `binding_changed` on `orderChannel` fires ONLY at
  `assignments.ts:456` (`/abort`) and `:511` (`/cancel`); `/decline` (`:553`) emits `dashboardChannel`
  only. Event-driven eviction was indeed unworkable; fan-out revalidation is self-healing on all paths.
  Contingent on implementing BOTH relay surfaces (see NEW-E).
- **C2 — RESOLVED (design).** `RoomMember.user.sub` exists (`websocket.ts:9,167`); per-member DB
  revalidation targets the evictee without a `courierId` in any payload. Mechanism is implementable.
- **H1 — availability RESOLVED, but the fix SPAWNS NEW-A.** Tri-state stops fleet-wide `Forbidden` on a
  pool blip; the `UNAVAILABLE→don't-evict` half it relies on re-opens the confidentiality leak (NEW-A).
- **H2 — RESOLVED (design), DoD-contingent.** Atomic deploy + real-route E2E is the right shape; cannot
  verify (no code yet). The gate only bites if the §11 item-4 E2E truly drives `/courier/delivery/:id`.
- **M1 — NOT resolved; the "correction" is itself false → NEW-B.** Round-1 fiction ("runs under tenant
  context") was replaced by a second fiction ("sound regardless of BYPASSRLS").
- **M2 — mostly resolved; re-opened by NEW-A** under DB stress (revalidation `UNAVAILABLE`→no evict).
- **L1/L2 — RESOLVED (design):** eviction counter added; tri-state never throws.

## HIGH

### NEW-A · B-FAIL / B-SEC — tri-state `UNAVAILABLE→don't-evict` breaks the ≤TTL bound; leak becomes "until DB recovers", co-located with the deploy storm
The redesign's entire safety case for shipping a known leak on a 🔴 privacy line is the bound
"worst-case leak window after any terminalization = **≤ TTL (~10 s)**, on every path" (proposal §7,
resolution L77-79). But §7/resolution L111 also specify: on revalidation of an already-admitted member,
`UNAVAILABLE` (DB error/timeout/pool-exhausted) **does NOT evict** (retry ~2 s, relay this frame). These
two statements are mutually exclusive. When the DB/pool is degraded, every revalidation of a *displaced*
courier returns `UNAVAILABLE` → 0 evictions → the displaced courier **keeps receiving customer GPS
(~1/s) + chat** for the **entire degradation window**, not ≤TTL.
- **Scenario:** owner reassigns O from A→B at t0 (no deploy). At t0 the operational pool is degraded
  (the documented "public-menu pool starvation" mode, or a Supavisor/DB hiccup). For the next 30–60 s
  every `client_location`/`order.message` frame to A triggers a stale-cache revalidation → `UNAVAILABLE`
  → A is NOT evicted and the frame IS relayed. A receives the customer's live GPS track + the A↔B/customer
  thread for 30–60 s. Claimed bound 10 s; actual ≈ DB-recovery time (unbounded).
- **The two HIGH fixes compose into the leak:** H1 (Round-1) identified the deploy reconnect storm as the
  exact moment the pool saturates. NEW-A says pool saturation → revalidation `UNAVAILABLE` → no eviction.
  So a reassignment during a deploy/storm — when displacement (re-dispatch churn) AND pool stress co-occur
  — re-opens the leak fleet-wide for dozens of displaced couriers simultaneously, precisely when the
  design claims it is bounded.
- **Violated invariant:** the ≤TTL confidentiality bound (the sole basis for accepting the leak on a 🔴
  red-line) does not hold under the fail-safe the design itself chose. "Revocation fails safe to avoid
  mass-evicting legit couriers" silently trades a 🔴 confidentiality bound for availability — undocumented
  as such.

### NEW-B · B-SEC / B-DATA — the binding predicate reads FORCE-RLS `courier_assignments` with NO `app.current_tenant`; sound ONLY under BYPASSRLS — and THIS branch removes BYPASSRLS → fleet-wide courier over-deny
The M1 "correction" (proposal §5, ADR "Compliance", resolution M1) asserts: isolation "is the explicit
`courier_id=$sub` binding predicate — not RLS … **sound regardless of the operational role's BYPASSRLS
status**," justified by "the route demonstrably functions today without `app.current_tenant`, which …
confirms isolation on this surface is app-level." **This is inverted from reality.**
- Verified: `courier_assignments` is `FORCE ROW LEVEL SECURITY` with policy
  `USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid)`
  (`1790000000073…:43-47`). Both new read sites — `courierCanAccessRoom` (WS, `fastify.db`) and the REST
  `hasCourier`/binding predicate (`order-messages.ts:22`, plain `db.query`) — set **no** tenant context.
- Under `NOBYPASSRLS`, `current_setting('app.current_tenant', true)` is empty → `NULLIF→NULL` →
  `location_id = NULL` → **0 rows for every order** → `courierCanAccessRoom`/binding → DENY → **every
  courier is `Forbidden room` on every order + 404 on every message thread.** Fleet-wide realtime +
  messaging outage, not a leak.
- "Functions today" proves the OPPOSITE of the claim: owners receiving order feeds (`ownerCanAccessRoom`
  reads FORCE-RLS `orders` with no tenant set, yet QA is 6/6 green) is only possible if the operational
  role is **BYPASSRLS at runtime**. So the predicate is sound **iff BYPASSRLS** — the exact inverse of
  "sound regardless." 
- **The landmine is on this same branch:** `feat/mvp-sensor-seams` carries the pg-privilege-hardening
  work whose explicit goal is `deliveryos_api_user` → NOBYPASSRLS (`1790000000069…:19-20` already states
  "role effectively NOBYPASSRLS"; `1780691681296…` toggles BYPASSRLS on/off transactionally). The day
  that lands, this PR's predicate (which §5 *deliberately* refuses to wrap in `withTenant` as
  "scope-creep") flips to deny-all-couriers. The design bakes in a cross-cutting outage and labels the
  dependency a non-dependency.
- **Violated invariant:** 🔴 RLS correctness claim is false; the predicate's correctness is coupled to a
  BYPASSRLS posture the project is actively dismantling, with the coupling explicitly waved off.

## MEDIUM

### NEW-C · B-CONSIST / B-ANTIPATTERN — async per-member revalidation grafted onto two SYNCHRONOUS fan-out loops; relay-then-revalidate leaks the trigger frame each TTL boundary
The bus room handler (`websocket.ts:36-45`) is `(msg)=>void` — a synchronous `for…m.ws.send()` loop. The
GPS relay loops (`:231-235`, `:246-250`) are likewise synchronous. The spec ("cache miss/stale → run the
point-read, THEN relay/evict") requires a per-member `await` on the hot path. Two non-equivalent
implementations result, neither clean: (a) make the loop async and await per member → serializes fan-out
and adds DB latency to delivery; or (b) keep the loop sync, relay immediately and revalidate
out-of-band → **the very frame that crosses the stale-TTL boundary is relayed to the now-unauthorized
member before the read returns** (a fresh per-TTL leak of exactly one customer-GPS/chat frame to a
displaced courier). The DoD (§11 item 6) asserts "ZERO further frames within ≤TTL" but does not pin the
sync-vs-async choice, so a green test with implementation (b) still leaks one frame per TTL tick.

### NEW-D · B-CONSIST — self-contradiction: subscribe gate is both "must be live/uncached for correctness" AND "backed by the shared TTL cache"; TTL semantics (fixed vs sliding) unspecified
§6 (L213-215) and §2 (L69-70): "re-evaluates authz each time … the live check is the correctness
mechanism (another reason **not to cache**)." §2 (L78) and resolution (L96): "the **same** `(orderId,sub)`
cache backs the **subscribe gate**, the fan-out guard, and REST." Both cannot hold. If subscribe reads
the cache, a just-revoked courier re-subscribing within TTL is **re-admitted on a stale ALLOW** — the
subscribe gate is no longer the live correctness check §6 relies on. Worse, the TTL discipline is
unspecified (fixed vs sliding): a **sliding** TTL refreshed on each access + a courier flapping reconnect
every <10 s (WiFi↔LTE, named in §2) keeps the entry warm indefinitely → eviction **never fires** → leak
unbounded for a flapping displaced courier. The load model (which counts subscribe reads as cache-served)
and the correctness claim (which needs them live) are mutually exclusive.

### NEW-E · B-SEC / regression — the Round-1 bug class reincarnated: identical NEW revalidation+cache+evict logic must be hand-duplicated across two structurally different relay sites
Round-1's root cause was "two fan-out paths, one guarded." The redesign now requires the *same* nontrivial
logic (cache lookup → point-read on miss → tri-state → evict-with-`binding_revoked` → optional cache-bust)
in two sites with **different execution contexts**: the bus subscription callback (`:36-45`, sync, runs
for every bus message on every room) and the customer-message GPS loop (`:220-253`, async, runs in the
customer's `ws.on('message')`). Divergence (one caches and the other re-reads; one busts on
`binding_changed` and the other doesn't; one evicts and the other only stops-relaying) re-creates the
exact "guarded on path X, open on path Y" leak. The only backstop is §11 item 6 covering both surfaces —
a test-discipline mitigation, not a structural one.

## LOW

### NEW-F · B-DATA — process-local authz cache has no size bound or sweep; grows with order volume between deploys
The cache is a `Map` keyed `(orderId, courierSub)` with lazy TTL (entries are only re-evaluated on the
next frame for that key). Completed/terminal orders' entries are never accessed again → never removed.
At 10× (~2000 orders/hr × multiple couriers each) the Map accretes tens of thousands of dead entries/hr
with no LRU or periodic sweep specified → unbounded heap growth between deploys. Practically capped only
by restart cadence — a smell, not a crash, but unspecified.

### NEW-G · B-SEC — "FE purges customer PII on eviction" is not a boundary; the in-PR claim "closes the live-session half of §5" is overstated
The real privacy boundary is server-side stop-sending; the FE purge is enforceable only on an honest,
online client. An evicted courier whose app is offline, backgrounded, or modified simply keeps the last
frame — the server cannot recall already-delivered GPS/chat. Worse, an *offered-then-declined* courier
has already fetched the customer's **delivery address via REST** (`/courier/assignments/:id`) onto the
device before declining; eviction cannot recall it. So against the actual threat model (a displaced
courier who wants the customer's address/track), the purge closes nothing — the ≤TTL (or, per NEW-A,
unbounded) window of already-delivered customer PII is permanently device-resident. Disposition should be
"accept the already-delivered PII is unrecoverable," not "live-session half closed."

### NEW-H · B-SCALE — H1c cold-cache "sub-ms read" bound is circular; the real isolation (dedicated pool) is deferred, and degradation feeds NEW-A
The cold-cache reconnect-wave bound rests on "5 conns × **sub-ms point-reads** ≫ 1000 reads/s." But the
sub-ms latency assumption holds only when the operational pool is healthy — the very condition that fails
under the documented starvation mode the mitigation exists to survive. A real at-scale deploy: cold cache
→ all-miss subscribe wave + resuming revalidation reads, all multiplexed over the `max=20` operational
pool that has starved before → read latency degrades → timeouts → `UNAVAILABLE` → (subscribe) retry storm
+ (revalidation) **no eviction (NEW-A) → leak**. The one structural fix (dedicated 5-conn authz pool) is
DEFERRED/accepted-risk, so the accepted risk is "realtime authz degrades on every at-scale deploy," and
that degradation is the trigger for NEW-A's confidentiality leak.

## Verified NON-findings (Round 2)
- **Eviction IS targetable** without a `courierId` event: `RoomMember.user.sub` (`:9,167`) + DB binding
  state is sufficient — the redesign's core premise holds. ✓
- **C1's victim-path inventory confirmed:** `orderChannel binding_changed` only at `:456/:511`; abandoning
  event-driven eviction was the correct call. ✓
- **GPS-vs-bus reachability asymmetry persists (pre-existing, not introduced):** the GPS relay is
  local-Set only, so the GPS half of the NEW-A leak requires displaced-courier + customer on the same
  instance; the `order.message` half is cross-instance (bus). Narrows GPS leak reach, not the chat leak. ✓

## Exit-bar verdict
**0 unresolved CRITICAL** (C1/C2 genuinely fixed by the redesign). **NOT 0 unresolved HIGH:** NEW-A
(≤TTL bound false under DB degradation, co-located with the deploy storm — 🔴 confidentiality) and NEW-B
(predicate sound only under BYPASSRLS, which this branch removes → fleet-wide courier over-deny + false
🔴 RLS invariant) both stand. **Exit bar NOT met.** MEDIUM (NEW-C/D/E) and LOW (NEW-F/G/H) should be
dispositioned but do not block on their own.

---

# Breaker Findings — Round 3 (RE-ATTACK — the EXIT-BAR check)

**Breaker:** System Breaker DeliveryOS · **Round:** 3 · **Date:** 2026-06-29
**Target:** Round-2 resolution (`resolution.md` §"Round 2") + revised `proposal.md` (§2/§5/§6/§7/§9/§10/§11)
+ `ADR-0013` — the NEW-A fail-safe eviction ceiling and NEW-B set-tenant-context fixes.
**Verified in code:** `websocket.ts:7-10,33-52,165-253` (RoomMember=AuthToken; sync bus handler; sync GPS +
`client_location_stop` loops), `courier/assignments.ts:78-98,109-122,138-153` (the cited tenant convention),
`packages/db/migrations/1790000000073…:42-50` (FORCE-RLS policy: courier branch = `app.current_tenant`,
owner branch = `app_member_location_ids()`), `packages/shared-types/src/legacy.ts:163-175`
(`AuthToken` courier variant carries required `activeLocationId`).

**Verdict (headline): the 🔴 confidentiality/isolation line now HOLDS. Exit bar IS MET for CRITICAL/HIGH on
the security axis.** NEW-A's ceiling genuinely bounds the leak to ≤60 s **decoupled from pool health**, and
NEW-B's tenant-context fix is correct and order-independent vs. B3. C1/C2/H1/H2 and the round-1 fixes are
**not regressed**. The residuals are (a) one **availability** compounding the design names but does not
honestly re-quantify (R3-1), (b) two **implementation/guardrail-DoD** hazards where following the design's
own citation/wording silently reintroduces the very hole it closes (R3-2, R3-3), and (c) named tuning
items. **None re-open a confidentiality 🔴 hole.** Per the exit-bar rubric these are accepted-risks / DoD
to tighten before merge, not design blockers.

## Regression re-verification (round-2 + round-1 fixes still closed?)

- **C1 — STILL RESOLVED.** Fan-out revalidation is unchanged by the round-2 fixes (they *add* the ceiling +
  tenant context to it). Self-healing on all victim paths intact.
- **C2 — STILL RESOLVED.** `RoomMember.user` is a full `AuthToken` (`websocket.ts:9`); the evictee is
  targetable via `user.sub` + binding state, no `courierId` payload needed. Confirmed.
- **H1 — RESOLVED (availability) and the NEW-A spawn is now closed by the ceiling** — but the NEW-B fix
  re-loads the pool (R3-1 below). Net: availability still depends on the deferred dedicated pool.
- **H2 — RESOLVED-in-design (DoD-contingent).** Atomic FE + real-route E2E unchanged. `toTaskShape`
  exposes `orderId` (assignments.ts:25) — the FE *can* subscribe `order:${task.orderId}`. Only the E2E
  driving the real `/courier/delivery/:id` proves it; cannot verify pre-code.
- **C1/C2 fan-out self-heal on owner-reassign/decline/reject/sweep — STILL RESOLVED** (revalidation is
  path-agnostic; the optional cache-bust only accelerates).
- **H2 atomic-FE, GPS-loop-vs-bus parity — see R3-3** (parity is *designed* but the stated guardrail
  predicate has a blind spot at the bus handler).

## NEW-A ceiling — ATTACKED, HOLDS

- **Dodge via intermittent ALLOW resetting the counter+wall? NO.** The spec resets both
  `consecutiveUnavailable` and `firstUnavailableAt` on `ALLOW`. But a **displaced** courier (the leak
  population) can *never* receive an `ALLOW` from a live read: their binding is gone, so a *successful*
  read returns `DENY` (→ immediate evict), and a *failed* read returns `UNAVAILABLE`. During a flapping
  pool a displaced courier sees only `{UNAVAILABLE | DENY}` — never `ALLOW` — so the streak/wall cannot be
  reset by them. The reset only ever benefits a *legitimately-bound* courier (correct). The dodge does not
  exist for the actual threat. **HOLDS.**
- **Does the ~60 s wall fire during pool starvation (when revalidation itself needs the pool)? YES.** The
  ceiling test (`consecutiveUnavailable>=N` OR `now−firstUnavailableAt>CEILING`) is **in-memory
  arithmetic** evaluated on each fan-out frame arrival, and the eviction itself (remove from `Set` +
  `ws.send(binding_revoked)`) needs **no DB**. So even under a 100 %-down pool the next GPS/order frame
  triggers a sync check that evicts. The ceiling is genuinely **decoupled from pool health** — this is the
  load-bearing property and it holds. (Edge: if frames *stop* arriving, no eviction fires — but no frame =
  no leak, so confidentiality is preserved regardless; only the `binding_revoked`/FE-purge signal is
  delayed until traffic resumes. Not a hole.)

## R3-1 · B-SCALE / B-FAIL — NEW-B's `set_config` REQUIRES an explicit transaction → every authz read becomes a connection-pinning tx, compounding H1/NEW-A; §2 back-of-envelope NOT re-derived [HIGH-availability, dispositioned — accepted-risk, math must be redone]

`set_config('app.current_tenant', $1, true)` (`is_local=true`) only persists **for the current
transaction**. Verified in the cited convention itself: `assignments.ts:79` wraps it in
`BEGIN…set_config…SELECT…COMMIT`. So NEW-B converts each authz read — at **subscribe**, at **every fan-out
revalidation re-read**, and at **REST** — from a single multiplexed `db.query` (round-1's deliberate M1
choice) into a **multi-statement transaction** (`connect → BEGIN → set_config → SELECT → COMMIT →
release`) that **pins a pooled server connection for ~4 round-trips**, not one statement.

- **The §2 math is now stale.** §2 still reads "sub-ms indexed point-reads behind **Supavisor txn-mode
  multiplexing** over the `max=20` operational pool." In txn-mode Supavisor pins a server connection for
  the **whole transaction** — and there now *is* an explicit transaction. Concurrency =
  `rate × tx-duration`. Under the H1 deploy storm (~700–1000 authz reads/s) with a **latent** pool (the
  documented starvation mode), tx-duration balloons from sub-ms to tens of ms → required concurrent
  connections blow past the `max=20` pool (minus order/menu traffic) → exhaustion → `UNAVAILABLE` →
  NEW-A's ceiling **bounces couriers fleet-wide**. NEW-B's fix therefore *increases* the very pool pressure
  that triggers the ceiling it is co-shipped with.
- **Why this is NOT a confidentiality blocker:** NEW-A decouples the 🔴 leak bound from pool health, so the
  failure mode here is **availability** (couriers soft-bounce, recoverable), and the design names the
  mitigations (bounded-concurrency semaphore ≈10, jittered reconnect) + a **measurable** scaling-gate
  trigger (`authz_unavailable_ceiling` eviction rate). 
- **The real gap:** the design **acknowledges** the tx-pin in one clause ("holds the pool conn for the tx
  duration … why the semaphore + dedicated-pool trigger matter") but **does not re-run the back-of-envelope
  with the transaction-pinning cost** — the numbers still assume single-statement multiplexing, and the
  semaphore (≈10) now caps *transactions holding connections*, not *statements*, so it can consume up to 10
  of 20 pool connections for the full latent-tx duration during a storm. **DoD ask:** redo §2's burst math
  for `connect+BEGIN+set_config+SELECT+COMMIT` connection-hold time and state the semaphore-vs-pool
  budget honestly. **Violated invariant:** B-SCALE — back-of-envelope must hold at the real cost shape;
  the costed shape changed (statement→transaction) and the number did not.

## R3-2 · B-SEC / B-DATA — the cited tenant convention is INCONSISTENT; `assignments.ts:111` omits BEGIN → `set_config(true)` is lost → copying it reintroduces NEW-B's deny-all under NOBYPASSRLS [MEDIUM implementation-hazard, backstopped by §11 item-8 IFF it runs NOBYPASSRLS]

§5 / ADR cite `assignments.ts:79/111/139/190` as "the exact existing convention." They are **not
uniform**: `:79/:139/:190` wrap `set_config(...,true)` in `BEGIN…COMMIT`; **`:111` (the `/assignments/:id`
single-fetch DeliveryPage uses) does `set_config(...,true)` with NO transaction** — so the `is_local=true`
setting applies only to that one autocommit statement and is **gone before the SELECT**. Under BYPASSRLS
this "works" (RLS bypassed); under NOBYPASSRLS the SELECT runs with empty `app.current_tenant` → 0 rows.
- **Demonstrable hazard:** if the implementer models `courierCanAccessRoom`/`courierHasBinding` on the
  `:111` shape (the nearest single-statement read, and the one for the *same* DeliveryPage surface), the
  gate **silently re-creates the exact deny-all-couriers outage NEW-B claims to fix** — the failure is
  invisible under today's BYPASSRLS runtime and only bites when B3 lands.
- **Collateral (pre-existing, out-of-PR but undercuts the "NOBYPASSRLS is safe here" confidence):**
  `:111` itself means the DeliveryPage assignment fetch already breaks under NOBYPASSRLS regardless of this
  PR. The design's "this surface functions / is order-independent" reasoning rests on a sibling that is
  itself a latent NOBYPASSRLS landmine.
- **Backstop:** §11 item-8 (NOBYPASSRLS-safety E2E, red against a no-`set_config` predicate) catches this
  **iff** the test truly forces `NOBYPASSRLS` on the role — pin that in the DoD, or the most likely
  copy-paste error ships green. **Violated invariant:** B-SEC — the predicate's soundness depends on a
  transaction the cited example omits; "follow the convention" is ambiguous and one branch is broken.

## R3-3 · B-SEC / regression — NEW-E drift guardrail predicate has a BLIND SPOT at the role-agnostic bus handler (the primary `order.message`/status leak site) [MEDIUM, backstopped by E2E item-6]

The NEW-E guardrail is worded: "a raw `member.ws.send(...)` to a **`role==='courier'`** member outside
`guardedCourierRelay` is a build error." But the **bus room handler** (`websocket.ts:36-45`) — the surface
that fans `order.message` + status deltas to courier members of `order:<O>` (the chat half of C1) — sends
**role-agnostically**: `for (const m of members) { … m.ws.send(payload) }`, with **no `role==='courier'`
test**. A static rule keyed on an explicit courier-role check **cannot see** that this generic loop
includes couriers, so:
- write the rule narrowly (match `role==='courier'`) → it **misses the bus handler** → the highest-value
  leak path can be left raw-sending to couriers while the guardrail is green ("guarded on the GPS loops,
  open on the bus handler" — the Round-1 root-cause class, reincarnated *inside the guardrail*);
- write it broadly (flag **all** `m.ws.send` in any fan-out) → it also flags owner/customer sends, forcing
  every room's fan-out through a courier-named helper.
Either way the design under-specifies the integration: the bus handler must be **restructured to branch by
role** and route courier members through `guardedCourierRelay`, and the guardrail predicate must be
written to catch the *role-agnostic* loop, not just the two explicitly-courier GPS loops. Note also there
are **three** courier-reachable raw-send sites, not "two surfaces": bus `:40-44`, `client_location`
`:231-235`, `client_location_stop` `:246-250`. **Backstop:** §11 item-6 covers "BOTH surfaces" behaviourally
— but a behavioural test on order:<O> reassignment will exercise the bus path, so it *does* bite; the
structural guardrail is the weaker leg. **Violated invariant:** B-SEC — the structural anti-drift control,
the stated cure for the Round-1 two-paths-one-guard bug, does not statically cover the main path.

## R3-4 · B-SCALE / B-FAIL — `N=3` consecutive UNAVAILABLE at ~1 Hz GPS makes the *effective* ceiling ~3 s, not 60 s → routine brief blips bounce LEGIT mid-delivery couriers [LOW/tuning, SAFE direction, DoD-tunable]

The ceiling fires on `N=3` consecutive `UNAVAILABLE` **OR** `>~60 s`. For an active GPS stream (~1 frame/s)
three withheld frames elapse in **~3 seconds**, so the **3-frame** arm — not the 60 s wall — is the live
trigger. A routine sub-5 s pool-contention spike (which the documented starvation history shows happens)
then **evicts every mid-delivery courier whose cache entry was stale at that instant**, legit and displaced
alike, forcing soft reconnects across the fleet. The "≤60 s" framing **understates** how aggressively the
ceiling bounces legitimate couriers under normal brief contention. This is the **safe** direction
(over-evict + recoverable, never under-evict), so it is **not** a confidentiality hole — but the
courier-bounce frequency under routine brief blips is **un-quantified**, and the per-frame-vs-per-re-read
increment semantics for `consecutiveUnavailable` are **unpinned** (both stay under the wall, but they give
different effective ceilings). **DoD:** name `N` and the increment unit as tunable accepted-risk with a
bounce-frequency budget; consider decoupling `N` from frame rate (e.g., time-since-first-UNAVAILABLE only).

## NEW-B cross-location edge — re-checked, correctly flagged, NOT a new hole

A courier whose `activeLocationId ≠ binding.location_id` (stale token / mid-shift location switch with a
live binding) is RLS-denied their own order under NOBYPASSRLS (policy courier branch =
`location_id = app.current_tenant = activeLocationId`). This **wrongly denies a legit cross-location
assignment** — but it is **identical to existing `assignments.ts` behaviour** (same `activeLocationId`
tenant source, same table, same policy), is **moot under BYPASSRLS**, and is **design-flagged** as a
watch-item. The revalidation uses the **subscribe-time** `member.user.activeLocationId` (frozen at
subscribe), adding a second staleness window — but it converges to the same deny, which is the safe
direction. **Not a NEW hole introduced by this PR;** a pre-existing latent multi-location gap that
NOBYPASSRLS would expose system-wide. Acceptable as the flagged watch-item.

## NEW-C / NEW-D — re-checked, HOLD

- **NEW-C (relay-only-on-fresh-ALLOW):** the cache lookup is a **sync `Map` read** with no `await` in the
  fan-out loop → the loop is NOT serialized and DB latency is NOT added to the hot path; the re-read is
  fire-and-forget + deduped. The trigger/TTL-boundary frame is **withheld**, not relayed-then-revalidated.
  Sync-safe. **HOLDS.** (The fire-and-forget re-read is now a tx — see R3-1 for the connection cost.)
- **NEW-D (fixed absolute TTL):** no refresh-on-access → a WiFi↔LTE flap cannot keep a revoked entry warm;
  subscribe-on-stale-ALLOW is ≤TTL-bounded by the *same* fixed entry the fan-out guard re-reads. **HOLDS.**

## Exit-bar verdict (Round 3)

- **CRITICAL:** C1/C2 **remain RESOLVED, not regressed** → **0 unresolved.**
- **HIGH (confidentiality / 🔴 isolation):** NEW-A ceiling **HOLDS** (leak ≤60 s, decoupled from pool
  health, no dodge for the displaced population); NEW-B tenant-context **correct + order-independent vs
  B3** → **0 unresolved confidentiality HIGH.**
- **HIGH (availability):** **R3-1** — NEW-B's `set_config`-in-a-tx compounds H1/NEW-A pool pressure; the
  design **names** the mitigations + a measurable scaling-gate trigger and NEW-A keeps it off the 🔴 line,
  so it is an **accepted-risk with an owner**, BUT §2's back-of-envelope **must be honestly re-derived** for
  the transaction-pinning cost before merge (DoD, not a design blocker).
- **MEDIUM:** **R3-2** (cited convention inconsistent; `:111` shape breaks under NOBYPASSRLS — pin the
  item-8 E2E to force NOBYPASSRLS) · **R3-3** (NEW-E guardrail predicate blind to the role-agnostic bus
  handler — the main leak site; tighten the rule + restructure the bus fan-out). Both are
  **implementation/guardrail DoD**, backstopped by the §11 E2E items, do not block the design.
- **LOW:** **R3-4** (N=3 ↔ 1 Hz ⇒ ~3 s effective ceiling bounces legit couriers; tunable, safe direction).

**EXIT BAR: MET on the 🔴 confidentiality/tenant-isolation axis — 0 unresolved CRITICAL/HIGH security
findings.** The design now holds: the round-2 fixes genuinely close NEW-A/NEW-B without re-opening C1/C2.
The remaining items are one availability accepted-risk whose **§2 math must be redone** (R3-1) and two
guardrail/convention DoD hazards (R3-2, R3-3) that are real and demonstrable but are backstopped by the
named E2E guardrails and do not constitute design blockers. Recommend: merge-gate on (1) §2 back-of-envelope
re-derived for the tx-pinning cost, (2) §11 item-8 pinned to NOBYPASSRLS, (3) NEW-E guardrail rewritten to
catch the bus handler. No ETHICAL-STOP. No new CRITICAL.
