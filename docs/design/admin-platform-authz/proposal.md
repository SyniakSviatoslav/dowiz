# Design Proposal — B4: Close BOLA on `/api/admin/*` via a platform-admin principal

- Status: DRAFT (design only — no production code in this artifact)
- Date: 2026-06-29
- Red-line: AUTH / TENANT-ISOLATION. Forward-only. Reversible (additive guard + drop-table down()).
- Relates: ADR-0004 (owner-token revocation, insider-removal `status='active'` pattern), ADR-0003
  (dev-kid fail-closed), B3 (admin pool NOBYPASSRLS — sibling, ordering-independent, see §7/§9).
- ADR: `docs/adr/ADR-admin-platform-authz.md`
- **RESOLVE round applied (2026-06-29):** folds in all 10 Breaker findings + Counsel ETHICAL-STOP-1.
  See `breaker-findings.md` (verbatim) + `resolution.md` (dispositions). Key deltas: structural parent
  encapsulation gate (F1), one-lock-one-owner dedicated-client single-flight (F2), DEFINER-fn re-check
  (F3 — **superseded in round 2**), double-prefix path fix (F4), write-ahead audit (F5), wired
  fail-closed (F6), bootstrap decoupled from migration (F7), drill-scoped kill-switch (E2). ETHICAL-
  STOP-1 routed to **STOP-ETHICS** (§11).
- **RESOLVE round 2 applied (2026-06-29):** folds in RA2-1..RA2-6. Key deltas: **RA2-3** — the DEFINER
  fn + FORCE-RLS + `pa_self_read`/`pa_audit_read` are **deleted**; `platform_admins` is a **non-tenant
  no-RLS global table** read by a plain point-read (genuinely B3-independent, dissolves F10/R8's RLS
  concern). **RA2-1** — parent registers `verifyAuth` THEN `requirePlatformAdmin`; children drop their
  own `verifyAuth`/`requireRole(['owner'])`. **RA2-5** — a boot-guard route-coverage check (authority)
  + lint (tripwire). **RA2-2** — one lock owner (the internal key-3 lock), leak fixed in place;
  route-layer lock deleted. **RA2-4** — audit `started` row in its OWN short committed tx. **RA2-6** —
  R11 re-graded to a HARD blocking dependency of B3 (cross-finding). See `resolution.md` §"Resolution
  round 2".
- **RESOLVE round 3 applied (2026-06-29):** folds in the Breaker's final-confirmation finding **RA2-5
  (boot-guard unrealizable).** Empirically (Fastify 5.8.5 probe + source: `route.js:513/556`,
  `request.js:180-206`) the `onRoute`/`fastify.routes` introspection surface does **NOT** expose
  context-INHERITED `addHook('onRequest', …)` hooks — a correctly gated-by-inheritance child shows
  `onRequest: []`, identical to an ungated sibling — so a route-tree boot-guard cannot distinguish
  gated from ungated and is non-functional. **Delta:** the structural authority for sibling/future
  closure moves to a **root-instance `onRequest` hook** (registered once in `server.ts` on the
  top-level Fastify instance) that runs for EVERY request and gates any request whose **matched route
  pattern** (`request.routeOptions.url`, not the raw string) is under `/api/admin` — closing F1/RA2-5
  by construction without any detection. The encapsulated parent plugin stays as the organizational
  primary for the 3 known routes (defense-in-depth); the enforced eslint rule stays a tripwire; the
  route-tree boot-guard is **replaced by an optional boot-time visibility log** (operability, not
  authority). DoD #4e is replaced with a provable throwaway-sibling E2E. See `resolution.md`
  §"Resolution round 3".

---

## 1. Problem + non-goals

### Problem (verified live)
`/api/admin/*` exposes **cross-tenant platform operations** but is gated only by
`requireRole(['owner'])` (`apps/api/src/plugins/auth.ts:105-114`) — any of the (potentially
thousands of) tenant owners passes that check. None of the three handlers consult `request.user`
to scope, and they read on the **BYPASSRLS hot-path pool**, so RLS is not a backstop today. This is
**Broken Object-Level / Function-Level Authorization (BOLA/BFLA)** — a launch-blocker red-line.

Six endpoints, each over-authorized to every owner (file:line verified):

| # | Endpoint | File:line | Leak / weaponization today |
|---|----------|-----------|----------------------------|
| 1 | `GET /backups` | `routes/admin/backups.ts:12,18` | All-tenant `backup_metadata WHERE 1=1` — fleet backup inventory |
| 2 | `POST /backups/verify` | `:68-73` | Any owner triggers `runRestoreVerify(any backupId)` — DR-drill weaponization (resource exhaustion) |
| 3 | `GET /backups/dr-report` | `:78-79` | Platform-wide `runRestoreVerify(fullHash:true)` — heavy, fleet-wide |
| 4 | `GET /fallback/health` | `routes/admin/fallback.ts:13-24` | All locations' name/slug/**public phone**/fallback_config — cross-tenant PII |
| 5 | `POST /fallback/r2-check` | `:46-51` | Platform-wide coverage counts |
| 6 | `GET /notification-audit` | `routes/admin/notification-audit.ts:14` | Optional unvalidated `locationId` → unscoped cross-tenant audit counts |

> **Path correction (Breaker F4):** the handler currently declares `fastify.get('/admin/notification-audit')`
> under `prefix:/api/admin`, so the *real* path today is `/api/admin/admin/notification-audit` (double
> prefix) and the single-prefix path is UNREGISTERED (falls to the SPA handler → 200 `index.html` / 404).
> The fix-PR moves the handler to declare `/notification-audit` so the real path is
> `/api/admin/notification-audit`. All references (this table, §4 table, DoD #6) use the corrected path.

Root cause: there is **no principal distinct from `owner`**. The role discriminatedUnion has exactly
three roles — `owner`/`courier`/`customer` (`packages/shared-types/src/legacy.ts:163-174`). "Admin"
was modeled as "an owner" — collapsing the tenant plane and the platform/ops plane onto one role.

### Non-goals
- **Not** building a generic RBAC/permission matrix. We need exactly one new boolean authority
  (platform-admin: yes/no), not a role engine. (YAGNI / ponytail.)
- **Not** building owner-facing self-service views of fallback/notification health in v1. The seam is
  designed (deferred owner plane, §3) but runtime is not enabled — "schema rich, runtime minimal".
- **Not** fixing the admin pool's BYPASSRLS posture — that is sibling B3. This gate is designed to
  hold **independently of B3 ordering** (§7).
- **Not** an admin UI. This closes the API authorization hole; the existing SPA admin screens consume
  the same routes and continue to work for a provisioned platform-admin.

---

## 2. Back-of-envelope

**Population of platform-admins.** This is an ops/founder population, NOT per-tenant. Realistically
**1–5 today** (founder + 1–2 ops), growing to **~10–20** at hundreds of locations. It does **not**
scale with tenant count. (Contrast: thousands of owners.)

**Admin-route request volume.** These are ops-console + release-gate calls, not customer traffic:
- Sustained: **< 1 req/s** (a human refreshing a dashboard; a CI release-gate poll on
  `notification-audit`).
- Burst: **~10 req/s** during a DR drill or an incident page-refresh storm.
- This is **2–3 orders of magnitude below** the owner hot path (projected 15→150 req/s, ADR-0004).

**Cost of a per-request DB re-check.** The chosen guard does one indexed lookup
`SELECT 1 FROM platform_admins WHERE user_id=$1 AND revoked_at IS NULL` (PK on `user_id` + partial
active index) →
**~0.2 ms + one pool checkout**. At < 1 req/s sustained / ~10 req/s burst that is **≤ ~10
checkouts/min** added to the existing pool. **This is the load-bearing BOE:** ADR-0004 *rejected* a
per-request owner re-check because at 150 req/s it meant a PG checkout per hot-path request. Here the
identical mechanism is **trivially affordable** because the platform plane runs at < 1 req/s. The
mechanism that was wrong for owners is *right* for platform-admin — purely a volume argument. **No new
pool** is provisioned (that would be over-engineering against the connection budget: API + worker +
analytics + migrations); the re-check reuses the existing pool.

**Connection budget impact:** negligible — bounded by admin-route volume (< 1 req/s), no new
long-lived connections, no new pool. Does not move the API+worker+analytics+migrations envelope.

---

## 3. Options (≥2, named concepts + tradeoffs)

### Option A — 4th JWT role `platform_admin` in the discriminatedUnion
*Concept: claim-based authorization (authority baked into the token at mint).*

Add `z.literal('platform_admin')` to `AuthToken` (`legacy.ts:163`), gate with
`requireRole(['platform_admin'])`.

- + Reuses the existing `requireRole` mechanism; one-line guard.
- − **Insider-removal regresses to a token-lifetime window.** A platform-admin claim is baked for the
  access-token TTL (24h, ADR-0004). Revoking a compromised/rogue platform-admin would not take effect
  until the token expires — exactly the gap ADR-0004 fought for owners, re-opened at the *highest*
  privilege tier.
- − **Requires a mint site** that issues `platform_admin` tokens — and any mint site is a potential
  self-serve escalation surface that must be proven un-reachable by an owner. More attack surface.
- − **Ripples across every consumer** of the discriminatedUnion (`.strict()` parsers, role switches,
  WS auth, refresh re-derive). A 4-way union touched on a red-line path.
- − The token claim is forgeable only via RS256 key compromise — but on key compromise, *all* roles
  fall; this adds nothing there while adding the staleness gap above.

### Option B — allowlist table + per-request DB re-check (`requirePlatformAdmin`)
*Concept: server-authoritative principal / per-request re-derivation — mirrors ADR-0004 P-d
(`status='active'` insider-removal) and courier-session liveness (`auth.ts:74-86`).*

The JWT stays `owner` (or any authenticated principal). Platform-admin authority is a **server-side
fact** in a `platform_admins` allowlist, re-read on every admin request by a new
`requirePlatformAdmin` hook. There is **no platform-admin claim in any token**.

- + **Immediate revocation.** setting `revoked_at` takes effect on the next request — no token-lifetime
  window. Consistent with ADR-0004's enforcement philosophy.
- + **No forgeable claim** — authority is never in the token, so an owner cannot fabricate it even
  with a forged/leaked token; the DB is authoritative.
- + **No mint site, no discriminatedUnion change** → no self-serve escalation surface, no ripple on
  the red-line union.
- + **BYPASSRLS-independent** (§7): the gate is an explicit application predicate evaluated *before*
  any data query, not an RLS policy.
- − One indexed DB lookup per admin request — but the BOE (§2) shows this is ≤ ~10 checkouts/min, a
  non-issue at admin-plane volume (and *consistent* with ADR-0004, which only rejected per-request
  re-check on the 150 req/s **hot** path; the admin plane is cold).
- − Needs a forward-only migration (one new table + audit table).

### Option C — network-isolated internal ops service (mTLS / ops-secret header, off the public API)
*Concept: network segmentation / out-of-band control plane.*

Move `/api/admin/*` off the owner-facing app onto an internal-only service reachable only via mTLS or
an ops-secret header.

- + Strongest blast-radius reduction — the public edge cannot reach admin routes at all.
- − **Over-engineered for 1–5 admins.** A separate service/deploy/mTLS PKI for a cold, low-volume
  plane; the existing SPA admin screens would need re-homing.
- − Ops-secret-only (no per-actor identity) loses the **audit actor** requirement unless paired with
  a principal anyway.
- − Does not by itself give insider-removal of a specific human admin (a shared secret is all-or-
  nothing). Boring-but-blunt.

### Option D — hybrid (chosen): B as the authorization gate + a thin C-style hardening on the two destructive DR endpoints
*Concept: defense-in-depth — server-authoritative principal everywhere, plus rate-limit + single-flight
+ audit on the weaponizable DR-drill endpoints.*

### §3.5 — Wiring decision: STRUCTURAL plane gate (RESOLVE F1, CRITICAL)

The gate must close the plane **by construction**, not by per-file discipline. Today
`server.ts:793-799` registers the three admin route files as **three independent siblings**
(`fastify.register(child, {prefix:'/api/admin'})`); Fastify hooks do not cross sibling encapsulation,
so a future `routes/admin/metrics.ts` registered the same way with no hook would re-open BOLA. An
"optional eslint rule" is not closure.

**Chosen: a single parent encapsulation plugin** (`routes/admin/index.ts`) is the only thing mounted
at `prefix:/api/admin`. It adds **two `onRequest` hooks in order — `verifyAuth` THEN
`requirePlatformAdmin`** (RA2-1) — then registers the three children **with no prefix** (they inherit
`/api/admin` and both hooks). The three children **delete their own `verifyAuth` and
`requireRole(['owner'])` hooks** (live today at `backups.ts:8-9`, `fallback.ts:9-10`,
`notification-audit.ts:8-9`); the parent fully replaces both. Because Fastify inherits parent-context
hooks by every route registered within that context — and runs **parent `onRequest` hooks before child
`onRequest` hooks, in registration order** — `request.user` is populated by `verifyAuth` *before*
`requirePlatformAdmin` dereferences `request.user.userId`, and every current and future admin route is
gated.

```
// server.ts (sketch — NOT production code in this artifact)
const { default: adminPlane } = await import('./routes/admin/index.js');
fastify.register(adminPlane, { prefix: '/api/admin', db: pool, queue, storage });

// routes/admin/index.ts  (the ONLY register under /api/admin)
const adminPlane: FastifyPluginAsync = async (f, opts) => {
  f.addHook('onRequest', f.verifyAuth);           // (1) populates request.user — MUST be first
  f.addHook('onRequest', requirePlatformAdmin);   // (2) plain point-read on platform_admins → 403/503
  await f.register(backupAdminRoutes, opts);       // children: NO prefix, NO own verifyAuth/requireRole
  await f.register(fallbackAdminRoutes, opts);
  await f.register(notificationAuditRoutes, opts);
};
```

*Why the order is load-bearing (RA2-1):* if only `requirePlatformAdmin` were on the parent, it would
run before each child's own `verifyAuth`, see `request.user === null` (the decorated default,
`auth.ts:162`), throw on `.userId`, and 503 **every** caller — a fail-closed but total self-DoS of the
plane. Hoisting `verifyAuth` to the parent ahead of the gate is the fix; the children must not keep a
second `verifyAuth` (redundant) nor a `requireRole(['owner'])` (it would 403 a legitimate
platform-admin whose JWT role ≠ `owner`, contradicting "the JWT stays any authenticated principal").

**Structural authority for siblings/future = a ROOT-instance `onRequest` hook (RESOLVE round 3,
RA2-5 supersede).** Round 2 named a **boot-guard** (route-tree introspection, FATAL on ungated
`/api/admin*`) as the authority that closes the sibling hole. The Breaker's final-confirmation round
proved that mechanism **unrealizable**: Fastify's public introspection (`onRoute` hook,
`fastify.routes`/`routeOptions.onRequest`) does **NOT** expose context-INHERITED `addHook('onRequest',
…)` hooks. Empirical Fastify 5.8.5 probe (parent `addHook(verifyAuth)` + `addHook(requirePlatformAdmin)`,
child `c.get('/notification-audit', …)`) →

```
onRoute view → { url: '/api/admin/notification-audit', onRequest: [] }
```

A correctly gated-by-inheritance child shows `onRequest: []` — **identical** to an ungated sibling. So
a boot-guard walking the route tree can neither (a) implement "FATAL if any `/api/admin*` lacks the
hook" without FATAL-ing every legitimate child on every boot (total self-DoS), nor (b) relax to a form
that distinguishes inherited-hook from no-hook (the surface cannot tell them apart). The route-tree
boot-guard is therefore **deleted as an authority.**

**Chosen replacement — make closure STRUCTURAL BY CONSTRUCTION, not by detection.** Register **one
root-instance `onRequest` hook** on the top-level Fastify instance in `server.ts`, around/before all
route registration:

```
// server.ts (sketch — NOT production code). Registered ONCE on the top-level instance.
fastify.addHook('onRequest', async (req, reply) => {
  if (!isAdminRoutedPath(req)) return;            // cheap early-return for every non-admin request
  await fastify.verifyAuth(req, reply);           // populate request.user (401 short-circuits here)
  if (reply.sent) return;
  await requirePlatformAdmin(req, reply);         // plain point-read → 403 / 503-fail-closed
});
```

Because a **root-instance hook flows down into every child context** (ancestor `onRequest` hooks are
copied into every route's `context.onRequest` and run **first**, in registration order — verified:
`route.js:556-565` runs `context.onRequest`, and `context` is the matched-route context built at
`route.js:513`), this single hook gates **every** route matched under `/api/admin` — child OR sibling
OR a route added in a totally different plugin OR a future route — **with zero detection.** There is no
route-tree to walk and no "did this route inherit the hook" question to answer: the hook *is* on every
context by construction. This closes F1/RA2-5 for siblings the same way encapsulation closes it for
children.

*Why it runs before the children's own hooks:* root-context `onRequest` hooks run before child-context
ones, so the root hook cannot rely on a child's `verifyAuth` (which runs later) — it must itself run
`verifyAuth` then `requirePlatformAdmin` for admin-matched paths (mirrors §3.5/RA2-1's ordering, hoisted
one level up). For every **non-admin** request the hook is a single string-predicate check then an
immediate `return` — negligible cost on the hot path (no DB, no allocation).

**Path-matching robustness — gate on the MATCHED ROUTE PATTERN, never the raw URL (load-bearing).**
A naive `req.url.startsWith('/api/admin')` is bypassable via case / `%2e` / `%2f` / trailing-slash /
double-slash / query-string tricks, and conversely false-positives on `/api/administrators`. The robust
predicate keys on the **routed path**, i.e. the registered pattern of the route find-my-way already
matched — exposed as `request.routeOptions.url`, which returns `context.config?.url` (verified
`request.js:188`), the canonical registered route string **after** find-my-way decoded, normalized, and
applied any `rewriteUrl`:

```
function isAdminRoutedPath(req): boolean {
  const u = req.routeOptions?.url;               // matched route PATTERN, not req.url
  if (u === undefined) return false;             // req.is404 — no route matched → no admin handler to gate
  return u === '/api/admin' || u.startsWith('/api/admin/');   // prefix + boundary (/ or end)
}
```

- **Zero false-negative.** Any request that routes to an admin *handler* has, by definition, matched a
  route whose registered pattern is `/api/admin/…` (that is how the handler was registered). There is no
  crafted path that reaches an admin handler while producing a non-admin `routeOptions.url` — routing
  already normalized/decoded/rewrote the URL *before* onRequest, and `routeOptions.url` is the canonical
  pattern, not the attacker-controlled string. Case (`/api/ADMIN/…`), encoded-slash (`/api/admin%2f…`),
  and lookalikes route either to the admin pattern (→ gated) or to no admin route at all (→ 404, no
  handler).
- **Zero false-positive.** The `=== '/api/admin' || startsWith('/api/admin/')` boundary excludes
  `/api/administrators` (next char `i`, not `/`) and any other prefix-sharing non-admin route.
- **The `is404`/`undefined` case is not a gap.** An unmatched `/api/admin/does-not-exist` has
  `routeOptions.url === undefined` → not gated → falls to the 404/SPA handler, which reaches **no admin
  handler** and leaks no cross-tenant data (it returns 404 or the public SPA shell). The gate's job is to
  protect admin *handlers*; every admin handler carries a registered `/api/admin/…` pattern → gated.

**Defense-in-depth — keep the encapsulated parent, demote the boot-guard (RESOLVE round 3).**
1. **Root `onRequest` hook = the structural authority** (above) — closes children, siblings, and
   future routes by construction.
2. **Encapsulated parent plugin = organizational primary for the 3 known routes** (§3.5 above): the
   parent at `prefix:/api/admin` still registers `verifyAuth` then `requirePlatformAdmin` and the 3
   children still inherit them. For the known routes this means the gate runs twice (root first, then
   parent) — a redundant point-read on a < 1 req/s plane (§2 BOE), accepted as belt-and-suspenders; the
   root denies short-circuit so an owner pays exactly one read. (An equally valid simplification is to
   drop the parent's two hooks and let the root hook be the sole gate, keeping the parent purely for
   file organization; either is correct — the root hook is the authority in both.)
3. **Enforced eslint rule = fast pre-commit tripwire** (unchanged): (a) no
   `fastify.register(..., {prefix:'/api/admin'})` outside `routes/admin/index.ts`; (b) the root
   `onRequest` admin gate is present in `server.ts`. It is a tripwire, never the authority.
4. **Boot-time visibility log (operability, NOT authority — replaces the boot-guard).** At boot,
   enumerate every route whose pattern matches `/api/admin*` (route URLs *are* enumerable — only the
   inherited-hook coverage is not) and log them, so a new admin route is *visible* in deploy logs. This
   is observability, not enforcement — enforcement is the root hook, which needs no enumeration.

Honest scope (RA2-5, round 3): the root `onRequest` hook is structural for **all** `/api/admin*`
matched routes — children, siblings, and future — because the hook is on every context by construction,
not by detection. The only residual is by **definition of the plane**: a cross-tenant handler mounted
under a *different* prefix (e.g. `/api/internal/…`) is a different route outside this gate's scope and
would need its own gate — the same scope boundary every prefix-based gate has (§10 R13).

---

## 4. Decision + rationale (ADR-format → `docs/adr/ADR-admin-platform-authz.md`)

**Adopt Option D: Option B (`requirePlatformAdmin` per-request re-check) as the uniform gate on the
entire `/api/admin/*` plane, plus rate-limit + single-flight + audit on the two DR-drill endpoints.**

1. **One new authority, server-authoritative.** A `platform_admins` **non-tenant global** allowlist
   table (no RLS — RA2-3); a `requirePlatformAdmin` Fastify hook re-checks authority with a **plain
   indexed point-read** `SELECT 1 FROM platform_admins WHERE user_id = $1 AND revoked_at IS NULL`
   (§5/§7) and 403s otherwise. It is registered on the parent encapsulation plugin (§3.5, RESOLVE F1),
   **after** `verifyAuth` (RA2-1 — `request.user` must be populated before the gate dereferences
   `userId`), and the three child route files **drop their own `verifyAuth` AND `requireRole(['owner'])`
   hooks** — the parent gate fully replaces both, so the plane is gated once, structurally.
   **Canonical principal field is `request.user.userId`** (RESOLVE F9 — `requireLocationAccess` and
   membership checks all key on `userId`; owner mint sets `sub == userId`, documented coupling). The
   JWT model is **unchanged** (no 4th role, no mint site). The prior round's `SECURITY DEFINER`
   `is_platform_admin()` is **deleted** (RA2-3 showed it only relocated the BYPASSRLS dependency to the
   function owner; a non-RLS table removes it entirely).
2. **The admin plane is the platform plane — uniformly platform-admin-only.** All 6 endpoints are
   cross-tenant/ops operations; we **do not branch tenant-vs-platform scoping inside admin handlers**
   (in-handler authZ branching is the BOLA-prone anti-pattern we are removing). Owner self-views, if
   product wants them, live on the **owner plane** (`/api/owner/*`) with `withTenant`/membership
   scoping — designed as a deferred seam (§3 non-goal), not built in v1.
3. **DR-drill hardening (endpoints 2 & 3):** platform-admin **AND** a per-actor rate-limit **AND** a
   global single-flight guard (at most one restore-drill in flight) **AND** an audit row. Closes the
   weaponization (unbounded concurrent restore-drills = resource exhaustion).
4. **Audit:** every platform-admin action writes a `platform_admin_audit_log` row with `actor_id`,
   `action`, `target`, hashed ip/ua — mirroring `courier_audit_log`
   (`migrations/1780421034567_courier-audit-log.ts`).
5. **Provisioning is ops-script-only** (no API write path) — §8.

**Rationale.** B dominates A on the only axis that matters at the top privilege tier — *immediate
insider-removal* — while avoiding the discriminatedUnion ripple and the self-serve-escalation surface
of a new mint site. C alone is over-engineered for 1–5 admins and loses per-actor audit. The BOE
proves B's one-cost objection (per-request DB read) is a non-issue at admin-plane volume, and is
*consistent* with ADR-0004 (which rejected the same read only on the 150 req/s hot path). Boring,
proven, minimal: one table + one hook + one audit table.

### Per-endpoint scoping decision

| # | Endpoint | Decision | Notes |
|---|----------|----------|-------|
| 1 | `GET /backups` | platform-admin-only | `backup_metadata` is system-level (no tenant column); fleet inventory is inherently platform-scoped. |
| 2 | `POST /backups/verify` | platform-admin-only **+ Zod uuid backupId + rate-limit + single-flight + audit** | DR-drill weaponization closed. |
| 3 | `GET /backups/dr-report` | platform-admin-only **+ rate-limit + single-flight + audit** | Heavy fleet-wide drill. |
| 4 | `GET /fallback/health` | platform-admin-only | Fleet view. Owner self-view (own location only) = **deferred** `/api/owner/fallback/health`, membership-scoped — not built v1. |
| 5 | `POST /fallback/r2-check` | platform-admin-only | Platform coverage metric. |
| 6 | `GET /notification-audit` | platform-admin-only | Real path `/api/admin/notification-audit` (double-prefix bug fixed, F4). Cross-tenant release-gate tool. Owner self-view = **deferred** `/api/owner/notification-audit`, `locationId` REQUIRED + membership-checked. |

We **deliberately do not blanket-deny owners a self-view forever** — but we refuse to serve it by
weakening the admin gate. The owner-scoped variants are a clean owner-plane seam (designed, deferred),
keeping the admin gate single-purpose and BOLA-proof.

---

## 5. Data / migrations (forward-only, atomic, non-tenant global tables, integer)

One forward-only, atomic migration (sketch — actual code in the fix-PR, not this artifact). Timestamp
must exceed the current max (`1790000000028`), e.g. `1790000000030_platform-admins-and-audit.ts`.

**RA2-3 (round 2) — the crux, SIMPLIFIED.** `platform_admins` is a **non-tenant GLOBAL table**: it
holds only platform-admin `user_id`s, with **no tenant column and no tenant data**. It is therefore
**NOT RLS-enabled at all** — there is no tenant predicate to enforce and no per-tenant row to hide.
Protection is **table GRANTs + the application gate**, exactly the model the codebase already uses for
non-tenant tables. The re-check becomes a **plain indexed point-read** that returns identical results
under BYPASSRLS (today) and NOBYPASSRLS (post-B3): with no RLS on the table, a NOBYPASSRLS role with
`GRANT SELECT` reads every row, and a BYPASSRLS role reads every row — genuinely B3-independent, with
**no GUC, no `is_platform_admin` DEFINER fn, no FORCE-RLS, no `pa_self_read`, no `pa_audit_read`**.
This dissolves RA2-3 (the relocated owner-BYPASSRLS dependency is gone — there is no DEFINER owner to
depend on) and F10/R8's `USING(true)` concern (there is no RLS policy to misconfigure; audit-read
least-privilege is now a plain `GRANT`).

> **Honest divergence from the cited precedent.** The conductor's steer pointed at
> `users`/`ops_worker_heartbeat`/`auth_refresh_tokens` as "intentionally NOT FORCE-RLS". Verified
> against live source (`migrations/1780421100065_lockdown-nontenant-api-surface.ts:26-31`), those
> tables are in fact `ENABLE`+`FORCE ROW LEVEL SECURITY` **with no policies**, read via a **BYPASSRLS**
> pool — which is itself the RA2-3 trap and is only safe because they are read on the session pool that
> B3 does **not** flip. Our admin re-check runs on the **operational** pool that B3 **does** flip to
> NOBYPASSRLS, so we deliberately do the opposite: **no RLS at all** + `GRANT SELECT`. This is strictly
> more B3-safe than the lockdown pattern, and is the correct posture for a non-tenant table read on the
> NOBYPASSRLS pool. (Residual, accepted: a no-RLS table trips Supabase linter 0013 — a cosmetic
> advisory, not a security gap, since the lockdown's `ALTER DEFAULT PRIVILEGES`/`REVOKE USAGE` already
> bar `anon`/`authenticated` from the table. See §10 R12.)

```sql
-- platform_admins: the allowlist. Non-tenant GLOBAL table — NO RLS.
-- 'active' is expressed as revoked_at IS NULL (no separate status column needed).
CREATE TABLE platform_admins (
  user_id     uuid PRIMARY KEY REFERENCES users(id),
  granted_by  uuid,                       -- another platform_admin's user_id, or NULL for bootstrap
  granted_at  timestamptz NOT NULL DEFAULT now(),
  revoked_at  timestamptz                 -- NULL = active; non-NULL = revoked
);
-- Re-check is a PK lookup; a partial index serves the active predicate.
CREATE INDEX platform_admins_active_idx ON platform_admins(user_id) WHERE revoked_at IS NULL;

-- platform_admin_audit_log: append-only actor trail (mirror courier_audit_log). Non-tenant — NO RLS.
-- `status` supports the WRITE-AHEAD INTENT pattern (RESOLVE F5 / RA2-4): a 'started' row is committed
-- in its OWN short tx BEFORE any destructive drill, then UPDATEd to 'completed'/'failed' — so no
-- side-effect can occur without a pre-committed trail. Read-only endpoints write a single 'completed' row.
CREATE TABLE platform_admin_audit_log (
  id              bigserial PRIMARY KEY,
  actor_id        uuid NOT NULL,          -- the platform-admin user_id
  action          text NOT NULL,          -- 'backups.list' | 'backups.verify' | 'dr_report' | ...
  target          text,                   -- e.g. the backupId for a verify
  status          text NOT NULL DEFAULT 'completed'
                    CHECK (status IN ('started','completed','failed')),
  metadata        jsonb NOT NULL DEFAULT '{}'::jsonb,
  ip_hash         text,
  user_agent_hash text,
  created_at      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX platform_admin_audit_actor_idx ON platform_admin_audit_log(actor_id, created_at DESC);

-- ── Privilege posture: table GRANTs, NOT RLS (RA2-3) ──
-- Neither table is RLS-enabled. Reachability is controlled purely by GRANTs; the lockdown migration
-- already revokes anon/authenticated/service_role and schema USAGE for the Data API perimeter.
REVOKE ALL   ON TABLE platform_admins          FROM PUBLIC;
REVOKE ALL   ON TABLE platform_admin_audit_log FROM PUBLIC;

-- Operational role (NOBYPASSRLS, deliveryos_operational_user): read the allowlist, append+read audit.
-- NOTE: the allowlist gets SELECT ONLY — no INSERT/UPDATE/DELETE → self-serve escalation is structurally
-- impossible from any API code path (writes require DB creds via the ops CLI, §8). The operational
-- role's default-privilege auto-grant (1790000000015) already gives SELECT on future tables; the
-- explicit grants below are belt-and-suspenders and make the audit-log INSERT explicit.
GRANT SELECT          ON TABLE platform_admins          TO deliveryos_operational_user;
GRANT SELECT, INSERT  ON TABLE platform_admin_audit_log TO deliveryos_operational_user;
GRANT USAGE, SELECT   ON SEQUENCE platform_admin_audit_log_id_seq TO deliveryos_operational_user;
```

The re-check the `requirePlatformAdmin` hook runs is the plain point-read (no DEFINER fn):

```sql
SELECT 1 FROM platform_admins WHERE user_id = $1 AND revoked_at IS NULL;
```

Notes:
- **No money fields** here → integer-money rule N/A.
- **Forward-only / atomic:** single migration, `down()` drops both tables (additive, reversible).
- **B3-independent by construction (RA2-3):** with no RLS on `platform_admins`, the read returns the
  same rows whether the pool role is BYPASSRLS or NOBYPASSRLS — it never depends on `app.user_id`, on
  any policy, or on a DEFINER owner's role attribute. The admin handlers never set `app.user_id`
  (verified: `backups.ts:34,40`, `fallback.ts:14,47`, `notification-audit.ts:43` all query the raw
  pool) and they no longer need to. The prior round's DEFINER fn is **deleted** — it only relocated
  the BYPASSRLS dependency to the function owner (RA2-3); a non-RLS table removes it outright.
- **Bootstrap is decoupled from the migration** (RESOLVE F7): the migration creates tables + GRANTs
  ONLY; it never reads `PLATFORM_ADMIN_BOOTSTRAP_USER_ID` and never INSERTs an FK-bearing row, so it
  cannot FK-fail and FATAL the deploy. Seeding is the ops CLI run after deploy (§8).

---

## 6. Consistency + idempotency

- **Authority read is point-in-time authoritative** per request (immediate revoke; same model as
  ADR-0004 `status='active'`). No caching of platform-admin status (cold path — caching would add a
  staleness window for zero perf benefit; ponytail says don't).
- **DR-drill single-flight — ONE lock, ONE owner (Postgres, not Redis; RESOLVE F2 / RA2-2).** There is
  exactly **ONE** lock owner on key 3 = the existing internal `acquireLock`/`releaseLock` inside
  `runRestoreVerify` (live: `acquireLock(pool, BACKUP_VERIFY_LOCK)` at `backup-verify.ts:259`,
  `releaseLock(pool, BACKUP_VERIFY_LOCK)` in `finally` at `:360`). The route handler does **NOT** take
  its own lock — the prior round's proposed route-layer lock is **deleted** (RA2-2 showed it
  self-collides: the route would hold key 3, then the inner `acquireLock(3)` on a different session
  returns false → `runRestoreVerify` short-circuits at `:261` with `'Another verify in progress'` and
  the drill **never runs** — and `runRestoreVerify(pool, …)` takes a `Pool`, not a client, so passing a
  single `client` would break `acquireLock`'s `pool.connect()` at `:63`).
  **The LEAK in the existing internal lock is fixed in place** (`acquireLock`/`releaseLock`,
  `backup-verify.ts:62-79`): today it `pool.connect()` → `pg_try_advisory_lock(3)` → `client.release()`
  in `finally`, returning the connection to the pool **while the session-level lock is still held** →
  the lock leaks forever (every later drill 409s) and `releaseLock`'s separate `pool.connect()` unlocks
  a *different* pooled session → no-op. Fix: acquire the lock on a **dedicated client**, **hold that
  SAME client** across the whole drill, `pg_advisory_unlock(3)` on that same client, **then** release
  it. `acquireLock` returns the locked client (or null) instead of releasing it; `runRestoreVerify`
  threads that client through and `releaseLock(client)` unlocks-then-releases:
  ```
  // backup-verify.ts (sketch — reconciles the Pool-vs-PoolClient signature; ONE owner)
  async function acquireLock(pool: Pool, lockId): Promise<PoolClient | null> {
    const client = await pool.connect();
    const { rows } = await client.query('SELECT pg_try_advisory_lock($1) AS locked', [lockId]);
    if (!rows[0].locked) { client.release(); return null; }   // route → 409 drill_in_progress
    return client;                                             // HOLD this client for the drill
  }
  async function releaseLock(client: PoolClient, lockId): Promise<void> {
    try { await client.query('SELECT pg_advisory_unlock($1)', [lockId]); }
    finally { client.release(); }                             // unlock BEFORE release, SAME session
  }
  // runRestoreVerify keeps its Pool signature; takes the lock once, threads the client, releases in finally.
  ```
  Crash safety is now real: if the process dies the backend session ends and PG releases the session
  lock. `pg_advisory_xact_lock` (tx-scoped auto-release) was rejected because the drill runs up to
  ~30 min and a 30-min open transaction means idle-in-tx bloat; single-flight already bounds the held
  client to exactly one slot. `backupId` is Zod-`uuid`-validated and recorded as the audit `target`.
- **Audit is WRITE-AHEAD in its OWN short tx (RESOLVE F5 / RA2-4).** The `platform_admin_audit_log`
  intent row is **NOT** wrapped around the 30-min drill (RA2-4: "same tx as the lock" was a phantom —
  the lock is a session-level advisory lock, not a transaction; wrapping the drill in one tx would
  either un-commit the `started` row until the end (defeating write-ahead) or hold a 30-min
  idle-in-tx — the exact bloat `pg_advisory_xact_lock` was rejected to avoid). Correct shape, three
  steps: **tx1** = `INSERT … status='started'` + **COMMIT** (durable BEFORE any side-effect) → run
  `runRestoreVerify` (no enclosing tx) → **tx2** = `UPDATE … status='completed'|'failed'`. A crash
  mid-drill leaves a committed `started` row (trail without a silent side-effect). Read-only endpoints
  write one `completed` row. Append-only otherwise; at-least-once duplicates remain harmless (R5).

---

## 7. Failures + degradation (every external call: timeout + fallback, zero cascade)

- **Fail-closed is a WIRING property, not an assertion (RESOLVE F6 / round 3).** The structural
  authority is the **root-instance `onRequest` hook** (§3.5, RESOLVE round 3): for an admin-matched path
  it runs `verifyAuth` (so `request.user` is populated, 401 short-circuiting a no-token request) then
  the gate. The encapsulated parent `onRequest` hook (organizational primary for the 3 known routes)
  runs the same `verifyAuth` THEN `requirePlatformAdmin` ordering (RA2-1). Both layers are
  structurally `const ok = await isPlatformAdmin(req.user.userId); if (!ok) return
  reply.code(403).send(envelope('forbidden'));` where `isPlatformAdmin` runs the plain point-read — and
  `reply`-and-`return` short-circuits the request *before any child handler runs*, so a handler-level
  `try/catch` that swallows (the in-tree precedent at `notification-audit.ts:42-47`, which today even
  leaks `err.message`) **cannot admit**: the handler is never reached unless the gate already passed.
  Any throw inside the hook (timeout / pool exhaustion / DB blip) is caught → **503 `admin_unavailable`**,
  deny — never fail-open (highest privilege tier; a fail-open here is total platform compromise). The
  `await` is load-bearing and is enforced by the wired integration test (DoD #4b) that mounts the real
  parent plugin with a stub `isPlatformAdmin` that throws and asserts 503 + handler-not-invoked.
  Separately, the `notification-audit.ts:46` `err.message` schema leak is replaced with a generic 500
  envelope. A short `statement_timeout` on the re-check bounds the blip. **Zero cascade:** the failure
  is confined to the admin plane; owner/courier/customer planes are untouched.
- **Rate-limit / advisory-lock store unavailable for DR endpoints:** the advisory lock lives in the
  same Postgres; if Postgres is down the route already 503s at the re-check. If the rate-limiter
  (in-process token bucket) is somehow unavailable → fail-closed (deny the drill). A destructive op
  defaults to deny on uncertainty.
- **B3 ordering independence (the cross-cutting concern):**
  - The **authorization gate** is an explicit application predicate evaluated *before* any data query:
    the plain point-read on the **non-tenant, no-RLS** `platform_admins` table (RA2-3). Because the
    table carries no RLS, the read returns the same rows whether the admin pool is **BYPASSRLS**
    (today) or **NOBYPASSRLS** (post-B3) — it never depends on `app.user_id`, on a policy, or on a
    DEFINER owner's role attribute. An owner is 403'd at the hook and never reaches a query. The prior
    round's `is_platform_admin()` DEFINER fn is deleted: RA2-3 proved it merely **relocated** the
    BYPASSRLS dependency to the function owner (under FORCE-RLS + a non-BYPASSRLS owner it would have
    returned false for every caller from day 1 — the live behavior of `memberships` FORCE-RLS +
    NOBYPASSRLS operational role). A no-RLS table removes the dependency outright. **The fix can ship
    before, with, or after B3.**
  - **Separate concern (data-access mechanism, not authZ):** several platform endpoints are
    *intentionally cross-tenant* (`fallback/health` reads all `locations`; `r2-check` counts all).
    Today they work because the pool is BYPASSRLS. **When B3 flips admin to NOBYPASSRLS + FORCE RLS,
    these cross-tenant platform reads must run via an explicit platform-read path** (a SECURITY-DEFINER
    function or a dedicated platform-read role) — *not* by relying on BYPASSRLS. `backup_metadata`
    already has a system policy (`migrations/1780421100050_backup-system-policy.ts`), so backups
    reads survive; the `locations`-spanning reads need the platform-read path. **This is a coordinated
    dependency with B3, owned jointly (Architect + B3 owner), and is orthogonal to the authZ gate
    shipping.** Flagged in §10.

---

## 8. Security + tenant-isolation

- **No self-serve escalation (structural).** `platform_admins` is granted **SELECT only** to the
  operational role (§5) → no API request can INSERT/UPDATE/DELETE the allowlist (no write GRANT, and
  the table is not RLS-gated either way). An owner literally cannot grant themselves platform-admin
  through any code path that touches the operational pool. The only writer is the DB-creds CLI
  (provisioning script, §8), which is not reachable from the API.
- **Provisioning — bootstrap is decoupled from the migration (RESOLVE F7).** The migration must NOT
  seed from env: `platform_admins.user_id REFERENCES users(id)`, so an env UUID not yet in `users` →
  FK violation → migration fails → boot-guard **FATAL-exits the deploy** (crash-loop), and an
  unset/empty env → 0 admins anyway. Decision recorded: **the migration creates tables + the DEFINER
  fn ONLY; it never reads `PLATFORM_ADMIN_BOOTSTRAP_USER_ID` and never INSERTs an FK-bearing row.**
  - **Bootstrap:** run the ops CLI `scripts/platform-admin-grant.ts <userId>` **after** deploy with
    **DB credentials**. It (a) verifies the user row exists first (clean error, never an FK crash),
    (b) is idempotent (`INSERT ... ON CONFLICT (user_id) DO UPDATE SET revoked_at = NULL`), (c) records
    `granted_by`. Provision **≥2 at bootstrap** (bus-factor, R3).
  - **0-admins is a SAFE, recoverable state:** an empty allowlist means the plane 403s everyone
    (fail-closed — correct), recoverable at any time via the same DB-creds break-glass CLI. This is
    strictly better than a FATAL crash-loop: **warn-and-leave-empty beats FATAL-on-missing-env.**
  - **Subsequent grants/revokes:** the same ops CLI (`--revoke` → `revoked_at = now()`). v1 has **no API
    write path at all** — the simplest possible zero-escalation surface. (A future, tightly-guarded
    `requirePlatformAdmin`-gated + audited grant endpoint is explicitly deferred.)
  - **Runbook line (insider-removal exercised, per Counsel):** de-provisioning a departed founder
    runs `--revoke` and is part of the offboarding checklist — the ops script is exercised, not just
    written (the insider-removal story is the whole reason B was chosen over A).
- **Token model unchanged** → no new forgeable claim, no new mint site, no discriminatedUnion change
  on the AUTH red-line.
- **Tenant-isolation:** the admin plane is platform-scoped; non-admins get **403** (the resource is
  platform-level, so 403 is correct — we are not leaking another *tenant's* existence). The deferred
  owner-plane self-views use **404 on cross-tenant** (mirroring `requireLocationAccess`,
  `auth.ts:129/137/153`) since there leaking tenant existence matters.
- **PII:** audit rows carry `actor_id` + **hashed** ip/ua only (mirror `courier_audit_log`) — no raw
  PII. Endpoint 4's cross-tenant phone exposure is closed by the gate (only platform-admin, who is
  ops, sees fleet phones — an accepted ops capability, audited).
- **Audit read isolation (RESOLVE F10, ACCEPT-RISK R8 — now via GRANTs, not RLS).** With no RLS on
  `platform_admin_audit_log` (RA2-3), there is **no `pa_audit_read USING(true)` policy to
  misconfigure** — read access is a plain `GRANT SELECT` to the operational role, and which admin may
  read is decided at the **app gate** (`requirePlatformAdmin`). Any platform-admin can read every
  other admin's trail; this is the same accepted posture as before (no per-admin row isolation),
  accepted at N=1–5. RLS was never the watcher-of-watchers here — the **out-of-band append-only audit
  mirror** is (the ETHICAL-STOP-1 legibility floor, deferred to the named trigger — §"Ethics/
  legibility" below and R8).
- **DR-drill `backupId`** is Zod-`uuid`-validated (closes the unvalidated-body issue at `backups.ts:70`).

---

## 9. Operability

- **Provision / rotate:** ops script with DB creds (§8). Rotate-out = `--revoke` → sets `revoked_at =
  now()` → **denied at next request-entry** (the point-read's `revoked_at IS NULL` predicate fails; no
  token-lifetime wait). **Scope of "immediate" (RESOLVE F8,
  ACCEPT-RISK R7):** revocation takes effect at request *entry*; an already-in-flight drill
  (`runRestoreVerify`, `TIMEOUT_MS` ≈ 30 min) runs to completion. Bounded by the timeout, only the two
  long endpoints, accepted. Rotate-in = grant a new user_id.
- **Health — degraded vs down:** the admin plane health is *isolated*. If the re-check DB read fails,
  the admin plane is **down (503)** while owner/courier/customer planes stay **up** — the existing
  `/healthz` degraded-vs-down signal should attribute admin-plane failure separately, not flip global
  health.
- **Observability (< 1 min):** every platform-admin action emits (a) a `platform_admin_audit_log` row
  and (b) a structured log line `{actor_id, action, target}`. Both queryable within < 1 min. A 403
  spike on `/api/admin/*` is the BOLA-attempt signal to alert on.
- **Rollback:** the guard is additive. The migration is forward-only with a `down()` that drops both
  tables (no DEFINER fn exists anymore — RA2-3) (reversible). The safe rollback is *never* reverting to
  `requireRole(['owner'])` (that is the vulnerability).
- **Structural coverage = root `onRequest` hook, NOT a boot-guard (RA2-5, RESOLVE round 3).** Coverage
  for siblings/future routes is enforced **at request time** by the root-instance `onRequest` hook
  (§3.5): every request whose matched route pattern (`request.routeOptions.url`) is under `/api/admin`
  is gated by construction — children, siblings, and future routes alike. The round-2 route-tree
  boot-guard is **deleted**: Fastify's introspection surface cannot see context-inherited hooks (an
  inherited-gate child is indistinguishable from an ungated sibling — both report `onRequest: []`), so
  the boot-guard was non-functional. **Replacement (operability only):** an optional **boot-time
  visibility log** enumerates all `/api/admin*` route *patterns* (these ARE enumerable) so a newly
  added admin route shows up in deploy logs — observability, not authority. Enforcement is the root
  hook, which requires no enumeration to be correct.
- **Kill-switch granularity (RESOLVE E2 — Counsel non-blocking #1).** A blunt `ADMIN_PLANE_ENABLED=
  false` would darken the **recovery tools** (`GET /backups`, `dr-report`) during the very incident
  you need them — wrong-way-round. **Decision:** there is NO off-switch on the authZ gate
  (`requirePlatformAdmin` always runs). The only kill-switch is **`ADMIN_DRILLS_ENABLED`** (default
  true) which scopes ONLY the two weaponizable, resource-heavy drills (`POST /backups/verify`,
  `GET /backups/dr-report`). Recovery reads (`GET /backups`, `GET /fallback/health`,
  `POST /fallback/r2-check`, `GET /notification-audit`) are **never** darkened — they are exactly what
  ops needs mid-incident.
- **Flag / scaling-gate:** `ADMIN_DRILLS_ENABLED` (default true) lets ops dark only the heavy drills
  in an incident without a redeploy; the gate and recovery reads stay up.

---

## 10. Open / accepted risks (with owner)

| ID | Risk | Disposition | Owner |
|----|------|-------------|-------|
| R1 | **B3 cross-tenant data path.** Post-B3 (NOBYPASSRLS+FORCE), `fallback/health` / `r2-check` cross-tenant `locations` reads break unless routed via a platform-read SECURITY-DEFINER/role. AuthZ gate is independent; **data path is a coordinated dependency.** | DEFER-coordinate with B3 (not a v1-gate blocker — admin pool is BYPASSRLS today). | Architect + B3 owner |
| R2 | **v1 blanket-locks the admin plane** → owners lose any (currently insecure) self-view of fallback/notification health. | ACCEPT — those views were never safely tenant-scoped; owner-plane self-views are a designed deferred seam. | Product + Architect |
| R3 | **Bootstrap bus-factor** — a single platform-admin could be locked out. | MITIGATE — provision ≥2 at bootstrap; documented in runbook. | Ops |
| R4 | **`app.user_id` GUC dependency** for `pa_self_read`. | **DISSOLVED (RA2-3)** — `platform_admins` is a non-tenant, no-RLS table read by a plain point-read. No GUC, no `pa_self_read`, no DEFINER fn — there is nothing to depend on the pool's RLS posture. (Supersedes the round-1 DEFINER-fn resolution, which RA2-3 showed only relocated the dependency.) | Architect |
| R5 | **Audit at-least-once** may duplicate rows on retry. | ACCEPT — harmless, rare, append-only. | Architect |
| R6 | **DR-drill single-flight is global** (one drill at a time across the fleet). | ACCEPT — drills are infrequent ops actions; serialization is desirable, prevents weaponization. | Ops |
| R7 | **In-flight drill survives a mid-flight revoke** (≤30 min, F8). | ACCEPT — bounded by `TIMEOUT_MS`; only the two long endpoints; "immediate" scoped to request-entry. | Ops |
| R8 | **Audit-read has no per-admin isolation** — any admin reads all admins' trail (F10). | ACCEPT at N=1–5; now via plain `GRANT` + app gate (no RLS policy — RA2-3), so nothing to misconfigure. Real mitigation = out-of-band audit mirror (E1 floor), deferred to trigger. | Architect |
| R9 | **Out-of-band audit mirror not built in v1** (Counsel #2 / E1 floor). | DEFER-FLAG — trigger: first non-founder ops hire OR tenant-count ≥ threshold. | Ops + Architect |
| R10 | **Option C (network-isolated ops service) not built** (Counsel steel-man). | DEFER-FLAG — scheduled as next hardening at headcount/tenant threshold; not dismissed. | Architect |
| R11 | **`requireLocationAccess` raw `pool.query` on memberships** (`auth.ts:148`) breaks under B3 — `memberships` is FORCE-RLS (`core-identity.ts:91-92`), the operational role is NOBYPASSRLS (`1790000000015`); the read works today ONLY because the live pool is still BYPASSRLS. When B3 flips it → 0 rows → **404 for every owner → fleet-wide owner-plane self-DoS.** | **DEFER OUT of B4 + record as a HARD BLOCKING dependency of B3 (RA2-6).** Not B4's code to fix, but B3 **cannot** flip the operational pool to NOBYPASSRLS without simultaneously making `requireLocationAccess` NOBYPASSRLS-safe (same pattern as B4's re-check). Written as a cross-finding so the B3 operator handoff inherits it — not a soft "tracked" residual. | Architect → B3 owner |
| R12 | **No-RLS tables trip Supabase linter 0013** (`platform_admins`, `platform_admin_audit_log`). | ACCEPT — cosmetic advisory, not a security gap: the lockdown migration's `ALTER DEFAULT PRIVILEGES` + `REVOKE USAGE ON SCHEMA public` already bar `anon`/`authenticated`/`service_role`; `REVOKE ALL FROM PUBLIC` + SELECT-only operational grant complete the perimeter. Enabling RLS to silence 0013 would re-introduce the RA2-3 NOBYPASSRLS-deny trap. Suppress the advisory in the linter config if noise matters. | Architect |
| R13 | **Gate closes the NAMED `/api/admin` plane, not "all cross-tenant handlers everywhere."** A cross-tenant handler mounted under a *different* prefix (e.g. `/api/internal/*`) is outside this gate's matched-path predicate and would need its own gate. | ACCEPT — the same scope boundary every prefix-based gate has; the root hook is structural for *all* `/api/admin*` matched routes (children/siblings/future) by construction. A new cross-tenant plane is a new design, not a silent escape from this one. The boot-time visibility log + eslint plane convention keep new admin surfaces visible. | Architect |
| E1 | **Tenant legibility of platform access** — self-watched audit, tenant-invisible, no appeal (Counsel ETHICAL-STOP-1). | **NEEDS HUMAN DECISION → STOP-ETHICS.** Gate ships regardless; one recorded human decision + date on the minimum legibility floor (recommended: out-of-band audit mirror). | Human (founder) |

---

## 11. Ethics / tenant legibility (Counsel ETHICAL-STOP-1 → STOP-ETHICS)

The platform-admin tier spans restaurants' fleet data (backup inventory, public phones, fallback
config, notification counts); the audit that legitimizes it is **self-watched** (operator = admin =
auditor = sole reader — read access is a plain `GRANT` gated only at the app layer, no row isolation,
RA2-3) and **invisible to the affected tenant** — no
notification, no appeal. Counsel grades this **friction, not veto** (the gate is a net narrowing of
who holds cross-tenant power, and the data is ops-metadata, not the deep-PII surveillance red-line).
The gate **ships regardless**; what is required is **one recorded human decision + a date** on the
minimum legibility floor.

I cannot make this ethical trade unilaterally — it is routed to **STOP-ETHICS**. To make ratification
cheap, the recommended floor is pre-staged:

- **Recommended floor (cheapest, highest leverage):** an append-only **out-of-band mirror** of
  `platform_admin_audit_log` to a sink the platform-admins cannot silently rewrite — so the watcher is
  watched by *something*. (Same lever as Counsel non-blocking #2 / R9.)
- **Recommended enact-trigger:** the FIRST of {first non-founder ops hire, tenant-count ≥ a threshold
  the human sets, first tenant data dispute, first acquirer due-diligence request}.
- **Recommended explicit deferrals (recorded as DECIDED, not defaulted):** tenant right-to-know
  channel and per-drill second-admin/break-glass confirmation deferred to a dated review; Option C
  (network-isolated ops service, R10) scheduled as the next hardening at the same threshold.

The recorded human decision unblocks the council; the v1 gate does not wait on it.

---

## DoD — red→green the fix-PR must satisfy

**Guardrail (mandatory):** a regression test proven **red→green** + a `docs/regressions/` ledger row.
**Structural authority (RA2-5, RESOLVE round 3) = a ROOT-instance `onRequest` hook** in `server.ts`
that, for any request whose matched route pattern (`request.routeOptions.url`) is `=== '/api/admin'` or
`startsWith('/api/admin/')`, runs `verifyAuth` then `requirePlatformAdmin`. NOT a route-tree boot-guard
(deleted — Fastify introspection cannot see context-inherited hooks, RA2-5 final-confirm). Proven by
DoD #4e (throwaway-sibling E2E).
**Boot-time visibility log (operability, not authority):** boot enumerates all `/api/admin*` route
patterns to deploy logs.
**Enforced (build-error, not optional) `tools/eslint-plugin-local` rule (RESOLVE F1 / RA2-1) — fast
tripwire, not the authority:** (a) no `fastify.register(..., {prefix:'/api/admin'})` outside
`routes/admin/index.ts`; (b) the root `onRequest` admin gate is present in `server.ts`; (c)
`routes/admin/index.ts` registers `verifyAuth` **then** `requirePlatformAdmin` as `onRequest` hooks
(order asserted).

**Unit (`requirePlatformAdmin` + Zod):**
1. RED→GREEN: non-allowlisted user (valid owner JWT) → **403**.
2. allowlisted (`revoked_at IS NULL`) → **pass**.
3. allowlisted then revoked (`revoked_at` set) → **403** (revoke at request-entry).
4. the re-check query throws/timeouts → **503 fail-closed** (asserts no fail-open).
4b. **WIRED fail-closed (RESOLVE F6):** mount the REAL parent plugin (`routes/admin/index.ts`) with a
   stub `isPlatformAdmin` that throws → assert **503** AND the child handler was **never invoked**
   (the swallow-precedent at `notification-audit.ts:42-47` cannot admit). Asserts the `await` is
   load-bearing.
4c. **Structural coverage — children (RESOLVE F1):** a route registered as a NEW child of
   `routes/admin/index.ts` with no per-file hook still returns **403** to an owner (coverage by
   construction for in-parent routes).
4d. **verifyAuth ordering (RA2-1):** mount the REAL parent plugin and hit a child with a VALID
   platform-admin JWT → **200** (proves `request.user` is populated by the parent `verifyAuth` BEFORE
   the gate dereferences `userId` — i.e. the gate does NOT 503 every caller on a null `request.user`);
   and a no-token request → **401** from `verifyAuth` (not a 503 from a null deref). Asserts the
   children no longer carry their own `verifyAuth`/`requireRole(['owner'])`.
4e. **Structural sibling closure (RA2-5, RESOLVE round 3 — REPLACES the unrealizable boot-guard
   item):** register a THROWAWAY route at `{prefix:'/api/admin'}` as a **sibling OUTSIDE** the parent
   encapsulation plugin (the exact computed-prefix shape the AST lint evades, e.g. directly in the test
   harness's server build) with **no per-route auth hook** → assert it returns **403** to an owner JWT
   (no platform-admin) and **401** to a no-token request, and a platform-admin gets through. Proves the
   **root `onRequest` hook gates siblings by construction** at request time — not by detection. Also
   assert `routeOptions.url` precision: a non-admin lookalike route `/api/administrators` is **NOT**
   gated (returns its normal response without the platform-admin check). (The round-2 "ungated sibling
   → FATAL boot" item is deleted: Fastify's introspection surface cannot distinguish an inherited-hook
   child from an ungated sibling, so a route-tree boot-guard is non-functional — RA2-5 final-confirm.)
5. `POST /backups/verify` with non-uuid `backupId` → **400/422** (Zod).
5b. **Single-flight correctness (RESOLVE F2):** run a drill, let it finish, run a SECOND drill →
   **acquires the lock and runs** (proves the lock was actually released — guards the leak-forever
   bug). Concurrent second drill → **409 `drill_in_progress`** (5d below).
5c. **Write-ahead audit (RESOLVE F5):** a `status='started'` row exists BEFORE `runRestoreVerify`
   completes (assert via a hook/spy or a committed-row read mid-drill); it transitions to
   `completed`/`failed`. A simulated crash after `started` leaves a `started` row (trail without a
   silent side-effect).

**E2E (Playwright against staging — Mandatory Proof Rule):**
6. **owner JWT → 403** on *each* of the 6 (REAL paths — note `notification-audit` is single-prefix
   after the F4 fix): `GET /api/admin/backups`, `POST /api/admin/backups/verify`,
   `GET /api/admin/backups/dr-report`, `GET /api/admin/fallback/health`,
   `POST /api/admin/fallback/r2-check`, `GET /api/admin/notification-audit`. (Assert a JSON **403**
   envelope, NOT a 200 `index.html` — guards against the route silently falling through to the SPA
   handler, the false-green the double-prefix bug caused.)
7. **platform-admin → 200** on each of the 6 (read endpoints) / accepted (drill endpoints).
8. **courier JWT and customer JWT → 401/403** on all 6.
9. **DR-drill rate-limit:** 2nd rapid `POST /backups/verify` within the window → **429**.
10. **DR-drill single-flight:** concurrent `dr-report`/`verify` → **409 `drill_in_progress`** on the
    second.
11. **Audit:** after a platform-admin action, a `platform_admin_audit_log` row exists with the correct
    `actor_id` and `action` (assert via an admin-gated read or a direct DB assertion in the test).
12. **(Deferred owner-plane, only when those routes are built):** owner sees **only their own
    location's** data on `/api/owner/fallback/health` and `/api/owner/notification-audit`; cross-tenant
    `locationId` → **404**.

A passing typecheck/build is **not** proof — paste the `playwright test --reporter=list` output and
the unit run.
