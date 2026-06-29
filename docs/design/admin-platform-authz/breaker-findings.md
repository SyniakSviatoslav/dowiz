# Breaker Findings — B4 (`/api/admin/*` platform-admin gate)

> Returned INLINE by the Breaker (it did not write this file). Reproduced here VERBATIM so the
> artifact exists. Dispositions live in `resolution.md`.

---

**[CRITICAL] Gate is per-file opt-in, not an encapsulated plane gate.** server.ts:794,797,799 registers the three admin plugins as three independent `fastify.register(..., {prefix:'/api/admin'})` — no parent plugin wraps /api/admin. Fastify hooks don't cross sibling encapsulation, so `requirePlatformAdmin` only protects a route if a dev re-adds the hook in that file. A future routes/admin/metrics.ts registered at prefix:/api/admin with no hook → ungated cross-tenant BOLA reopened. The only net is an "optional" eslint rule. Closure must be STRUCTURAL for every current+future admin route.

**[HIGH] DR-drill single-flight advisory lock leaks/self-deadlocks.** backup-verify.ts acquireLock (:62-70) does pool.connect() → pg_try_advisory_lock(3) → client.release() in finally — returns the connection to the pool WHILE the session-level lock is still held → never released → first drill leaks the lock → every subsequent drill 409s forever (permanent DR self-DoS). releaseLock (:72-79) does a SEPARATE pool.connect() + pg_advisory_unlock(3) on a different session → no-op. §6's "advisory locks release on session end, crashed handler auto-releases" is FALSE under pooling (the session is the pooled backend, survives the handler). Also a NEW route-layer pg_try_advisory_lock(<const>): if <const> collides with key 3, route holds 3 then runRestoreVerify tries 3 on a different session → self-deadlock.

**[HIGH] R4 bites: admin routes never set app.user_id → pa_self_read fails closed for ALL platform-admins the moment B3 lands.** Every admin handler calls db.query on the RAW pool (backups.ts:34,40; fallback.ts:14,47; notification-audit.ts:43); zero withTenant/set_config('app.user_id') under routes/admin/. pa_self_read USING(user_id = current_setting('app.user_id',true)::uuid) works today ONLY because the pool is BYPASSRLS. When B3 flips admin to NOBYPASSRLS+FORCE RLS, current_setting('app.user_id',true) is NULL → 0 rows → 503 for every platform-admin → admin-plane self-DoS. AuthZ LOGIC is B3-independent but the authZ DATA READ is not. (Same fragility exists in requireLocationAccess auth.ts:153 raw pool.query on memberships.)

**[MED] Endpoint #6 real path is /api/admin/admin/notification-audit (double prefix).** notification-audit.ts:14 declares fastify.get('/admin/notification-audit') under prefix:/api/admin ⇒ actual = /api/admin/admin/notification-audit. The §1 table + DoD E2E #6 reference the single-admin path which is UNREGISTERED → setNotFoundHandler → HTML GET returns 200 index.html, JSON returns 404 — never the gate. DoD #6 owner→403 is vacuous false-green and the reachable endpoint is mislabeled.

**[MED] Audit row written after the side-effect, separate statement → action with no audit row.** backups.ts:73,79 runs runRestoreVerify (destructive) BEFORE any audit insert; the insert is a separate db.query (own implicit tx). Crash/pool-exhaustion/blip before the INSERT commits → side-effect happened, no audit trail. R5 only covers duplicates, not the gap. Need audit-before-or-same-tx-as side-effect.

**[MED] "Fail-closed" asserted, not structurally guaranteed.** notification-audit.ts:42-47 wraps query in try/catch and returns 500 with err.message (error/schema leak) — the in-tree precedent is swallow, not deny-stop. A missing await on db.query → truthy Promise → admit; a try/catch that logs and falls through → admit. Unit DoD #4 tests the hook in isolation; nothing asserts the WIRED hook denies when a handler-level catch swallows. Fail-closed must be a wiring property proven against the real route.

**[MED] Bootstrap seed can hard-lock or FATAL-crash the deploy.** platform_admins.user_id REFERENCES users(id) + seed from PLATFORM_ADMIN_BOOTSTRAP_USER_ID at migration time: (a) env unset/empty → 0 admins → whole plane 403 for everyone, recoverable only via DB-creds CLI; (b) env UUID not yet in users → FK violation → migration fails → boot-guard FATAL-exits the deploy. Sequencing (admin user must exist before migration) unspecified.

**[LOW] Insider-removal latency is "next request" not in-flight** — runRestoreVerify TIMEOUT_MS=30min; a revoked admin already inside dr-report/verify runs to completion. Scope the "immediate revocation" claim to request-entry.

**[LOW] Re-check keys on $sub; rest of auth keys on userId** — §4 uses WHERE user_id=$sub but requireLocationAccess/membership use request.user.userId; they coincide only because owner mint sets sub==userId. Pick the canonical field; document the coupling.

**[LOW] pa_audit_read USING(true) doesn't enforce least-privilege** — any platform-admin reads every other admin's actor trail; RLS gives no isolation, only the app gate.

---

## Counsel ETHICAL-STOP (friction, needs human decision at STOP-ETHICS)

Audit log is self-watched (operator = admin = auditor = sole reader, pa_audit_read USING(true)), invisible to affected restaurants, no notification/appeal. Asks for ONE recorded human decision + date on a minimum legibility floor (cheapest lever: append-only out-of-band audit mirror). Non-blocking flags: (1) kill-switch ADMIN_PLANE_ENABLED=false must NOT darken recovery tools (backups/dr-report) during an incident — scope it to destructive/drill endpoints only; (2) mirror the audit reader out-of-band before the first non-founder ops hire. Steel-man: schedule Option C (network-isolated ops service) as the next hardening at a tenant/headcount threshold.

---

# RE-ATTACK round 2 — regression check on the RESOLVE revision (F1–F10)

Verified each load-bearing fix against live Fastify 5 / Postgres semantics and the actual route files
(`routes/admin/{backups,fallback,notification-audit}.ts`, `auth.ts`, `backup-verify.ts`,
`migrations/1780310071220_core-identity.ts`, `1790000000015_operational-pool-role.ts`). Verdicts below.
Accepted LOWs (F8/F10/R5) not re-litigated.

### Per-fix verdict

- **F1 encapsulation direction — HOLDS (partly).** Parent `addHook('onRequest', …)` IS inherited by
  children registered inside the same context (standard Fastify 5; the children are plain
  `FastifyPluginAsync` default exports, not `fp`-wrapped, so they create child contexts that inherit
  the parent hook). The *direction* is correct. But see RA2-1 (ordering self-DoS) and RA2-5 ("by
  construction" overstated).
- **F2 one-lock-one-owner — NEW FINDING (RA2-2).** The internal lock IS the broken one, but the fix is
  ambiguous and as-sketched self-collides. See below.
- **F3 SECURITY DEFINER — NEW FINDING (RA2-3, HIGH).** The fix relocates, not removes, the RLS
  dependency; its correctness rests on an unstated function-owner BYPASSRLS attribute. See below.
- **F4 double-prefix — HOLDS.** Moving the handler decl to `/notification-audit` yields the single
  `/api/admin/notification-audit` path; DoD #6 asserting a JSON 403 (not 200 index.html) closes the
  false-green. No regression.
- **F5 write-ahead audit — NEW FINDING (RA2-4).** The "started" intent closes the no-trail gap for the
  drill (side-effect is a sandbox DB, not prod), BUT the "in the SAME tx that takes the lock" phrasing
  is self-contradictory. See below.
- **F6 fail-closed wiring — HOLDS, contingent on RA2-1.** `reply…return` in the parent `onRequest`
  short-circuits before any child handler, and a throw → 503 catch is correct deny-on-uncertainty. The
  swallow-precedent at notification-audit.ts:46 genuinely cannot admit because the gate ran at
  `onRequest`. Caveat: this only holds once `request.user` is actually populated at gate time (RA2-1).
- **F7 bootstrap decoupling — HOLDS.** Migration creating tables+fn only (no FK seed) removes the
  FK-FATAL and empty-env brick; 0-admins is fail-closed + recoverable; the ops CLI requires DB creds,
  which is already god-mode, so it adds NO escalation surface beyond what DB-creds already grant. One
  caveat (not a finding): keep `scripts/platform-admin-grant.ts` out of any CI/API path with ambient
  prod creds — its safety is "no ambient creds," not the script itself.
- **F9 canonical userId — HOLDS.** Keying the DEFINER fn on `request.user.userId` matches
  requireLocationAccess (auth.ts:150) and the documented `sub==userId` owner-mint coupling.

### New / surviving findings

**[HIGH] RA2-3 · B-SEC/B-CONSIST · F3 DEFINER fn over a FORCE-RLS table silently depends on an
unstated owner-BYPASSRLS attribute — the "B3-order-independent, no RLS dependency" claim is false.**
The migration does `ALTER TABLE platform_admins FORCE ROW LEVEL SECURITY`, and `is_platform_admin` is
`SECURITY DEFINER` running as the *function owner*. Postgres semantics: FORCE RLS removes the table
owner's implicit RLS bypass; **only a role with the BYPASSRLS attribute (or superuser) bypasses FORCE
RLS.** The DEFINER fn never sets `app.user_id`, and the only SELECT policy is `pa_self_read`
(`USING user_id = current_setting('app.user_id')`). Therefore:
- If the fn owner has BYPASSRLS/superuser (true in the current Supabase superuser-migration env) → it
  reads all rows → works. **But that is exactly a BYPASSRLS-equivalent read of a privilege table, just
  wrapped in a function** — the proposal's claim "even on a NOBYPASSRLS pool the operational role
  cannot read the allowlist … the DEFINER fn removes that failure mode entirely / does NOT reintroduce
  BYPASSRLS" is materially imprecise: it RELOCATES the BYPASSRLS dependency from the pool role to the
  fn-owner role, unstated and unguarded.
- If the fn owner is a non-BYPASSRLS role (hardened/Supabase-non-superuser migrations — the direction
  B3 is pushing the whole system) → FORCE RLS + no `app.user_id` → 0 rows → `is_platform_admin`
  returns **false for every caller from day 1, before B3 even lands** → total admin-plane self-DoS.
  This is the *same* fail-closed brick F3 claimed to fix (R4's GUC dependency), merely moved inside the
  DEFINER fn. `pa_self_read` is described as "no longer load-bearing"; under FORCE RLS with a
  non-BYPASSRLS owner it becomes the ONLY policy and is load-bearing again → fails.
- **Demonstrable in-repo:** `memberships` is ALREADY `FORCE ROW LEVEL SECURITY`
  (core-identity.ts:91-92) and the operational role is `NOBYPASSRLS`
  (1790000000015_operational-pool-role.ts) — proving FORCE-RLS-without-GUC = 0 rows is the live
  behavior here, not theory. The admin gate's only escape from that is the (unstated) fn-owner
  BYPASSRLS.
- **Invariant violated:** "the authZ gate holds identically whether admin pool is BYPASSRLS or
  NOBYPASSRLS, independent of RLS posture." It does not — it depends on the fn-owner's role attribute,
  which the design neither states nor guards (and `verify:rls` is noted flaky/BYPASSRLS-env-artifact).
  Either drop FORCE on `platform_admins` (plain owner-bypass read is then sufficient and simpler) or
  state+guard the owner-BYPASSRLS precondition. As written the FORCE-RLS × DEFINER-owner interaction is
  unanalyzed.

**[MED] RA2-1 · B-FAIL/B-CONSIST · F1 parent `onRequest` runs BEFORE child `verifyAuth` → `request.user`
is null at the gate → entire admin plane self-DoS.** `requirePlatformAdmin` dereferences
`req.user.userId`. `request.user` is populated by `verifyAuth`, which each child plugin registers as
its OWN `onRequest` hook (backups.ts:8, fallback.ts:9, notification-audit.ts:8). In Fastify 5,
parent-context `onRequest` hooks run **before** child-context `onRequest` hooks. So the parent gate
fires with `request.user === null` (the decorated default, auth.ts:162) → `req.user.userId` throws →
fail-closed catch → **503 for EVERY caller, including legitimate platform-admins**. The §3.5 sketch
registers ONLY `requirePlatformAdmin` on the parent; it does not state that `verifyAuth` must be
hoisted into the parent and ordered *before* it. Fail-closed (no leak) but bricks the plane; DoD #7
(admin→200) would catch it at test time, but the design as written is incomplete.
*Coupled sub-issue:* the children still register `requireRole(['owner'])` (backups.ts:9 etc.). If those
hooks remain, a platform-admin whose JWT role ≠ `owner` is 403'd by the child even after passing the
gate — contradicting §3's "JWT stays owner (or any authenticated principal)." If they're removed,
`verifyAuth` disappears with them unless hoisted. **Invariant:** the gate must run with an authenticated
principal; hook-ordering and the fate of the existing per-child `verifyAuth`/`requireRole` hooks are
load-bearing and unspecified.

**[MED] RA2-2 · B-SCALE/B-CONSIST · F2 dedicated-client lock + the unchanged internal lock collide on
key 3 → drill permanently returns "Another verify in progress."** `runRestoreVerify(pool, …)` takes a
**Pool** and internally calls `acquireLock(pool, BACKUP_VERIFY_LOCK=3)` at backup-verify.ts:259 (its own
`pool.connect()`) and `releaseLock` at :360. The F2 sketch wraps a route-layer dedicated client that
takes `pg_try_advisory_lock(3)` then calls `runRestoreVerify(client, …)`. Two problems: (a) the sketch
passes a single `client` where the signature expects a `Pool`, and `acquireLock` calls
`pool.connect()` — a PoolClient has no `.connect()`; (b) even if it still locks via a separate
connection, the route already holds key 3 → the inner `pg_try_advisory_lock(3)` returns false →
`runRestoreVerify` returns `'Another verify in progress'` at :261 and the drill **never runs** —
a self-DoS of the same outcome class as the leak it replaced. The resolution says "refactor the
existing key-3 lock to the dedicated-client pattern" but the design never says **delete
backup-verify.ts:259/360** nor reconciles the `Pool`-vs-client signature. **Invariant:** exactly ONE
lock owner on key 3; as sketched there are two acquirers on the same key from different sessions.

**[MED] RA2-4 · B-DATA/B-OPS · F5 "INSERT started in the SAME tx that takes the lock … UPDATE after"
either defeats write-ahead or reintroduces the 30-min idle-in-tx they rejected.** The drill lock is a
**session-level** `pg_try_advisory_lock` on a dedicated client — there is no transaction "taking the
lock," so "same tx as the lock" is a phantom. If read literally as one transaction wrapping
INSERT-started → 30-min `runRestoreVerify` → UPDATE-completed, then (a) the `started` row is NOT
committed until the end → **write-ahead is defeated** (a crash mid-drill leaves no committed intent —
exactly the gap F5 set out to close), and (b) a 30-min open transaction is **the idle-in-tx bloat they
explicitly rejected `pg_advisory_xact_lock` to avoid** (§6). The correct shape is: short tx1 = INSERT
started + COMMIT (before any side-effect) → drill (no tx) → tx2 = UPDATE. The design must say the intent
row is committed in its own short tx, not held open. **Invariant:** the audit intent must be durable
*before* the side-effect; the stated "same tx" wording contradicts that.

**[MED] RA2-5 · B-ANTIPATTERN/B-SEC · "every current AND future admin route is gated by construction"
is overstated — encapsulation gates only routes registered INSIDE the parent; a sibling register at
`/api/admin` is caught ONLY by the lint, which is evadable.** Encapsulation guarantees the hook for
children of `routes/admin/index.ts`. It does NOT prevent someone adding a NEW
`fastify.register(evil, {prefix:'/api/admin'})` directly in server.ts as a *sibling* of `adminPlane`
(precisely the shape server.ts:794/797/799 use today) — that sibling is outside the parent context,
inherits no hook, and reopens BOLA. The only net for that is the promoted eslint rule. The live
`tools/eslint-plugin-local` is a single inline `src/index.js` of AST matchers; a literal-string match
on `{prefix:'/api/admin'}` is **evaded by a computed prefix** (`const P='/api/admin'; register(x,{prefix:P})`)
or by registering the sibling inside another already-mounted plugin. **Invariant:** the structural claim
holds for in-parent routes only; out-of-parent siblings at the same prefix remain a lint-only (evadable)
guarantee. Call it "structural for children + lint tripwire for siblings," not "by construction for all."

**[MED] RA2-6 · B-CONSIST · R11 is a BLOCKING co-dependency for B3, not a soft "coordinate/track."**
`memberships` is ALREADY `FORCE ROW LEVEL SECURITY` (core-identity.ts:91-92) and the operational role
is `NOBYPASSRLS` (1790000000015). `requireLocationAccess` (auth.ts:148) does a raw
`request.server.db.query` on `memberships` with NO `app.user_id`/`withTenant` — it works **today only
because the live `fastify.db` pool is still BYPASSRLS** (the very thing B3 flips). The instant B3 flips
that pool to NOBYPASSRLS, `auth.ts:148` returns 0 rows → **404 for every owner → owner-plane self-DoS**,
a fleet-wide outage. B4 hardens the admin plane to a NOBYPASSRLS-safe pattern (DEFINER fn) while leaving
the owner-facing re-check B3-fragile → an inconsistency that detonates when B3 lands. Filing this as a
soft DEFER-FLAG ("coordinate/tracked") understates it: **B3 cannot ship the pool flip without
simultaneously fixing auth.ts:148**, or it self-DoSes every owner. This should be a hard, named blocking
dependency on the B3 change, not a tracked residual. **Invariant:** no per-request authZ DATA read may
rely on the pool's BYPASSRLS posture across the B3 cutover.

### Regression confirmation (round-1 findings → still closed?)
- F1 (CRITICAL, structural gate): closed in DIRECTION (parent hook inherits), but two new gaps opened by
  the refactor — RA2-1 (ordering) and RA2-5 (sibling/"by construction" overstated).
- F2 (HIGH, lock leak): leak addressed in intent but RA2-2 shows the sketch can re-create an equivalent
  self-DoS via key-3 self-collision; under-specified.
- F3 (HIGH, R4/GUC): NOT cleanly closed — RA2-3 shows the failure mode is relocated, not removed, and
  the independence claim is false absent an unstated owner-BYPASSRLS precondition.
- F4/F6/F7/F9: closed, no regression.
- F5 (MED, write-ahead): gap closed for the no-trail case but RA2-4 flags a tx-scoping contradiction.
- R11: re-graded — should be a blocking B3 dependency (RA2-6), not a soft defer.
- F8/F10/R5/R8/E1: accepted as-is, not re-litigated.

---

# Final confirmation round (FOCUSED — RA2-3 / RA2-1 / boot-guard only)

Scope: confirm-or-break the two load-bearing round-2 fixes + the boot-guard. Accepted LOWs/MEDs and
deferred items (R11/RA2-6, E1, R12) NOT re-litigated. All claims checked against live source +
empirical Fastify 5.8.5 probe.

**1. RA2-3 (non-RLS global `platform_admins` + plain point-read) → HOLDS.** Operational role
`deliveryos_operational_user` is `LOGIN NOBYPASSRLS` (1790000000015_operational-pool-role.ts:19) and
the proposal grants it explicit `GRANT SELECT ON TABLE platform_admins` (§5) on a table with NO RLS.
PG semantics confirmed: RLS disabled ⇒ row visibility is governed solely by table GRANTs ⇒ a
NOBYPASSRLS role with `GRANT SELECT` reads every row, identical to a BYPASSRLS role. The point-read
`SELECT 1 FROM platform_admins WHERE user_id=$1 AND revoked_at IS NULL` is backed by the partial index
`platform_admins_active_idx`. No DEFINER fn, no `app.user_id` GUC, no `pa_self_read` survives in the
re-check path. Genuinely B3-order-independent. No path where the operational role lacks the GRANT
(explicit GRANT + `ALTER DEFAULT PRIVILEGES FOR ROLE postgres … GRANT SELECT` both cover it).

**2. RA2-1 (parent `verifyAuth` → `requirePlatformAdmin`; children drop `verifyAuth`+`requireRole`) →
HOLDS.** Fastify runs parent-context `onRequest` hooks before child-context hooks, in registration
order — so `verifyAuth` (registered first on the parent, §3.5) populates `request.user` before
`requirePlatformAdmin` dereferences `userId`. No child needs its own `verifyAuth`: every `/api/admin`
sub-route is gated (no public admin sub-route exists). 401-vs-503 ordering is correct:
`verifyAuth` (plugins/auth.ts:47) `return reply.status(401)` short-circuits the lifecycle on
missing/invalid token, so `requirePlatformAdmin` (the only 503 source) never runs for a no-token
request — no-token = 401, not 503.

**3. RA2-5 boot-guard (route-tree introspection, FATAL on ungated `/api/admin*`) → NEW [HIGH].**
The specified introspection surface ("`onRoute` hook / `fastify.routes`", proposal §3.5 / §9) does NOT
expose context-inherited `addHook('onRequest', …)` hooks. The gate is added via `addHook` on the
parent encapsulation context (the exact live pattern, backups.ts:8 / notification-audit.ts:8), NOT as
a per-route `onRequest`. Empirical probe (Fastify 5.8.5, parent `addHook(verifyAuth)` +
`addHook(requirePlatformAdmin)`, child `c.get('/notification-audit', handler)`):
```
onRoute view → { url: '/api/admin/notification-audit', onRequest: [] }
```
The child's `routeOptions.onRequest` is EMPTY for a route that IS correctly gated at request time.
Consequence — the boot-guard as written is non-functional either way:
- Implemented literally ("FATAL if any `/api/admin*` route lacks `requirePlatformAdmin` in its
  onRequest chain") → it FATALs on EVERY legitimately-gated child on every boot → the API never
  starts → total admin-plane self-DoS at deploy.
- Relaxed so it doesn't brick boot → it cannot distinguish a gated-by-inheritance child (`onRequest:[]`)
  from an ungated sibling registered in a different context (`onRequest:[]` too) → it is a no-op that
  catches nothing → the RA2-5 sibling-BOLA hole degrades back to the evadable AST lint, which RA2-5
  itself conceded is insufficient.
The route-level surface cannot tell "inherited the hook" from "has no hook"; verifying real coverage
would require walking Fastify's private encapsulation `kHooks` symbol tree (not a stable API), which
the design does not specify. The boot-guard is named "the authority that cannot be evaded by computed
strings" closing the original CRITICAL F1 for siblings; as specified that authority is unrealizable.
*Invariant at risk:* "every current AND future `/api/admin*` route is structurally gated" — true for
in-parent children (hooks DO run at request time), but the sibling case is NOT closed by a working
boot-guard. **Blocks hard-exit:** RA2-5's resolution rests entirely on "boot-guard = authority";
that mechanism does not work via the named surface.

**Verdict for council:** RA2-3 HOLDS · RA2-1 HOLDS · boot-guard NEW [HIGH] (mechanism non-functional
via the named introspection surface — needs a coverage-detection method that sees context-inherited
hooks, or the sibling-BOLA residual stays open under the lint only). The two load-bearing data/wiring
fixes are sound; the third leg (sibling structural closure) is not yet demonstrable.
