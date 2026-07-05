# ADR — Platform-admin principal for `/api/admin/*` (close BOLA / B4)

- Status: **ACCEPTED** — Triadic Council converged (architect ×3 / breaker ×3 / counsel ×2): findings 10→6→1→0, 0 unresolved CRITICAL/HIGH, structural sibling-closure empirically verified on Fastify 5.8.5, ETHICAL-STOP-1 ratified by operator 2026-06-29 (docs/design/admin-platform-authz/ethical-decisions.md). Code gate cleared. NO production code in this design change.
- Date: 2026-06-29
- Red-line: AUTH / TENANT-ISOLATION. Forward-only. Reversible (additive guard + drop-table `down()`).
- Relates: ADR-0004 (owner-token revocation — reuses the `status='active'` per-request insider-removal
  pattern), ADR-0003 (dev-kid fail-closed). Does **not** contradict either: ADR-0004 rejected a
  per-request DB re-check only on the **150 req/s owner hot path**; this re-check is on the **< 1 req/s
  admin cold path**, so the same mechanism is consistent here.
- Sibling: B3 (admin pool BYPASSRLS → NOBYPASSRLS) — the authZ gate is **ordering-independent** (see
  Consequences). **B3 inherits a HARD blocking dependency from this ADR (RA2-6 / R11):** it cannot flip
  the operational pool to NOBYPASSRLS without simultaneously fixing `requireLocationAccess`
  (`auth.ts:148`) to be NOBYPASSRLS-safe, or it self-DoSes every owner.
- Full design: `docs/design/admin-platform-authz/proposal.md`
- **Round 2 (2026-06-29):** RA2-3 simplification — `platform_admins` is a **non-tenant no-RLS table**
  (no DEFINER fn, no FORCE-RLS, no `pa_self_read`/`pa_audit_read`); RA2-1 verifyAuth ordering; RA2-5
  boot-guard; RA2-2 single lock owner; RA2-4 own-tx write-ahead; RA2-6 R11 → hard B3 blocker. See
  `docs/design/admin-platform-authz/resolution.md` §"Resolution round 2".
- **Round 3 (2026-06-29):** RA2-5 boot-guard **deleted as unrealizable** (Fastify introspection cannot
  see context-inherited `onRequest` hooks — verified empirically + `route.js:513/556`,
  `request.js:188/206`). Structural authority for siblings/future moves to a **root-instance
  `onRequest` hook** that gates any request whose **matched route pattern** (`request.routeOptions.url`,
  not raw URL) is under `/api/admin`. Encapsulated parent stays organizational; eslint stays tripwire;
  boot-guard → optional boot-time visibility log. See `resolution.md` §"Resolution round 3".

## Context

`/api/admin/*` (3 route files, 6 endpoints — `routes/admin/backups.ts`, `fallback.ts`,
`notification-audit.ts`; registered `server.ts:793-799`) performs **cross-tenant platform operations**
(fleet backups, DR drills, all-location fallback health incl. public phones, cross-tenant notification
audit) but is gated only by `requireRole(['owner'])` (`plugins/auth.ts:105-114`). Every tenant owner
passes that check; none of the handlers scope by `request.user`; reads run on the **BYPASSRLS** pool,
so RLS is not a backstop. This is **BOLA/BFLA** — a launch-blocker.

The role discriminatedUnion has exactly three roles — `owner`/`courier`/`customer`
(`shared-types/src/legacy.ts:163-174`). There is **no principal distinct from `owner`**; "admin" was
modeled as "an owner", collapsing the tenant plane onto the platform/ops plane.

## Decision

Introduce a **server-authoritative platform-admin principal** (not a JWT role) and gate the entire
`/api/admin/*` plane on it.

1. **`platform_admins` NON-TENANT, NO-RLS allowlist table** (RA2-3) + a **`requirePlatformAdmin`**
   Fastify hook that re-checks with a **plain indexed point-read** `SELECT 1 FROM platform_admins WHERE
   user_id = $1 AND revoked_at IS NULL` and 403s otherwise. **Structural gate (RESOLVE F1 / RA2-1):**
   the hook is registered on a single **parent encapsulation plugin** (`routes/admin/index.ts`) that is
   the only thing mounted at `prefix:/api/admin`, as the **second** `onRequest` hook **after
   `verifyAuth`** (RA2-1 — `request.user` must be populated before the gate reads `userId`); the three
   route files register as its children with **no prefix**, inherit both hooks, and **drop their own
   `verifyAuth`/`requireRole(['owner'])`**. **Coverage authority (RA2-5, RESOLVE round 3):** a
   **root-instance `onRequest` hook** in `server.ts` gates every request whose **matched route pattern**
   (`request.routeOptions.url`, predicate `=== '/api/admin' || startsWith('/api/admin/')` — NOT the raw
   URL, immune to case/`%2e`/`%2f`/trailing-slash tricks and false-positive on `/api/administrators`) is
   under `/api/admin` — running `verifyAuth` then `requirePlatformAdmin`. Because a root hook is in every
   child context by construction, it gates children, siblings, AND future routes with **zero detection**.
   The round-2 route-tree **boot-guard is deleted** (Fastify introspection cannot see context-inherited
   hooks → non-functional); replaced by an optional **boot-time visibility log** that enumerates
   `/api/admin*` route patterns (operability, not authority). An **enforced (build-error)** eslint rule
   is the fast tripwire. Canonical principal field =
   `request.user.userId` (RESOLVE F9). **No 4th JWT role, no new mint site, no discriminatedUnion
   change. No DEFINER fn** (RA2-3 — it only relocated the BYPASSRLS dependency to the function owner).
2. **Uniform platform-admin-only** on all 6 endpoints (they are all cross-tenant ops). **No
   tenant-vs-platform branching inside admin handlers** (that branching is the BOLA anti-pattern being
   removed). Owner self-views, if ever wanted, are a **deferred owner-plane seam** (`/api/owner/*`,
   `withTenant`/membership-scoped) — designed, not built (schema rich, runtime minimal). Real path of
   endpoint #6 is `/api/admin/notification-audit` (RESOLVE F4 — prior code double-prefixed it).
3. **DR-drill hardening** on `POST /backups/verify` + `GET /backups/dr-report`: platform-admin **AND**
   Zod-`uuid` `backupId` **AND** per-actor rate-limit **AND** a Postgres **single-flight via ONE
   advisory lock (key 3), ONE owner = the existing internal lock inside `runRestoreVerify`** (RESOLVE
   F2 / RA2-2). The leak is fixed **in place**: `acquireLock` returns the locked dedicated client,
   `runRestoreVerify` holds that SAME client across the drill, `releaseLock(client)` unlocks **then**
   releases (the prior code released the client while the session lock was held → permanent
   leak/self-DoS). The proposed extra route-layer lock is **deleted** (RA2-2: it would self-collide on
   key 3 and pass a client where a `Pool` is expected) → ≤ 1 drill in flight → 409 **AND** a
   **write-ahead** audit row.
4. **Audit:** `platform_admin_audit_log` (actor_id, action, target, **status**, hashed ip/ua), a
   non-tenant no-RLS table — mirrors `courier_audit_log`
   (`migrations/1780421034567_courier-audit-log.ts`). **Write-ahead intent (RESOLVE F5 / RA2-4):** a
   `status='started'` row is committed in its **OWN short tx** BEFORE the destructive `runRestoreVerify`
   (NOT a 30-min tx wrapping the drill — RA2-4: that would defeat write-ahead and re-introduce
   idle-in-tx bloat), then UPDATEd to `completed`/`failed` in a second short tx — no side-effect without
   a pre-committed trail.
5. **Provisioning is ops-script-only** (DB creds, not API). **Bootstrap is DECOUPLED from the migration
   (RESOLVE F7):** the migration creates tables + GRANTs ONLY and NEVER seeds an FK-bearing row
   from env (which would FK-fail → boot-guard FATAL-crash the deploy). Bootstrap ≥2 admins via the ops
   CLI run AFTER deploy (verifies the user exists, idempotent `ON CONFLICT`). 0-admins = SAFE
   fail-closed 403, recoverable by the DB-creds break-glass CLI (warn-and-empty beats FATAL).
   `platform_admins` is granted **SELECT only** to the operational role (no write GRANT) → self-serve
   escalation is structurally impossible.

**Enforcement lives** in the parent `requirePlatformAdmin` `onRequest` hook (per-request,
immediate-revoke) — an explicit application predicate (the plain point-read on the **non-tenant, no-RLS**
`platform_admins` table), evaluated *before* any data query and **independent of RLS / `app.user_id` /
pool BYPASSRLS posture** (RA2-3, B3-order-independent: with no RLS the read returns identical rows under
BYPASSRLS and NOBYPASSRLS). Affordable because the admin plane runs at < 1 req/s (~10 checkouts/min —
BOE in the proposal §2), unlike the owner hot path of ADR-0004.

**Fail-closed is a WIRING property (RESOLVE F6):** the parent `onRequest` hook replies-and-returns on
deny (handler never reached) and catches any throw → **503**, deny. Never fail-open (top privilege
tier). Proven by a wired integration test, not asserted.

**Flag:** `ADMIN_DRILLS_ENABLED` (default true) kill-switch scopes ONLY the two heavy/weaponizable
drills (RESOLVE E2) — the authZ gate has no off-switch and the recovery reads (`backups` list,
`fallback/health`) are never darkened during an incident.

## Alternatives considered

- **A — 4th JWT role `platform_admin` (claim-based authZ):** REJECTED. Bakes authority into a 24h
  token → insider-removal regresses to a token-lifetime window at the highest privilege tier (the gap
  ADR-0004 fought for owners); requires a mint site (self-serve-escalation surface); ripples the
  red-line discriminatedUnion across every `.strict()` consumer. No upside over B except reusing
  `requireRole`.
- **C — network-isolated internal ops service (mTLS / ops-secret):** REJECTED as primary. Over-
  engineered for 1–5 admins; a shared secret loses per-actor audit and per-human insider-removal. Kept
  as a **future defense-in-depth** layer (network segmentation), not v1.
- **"Do nothing":** rejected — this is a launch-blocking BOLA red-line.

## Consequences

- + Cross-tenant admin exposure closed **structurally** (parent encapsulation for children + **root
  `onRequest` hook for siblings/future**, F1/RA2-1/RA2-5 round 3): non-platform-admins (incl. every
  owner) → 403 **before** any data query — independent of pool RLS posture (non-tenant no-RLS table,
  RA2-3), and a future admin route (in-parent OR sibling) cannot escape the gate because the root hook
  is on every route context by construction and keys on the matched route pattern. Residual by
  definition: a cross-tenant handler under a *different* prefix is a different plane needing its own
  gate (R13).
- + **Immediate revocation** of a platform-admin (`status='revoked'` → next request denied), no
  token-lifetime window — consistent with ADR-0004's enforcement philosophy.
- + **No new forgeable claim, no new mint site, no discriminatedUnion change** on the AUTH red-line.
- + **B3 ordering-independent:** the authZ gate holds under BYPASSRLS (today) and NOBYPASSRLS
  (post-B3). **Separate, coordinated dependency:** the *intentionally cross-tenant* reads
  (`fallback/health`, `r2-check` over all `locations`) must, once B3 lands, run via an explicit
  platform-read SECURITY-DEFINER/role rather than relying on BYPASSRLS (`backup_metadata` already has
  a system policy). Orthogonal to this gate shipping. Owner: Architect + B3 owner.
- + DR-drill weaponization closed (rate-limit + single-flight + audit + uuid validation).
- − Per-request indexed DB read on the admin plane (≤ ~10 checkouts/min — negligible; no new pool).
- − v1 blanket-locks owners out of any (currently insecure) self-view; owner-plane self-views deferred.
- − Forward-only migration (2 tables); reversible via `down()` drop.

## Open items / human decisions (post-RESOLVE)

- **R1 (Architect + B3 owner):** platform-read mechanism for cross-tenant `locations` reads under
  post-B3 FORCE RLS (SECURITY-DEFINER fn vs platform-read role). Not a v1 blocker.
- **R3 (Ops):** provision ≥2 bootstrap admins (bus-factor); exercise the `--revoke` offboarding path.
- **R4: DISSOLVED (RA2-3)** — `platform_admins` is a non-tenant, no-RLS table read by a plain
  point-read; no GUC, no `pa_self_read`, no DEFINER fn to depend on the pool's RLS posture. (Supersedes
  the round-1 DEFINER-fn resolution, which RA2-3 showed only relocated the dependency to the fn owner.)
- **R8/R9 (Architect + Ops):** audit-read has no per-admin isolation (now via plain `GRANT` + app gate,
  no RLS policy — RA2-3); mitigation = out-of-band append-only audit mirror, DEFERRED to trigger (first
  non-founder ops hire / tenant threshold).
- **R10 (Architect):** Option C (network-isolated ops service) scheduled as next hardening at a
  headcount/tenant threshold — not dismissed.
- **R11 → HARD BLOCKING DEPENDENCY OF B3 (RA2-6, Architect → B3 owner):** `requireLocationAccess` raw
  `pool.query` on `memberships` (`auth.ts:148`) works today ONLY because the pool is BYPASSRLS;
  `memberships` is FORCE-RLS (`core-identity.ts:91-92`) + the operational role is NOBYPASSRLS
  (`1790000000015`). **B3 cannot flip the operational pool to NOBYPASSRLS without simultaneously making
  `auth.ts:148` NOBYPASSRLS-safe** (same pattern as B4's re-check) — otherwise 0 rows → 404 for every
  owner → fleet-wide owner-plane self-DoS. Recorded as a cross-finding the B3 operator handoff inherits,
  **not** a soft "tracked" residual.
- **R12 (Architect):** no-RLS tables trip Supabase linter 0013 — cosmetic advisory (perimeter is closed
  by the lockdown GRANT revokes); enabling RLS to silence it would re-introduce the RA2-3 trap.
- **ETHICAL-STOP-1 → STOP-ETHICS (Human/founder):** tenant legibility of platform access — one recorded
  human decision + date on the minimum legibility floor (recommended: out-of-band audit mirror).
  **Friction, not veto: the gate ships regardless.** See proposal §11.
- **Proof (Mandatory Proof Rule):** unit (owner→403 / active(`revoked_at IS NULL`)→200 / revoked→403 /
  re-check-throw→503 wired / structural-coverage-child→403 / **verifyAuth-ordering: valid admin→200,
  no-token→401** / **structural sibling closure: a throwaway ungated `{prefix:'/api/admin'}` sibling
  registered OUTSIDE the parent → 403 to owner / 401 no-token / platform-admin passes; `/api/administrators`
  lookalike NOT gated** (RA2-5 round 3 — replaces the deleted boot-guard FATAL item) / non-uuid→400 /
  single-flight re-acquire / write-ahead `started` row in its own committed tx) + Playwright on staging (owner→403 ×6
  asserting JSON-403-not-SPA-200, platform-admin→200 ×6, courier/customer→401/403, drill 429 + 409,
  audit row with actor_id). Red→green AUTH regression + ledger row + root `onRequest` admin gate +
  boot-time visibility log + enforced eslint gate.
  See proposal §DoD.
