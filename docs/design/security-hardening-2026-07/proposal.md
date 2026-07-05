# Design Proposal — Security hardening batch (blue-team sweep 2026-07-02)

Status: DESIGN (no production code; artifacts + ADR only). Date: 2026-07-02.
Author: System Architect. Inputs: `docs/security/hardening-findings-2026-07-02.md`,
`docs/design-review/AUTH-ROLES-AUDIT-2026-06-29.md`, the existing B3 track
(`docs/design/pg-privilege-hardening/remediation-plan.md`, `docs/adr/ADR-pg-privilege-hardening.md`).
Red-line: auth / RLS / money / `packages/db/migrations/**` → every migration + guardrail edit here is
operator-gated (protect-paths). This proposal does **not** contradict the B3 plan — it front-loads the
route/app-layer subset that is a **live bug today** (RLS is inert under the current BYPASSRLS pool) and
hands the two RLS-policy findings (#2, #3) to the B3 track with an explicit sequencing contract.

---

## 1. Problem + non-goals

The BLUE sweep found 8 red-line findings. The unifying root is **identity-split × RLS-reliance**:
privileged read/write paths omit an explicit `location_id` ownership predicate and lean on RLS for
tenant isolation, while couriers/customers carry **no `userId`** to seat the member GUC (`app.user_id`).
Two facts make this exploitable **now**:

1. The live operational pool connects as **`dowiz_app`, which holds `rolbypassrls=t`** (documented in
   `1790000000077_...ts:2` and `remediation-plan.md` §Phase-3). RLS is therefore **inert on the hot
   path** → the app-layer `WHERE location_id` predicate is the *only* tenant boundary. Any route missing
   it is a live cross-tenant read/write.
2. The C1 anonymous policies (`USING (app_current_user() IS NULL)`) are **table-wide TRUE** on any
   connection with no `app.user_id` — every courier/customer path. Latent under bypass, a cross-tenant
   siphon the instant the pool is flipped to NOBYPASSRLS.

**Non-goals.** This batch does NOT flip the pool role (that is B3 Phase-3, operator-gated). It does not
rework the courier/worker GUC seating (B3 Phase-1/2, already largely staged in migration 077). It does
not add new features. It does not touch money-coherence logic (`deliveryCompletion.ts` invariants are
orthogonal to row visibility and stay untouched). It corrects the two SAFE items only as a bundle rider.

---

## 2. Back-of-envelope (blast radius, not throughput)

This is a correctness/security batch, so "back-of-envelope" is the **blast radius + connection budget
delta**, not new load.

- **Scale frame:** N tenants (locations), each doing tens of order-status reads/min + a handful of owner
  writes/min. The two touched hot paths — `GET /orders/:id` (#1) and the spa-proxy owner surface (#6, the
  repo's #1 churn hotspot) — are the only ones that gain a query.
- **Connection budget:** #1 folds authorization INTO the read query (owner-JOIN on `memberships`; courier
  via the ADR-0013 `courierReadVerdict`), so the owner path stays **one** `withTenant` checkout — the JOIN
  authorizes inside the already-held client (revised after Breaker B-LOW-1: an earlier `getOwnerLocationId`
  pre-resolve would have been a *second* sequential checkout; the JOIN form avoids it). #6 (spa-proxy) adds
  **+1 indexed `memberships` read per owner request** across its ~14 `getLocationId`/`getOwnerContext`
  call-sites (it currently does zero live checks — trusts the baked JWT); this is a genuine extra checkout,
  stated honestly, still minor vs the API pool (`OPERATIONAL_POOL_SIZE` default 20) shared with worker +
  analytics + migrations. No new long-lived connections.
- **Migration cost:** #3 is a single `ALTER FUNCTION` (metadata-only, no table rewrite). #2 (B3 track) is
  a `DROP/CREATE POLICY` pair on `orders`/`order_items`/`customers` — metadata-only, inert under bypass.
- **Code churn:** ~7 files (`orders.ts`, `websocket.ts`, `spa-proxy.ts`, `couriers.ts`, `auth.ts` for the
  owner-recheck hook option, `jwt.ts`/`orders.ts` for #8, `server.ts` + new `lib/client-ip.ts` for #9) +
  1 new guardrail script + 1 migration (#3).

---

## 3. Options (≥2) for the RLS approach + tradeoffs

The findings split cleanly into **route/app-layer** (#1,#4,#5,#6,#7,#8 — the app WHERE predicate + WS
authz) and **RLS-policy** (#2 C1 fail-open, #3 C2 unpinned definer). The strategic choice is how to treat
tenant isolation given the pool is BYPASSRLS today and NOBYPASSRLS is the destination (B3).

### Option A — app-predicate-everywhere, keep RLS as defense-in-depth (do NOT flip)
Fix #1–#7 by making the **app layer authoritative**: every privileged read/write carries an explicit
`location_id = <authorized-loc>` predicate resolved from a *live* membership check (ADR-0004 pattern).
Leave the pool BYPASSRLS; RLS stays a latent second layer.
- **Concept:** application-level authorization + least-trust; RLS as belt-and-suspenders.
- **Pros:** ships now; correct under BOTH pool cases; zero break risk (no role flip); decouples the live
  bug from the large B3 change.
- **Cons:** RLS remains inert (single effective layer) until B3; the "two strategies on one resource"
  smell (M1) persists unless the guardrail forces the predicate everywhere.

### Option B — fix RLS policies (C1/C2 + courier dual policies) then flip to NOBYPASSRLS
Do the full B3: narrow C1, pin C2, add courier-context policies, seat worker GUCs, then flip `dowiz_app`
to NOBYPASSRLS so RLS becomes the authoritative boundary.
- **Concept:** database-enforced tenant isolation (RLS) as the primary gate; CAP-style "correctness at
  the data layer."
- **Pros:** true defense-in-depth; a forgotten predicate can no longer leak.
- **Cons:** high blast radius; the KNOWN TRAP — flipping before C1-narrowing + courier GUC seating either
  **leaks** (C1 fail-open) or **breaks** (courier/notification 0-rows). Cannot ship as one atomic step.
  Does not, on its own, fix the app-layer route bugs (#1/#7 still leak until the flip lands).

### Option C — BOTH, sequenced (RECOMMENDED)
Ship Option A **now** (pool-agnostic, closes the live bugs under both pool cases), pin C2 (#3) now as a
safe standalone, and hand C1-narrowing (#2) to the B3 track where it lands *with* the courier GUC seating
*before* any role flip. The durable root-class guardrail (routes/** predicate sweep + definer-pin,
CI-wired) is the batch DoD and the thing that keeps Option A honest until B3 completes Option B.
- **Concept:** "schema rich, runtime minimal" — app predicate is the runtime authority now; RLS policies
  go in dark and enforcement switches on once, later, behind the E2E gate + the flip.
- **Pros:** live bugs closed immediately and under both pool cases; B3 unblocked but not rushed; the
  guardrail prevents the class from recurring regardless of pool role.
- **Cons:** transitional period where RLS is still inert (accepted — the app predicate + guardrail cover
  it; this is strictly better than today).

**Decision: Option C.** Rationale in §4.

---

## 4. Decision + per-finding disposition (ADR-formatted → docs/adr/ADR-security-hardening-2026-07.md)

Sequencing tiers:
- **Tier 1 (ship now, pool-agnostic, independent):** #1, #3, #4, #5, #6, #7, #8 + the guardrail + the two
  SAFE patches. Each is correct whether the pool is BYPASSRLS or NOBYPASSRLS.
- **Tier 2 (B3-coupled, ship dark then flip-gated):** #2 — narrow the C1 anonymous policies; must land
  *with* courier GUC seating (migration 077 RC4/RC5, already staged) and *before* the pool flip.

| # | Finding | Disposition | Tier |
|---|---------|-------------|------|
| 1 | orders IDOR | **fix** — add explicit `location_id` predicate | 1 |
| 2 | C1 fail-open | **fix, B3-coupled** — narrow anonymous policies fail-closed | 2 |
| 3 | C2 unpinned definer | **fix** — forward-only re-pin migration | 1 |
| 4 | WS order-room revocation | **fix** — add `status='active'` | 1 |
| 5 | JWT-in-URL | **fix** — remove `?token=` path | 1 |
| 6 | spa-proxy owner-recheck | **fix** — route through live recheck | 1 |
| 7 | RLS-only invite | **fix** — explicit membership predicate | 1 |
| 8 | customer userId throttle | **fix** — use `sub` as customer identity | 1 |
| 9 | rate-limiter blind to real client IP | **fix** — global `Fly-Client-IP` key | 1 |

### #1 — orders IDOR (`orders.ts:730`) — REVISED after Breaker B2 / B-LOW-2, Counsel §3.2
**Fix — fold live authorization INTO the read query** (closes cross-tenant AND the insider-removal window
in one round trip; the earlier "add `AND location_id = $2` from a baked/resolved location" design was
insufficient — Breaker B2 showed it leaves an ex-insider read window open on `softVerifyAuth`):
- **Owner:** authorize-by-JOIN on live active membership:
  `SELECT o.* FROM orders o JOIN memberships m ON m.location_id = o.location_id
   WHERE o.id = $1 AND m.user_id = $2 AND m.role='owner' AND m.status='active'` → 404 on no row. This
  live-checks active membership every request (ADR-0004 — no baked-claim trust), works for **multi-location
  owners** (fixes B-LOW-2: an owner reading their own order at any of their locations gets 200, not a
  single-`activeLocationId` 404), and is a single `withTenant` checkout (fixes B-LOW-1).
- **Courier:** authorize-by-**live binding**, reusing the ADR-0013 primitive
  `courierReadVerdict(db, sub, activeLocationId, orderId)` (the same liveness the WS fan-out guard uses):
  require `ALLOW` before returning the row; `UNAVAILABLE` → retryable **503**; deny → **404**. Because
  binding-scoping is strictly narrower than location-scoping, this **also closes the insider-removal read
  window** (ex-courier whose `courier_locations`/session is revoked is denied even at their old location —
  Breaker B2) **and absorbs OR-3** (the within-tenant courier cross-customer PII read — Counsel §3.2): a
  courier sees only orders they hold a live binding for, not every order at the venue. `softVerifyAuth` no
  longer trusts the baked `activeLocationId` — the verdict is the authority.

**Blast radius:** one route, read-only. No schema change. Owner = one checkout (JOIN); courier = the
existing ADR-0013 verdict read. Symmetric with #4/#6 (all three now do a live recheck).
**Why both-cases-safe:** the JOIN/verdict is the authority under BYPASSRLS (RLS inert) AND NOBYPASSRLS
(where the courier path would still leak via C1 until #2 lands).
**Guardrail (red→green) — BEHAVIORAL (primary, per Breaker B1):**
(a) owner-A token → order owned by owner-B → **404**; owner-A → own order → 200;
(b) multi-location owner (locations X,Y, token pins X) → own order at Y → 200;
(c) courier bound at X → order at Y → denied; **ex-courier (revoked binding) → own-location order →
denied** (the insider-window proof);
(d) courier → an order at their venue assigned to a *different* courier → denied (OR-3 / PII-minimization).
Static scan is only a cheap secondary anti-omission lint (see §10).

### #2 — C1 anonymous policies fail-open (`1780338981783`, `1780338981782`, `1780315000000`)
**Fix, B3-coupled.** `USING (app_current_user() IS NULL)` is table-wide TRUE for any no-`app.user_id`
connection. Migration 077 (RC1) added the anonymous **INSERT** siblings but never narrowed the fail-open
**SELECT/UPDATE** — this is a genuine open gap in the staged B3 work. Narrow so a no-context session
matches **zero** rows (fail-closed), not all:
- Preferred: replace the anonymous SELECT/UPDATE `USING` predicates with a **scoped** discriminator that
  requires an explicit per-request order/customer scope GUC (e.g. `app.anon_order_id` / `app.current_tenant`
  set by the anon checkout + track-exchange paths), so `USING (id = NULLIF(current_setting('app.anon_order_id', true),'')::uuid)` / `location_id = NULLIF(current_setting('app.current_tenant', true),'')::uuid`.
  A connection that forgets to set the scope → 0 rows (fail-closed) instead of all-tenants.
- Alternative: drop the anonymous SELECT/UPDATE policies entirely and route the (few) anonymous read
  paths through a narrow `SECURITY DEFINER` function scoped by order-id / token-hash (pinned search_path).

**Preferred mechanism (Counsel §3.4):** narrow C1 via a `SECURITY DEFINER` function scoped by
order-id/token-hash (pinned search_path — the SAME primitive #3 pins and the guardrail guards) rather than
a *new* per-request `app.anon_order_id` GUC, which adds a fresh "forgot-to-set-the-GUC → fail-open
regression" surface. One isolation primitive, not two.
**CRITICAL sequencing + complete anon-read inventory (Breaker #2-MED):** under the current BYPASSRLS pool
this change is **inert** (RLS not consulted), so Tier-1 opens no window — but at the flip it must not 404
legitimate reads. The proposal previously listed only "anon checkout + track-exchange" as GUC-seaters; that
is incomplete. Before the flip, enumerate EVERY path that reads the fail-open tables (`orders`,
`order_items`, `customers`, `idempotency_keys`) without seating a GUC — at minimum the **customer
`GET /orders/:id` raw-pool read** (`orders.ts:762`, not `withTenant`), **spa-proxy order reads**, any
**status-poll**, track-exchange, and public order views — and give each a scope. Under NOBYPASSRLS with C1
narrowed, any missed path returns **0 rows → customer tracking 404s for everyone** (the KNOWN TRAP). This
inventory is a **B3 Phase-2 precondition** (an *anon-read* companion table in `remediation-plan.md`); the
flip-gate E2E must cover the **status-read** path, not just checkout. Land the narrowing in the B3 Phase-1
migration, dark, proven via the `SET LOCAL ROLE dowiz_app` per-policy proof, **before** the flip.
**Anti-orphan artifact — SHIPS WITH TIER 1, before the #2 fix (Counsel §3.1 — the load-bearing fix):** #2
already died once (migration 077 added anon INSERT siblings but never narrowed the fail-open SELECT/UPDATE).
Handing #2 back to the same track with its guardrail bundled into its own delivery is circular. So from the
moment Tier 1 ships: (a) a `docs/regressions/REGRESSION-LEDGER.md` red-line row "C1 anon SELECT/UPDATE
fail-open — narrow before NOBYPASSRLS flip"; (b) a **skip-registered, named** `verify:rls` probe
(`test.skip('C1 anon policies fail-closed under dowiz_app', …)`) impossible to remove without narrowing C1;
and/or (c) a `plane-guard.mjs` pending check. The gate exists *before* the fix, so it guards the interim.
**Blast radius:** `orders`, `order_items`, `customers` (+ `idempotency_keys` anon SELECT). Metadata-only.
**Guardrail:** the anti-orphan probe (above), flipped from skip→active by the narrowing: under
`SET LOCAL ROLE dowiz_app` with **no** GUC → `SELECT` on each returns **0 rows** (RED today = all rows;
GREEN after); plus the flip-gate lifecycle E2E covering the customer status-read path (with scope set).

### #3 — C2 `app_member_location_ids()` unpinned SECURITY DEFINER (`core-identity.ts:76`)
**Fix.** Forward-only migration re-pinning the keystone (and the M6 menu-read definers if still unpinned):
`ALTER FUNCTION app_member_location_ids() SET search_path = pg_catalog, public, pg_temp;` (or
`CREATE OR REPLACE … SET search_path …` with an identical body). Behavior-neutral for legitimate callers;
closes the search_path-spoof authz bypass through the exact predicate meant to enforce isolation.
**Blast radius:** the keystone helper behind ~40 member policies — but a pure metadata pin, no policy or
body logic change. Highest-leverage single fix.
**Why now (Tier 1):** pinning is safe under both pool cases and independent of the flip; do not defer the
keystone to Phase-3.
**Guardrail (red→green) — honest CI limitation (Breaker #3-MED):** the static
`scripts/guardrail-definer-search-path.mjs` (already `ci:true`, `verify-all.ts:37`) is the
CI-continuous **regression** net — a *new* migration re-`CREATE OR REPLACE`-ing the fn unpinned lands in a
new file the gate catches (`:18-19`). That is sufficient for *preventing new offenders*. The **runtime**
"prod actually has the pin" check **cannot** run in DB-less CI (`verify:rls` is `--env-file=.env`, NOT in
`ci.yml`'s `verify:all --ci`). Wire it where a DB exists: (a) the **staging deploy validation** step
(ship-discipline §3, live DB) and/or (b) an **API boot-guard** that FATAL-exits if `app_member_location_ids`
lacks a pinned `search_path` (like the pool-role boot-guard). Do **not** claim CI proves the live pin; the
static gate prevents regressions, the boot-guard/staging probe proves the live state. Do not widen the
baseline.

### #4 — WS owner `order:` room revocation (`websocket.ts:122`) — REVISED after Breaker B3
**Fix — two parts** (subscribe-only was insufficient: Breaker B3 showed the fan-out never re-authz's, so a
persistent socket keeps streaming after revocation until disconnect):
1. **Subscribe gate:** add `AND m.status = 'active'` to the `order:` room query (mirror the `location:`
   sibling at :117). Gates *new* subscribes.
2. **Fan-out re-authz (the load-bearing part):** the broadcast handler (`subscribeToRoom`, :38-49) routes
   every member through `relayGuard.relay(...)`, but the guard today re-validates **only couriers**
   (`relayGuard.check = courierReadVerdict`, :64). Extend the guard to **owners**: before relaying an
   `order:`/`location:` frame to an owner member, re-check active owner membership (the `ownerCanAccessRoom`
   query, short-TTL cached like the courier check) and **evict on fail** via the existing `evict` path
   (:65-79 — drop from room + `binding_revoked`, not socket-close). This mirrors the proven ADR-0013 courier
   fan-out revalidation.
**Residual (stated honestly):** this bounds the window to **≤ the guard's re-check TTL**, NOT literally
zero — identical to the guarantee couriers already have ("stops receiving within ≤TTL"). A true zero-window
needs a push-based `membership_revoked` signal emitted on the owner's rooms at the instant
`memberships.status` flips, dropping the socket — filed as **OR-9 (tracked follow-up)**. The proposal no
longer claims #4 closes the window to zero.
**Blast radius:** `ownerCanAccessRoom` + the fan-out guard's member-type branch; owner order/location
streams. Courier order-room path (`courierRoomVerdict`, :230) untouched (already sound).
**Guardrail (behavioral):** an owner subscribes while active, membership is then set `status='revoked'` →
within ≤TTL the owner is evicted from the fan-out and **no further `order_update` frame arrives** (assert
the `binding_revoked` notice + absence of subsequent frames); a *new* subscribe by a revoked owner → denied.

### #5 — JWT in WS `?token=` URL (`websocket.ts:154`) — REVISED after Breaker #5-MED, Counsel §5
**Fix — deprecation window, not immediate removal** (Breaker showed immediate removal, even FE-first, WS-
locks-out already-installed SW-cached PWA clients (`apps/api/public/sw.js`) at the 5s `auth_timeout`):
1. Keep BOTH the `?token=` path AND the message `{ type:'auth', token }` handshake working (dual-accept).
2. **Log** `?token=` usage (count + role, never the token value) so removal is data-gated.
3. Apply the SAFE log-redaction now: drop `role`/`sub` from the auth-success lines (:163/:192), keep IP.
4. Publish a **service-worker cache-bust** so installed clients update to a message-auth bundle.
5. Remove the `url.searchParams.get('token')` path **only after** access-log `?token=` usage hits zero.
**Puddle remediation — OR-8 (Counsel §5):** closing the faucet does not drain the puddle. Bearer tokens
(24h–14d) already written to Fly access logs / Referer / history during the `?token=` era are **valid
now**. Named follow-up: **owner = Operator + Security**; action = (a) rotate the JWT signing key
(invalidates all outstanding bearer tokens → forced re-auth) *or* forced session/refresh invalidation for
owner+courier roles, and (b) scrub `?token=` from historical Fly access logs. Rotate-vs-invalidate decided
at council; in scope as a named follow-up, not omitted.
**Blast radius:** additive during the window (no client breaks); the transport-leak surface persists (log-
redacted) for the window's duration — the accepted cost of not locking out live users.
**Guardrail:** E2E — message-auth reaches `auth_success`; a `?token=` connection still works during the
window AND increments the usage log; unit — the success line contains no `sub`/`role`.

### #6 — spa-proxy trusts baked `activeLocationId` (`spa-proxy.ts:66`, `auth.ts:62`)
**Fix.** `getLocationId`/`getOwnerContext` return the baked JWT `activeLocationId` with **no** live
membership recheck — the inverse of the canonical `get-owner-location.ts`. Route both through the live
ADR-0004 recheck: reuse `getOwnerLocationId(request, db)` (or the B3 `app_owner_location(p_user,p_location)`
DEFINER resolver, RC3 — pinned search_path, works pre-`withTenant` under both pool cases). Optionally also
enforce owner revocation at the `verifyAuth` hook (today only couriers get the per-request live recheck at
`auth.ts:62`; owners rely on per-route resolvers) — recommended as a belt-and-suspenders follow-up, but the
spa-proxy resolver fix closes the specific write surface.
**Blast radius:** the entire spa-proxy owner **write** surface (brand/settings/courier-invites/onboarding)
— the #1 churn hotspot. Adds one indexed `memberships` read per owner spa-proxy request.
**Guardrail:** E2E — an owner whose membership is revoked (or points at another tenant) is denied a
spa-proxy write (`PUT /api/owner/settings` → 401/404), asserted on real status.

### #7 — RLS-only courier-invite ownership check (`couriers.ts:16`)
**Fix.** `POST /couriers/invites` takes `locationId` from the **body** and its only ownership check is
`SELECT 1 FROM locations WHERE id = $1` inside `withTenant` — which relies on RLS to scope `locations`.
Under BYPASSRLS that SELECT returns the row **cross-tenant** → an owner can mint an invite into another
tenant. Replace with an **explicit membership predicate**:
`SELECT 1 FROM memberships WHERE user_id = $1 AND location_id = $2 AND role='owner' AND status='active'`
(the `requireLocationAccess` semantics, applied to the body param). 404 on miss.
**Blast radius:** one route (invite creation).
**Why both-cases-safe:** explicit predicate holds under BYPASSRLS (where RLS is inert) *and* NOBYPASSRLS.
**Guardrail:** the routes/** predicate sweep flags the RLS-only `SELECT 1 FROM locations` pattern; E2E —
owner-A posts an invite with tenant-B's `locationId` → 404, and no `courier_invites` row is created.

### #8 — customer identity missing from throttle/idempotency (`orders.ts:196,241`, `jwt.ts:117`)
**Fix.** `issueCustomerToken` sets `sub = customerId` but **no `userId`**, and `orders.ts` reads
`request.user.userId` for the customer branch → `undefined` → velocity/fraud `computeSignals` and the
idempotency `requestHash` silently degrade to phone/IP. Use the customer's `sub` (which *is* the
`customerId`) as the identity: `request.user.role === 'customer' ? request.user.sub : …` at both :196 and
:241. Minimal and correct; avoids widening the long-lived bearer token (rejected alternative: mint
`userId` into the customer token — redundant with `sub`, more surface).
**Blast radius:** throttle scoping + idempotency fingerprint only — **not** security-critical (fails toward
*less* throttle today; the fix strengthens it). The `requestHash` input changes → forward-only one-time
cache-key shift (acceptable; no persisted-state migration).
**Guardrail:** unit — `computeSignals` receives a defined `customerId` for a customer token (assert the
value equals `sub`); regression row that a customer-token order path no longer passes `customerId:
undefined`.

### #9 — rate limiter blind to the real client IP (`server.ts:81-100,354-363`; `orders.ts:240`)
**Fix.** Fastify is constructed with **no `trustProxy`**, so `request.ip` is the Fly proxy socket address,
not the client. Every IP-keyed defense keys on it: the global limiter (`server.ts:354-363`), the per-IP
order-flood throttle (`orders.ts:240` `clientIpHash = hash(request.ip)`, gated at :268-287), and the
login/register limits. Effect: the global 100/min collapses to **one shared bucket** (mass false-429 under
load), and every per-attacker throttle degrades to **global-not-per-attacker** (brute-force/flood defense
gutted). The correct pattern **already exists** — `routes/public/access-requests.ts:29 clientIp()` reads
`Fly-Client-IP` (edge-set, non-injectable) and **fail-closes** to a single shared bucket in prod if the
header is absent, deliberately *never* trusting the client-injectable `X-Forwarded-For`. It was applied to
only two routes (`access-requests.ts`, `funnel.ts`).
- Promote `clientIp(request)` to a shared module (e.g. `apps/api/src/lib/client-ip.ts`); re-export from the
  two existing call-sites so there is one source of truth.
- Set the global `@fastify/rate-limit` **`keyGenerator: (req) => clientIp(req)`** so all IP-keyed limiters
  resolve the real client IP.
- Replace `hash(request.ip)` at `orders.ts:240` (and any auth login/register limiter IP key) with
  `hash(clientIp(request))`.
- Do **not** set Fastify `trustProxy` to trust `X-Forwarded-For` — that reintroduces the spoofable header;
  `Fly-Client-IP` is the authority (canon: server-authoritative, never trust client-injectable input).
- **IPv6 robustness:** normalize the key (strip `::ffff:` v4-mapped prefixes, lowercase) so casing / mapped
  forms don't fragment a single client across buckets. Handle header-absent (fail-closed to
  `shared:no-fly-ip` in prod).

**Operator precondition (Breaker #9-MED) — the stakes jumped.** `auth.ts` login/OTP/reset limiters carry no
per-route `keyGenerator`, so a global `keyGenerator = clientIp(req)` silently rekeys **every** brute-force
limiter onto `Fly-Client-IP`. Today's `request.ip` (Fly socket) is *not* client-controllable; the header
*is* — if it is spoofable on **any** reachable ingress (fly-replay, 6PN/private-network, the WS upgrade, a
future direct route), header rotation fragments every limiter → **brute-force evasion on login/OTP** (blast
radius now money/auth, not the funnel). This is NOT merely "matches `access-requests.ts` posture" — the
surface is an order larger. **Gate:** operator must confirm the Fly edge sets/overwrites `Fly-Client-IP` on
ALL ingress (HTTP + WS upgrade + health + internal) with no non-edge path reachable with an attacker-set
header. **If unconfirmed → do NOT rekey the auth limiters onto the header**; keep them on `request.ip` and
apply `clientIp()` only to the funnel/access-gate routes already using it. #9's global rekey is
**conditional on the ingress guarantee** (operator-gated).
**Blast radius:** anti-abuse semantics across the whole API (global limiter + order/auth throttles) —
RED-LINE-adjacent → in-council. No schema, no data. Pool-independent.
**Guardrail (red→green) — must force production mode** (Breaker sub-note: `clientIp` only fail-closes when
`NODE_ENV==='production'`, so the spoof assertion is vacuous otherwise): with the harness in forced prod
mode — (a) two requests with **different** `Fly-Client-IP` → **separate** buckets (second not throttled by
first); (b) a request with a spoofed `X-Forwarded-For` and **no** `Fly-Client-IP` falls to the single
`shared:no-fly-ip` bucket (XFF cannot evade or fragment). Assert on `x-ratelimit-remaining` / 429. RED
against `request.ip` keying, GREEN after.

### SAFE riders (bundle with the auth diff, per findings doc §SAFE)
- WS auth-success log redaction (part of #5's guardrail).
- `pnpm.overrides` `tmp@>=0.2.6` (DEV/CI-only `@lhci/cli` path-traversal) — operator applies (root
  `package.json` is protect-path). Not a prod risk; noted for completeness.

---

## 5. Data / migrations (forward-only, atomic, RLS FORCE, integer)

- **#3 (Tier 1):** one forward-only migration — `ALTER FUNCTION app_member_location_ids() SET search_path
  = pg_catalog, public, pg_temp;` (metadata-only, atomic, no table rewrite, no RLS change, no money).
- **#2 (Tier 2, B3 track):** forward-only `DROP POLICY IF EXISTS … ; CREATE POLICY …` on `orders`,
  `order_items`, `customers`, `idempotency_keys` — narrowed anon predicates. RLS already ENABLE+FORCE on
  all four (do not re-toggle). Inert under bypass; proven via `SET LOCAL ROLE` per-policy proof before the
  flip. Lands in the B3 Phase-1 migration alongside migration 077's RC-set (which it extends).
- No integer-money columns touched anywhere in this batch. No `down` logic beyond forward-only discipline.
- All migrations operator-gated (`packages/db/migrations/**` is protect-paths); staging-first via the
  ship-discipline loop; prod human-gated.

---

## 6. Consistency + idempotency

- **#8** restores the customer identity into the idempotency `requestHash` and velocity signals — the
  fingerprint becomes correctly per-customer. Forward-only; a one-time key shift is acceptable (no stored
  idempotency rows are invalidated destructively — a shifted key is a fresh key, and the server remains
  authoritative for price/status).
- **#1/#7** are read/write authorization predicates — no consistency semantics changed; the server stays
  authoritative for order visibility and invite creation.
- Idempotency stays in Postgres (not Redis), per canon; no change to that seam.

---

## 7. Failures + degradation (per external/privileged call: timeout + fallback, zero cascade)

- **#1/#6/#7** each add one indexed `memberships` SELECT on the operational pool (already
  `statement_timeout=10s`, `connectionTimeoutMillis=5000`). On DB error → the route returns 500 (existing
  handler) or, for the authorization resolvers, a **fail-closed** null → 401/404. No fallback that widens
  access: a failed membership check must deny, never default-allow.
- **#4** WS authz query already wraps its `try/catch` returning `false` (fail-closed) on error (:134) —
  the `status='active'` add preserves that; a query failure denies the room.
- **#5** removes a code path (no new external call); pure reduction of attack surface.
- **#2** post-flip: a no-context connection → 0 rows (clean deny) instead of a throw or all-rows — the
  fail-closed degradation is the *point* of the narrowing.
- No new cross-service calls, no new queues, no cascade surface introduced.

---

## 8. Security + tenant isolation

- Every fix moves the tenant boundary to an **explicit, live-rechecked `location_id` predicate** at the
  app layer (authoritative under both pool cases), with RLS as the second layer that B3 later makes
  primary. This is the direct remedy for the identity-split × RLS-reliance root.
- ADR-0004 (owner-token revocation) is honored on every owner write path touched (#1, #6): baked
  `activeLocationId` is never trusted without a live active-membership check.
- No PII into logs (#5 redaction); no secrets; JWT crypto untouched (confirmed-solid, RS256 pinned).
- **Connect-guard gap (flag):** `packages/db/src/index.ts:32-39` only rejects the literal `postgres`
  role — a BYPASSRLS role named `dowiz_app` passes the guard while still bypassing RLS. Recommend
  hardening the guard to reject on `SELECT rolbypassrls FROM pg_roles WHERE rolname = current_user` = true
  (aligns the runtime guard with the B3 boot-guard). Independent, Tier 1 candidate.

---

## 9. Operability

- **Health:** no new health surface; the connect-guard hardening (§8) would turn a silent bypass into a
  boot-time FATAL (degraded-vs-down: a mis-roled pool = down, correctly).
- **Observability (<1 min):** the routes/** predicate sweep + definer-pin run in `verify:all` (CI) →
  a regression is caught pre-merge, not in prod. Post-deploy, the E2E cross-tenant 404 assertions are the
  runtime signal.
- **Rollback:** #1/#4/#5/#6/#7/#8 are app-code — revert the commit. #3 is forward-only metadata (a re-pin;
  rollback = a further migration, not a `down`). #2 ships dark and is only effective at the flip → its
  rollback is "don't flip."
- **Flag / scaling-gate:** #5's FE WS client migration should land before/with the server-side URL-path
  removal (or gate the removal behind a short overlap window where both paths work) to avoid breaking live
  WS clients — the one sequencing hazard inside Tier 1.

---

## 10. The durable guardrail (batch DoD — the root-class fix) — RESTRUCTURED after Breaker B1

Breaker B1 was correct and load-bearing: a substring sweep for `WHERE location_id` proves a *token is
present*, not that the *bound value is authorized*. `WHERE id=$1 AND location_id=$2` with
`$2 = request.body.locationId` (the #7 class) passes GREEN; a JOIN condition
`m.location_id = o.location_id` satisfies the substring without being a tenant filter; and `locations` (the
actual #7 table) was not even in the scanned set. **The DoD cannot rest on a gate that cannot certify the
property it claims.** Restructured:

1. **PRIMARY DoD = per-finding BEHAVIORAL red→green tests** (the real proof of the isolation property, not a
   proxy). Enumerated per finding above; the load-bearing set:
   - #1 owner-B order → 404; own → 200; multi-loc own order → 200.
   - #1 courier cross-tenant → denied; **ex-courier (revoked binding) own-location order → denied**;
     courier → another courier's venue order → denied (OR-3).
   - #7 owner-A posts invite with owner-B `locationId` → 404, no `courier_invites` row; own → created.
   - #4 revoked owner evicted from fan-out ≤TTL, no further frames; #6 revoked owner write → 401/404.
   - #5 message-auth works + `?token=` logged; #8 defined customer id; #9 (prod-forced) per-IP buckets.
   A finding is **not done** until its behavioral test is proven red→green. This is the batch DoD.
2. **SECONDARY = cheap static anti-omission lint (not a proof).** Extract the sweep out of the DB-gated
   `rls-adversarial.test.ts` into `scripts/guardrail-route-tenant-predicate.mjs` (DB-less, baseline'd,
   mirrors the definer guardrail). Scan `apps/api/src/routes/**` AND `apps/api/src/workers/**`. **Reframed
   claim:** it *"flags obviously unscoped `SELECT/UPDATE/DELETE` on high-value tables (the pure
   `WHERE id=$1`-with-nothing-else omission — #1's owner branch)"* — explicitly **NOT** "proves tenant
   isolation" and **NOT** claimed to catch #7 (a body-sourced authorized value is invisible to a
   substring). **Fix the table set** to include `locations`, `couriers`, `courier_invites` (not just
   `orders/customers/order_items/courier_positions`). Wire into `verify:all` with `ci: true`.
3. **Escape-hatch discipline (Counsel §3.6, Goodhart):** any *new* `-- no-location-id` escape comment
   requires a named reviewer — enforced in the guardrail's baseline-diff, not aspirational (the escape
   comment is the exact seam the class re-enters through).
4. **Definer-pin:** static gate already `ci:true`; runtime pin verified at boot-guard/staging (see #3
   above — NOT claimed as CI-continuous).

The behavioral tests are the definition of done; the static scan is a supporting lint that catches the
crude omission cheaply. The DB-backed `rls-adversarial.test.ts` stays skip-gated (needs a provisioned DB).
No finding is credited "done" by the static scan alone.

---

## The both-pool-cases matrix (open operator input: confirm `DATABASE_URL_OPERATIONAL`'s role)

Documentary evidence (migration 077, remediation-plan §Phase-3) says the live role is **`dowiz_app`,
`rolbypassrls=t`** — i.e. **Case A**. The architect cannot read Fly secrets; the operator must confirm the
deployed `DATABASE_URL_OPERATIONAL` role name + `rolbypassrls`. **This is a LIVE-EXPOSURE gate, not merely
an effectiveness gate (Counsel §2):** if the pool is *unexpectedly already NOBYPASSRLS today*, #2 is not
latent — it is a **live table-wide cross-tenant customer-PII siphon** and **promotes to Tier 1**. The batch
must **not** be declared "PII-resolved" without the operator's recorded pool-role confirmation. The batch is
designed to be correct under **both**:

| # | Case A — BYPASSRLS pool (today, `dowiz_app`) | Case B — NOBYPASSRLS pool (post-B3-flip) |
|---|----------------------------------------------|------------------------------------------|
| 1 orders IDOR | **LIVE leak** — RLS inert, no predicate → any order by UUID. Fix (explicit predicate) is the ONLY boundary. | Owner scoped by member policy; **courier still leaks via C1** (no `userId`) until #2 lands. Explicit predicate required regardless. |
| 2 C1 fail-open | Inert (RLS bypassed) — latent | **Table-wide siphon** on any no-context conn. #2 narrowing is what makes the flip safe. Must land before flip. |
| 3 C2 definer | Latent (RLS bypassed) | Live spoof vector via unpinned search_path. Pin (#3) closes it; safe to ship in Case A too. |
| 4 WS order-room | **LIVE** — app-layer authz, pool-independent (fix applies both cases) | Same (WS authz is app-layer, not RLS). |
| 5 JWT-in-URL | **LIVE** — transport leak, pool-independent | Same. |
| 6 spa-proxy recheck | **LIVE** — trusts baked JWT, pool-independent | Same (app-layer resolver). |
| 7 RLS-only invite | **LIVE cross-tenant invite** — `SELECT 1 FROM locations` returns cross-tenant under bypass | RLS scopes `locations` → SELECT returns 0 (already blocked) — but explicit predicate still added for defense-in-depth + pool-independence. |
| 8 customer identity | **LIVE** throttle/idempotency degradation, pool-independent | Same. |
| 9 rate-limiter IP | **LIVE** — global bucket + per-attacker throttles degraded, pool-independent | Same (rate-limit is app-layer, not RLS). |

**Takeaway:** #4/#5/#6/#8/#9 are pool-independent (pure app/transport) — fix now. #1/#7 are LIVE under Case A
(the current state) and defense-in-depth under Case B — fix now. #2 is the flip-gate — B3 track. #3 is safe
either case — fix now.

---

## Open / accepted risks (owner)

| # | Risk | Disposition | Owner |
|---|------|-------------|-------|
| OR-1 | Live pool role unconfirmed (secrets unreadable by architect) | **OPEN — LIVE-EXPOSURE gate (Counsel §2).** Confirm `DATABASE_URL_OPERATIONAL` role + `rolbypassrls`. If unexpectedly **NOBYPASSRLS today**, #2 is a **live table-wide cross-tenant PII siphon → PROMOTES to Tier 1** (not deferred). The batch is NOT "PII-resolved" without the operator's *recorded* confirmation. | Operator + DB owner |
| OR-2 | #5 FE WS clients may still pass `?token=` (incl. SW-cached PWA) | **REVISED → dual-accept deprecation window:** keep both paths, log usage, SW cache-bust, remove only after usage→zero. No client lockout. | Architect + FE |
| OR-3 | Courier within-tenant cross-customer PII read (courier enumerating all venue orders) | **FIXED IN TIER 1 (Counsel §3.2):** #1 courier branch now binding-scoped via `courierReadVerdict` (only assigned-binding orders); no longer a deferred accept. | Architect |
| OR-4 | #2 narrowing must not land without courier/anon GUC seating + **complete anon-read inventory** (else post-flip 404/break) | **Sequencing contract:** land in B3 Phase-1 with 077 RC-set + full anon-read inventory (incl. customer status-read); prove via `SET LOCAL ROLE dowiz_app`; flip only after | DB owner |
| OR-5 | Connect-guard only rejects literal `postgres`, not BYPASSRLS generally | **ACCEPT now / fix (Tier 1 candidate):** harden to `rolbypassrls` check | DB owner |
| OR-6 | Static lint false-positives on legitimate no-location reads | Baseline + `-- no-location-id` escape comment (mirrors `rls-adversarial.test.ts:235`); **new escape comments require a named reviewer** (Counsel §3.6) | Architect |
| OR-7 | #9 fail-closes to a shared bucket in prod if `Fly-Client-IP` is ever absent (legit false-429 for all) | **ACCEPT** — matches `access-requests.ts` posture (degrade, never trust XFF) + throttled ≤1/min warn; superseded in stakes by the #9 ingress precondition below | Architect + Operator |
| OR-8 | Already-leaked `?token=` bearer tokens (24h–14d) valid NOW in logs/Referer/history (Counsel §5) | **NAMED FOLLOW-UP:** rotate JWT signing key *or* forced session/refresh invalidation (owner+courier) + scrub `?token=` from Fly access logs. Rotate-vs-invalidate decided at council. | Operator + Security |
| OR-9 | #4 fan-out re-authz residual = **≤TTL, not zero** | **ACCEPT (parity with couriers) + follow-up:** push-based `membership_revoked` socket-drop for a true zero-window | Architect |
| OR-10 | #9 `Fly-Client-IP` trust across ALL ingress (spoof → brute-force evasion on auth) | **OPEN — operator gate:** confirm edge sets/overwrites the header on all ingress (HTTP+WS+internal); if unconfirmed, do NOT rekey auth limiters onto it (keep `request.ip`) | Operator |
| M1 | Dual-authority (app-predicate + inert-RLS) transitional state | **Time-boxed to the B3 flip (Counsel §3.3);** the #2 anti-orphan artifact is the trigger that forces closure — not open-ended | Architect |
