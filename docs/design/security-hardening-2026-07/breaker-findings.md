# Breaker findings — Security hardening batch (blue-team sweep 2026-07-02)

Attacker: System Breaker. Target: the FIX design in `proposal.md` + `ADR-security-hardening-2026-07.md`.
Method: READ-ONLY verification against live source. Goal: prove the fixes are wrong / incomplete / open
new holes. No fixes proposed — architect fixes.

Verified source anchors: `apps/api/src/routes/orders.ts:196,241,700-775`, `apps/api/src/websocket.ts:106-138,145-247,281-295`,
`apps/api/src/plugins/auth.ts:44-159`, `apps/api/src/lib/get-owner-location.ts`, `apps/api/src/routes/couriers.ts:8-45`,
`apps/api/src/routes/spa-proxy.ts:57-126`, `apps/api/src/routes/public/access-requests.ts:29-41`,
`packages/shared-types/src/legacy.ts:163-174`, `packages/platform/src/auth/jwt.ts:117-130`,
`apps/api/src/routes/auth.ts` (rate-limit configs), `scripts/verify-all.ts`, `scripts/guardrail-definer-search-path.mjs`, `.github/workflows/ci.yml:41`.

---

## [HIGH] B-SEC/B-ANTIPATTERN · The batch DoD guardrail cannot prove tenant isolation and cannot even detect finding #7 · undermines §10 DoD, #1, #7

The whole batch is declared "done" by `scripts/guardrail-route-tenant-predicate.mjs` (§10, ADR §Decision-3:
"This IS the batch's definition of done"). It is a **text-level substring sweep** for `WHERE location_id` /
`AND location_id` inside a `.query(...)` span. Two structural breaks:

1. **It checks for a token, not for authorization of the bound value.** Finding #7's actual bug is
   *body-sourced location value with no membership check*. A route written as
   `WHERE id=$1 AND location_id=$2` where `$2 = request.body.locationId` is fully cross-tenant-broken yet
   contains the literal `location_id` → the sweep reports **GREEN**. The guardrail is therefore
   constitutionally incapable of catching the #7 class it is claimed (line 220) to catch. Any future route
   can reintroduce #7 and pass the gate.
2. **`locations` is not in the scanned IDOR-table set.** §10 lists the scanned tables as
   `orders, customers, order_items, courier_positions`. Finding #7's offending query is
   `SELECT 1 FROM locations WHERE id=$1` (`couriers.ts:25`, verified). `locations` is not scanned → the sweep
   **does not flag #7 RED at all**. The proposal's own red→green proof ("#1 and #7 must be RED before fix")
   is false for #7 as the guardrail is specified.

Additional false-confidence surfaces: a `location_id` appearing in a JOIN condition
(`JOIN memberships m ON m.location_id = o.location_id` — exactly the WS order-room shape at `websocket.ts:127`)
satisfies the substring test without being a tenant filter; and the `-- no-location-id` escape comment (OR-6)
permanently blinds any line it is pasted on. Net: the gate is a useful anti-*omission* lint for the pure
`WHERE id=$1`-with-nothing-else case (#1's owner branch), but the proposal overclaims it as "the root-class
fix." **The DoD rests on a gate that does not prove the property it certifies.**

---

## [HIGH] B-SEC · #1 courier branch: fix closes cross-tenant but leaves the insider-removal read window open (customer PII) · undermines #1

`GET /orders/:id` uses `softVerifyAuth` (`orders.ts:701`), which — unlike `verifyAuth` (`auth.ts:62-91`) —
does **no** courier session/`courier_locations` recheck. The #1 courier fix scopes the read to the courier's
**baked** `activeLocationId` (unverified token claim; courier variant makes it mandatory,
`legacy.ts:165`). Consequence: a courier **removed from location X** (their `courier_locations` row deleted /
session revoked) still holds a valid ≤14d token with `activeLocationId=X`, and the fix's predicate
`AND location_id = X` **passes** → the ex-courier can enumerate/read **every live order at X** (status, total,
delivery_address = customer PII) until the token expires.

This is the same insider-removal window that #4 and #6 explicitly close for owners — left open for couriers.
The proposal flags "recommend the courier branch additionally require a live binding" and files OR-3, but OR-3
is framed as *assignment-level* scoping; the *insider-removal read* (ex-courier still reads) is a distinct,
under-tracked gap. The fix's own §7 promises "a failed membership check must deny" — but on this path there is
**no membership check at all** for couriers.

---

## [HIGH] B-FAIL/B-CONSIST · #4 WS revocation is enforced at subscribe only; the dispatch/broadcast path is never re-authz'd · undermines #4

Verified: `ownerCanAccessRoom` runs exactly once, in the `subscribe` handler (`websocket.ts:215`). The
broadcast fan-out iterates `rooms.get(room)` members and `ws.send(...)` with **no re-authorization**
(`websocket.ts:46,281,295`). WS sockets are long-lived (60s heartbeat, whole browser session). Therefore
adding `AND m.status='active'` to the `order:` room query gates only **new** subscribe attempts. An owner who
subscribed **while active** and is then revoked keeps receiving `order_update` frames on the already-open
socket until they disconnect or resubscribe. The identical gap exists for the `location:` sibling (which the
proposal cites as the "correct" model).

The proposal sells #4 as closing "the ADR-0004 insider-removal window" and §7 claims a query failure "denies
the room" — but the window is bounded to *socket lifetime*, not *zero*. A persistent owner tab across a
revocation continues to stream the tenant's live feed. #4 as scoped is a partial fix that the proposal
overstates as complete.

---

## [MED] B-SEC · #9 promotes the "trust Fly-Client-IP" assumption from 2 flag-gated routes to the entire auth brute-force surface · undermines #9

Confirmed: `auth.ts` login/register/OTP/reset use per-route `config.rateLimit` with **no** `keyGenerator`
(`:35,:63,:174,:192,:203,:236,:327,:340`), so they inherit the **global** `@fastify/rate-limit` key. Setting
the global `keyGenerator = clientIp(req)` therefore silently rekeys *every* brute-force limiter (login 10/min,
OTP 5/min, reset) onto `Fly-Client-IP`. `clientIp()` (`access-requests.ts:29-41`) reads the header verbatim and
trusts it as non-injectable. If `Fly-Client-IP` is spoofable on **any** ingress the app can be reached by
(fly-replay, private-network / 6PN, a non-edge health/WS path, or a future direct route), an attacker rotating
the header **fragments every limiter into unlimited buckets → full brute-force evasion** on login/OTP. Today's
`request.ip` (Fly socket) is *not* client-controllable, so #9 trades a false-429 collapse for a spoof-evasion
risk whose blast radius is now money/auth, not the access-gate funnel. The proposal treats this as "matches the
existing `access-requests.ts` posture" — it does not; the stakes and surface are an order larger. Operator must
confirm the Fly edge overwrites `Fly-Client-IP` on **all** ingress paths (incl. the WS upgrade and any
internal route) before this ships.

Sub-note: the fail-closed-to-`shared:no-fly-ip` path only triggers when `NODE_ENV==='production'`
(`access-requests.ts:32`); the #9 guardrail case (b) ("spoofed XFF falls to shared bucket") is untestable
unless the test harness forces production mode — otherwise `clientIp` returns `request.ip` and the assertion is
vacuous (green without proving anything).

---

## [MED] B-OPS/B-FAIL · #5 URL-token removal breaks cached PWA / service-worker WS clients even with "FE-first" sequencing · undermines #5, OR-2

The migration hazard is larger than OR-2 admits. "Migrate FE call-sites first" does not protect **already-
installed** clients: the PWA service worker (`apps/api/public/sw.js`, a flagged biomarker) caches the FE bundle;
an open tab or an offline-first install keeps running the old bundle that connects with `?token=` for days. When
the server removes the `url.searchParams.get('token')` path (`websocket.ts:154-167`), those clients get no
`authPromise`, never send `{type:'auth'}`, and are `auth_timeout`-closed at 5s (`:169-174`) — a silent WS
lockout for live/returning users. The "short overlap window" mitigation keeps the *transport-leak vulnerability
itself* live for the whole overlap, i.e. the fix's benefit is deferred exactly as long as the availability risk
persists. This is an availability regression, not a leak, but it is a real sequencing trap the proposal
under-weights.

---

## [MED] B-CONSIST/B-FAIL · #2 scope-GUC anon-read inventory is incomplete: the customer `GET /orders/:id` read seats no GUC → 0 rows at the flip · undermines #2, OR-4

Not a window in *this* batch (#2 is inert under BYPASSRLS), but the B3 sequencing contract is unsound as
written. #2 narrows the anon SELECT/UPDATE policies to require a per-request scope GUC
(`app.anon_order_id` / `app.current_tenant`). The proposal enumerates only "the anon checkout + track-exchange
paths" as GUC-seaters. But the customer branch of `GET /orders/:id` reads on the **raw pool**
(`orders.ts:762 db.query(...)`, not `withTenant`) and seats **no** GUC — as do the spa-proxy order reads
(`spa-proxy.ts:383`) and any status-poll path. Under NOBYPASSRLS with C1 narrowed, every such path returns
**0 rows → customer order-tracking 404s for everyone** at the flip. The flip-gate proof (`SET LOCAL ROLE
dowiz_app` per-policy + "anon-checkout lifecycle E2E") will only catch this if the E2E covers the *status-read*
path, not just checkout. The sequencing contract needs a **complete anon-read-path inventory**, not a two-item
list, or the flip "breaks" exactly as the KNOWN TRAP predicts.

Interleaving check (this batch): migration-lands-but-code-doesn't and code-lands-but-migration-doesn't are both
**harmless while the pool is BYPASSRLS** (RLS not consulted) — so no window opens in Tier 1. The risk is
entirely at the operator-gated flip and is correctly deferred; it is the *completeness of the handoff list*
that is wrong, not the Tier-1 sequencing.

---

## [MED] B-OPS · #3 runtime proconfig probe is not continuously enforced; the static definer gate baseline-exempts the same function · undermines #3 guardrail

`verify:rls` (the home of the proposed `pg_proc.proconfig` probe) is a DB-backed script
(`package.json:24`, `--env-file=.env`) and is **not** in the `verify:all` list that CI runs
(`ci.yml:41` runs `verify:all --ci`; the definer static gate is in that list, `verify:rls` is not). So the
"runtime probe that covers #3's live state" runs only where a provisioned DB exists — i.e. it shares the exact
skip-gated limitation §10 criticizes in `rls-adversarial.test.ts`. Meanwhile the static
`guardrail-definer-search-path.mjs` **baseline-exempts** `core-identity.ts` (proposal line 175). Mitigant: a
*new* migration that re-`CREATE OR REPLACE`s `app_member_location_ids` unpinned lands in a *new file*, which the
static gate keys as a new offender and catches (`guardrail-definer-search-path.mjs:18-19`). So future
regressions are caught; but "the DB actually has the pin applied in prod" is verified only by a non-CI,
manual probe. The guardrail is weaker than the "red→green, CI-wired" framing implies.

---

## [LOW] B-SCALE · #1 back-of-envelope is wrong: `getOwnerLocationId` is a separate pool checkout, not "on the already-held path" · undermines §2

§2 claims #1 adds "one indexed `memberships` read on the *already held* request path — no new pool checkout."
Verified false: `getOwnerLocationId(request, db)` (`get-owner-location.ts:12`) issues its own `db.query` on the
**pool** and must resolve *before* `withTenant` (`orders.ts:730`) acquires its tenant client. So the owner
`GET /orders/:id` path performs **two sequential pool checkouts** per request, not one-on-the-held-path. Against
`OPERATIONAL_POOL_SIZE=20` shared with worker/analytics/migrations this is minor, but the stated budget delta
is understated and the same pattern repeats on #6 (spa-proxy, the #1 churn hotspot, ~14 call-sites through
`getLocationId`/`getOwnerContext`).

---

## [LOW] B-CONSIST · #1/#6 scope multi-location owners to a single `activeLocationId` → 404 on their own other-location orders · functional, not security

`getOwnerLocationId` returns the (verified) baked `activeLocationId`, or the first active owner membership
(`LIMIT 1`, nondeterministic) when none is baked. An owner of locations X and Y whose token pins X requests an
order in Y (their own) → predicate `AND location_id = X` → 404. Not a leak (fails closed), but a real
regression for multi-location owners; the proposal does not enumerate this state.

---

## No-finding (regression sweep — confirmed NOT broken by the batch)

- **JWT double-pin / RS256 alg-pin:** untouched. `verifyAuthToken` (`platform/src/auth/jwt.ts`) unchanged; #5
  is transport-only, #8 uses `sub` from the already-verified claim.
- **#8 identity is correct:** the customer `AuthToken` variant is `.strict()` and has **no** `userId` field
  (`legacy.ts:166-173`), so `request.user.userId` is genuinely `undefined` today; `issueCustomerToken` sets
  `sub = customerId` (`jwt.ts:117-130`). Using `sub` is correct and strengthens throttle/idempotency. The only
  effect is the acknowledged one-time `requestHash` key shift (no stored-row invalidation) — sound.
- **Idempotency mechanism:** #8 makes `requestHash` *more* unique (adds a stable per-customer id), reducing —
  not creating — collisions. No mechanism change.
- **ADR-0013 courier WS tri-state:** #4 edits only `ownerCanAccessRoom` (`websocket.ts:122-131`); the courier
  path (`courierRoomVerdict`, `:230`) is untouched. No regression.

---

## Verdict

**Do NOT approve as "closes all 9 fail-closed" without addressing the HIGH items.** The Tier-1 app-layer
predicates for the *owner* paths (#6, #1-owner, #7) and the transport/identity fixes (#5, #8) are sound in
substance; #2/#3 sequencing is correctly deferred (no Tier-1 window opens). But three HIGH gaps mean the batch
would ship a **false sense of completeness**:

- **HIGH — guardrail (DoD):** the static sweep cannot prove isolation and cannot detect the #7 class (and does
  not even scan `locations`); the claimed "#7 RED→GREEN" proof is false. The batch's definition-of-done is
  built on a gate weaker than advertised.
- **HIGH — #1 courier:** cross-tenant closed, but the *insider-removal* read of live customer PII stays open
  for ex-couriers up to token lifetime (softVerifyAuth does no session recheck) — asymmetric with #4/#6.
- **HIGH — #4 WS:** revocation is enforced at subscribe only; a persistent owner socket keeps streaming after
  revocation (dispatch path never re-authz'd). The window is bounded to socket lifetime, not zero as claimed.

MED items (#9 trust-assumption amplification to the auth surface; #5 cached-client lockout; #2 incomplete
anon-read GUC inventory; #3 non-CI runtime probe) should be resolved or explicitly re-scoped before the flip
and before the anti-abuse rekey ships. LOW items are advisory.

---

## ROUND 3 — FINAL CONVERGENCE re-attack (regression on resolution.md, 2026-07-02)

Scope: verify ONLY that the 3 round-2 BLOCKING HIGH are genuinely CLOSED as written (not relabeled),
then regression-check whether any round-2 fix opened a NEW hole. Source re-verified:
`websocket.ts:6,35-79,215,282,296`, `lib/courier-room-authz.ts:32-92`, `lib/courier-relay-guard.ts`.

### B1 [HIGH] guardrail-too-weak / substring-sweep / missing `locations` → **CLOSED**
Resolution §A-B1 restructures the DoD to **per-finding BEHAVIORAL red→green tests as the primary DoD**,
with the exact assertions demanded: owner-A→owner-B order = 404; courier X→order at Y = denied; ex-courier
(revoked binding) → own-location order = denied (insider-window, not just cross-tenant); #7 body-`locationId`
not owned = 404 with **no `courier_invites` row created**. The static scan is explicitly **demoted** to
"flags obviously unscoped `SELECT/UPDATE/DELETE` on high-value tables… explicitly NOT proves tenant
isolation," and `locations` (+ `couriers`, `courier_invites`) is added to the scanned set. Goodhart
escape-hatch (`-- no-location-id`) now requires a named reviewer enforced in baseline-diff. This is a real
behavioral net, not a relabel of the substring sweep. **Genuinely closed.**

### B2 [HIGH] courier insider-window via softVerifyAuth + baked activeLocationId → **CLOSED (and upgraded)**
Resolution §A-B2 folds a **live** recheck into the authorizing read. The courier branch now requires an
`ALLOW` from `courierReadVerdict(db, sub, activeLocationId, orderId)` — verified at `courier-room-authz.ts:69`
→ `courierBindingVerdict:32-66`: a live tx-scoped `SELECT 1 FROM courier_assignments WHERE order_id=$1 AND
courier_id=$2 AND status=ANY(read-statuses)`. This is **binding-scoped** (strictly narrower than
location-scoping), so the "ex-courier enumerates every live order at X" attack is dead: a removed courier
holds no live per-order binding → DENY → 404. OR-3 (cross-customer read) is absorbed into Tier 1 (same tier,
not deferred/split). UNAVAILABLE→503 keeps it from fleet-denying on a pool blip. #1 is now symmetric with
#4/#6. **Genuinely closed.**

### B3 [HIGH] WS revocation subscribe-only; fan-out never re-authz's owners → **CLOSED (residual honest)**
Confirmed the gap is real in live source: `ownerCanAccessRoom` runs once at subscribe (`websocket.ts:215`);
the fan-out relays owners directly (`:282,:296` gate `relayGuard.relay` on `role==='courier'` only — owners
bypass the guard). Resolution §A-B3 extends the **existing** `relayGuard`/`evict` machinery (`:63-79`) to
owners: re-check active membership on the fan-out path, evict-on-fail via the existing `binding_revoked`
drop. This is a fix on the **broadcast path**, not another subscribe-time check. The residual is stated
honestly: window bounded to **≤ guard re-check TTL, not zero**; zero requires a push-based
`membership_revoked` socket-drop, filed as **OR-9**. The proposal no longer claims #4 closes the window to
zero. **Genuinely closed to the same guarantee couriers already have.**

### Regression — did any round-2 fix open a NEW hole?

**[LOW] B-CONSIST · B2 owner-JOIN + B-LOW-2 "multi-loc owner → 200" is valid only under BYPASSRLS; regresses to 404 at the B3 flip.**
The B2 owner branch authorizes via `SELECT o.* FROM orders o JOIN memberships m … WHERE m.user_id=$2 …
m.status='active'` and B-LOW-1 states it runs as **one `withTenant` checkout** (i.e. the tenant client seats
`app.current_tenant = activeLocationId = X`). Today (BYPASSRLS) the GUC is not enforced, so a multi-location
owner reading their own order at location Y returns the row → 200, as B-LOW-2 claims. **But post-B3
(NOBYPASSRLS), the `withTenant(X)` client RLS-restricts `orders` to tenant X → the Y-order 404s again.** This
authenticated owner orders-read path is NOT in #2's enumerated "anon-read inventory" (`orders/order_items/
customers/idempotency_keys` anon paths), so the flip-gate as scoped will not catch it. Fails **closed** (404,
not a leak) and lands only at the deferred flip → **LOW, does not reopen a HIGH.** Flag for the B3 handoff:
the anon-read inventory must also cover the *authenticated multi-location owner* orders-read, or the
owner-JOIN must resolve the target location's membership and seat that tenant (not the baked X) before the
read. No security regression; a deferred functional one.

No other new hole: B3 owner fan-out re-authz reuses the cached/TTL courier pattern (hot-path amortized,
evict not socket-close → no reconnect storm); #9 is correctly constrained (edge-only precondition, else keep
`request.ip`; IPv6-normalized; prod-forced test) and opens no spoof surface that wasn't already flagged.

### VERDICT — **CONVERGED (0 HIGH open).**
All three round-2 BLOCKING HIGH are genuinely closed in design (behavioral DoD; live courier binding
recheck; fan-out owner re-authz with honest ≤TTL residual + OR-9), not relabeled. One new **LOW** functional
regression (multi-loc owner 404 at the NOBYPASSRLS flip) surfaced by the B2 owner-JOIN — fails closed,
deferred, must be added to the B3 anon-read inventory handoff. Remaining gates are operator/human (OR-1
pool-role live-exposure confirmation; #9 ingress guarantee), not architectural.
