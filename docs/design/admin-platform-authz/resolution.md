# Resolution — B4 RESOLVE round (Breaker + Counsel)

- Re: `proposal.md`, `counsel-opinion.md`, `breaker-findings.md`
- Date: 2026-06-29
- Status: design only — NO production code. Every disposition is a design delta folded into
  `proposal.md` + `docs/adr/ADR-admin-platform-authz.md`.
- Honesty note: the Breaker re-attacks after this round. Residuals are named explicitly (R7–R11).

All 10 Breaker findings verified against live source before disposition (server.ts:793-799 three
sibling registers / no parent; notification-audit.ts:14 double-prefix; backup-verify.ts:62-79
connect→lock→release-while-held + separate-session unlock; notification-audit.ts:46 `err.message`
leak). None were spurious.

---

## Disposition table (one row per finding)

| # | Finding | Sev | Disposition | Design delta (where) |
|---|---------|-----|-------------|----------------------|
| F1 | Gate is per-file opt-in, not encapsulated | CRITICAL | **FIX** | §3.5 + §4.1 + §9: single **parent encapsulation plugin** at `/api/admin` registers `requirePlatformAdmin` as one `onRequest` hook, then registers the 3 children with **no prefix**; eslint rule promoted **optional → enforced (build error)** as defense-in-depth. |
| F2 | Advisory lock leaks / self-deadlocks under pooling | HIGH | **FIX** | §6: **one lock, one owner** — drop the proposed NEW route-layer lock; refactor the existing key-3 lock to the **dedicated-client** pattern (acquire→work→release on the SAME pooled client, release lock before returning client). Route maps a failed try-lock to 409. |
| F3 | R4 bites — admin routes never set `app.user_id`; `pa_self_read` fails closed under B3 | HIGH | **FIX** | §5 + §7: the re-check runs via a **search_path-pinned `SECURITY DEFINER` fn `is_platform_admin(uuid) → boolean`** — does NOT depend on the GUC or the pool's RLS posture. `pa_self_read` demoted to belt-and-suspenders. B3-order-independent by construction. |
| F4 | Endpoint #6 real path is double-prefixed | MED | **FIX** | §1 table + §4 table + DoD #6: child handler declares `/notification-audit` (under parent prefix `/api/admin` → real path `/api/admin/notification-audit`). All references corrected. |
| F5 | Audit row written after the side-effect | MED | **FIX** | §5 + §6: **write-ahead intent row** — audit table gains a `status` column; INSERT `status='started'` row in the SAME tx that takes the drill lock, BEFORE `runRestoreVerify`; UPDATE to `completed`/`failed` after. No side-effect without a trail. |
| F6 | "Fail-closed" asserted, not structurally guaranteed | MED | **FIX** | §7 + DoD: fail-closed is now a **parent `onRequest` wiring property** (`const ok = await is_platform_admin(...)`; throw→catch→503-deny; reply-and-return short-circuits the handler). Added a WIRED integration test (DoD #4b). Fixed the `err.message` schema leak (generic 500 envelope). |
| F7 | Bootstrap seed can hard-lock or FATAL the deploy | MED | **FIX** | §5 + §8: **decouple seed from migration** — migration creates tables only, never seeds from env, never FK-fails. Bootstrap is the ops CLI run AFTER deploy (verifies user exists, idempotent `ON CONFLICT`). 0-admins = safe fail-closed 403, recoverable via DB-creds break-glass. Never FATAL. |
| F8 | Insider-removal latency is request-entry, not in-flight | LOW | **ACCEPT-RISK** (R7) | §6 + §9 wording: claim scoped to **"takes effect at next request-entry; an in-flight drill (≤30 min) completes."** Owner: Ops. Bounded by drill TIMEOUT_MS; acceptable for the only long-running endpoint. |
| F9 | Re-check keys on `$sub`; rest of auth keys on `userId` | LOW | **FIX** | §4: canonical field is **`request.user.userId`**; the DEFINER fn takes it; coupling (owner mint sets `sub==userId`) documented. |
| F10 | `pa_audit_read USING(true)` no least-privilege isolation | LOW | **ACCEPT-RISK + DEFER-FLAG** (R8) | §8 + R8: accepted at N=1–5; the out-of-band audit mirror (ETHICAL-STOP floor) is the real mitigation, deferred to the named trigger. Owner: Architect. |
| E1 | **Counsel ETHICAL-STOP-1** — self-watched audit, tenant-invisible, no appeal | — | **NEEDS HUMAN DECISION → STOP-ETHICS** | Cannot be resolved unilaterally. Recommended floor pre-staged (out-of-band append-only audit mirror) + trigger; the recorded human decision unblocks the council. Gate ships regardless. |
| E2 | Counsel non-blocking #1 — kill-switch must not darken recovery tools | — | **FIX** | §9: split flags — `requirePlatformAdmin` always on; `ADMIN_DRILLS_ENABLED` scopes ONLY the weaponizable drill endpoints (`verify`, `dr-report`). Recovery reads (`backups` list, `fallback/health`) never darkened by the drill kill-switch. |
| E3 | Counsel non-blocking #2 — mirror audit reader out-of-band before first non-founder ops hire | — | **DEFER-FLAG** (R9) | Tracked with explicit trigger (first non-founder ops hire OR tenant-count threshold). Owner: Ops + Architect. Same lever as E1's floor. |
| E4 | Counsel steel-man — schedule Option C (network-isolated ops service) | — | **DEFER-FLAG** (R10) | §3 Option C re-credited: scheduled as the next hardening at a tenant/headcount threshold (not dismissed). Owner: Architect. |
| — | requireLocationAccess raw `pool.query` on memberships also breaks under B3 | (pre-existing) | **DEFER-FLAG** (R11) | Pre-existing, surfaced by F3; coordinate with B3. Out of B4 scope but tracked so it is not lost. Owner: Architect + B3 owner. |

---

## Concrete design deltas (the load-bearing detail)

### F1 — Structural plane gate (chosen mechanism: parent encapsulation + enforced eslint)

**Chosen: parent encapsulation plugin** (Fastify idiom — hooks are inherited by children registered
within the same encapsulation context, so coverage is by-construction for every current AND future
admin route). A path-prefix global `onRequest` was considered; encapsulation is preferred because it
scopes the gate to exactly the admin context and cannot be accidentally bypassed by a route that
mounts under a different prefix-string.

```
// server.ts (sketch — not production code)
const { default: adminPlane } = await import('./routes/admin/index.js');
fastify.register(adminPlane, { prefix: '/api/admin', db: pool, queue, storage });

// routes/admin/index.ts (the ONLY thing under /api/admin)
const adminPlane: FastifyPluginAsync = async (f, opts) => {
  f.addHook('onRequest', requirePlatformAdmin);          // ONE hook, covers all children
  await f.register(backupAdminRoutes, opts);             // NO prefix — inherit /api/admin
  await f.register(fallbackAdminRoutes, opts);
  await f.register(notificationAuditRoutes, opts);
};
```

Defense-in-depth: the `tools/eslint-plugin-local` rule is **promoted from optional to enforced (build
error)** — it asserts (a) no `fastify.register(..., {prefix:'/api/admin'})` exists outside
`routes/admin/index.ts`, and (b) `routes/admin/index.ts` registers `requirePlatformAdmin` as an
`onRequest` hook. Structure is the guarantee; the lint is the tripwire.

### F2 — One lock, one owner (dedicated-client single-flight)

Delete the proposed route-layer `pg_try_advisory_lock(<const>)`. Refactor `acquireLock`/`releaseLock`
(`backup-verify.ts:62-79`) so the lock is acquired and released on the **same** dedicated pooled
client, held for the drill's lifetime, released BEFORE the client returns to the pool:

```
// sketch
const client = await pool.connect();
try {
  const { rows } = await client.query('SELECT pg_try_advisory_lock(3) AS locked');
  if (!rows[0].locked) return { status: 'in_progress' };   // route → 409
  try { return await runRestoreVerify(client, ...); }       // use THIS client for the drill
  finally { await client.query('SELECT pg_advisory_unlock(3)'); }
} finally { client.release(); }
```

Transaction-scoped `pg_advisory_xact_lock` was rejected: the drill runs up to 30 min and an open tx
that long means idle-in-transaction bloat. The dedicated-client (session-level, explicit unlock in
`finally`, release-after-unlock) holds exactly one pool slot for the single in-flight drill —
acceptable because single-flight already bounds it to one. Crash safety: if the process dies the
backend session ends and PG releases the session lock; the previous code's bug was returning the
client to the pool (session survives) while the lock was held.

### F3 — Re-check is RLS-independent (SECURITY DEFINER, not GUC)

```sql
CREATE FUNCTION is_platform_admin(p_user_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public AS $$
    SELECT EXISTS (SELECT 1 FROM platform_admins
                   WHERE user_id = p_user_id AND status = 'active');
  $$;
REVOKE ALL ON FUNCTION is_platform_admin(uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION is_platform_admin(uuid) TO <operational_role>;
```

The hook calls `SELECT is_platform_admin($1)` with `request.user.userId`. The result does NOT depend
on `app.user_id`, on `pa_self_read`, or on whether the pool is BYPASSRLS (today) or NOBYPASSRLS
(post-B3). `search_path` is pinned (reuses the existing DEFINER search_path guardrail, ledger #33).
`pa_self_read` stays as belt-and-suspenders but is no longer load-bearing. This makes the authZ gate
genuinely B3-order-independent — F3's exact requirement.

### F5 — Write-ahead audit intent

`platform_admin_audit_log` gains `status text NOT NULL DEFAULT 'completed' CHECK (status IN
('started','completed','failed'))`. For the two drill endpoints: INSERT a `started` row in the same
tx that takes the lock, BEFORE `runRestoreVerify`; UPDATE to `completed`/`failed` after. Read-only
endpoints write a single `completed` row (no side-effect to precede). No destructive action can occur
without an intent row already committed.

### F6 — Fail-closed as a wiring property

The parent `onRequest` hook is the single choke point. Pattern: `const ok = await
isPlatformAdmin(req.user.userId); if (!ok) return reply.code(403).send(envelope('forbidden'));` —
reply-and-return short-circuits before any handler runs; a thrown error is caught → `reply.code(503)
.send(envelope('admin_unavailable'))` (deny). A handler-level swallow cannot admit because the gate
already ran at `onRequest`. Fixed the `notification-audit.ts:46` `err.message` leak → generic 500
envelope (no schema egress). DoD adds #4b: a WIRED integration test mounting the real parent plugin
with a stub `isPlatformAdmin` that throws → asserts 503, handler never invoked.

### F7 — Bootstrap can't FATAL or hard-lock

Migration: creates tables + the DEFINER fn ONLY. It does NOT read `PLATFORM_ADMIN_BOOTSTRAP_USER_ID`
and never INSERTs an FK-bearing row at migration time → no FK violation → no boot-guard FATAL.
Bootstrap = `scripts/platform-admin-grant.ts <userId>` run AFTER deploy with DB creds: verifies the
user row exists first (clean error if not), idempotent `INSERT ... ON CONFLICT (user_id) DO UPDATE
SET status='active'`. The 0-admins state is a SAFE fail-closed (everyone 403) recoverable by the same
break-glass CLI — documented in the runbook. Decision recorded: **warn-and-leave-empty beats
FATAL-on-missing-env**; the plane being closed is recoverable, a crash-looping deploy is worse.

### E2 — Kill-switch granularity

`ADMIN_PLANE_ENABLED` is **removed** as a blunt instrument. Replaced by `ADMIN_DRILLS_ENABLED`
(default true) which gates ONLY `POST /backups/verify` + `GET /backups/dr-report` (the weaponizable,
resource-heavy drills). `requirePlatformAdmin` and the recovery reads (`GET /backups`,
`GET /fallback/health`, `POST /fallback/r2-check`, `GET /notification-audit`) are NEVER darkened by a
kill-switch — they are exactly what ops needs during an incident. The authZ gate has no off switch.

### E1 — ETHICAL-STOP-1 → STOP-ETHICS (needs human decision)

I cannot resolve this unilaterally; it is routed to STOP-ETHICS for ONE recorded human decision + a
date. To make ratification cheap I pre-stage the recommended minimum legibility floor:

- **Recommended floor (cheapest, highest leverage):** append-only **out-of-band mirror** of
  `platform_admin_audit_log` to a sink the platform-admins cannot silently rewrite (so the watcher is
  watched by something). This is the same lever as Counsel non-blocking #2 (E3) and the F10 mitigation.
- **Recommended trigger to ENACT the mirror:** the FIRST of {first non-founder ops hire, tenant-count
  ≥ a threshold the human sets, first tenant data dispute, first acquirer due-diligence request}.
- **Recommended explicit deferrals (record as DECIDED, not defaulted):** tenant right-to-know channel
  and per-drill second-admin confirmation are deferred to a dated review; Option C (network-isolated
  ops service, E4/R10) scheduled as the next hardening at the same headcount/tenant threshold.

The human decides which floor + date. The v1 gate ships regardless of this decision (Counsel:
friction, not veto).

---

## Residual risks after this round (honest — re-attack will probe these)

| ID | Residual | Disposition | Owner |
|----|----------|-------------|-------|
| R7 | In-flight drill (≤30 min) survives a mid-flight revoke | ACCEPT — bounded by TIMEOUT_MS; only the long endpoints; claim scoped to request-entry | Ops |
| R8 | `pa_audit_read USING(true)` — any admin reads all admins' trail | ACCEPT at N=1–5; out-of-band mirror is the real fix, deferred to trigger | Architect |
| R9 | Out-of-band audit mirror not built in v1 | DEFER-FLAG — trigger: first non-founder ops hire / tenant threshold | Ops + Architect |
| R10 | Option C (network isolation) not built | DEFER-FLAG — scheduled at headcount/tenant threshold; not dismissed | Architect |
| R11 | `requireLocationAccess` raw `pool.query` on memberships breaks under B3 (pre-existing) | DEFER-FLAG — coordinate with B3; out of B4 scope but tracked | Architect + B3 owner |
| R1 | Cross-tenant `locations` reads (`fallback/health`, `r2-check`) need a platform-read path post-B3 | DEFER-coordinate with B3 (carried from proposal §10) | Architect + B3 owner |
| E1 | Tenant legibility floor | NEEDS HUMAN DECISION → STOP-ETHICS | Human (founder) |

R1, R8, R9, R10, R11 and E1 are the surface the re-attack should hit. F1–F7 + F9 are closed by
construction in the updated proposal; F8/F10 are explicitly accepted.

---

# Resolution round 2 (RE-ATTACK RA2-1..RA2-6)

- Date: 2026-06-29. Design only — NO production code.
- Every RA2 finding verified against LIVE source before disposition (cited below). None spurious.
- Deltas folded into `proposal.md` (§3.5/§4/§5/§6/§7/§8/§9/§10/§DoD) + `docs/adr/ADR-admin-platform-authz.md`.

## Disposition table (round 2)

| # | Finding | Sev | Disposition | Design delta |
|---|---------|-----|-------------|--------------|
| RA2-3 | DEFINER fn over FORCE-RLS silently depends on an unstated owner-BYPASSRLS attribute; "B3-order-independent, no RLS dependency" is false (it relocated the dependency). | HIGH | **FIX (simplify — the crux)** | §5 rewritten: `platform_admins` is a **non-tenant GLOBAL table with NO RLS**; protection = `REVOKE ALL FROM PUBLIC` + `GRANT SELECT` to the operational role (no write GRANT). Re-check = plain point-read `SELECT 1 … WHERE user_id=$1 AND revoked_at IS NULL`, identical under BYPASSRLS/NOBYPASSRLS. **Deleted: `is_platform_admin` DEFINER fn, FORCE-RLS, `pa_self_read`, `pa_audit_read`.** Dissolves F10/R8 (no RLS policy to misconfigure → audit-read least-privilege is a plain GRANT). |
| RA2-1 | Parent `onRequest` runs before child `verifyAuth` → `request.user` null at the gate → 503 for everyone. | MED | **FIX** | §3.5 + §4: parent registers `verifyAuth` **then** `requirePlatformAdmin` (Fastify runs parent `onRequest` hooks before child, in registration order); the 3 children **drop their own `verifyAuth` AND `requireRole(['owner'])`** (parent gate replaces both). DoD #4d added. |
| RA2-5 | "By construction for all" overstated — encapsulation gates children only; a sibling at `/api/admin` is caught only by an evadable lint. | MED | **FIX (strengthen beyond lint)** | §3.5 + §9 + DoD #4e: a **boot-guard** introspects the materialized route tree and FATAL-exits if any `/api/admin*` route lacks `requirePlatformAdmin` in its `onRequest` chain (authority); lint stays as the fast tripwire. Claim re-scoped: "structural for children + boot-guard for siblings." |
| RA2-2 | Dedicated-client route lock + unchanged internal key-3 lock collide → drill permanently "in progress"; Pool-vs-client signature mismatch. | MED | **FIX** | §6: route-layer lock **deleted**; ONE owner = the internal `acquireLock(pool,3)`/`releaseLock` at `backup-verify.ts:259/360`, leak fixed **in place** — `acquireLock` returns the locked dedicated client, `runRestoreVerify` holds that SAME client, `releaseLock(client)` unlocks-then-releases. Pool signature reconciled. |
| RA2-4 | "INSERT started in the SAME tx that takes the lock … UPDATE after" defeats write-ahead or holds a 30-min idle tx. | MED | **FIX** | §5 + §6: the `started` intent row is its **OWN short committed tx BEFORE** `runRestoreVerify`; `completed`/`failed` is a second short tx after. No tx wraps the 30-min drill. |
| RA2-6 | R11 is a BLOCKING B3 co-dependency, not a soft "coordinate/track." | MED (cross-cutting) | **DEFER OUT of B4 + record as a HARD B3 blocker** | §10 R11 re-graded + ADR header + ADR Open-items: B3 cannot flip the operational pool to NOBYPASSRLS without simultaneously making `requireLocationAccess` (`auth.ts:148`) NOBYPASSRLS-safe, or it self-DoSes every owner. Written as a cross-finding the B3 handoff inherits. Owner: Architect → B3. |

## Verified facts (live source)

- `backups.ts:8-9`, `fallback.ts:9-10`, `notification-audit.ts:8-9` — each child registers
  `verifyAuth` THEN `requireRole(['owner'])` as its own `onRequest` hooks → confirms RA2-1 and that the
  children must drop both.
- `migrations/1780421100065_lockdown-nontenant-api-surface.ts:26-31` — `users`/`ops_worker_heartbeat`/
  `auth_refresh_tokens` are in fact `ENABLE`+`FORCE ROW LEVEL SECURITY` **with no policies** (read via
  a BYPASSRLS pool), **not** "NOT FORCE-RLS" as the steer stated. Recorded as an honest divergence in
  §5: our re-check runs on the **operational** pool B3 flips to NOBYPASSRLS, so we deliberately use
  **no RLS** + `GRANT SELECT` — strictly more B3-safe than the lockdown pattern for a table read on
  the NOBYPASSRLS pool. The conductor's *conclusion* (non-tenant global, table-GRANT-protected) holds;
  the cited *precedent* was imperfect and is not relied on.
- `1790000000015_operational-pool-role.ts` — operational role `deliveryos_operational_user` is
  `NOBYPASSRLS`, auto-granted `SELECT` on future tables. Confirms a no-RLS `platform_admins` is readable
  by it with no policy, and that a FORCE-RLS table would have returned 0 rows (the RA2-3 trap).
- `core-identity.ts:91-92` — `memberships` is `FORCE ROW LEVEL SECURITY`; `auth.ts:148` does a raw
  `pool.query` on it with no `app.user_id` → confirms RA2-6 (works only under BYPASSRLS today).
- `backup-verify.ts:62-79` (leaky `acquireLock`/`releaseLock`), `:249` (`runRestoreVerify(pool,…)`),
  `:259/:360` (internal key-3 lock), `:261` (`'Another verify in progress'` short-circuit) — confirm
  RA2-2 (the route-layer lock would self-collide; signature is `Pool`).

## Honest residuals after round 2 (the next focused re-attack should probe RA2-3 + RA2-1)

| ID | Residual | Disposition | Owner |
|----|----------|-------------|-------|
| R11/RA2-6 | `requireLocationAccess` (`auth.ts:148`) NOBYPASSRLS-unsafe | **HARD blocking dependency of B3** (not B4 code) | Architect → B3 owner |
| R1 | Cross-tenant `locations` reads (`fallback/health`, `r2-check`) need a platform-read path post-B3 | DEFER-coordinate with B3 | Architect + B3 owner |
| R12 | No-RLS tables trip Supabase linter 0013 | ACCEPT — cosmetic; perimeter closed by lockdown GRANT revokes; RLS would re-trip RA2-3 | Architect |
| R8 | Audit-read no per-admin isolation (now plain GRANT + app gate) | ACCEPT at N=1–5; out-of-band mirror is the real fix | Architect |
| R7 | In-flight drill (≤30 min) survives a mid-flight revoke | ACCEPT — bounded by TIMEOUT_MS | Ops |
| E1 | Tenant legibility floor | NEEDS HUMAN DECISION → STOP-ETHICS (unchanged) | Human (founder) |

**Net round-2 effect:** RA2-3 removed an entire machinery layer (DEFINER fn + FORCE-RLS + two RLS
policies + GUC dependency) and replaced it with a plain point-read on a non-tenant table — simpler AND
genuinely B3-independent. RA2-1/RA2-2/RA2-4/RA2-5 are wiring/lock/tx/coverage fixes. RA2-6 is pushed
to B3 as a hard, named blocker. No production code written.

---

# Resolution round 3 (FINAL-CONFIRM finding — RA2-5 boot-guard unrealizable)

- Date: 2026-06-29. Design only — NO production code.
- Scope: ONE finding — RA2-5, the route-tree boot-guard named in round 2 as "the structural authority
  that closes sibling-BOLA." The Breaker's final-confirmation round graded it **NEW [HIGH]: mechanism
  non-functional via the named introspection surface.** RA2-3 and RA2-1 were re-confirmed HOLDS and are
  not re-litigated. Verified against live Fastify 5.8.5 source before disposition.

## The verified problem (confirmed, not spurious)

Fastify's public route introspection (`onRoute` hook, `fastify.routes`, `routeOptions.onRequest`) does
**NOT** expose context-INHERITED `addHook('onRequest', …)` hooks. A child route correctly gated by an
inherited parent/root hook reports `onRequest: []` — **identical** to an ungated sibling. Verified:
- `apps/api/node_modules/.../fastify/lib/route.js:556-565` runs `context.onRequest` (the *materialized*
  per-context hook chain at request time), with `context` = the matched-route context built at
  `route.js:513` — so inherited hooks DO run, but they live on the context's hook chain, not on the
  per-route `routeOptions` the boot-time introspection surface exposes.
- The Breaker's empirical 5.8.5 probe (`onRoute view → { url: '/api/admin/notification-audit',
  onRequest: [] }`) confirms the route-level surface shows `[]` for a route that IS gated by inheritance.

Consequence — the boot-guard as specified is non-functional either way: implemented literally it FATALs
every legitimately-gated child on every boot (total self-DoS); relaxed, it cannot tell inherited-hook
from no-hook, catching nothing. The round-2 claim "boot-guard = the authority that closes siblings"
does not hold. Finding **CONFIRMED.**

## Disposition

| # | Finding | Sev | Disposition | Design delta |
|---|---------|-----|-------------|--------------|
| RA2-5 (round 3) | Route-tree boot-guard cannot see context-inherited hooks → unrealizable as the sibling-closure authority. | HIGH | **FIX — replace detection with construction** | §3.5 + §7 + §9 + DoD #4e + header: structural authority moves to a **root-instance `onRequest` hook** (registered once in `server.ts`); boot-guard deleted → optional boot-time visibility log; DoD #4e replaced with a provable throwaway-sibling E2E. |

## The fix — STRUCTURAL BY CONSTRUCTION, not by detection

Register **one root-instance `onRequest` hook** on the top-level Fastify instance in `server.ts`,
before/around all route registration. For any request whose **matched route pattern** is under
`/api/admin`, it runs `verifyAuth` then `requirePlatformAdmin`; for everything else it early-returns.

- **Why it closes siblings/future by construction:** a root-instance `onRequest` hook flows down into
  **every** child context (ancestor hooks are copied into every route's `context.onRequest` and run
  first, in registration order — `route.js:556-565`). So the hook is on every route's materialized
  chain — child, sibling, another-plugin, future — with **zero detection.** There is no route tree to
  walk and no "did it inherit the hook" question to answer. This is the dual of how encapsulation
  closes children, applied one level up to close the whole plane.
- **Path-matching robustness (the load-bearing detail):** gate on `request.routeOptions.url` (=
  `context.config?.url`, verified `request.js:188`), the **matched route PATTERN** that find-my-way
  already decoded/normalized/`rewriteUrl`-applied — **never** the raw `req.url`. Predicate:
  `url === '/api/admin' || url.startsWith('/api/admin/')`.
  - **Zero false-negative:** any request that routes to an admin *handler* matched a route whose
    registered pattern is `/api/admin/…` by definition; no crafted case/`%2e`/`%2f`/slash path reaches
    an admin handler while yielding a non-admin `routeOptions.url` (routing already normalized before
    onRequest runs).
  - **Zero false-positive:** the `/`-or-end boundary excludes `/api/administrators` and other
    prefix-sharing non-admin routes.
  - **`is404`/undefined `routeOptions.url`** (`request.js:206`) = no route matched → not gated → falls
    to the 404/SPA handler, which reaches no admin handler and leaks no cross-tenant data — not a gap.
- **Order:** the root hook runs before any child's own hooks, so it must itself run `verifyAuth` first
  (401 short-circuits a no-token request) then the gate — it cannot rely on a child `verifyAuth` that
  runs later. Mirrors RA2-1's ordering, hoisted to the root.

## Defense-in-depth, demoted

1. **Root `onRequest` hook = structural authority** (closes children + siblings + future, by
   construction).
2. **Encapsulated parent plugin = organizational primary** for the 3 known routes (still registers
   `verifyAuth` → `requirePlatformAdmin`; redundant second read on a < 1 req/s plane, accepted — or
   optionally drop its hooks and keep it purely for file grouping; the root hook is the authority
   either way).
3. **Enforced eslint rule = fast tripwire** (no `register({prefix:'/api/admin'})` outside
   `routes/admin/index.ts`; root admin gate present in `server.ts`).
4. **Boot-time visibility log = operability, NOT authority** — enumerates `/api/admin*` route patterns
   (these ARE enumerable; only inherited-hook coverage is not) so new admin routes show in deploy logs.

## DoD delta

DoD #4e is **replaced**: the unrealizable "ungated sibling → FATAL boot" item is deleted; the new item
registers a THROWAWAY route at `{prefix:'/api/admin'}` **outside** the parent plugin (no per-route auth
hook — the computed-prefix shape the lint evades) and asserts it returns **403** to an owner JWT,
**401** to a no-token request, and passes a platform-admin — proving structural closure at request time
by the root hook. Plus a `routeOptions.url` precision assertion: `/api/administrators` is NOT gated.

## Fastify-5.8.5 verification (done before asserting)

- `apps/api/package.json:34` → `fastify ^5.8.5`; installed `node_modules/.../fastify@5.8.5`.
- `route.js:513` — request built with the matched-route `context`; `:556-565` — `context.onRequest`
  (materialized inherited chain) runs **after routing, before the handler**.
- `request.js:180-188` — `request.routeOptions.url` returns `context.config?.url` (the matched route
  pattern); `:204-206` — `request.is404` is `context.config?.url === undefined`.
- Root-instance `addHook('onRequest', …)` flows into every child context (standard Fastify
  encapsulation: descendants inherit ancestor hooks, ancestor-first) → on every route's context chain
  by construction, including sibling-plugin and future routes.

## Residual after round 3 (honest)

- **R13 (ACCEPT, Architect):** the gate closes the NAMED `/api/admin` plane, not "every cross-tenant
  handler everywhere." A cross-tenant handler mounted under a *different* prefix (`/api/internal/*`) is
  a different plane needing its own gate — the same scope boundary every prefix-based gate has. The
  boot-time visibility log + the eslint plane convention keep new admin surfaces visible; a brand-new
  cross-tenant plane is a new design decision, not a silent escape from this gate.
- RA2-3 (point-read), RA2-1 (verifyAuth ordering) re-confirmed HOLDS by the final-confirm round; not
  re-opened. R7/R8/R11/R1/R12/E1 unchanged.

**Net round-3 effect:** the only surviving authority claim that did NOT hold — "boot-guard closes
siblings" — is replaced by a mechanism that holds by construction (root `onRequest` hook keyed on the
matched route pattern), verified against live Fastify 5.8.5. Detection (which the surface cannot do) is
removed entirely. No production code written.
