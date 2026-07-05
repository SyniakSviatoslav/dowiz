# B3 — Deep Auth Hardening (the red-line layer beneath the shipped Tier-1 batch)

Status: **DESIGN / PROPOSAL — REVISED after Council round 1 (see `resolution.md`).**
(No production code, no migration files placed by this doc.)
Date: 2026-07-03. Author: System Architect. Direction to be gated by system-breaker + counsel.

> **RESOLUTION HEADLINE (round 1, option b).** NOBYPASSRLS-on-prod is **not safely shippable yet.**
> The breaker proved three in-code facts that break Option B (C1 role-grant chain → login lockout;
> C2 `is_local=false` on a transaction-mode pool; H2 money-writes key on `app.current_tenant` which
> `withTenant` never sets) plus H3 (~123 raw DML paths vs ~116 `withTenant` — half the surface outside
> the seam). **The enforcement flip is DEFERRED** behind a pre-flip enumeration + proof program
> (`resolution.md` §4). What ships now = **Phase-0**: decoupled credential rotation (incl. the postgres
> SUPERUSER — ES-1), C2 fix, JWT-off-WS-URL, two-mode W1 boot-guard, guardrail locks, in-memory
> rate-limit kept (pg store deferred — M5). Sections below are annotated `[REVISED r1]` where the
> resolution changed them; read `resolution.md` for the authoritative per-finding disposition.
Red-line globs touched by the *implementation* of this design: auth / money / RLS /
`packages/db/migrations/**` → every migration + role/credential step is **operator-gated** (protect-paths).

Inputs / prior art (do not restate, build on):
- `docs/design/pg-privilege-hardening/remediation-plan.md` (Phase 1–4 NOBYPASSRLS plan; **already staged**)
- `packages/db/migrations/1790000000077_rls-nobypassrls-phase1-policies.ts` (Phase-1 policies, **already in-tree, dark/inert**)
- `docs/adr/ADR-pg-privilege-hardening.md` (direction approved)
- `docs/design/ci-pre-prod-verification/proposal.md` (§P2 secret-store fragmentation → unification)
- MEMORY: `secrets-exposure-incident-2026-07-03`, `pg-privilege-hardening`, `prod-outage-schema-drift-2026-06-20`
- Shipped Tier-1 (live on prod, **not re-litigated here**): orders IDOR, WS owner revocation, spa-proxy recheck,
  courier-invite predicate, customer identity, rate-limit real-IP, JWT alg/kid double-pin.

Verified seams (source, 2026-07-03):
- `packages/db/src/index.ts` — `createOperationalPool()` boot-guard rejects only `current_user='postgres'`;
  it does **not** assert `NOT rolbypassrls`. Runtime role = whatever `DATABASE_URL_OPERATIONAL` logs in as.
- `packages/platform/src/auth/tenant.ts` — `withTenant(pool,userId,fn)` = `BEGIN; set_config('app.user_id',…,true); … ; COMMIT`. Sets **only** `app.user_id`. Courier/webhook paths set `app.current_tenant` themselves.
- `apps/api/src/plugins/auth.ts` — `verifyAuth` (Bearer header only), `requireRole`, `requireLocationAccess`
  (live membership read). **No central owner→tenant preHandler exists.** `getOwnerLocationId`
  (`apps/api/src/lib/get-owner-location.ts`) reads `memberships` on the **raw pool, pre-`withTenant`** (RC3 hazard).
- `apps/api/src/websocket.ts` — WS **dual-accepts** `?token=` (URL, L338–355) and `{type:'auth',token}` (message).
  FE: `apps/api/src/client/status/ws.ts` uses **URL** `?token=`; `apps/web/src/lib/useWebSocket.ts` uses **message** auth.
- `apps/api/src/lib/resilience/rate-limit.ts` — token buckets in a **process-local `Map`** (`buckets`, `inflightCounts`). Per-instance. `AUTH_OPTS` exists.
- `packages/platform/src/auth/jwt.ts` — RS256-only, kid-select + alg double-check, dev-kid segregated (ADR-0003).
  Access `24h`, customer `7d`; refresh 7d w/ rotation + reuse-detection (family delete) + live-membership re-derive.
- argon2: `argon2id`, memoryCost 65536, timeCost 3, parallelism 4 (`courier/auth.ts`, `courier/me.ts`) — OWASP-solid.
- `packages/db/migrations/1780348982031_telegram_connect_tokens.ts` — base schema `owner_id uuid NOT NULL`.
  **Drift claim to reconcile:** prod keys this table on `user_id` (a rename migration applied on staging, skipped on prod).

---

## 1. Problem + non-goals

**Problem.** App-layer authz is now strong (Tier-1 shipped), but it is the *only* layer. The runtime DB
role effectively bypasses RLS (`dowiz_app`/`deliveryos_api_user` are BYPASSRLS-class; the boot-guard only
blocks literal `postgres`). A single missed `WHERE location_id = …` — one IDOR the app layer forgets —
is a cross-tenant read with **no second net**. B3 is the defense-in-depth layer: make the runtime pool
RLS-**enforced**, structurally shrink the IDOR surface, remove credential/transport leak vectors, and
close the brute-force gap across Fly machines. Plus: a leaked operational credential is still unrotated,
and a prod↔staging schema drift makes the staged RLS policies unsafe to run on prod as-written.

**Non-goals.**
- Re-doing Tier-1 (already live). No changes to JWT alg/kid pinning, orders IDOR fix, WS revocation guards.
- Zero-cookie posture is **kept** — this design introduces **no cookies**; the "cookie flags / CSRF"
  scope item resolves to "N/A — bearer-only, documented" (see §8), not "add cookies".
- Not building a new secrets manager. Secret-store unification is deferred to the ci-pre-prod P2 track;
  here we only *rotate + re-role* and *name the canonical store*.
- Not moving money math, not changing integer-money, not touching Supavisor topology.

---

## 2. Back-of-envelope (scale, growth, connection budget)

**Scale target (design horizon, not today).** Today: MVP, ~tens of tenants (demos + a few live).
Design for: **100 locations**, busiest 20% doing lunch+dinner rush. Per-location peak ≈ **5 orders/min**;
assume ~20 locations concurrently in rush → **~100 orders/min system-wide peak** (~1.7/s). Owner dashboards
poll/stream on top; couriers stream position. Growth: assume 3× in 12 months → ~300 orders/min → still < 10/s.
This is a **small** system. Boring wins; do not shard, do not add Redis "for scale" — add it only if a
failure mode (§5 shared limiter) demands it.

**Connection budget (the real constraint under NOBYPASSRLS).** RLS enforcement adds a `set_config` GUC
(already present in `withTenant`) and — under the recommended rollout (§3 Option B) — a `SET LOCAL ROLE`
per owner transaction. Both are txn-local, so each enforced owner request **pins one server connection for
the duration of its transaction**. Budget, per Fly API machine:

| Consumer | Pool | max conns/machine | Notes |
|---|---|---|---|
| API operational (hot path) | `createOperationalPool` | `OPERATIONAL_POOL_SIZE` (default **20**) | via Supavisor **transaction mode** `:6543` (multiplexes to fewer PG backends) |
| API session (rare: SET/DDL) | `createSessionPool` | **3** | `:5432` session mode |
| Workers (per worker process) | session pool | **3** | per-location loops, `set_config('app.current_tenant',…,true)` |
| Analytics / exports | transient | ~2 | JSON/JSONL owner export path |
| Migrations | transient (release_command) | 1 | forward-only, run on boot |

With 2 API machines: `2 × 20 = 40` operational client sessions to Supavisor + `2 × 3` session + workers.
At 100 orders/min + owner streams, expected **concurrent** enforced txns are ~20–50 system-wide — inside
`40` operational slots **only if txns stay short**. **Scaling-gate (§9):** `withTenant` wraps each owner
request in an explicit `BEGIN…COMMIT`; enforcement makes these txns strictly longer-lived than a single
autocommit statement. The connection-pinning cost is the item to watch. Gate: alert if operational-pool
`connectionTimeoutMillis` (5s) breaches rise post-flip → raise `OPERATIONAL_POOL_SIZE` or add a machine
**before** widening enforcement to anon/courier read paths. This is the exact class that caused the
2026-06-20 pool-starvation outage — treat pool headroom as a launch-gate, not an afterthought.

**Rate-limiter store sizing (§5).** Auth-sensitive limits only (login, OTP, refresh, claim, courier-activate).
At 100 locations, auth attempts are ~single-digit/s even under attack throttling. A **pg-backed** counter
(one upsert per attempt on an existing operational connection) is ~< 200 writes/min steady — trivial for
Postgres. No Redis needed for this volume; pg is the boring choice that adds zero new infra.

---

## 3. NOBYPASSRLS rollout — ≥2 options with tradeoffs (the big one, HIGH-RISK)

Goal: the runtime operational pool executes as a **NOBYPASSRLS** role, so every tenant table's `FORCE ROW
LEVEL SECURITY` policy is actually consulted beneath the app-layer WHERE clauses. Phase-1 policies already
exist and are **inert** under today's bypass (permissive policies OR-combine → only admit rows → adding
them cannot deny anything while the role bypasses). The decision here is **how enforcement is switched on**.

### Option A — "Big-bang role flip" (the currently-staged plan)
Concept: **single global cutover.** After Phase-1 policies + Phase-2 worker GUCs are live-dark, run one
migration `ALTER ROLE <op-role> NOBYPASSRLS`. From that instant every connection in the pool enforces RLS.
- **Pros:** simplest; one probe (`rolbypassrls` → 0 rows); matches the existing plan; no per-txn cost;
  fully "boring".
- **Cons:** **blast radius = the whole fleet, atomically.** Any un-enumerated path silently returns 0 rows
  post-flip → a partial outage that looks like "empty data" not an error (the hardest class to detect fast).
  Rollback is `ALTER ROLE … BYPASSRLS`, which *reopens B3 entirely* — an all-or-nothing lever. No canary.

### Option B — "Txn-scoped role-switch ratchet" (**RECOMMENDED**)
Concept: **progressive, reversible enablement without changing the login role.** Keep the login role as-is
(BYPASSRLS) but create a second NOBYPASSRLS role `dowiz_app_rls` and `GRANT dowiz_app_rls TO <login-role>`.
Inside `withTenant` (and the courier/webhook context setters), when a flag is on, prepend
`SET LOCAL ROLE dowiz_app_rls` inside the existing `BEGIN…COMMIT`. Because `SET LOCAL ROLE` is txn-scoped
and reset at COMMIT, enforcement can be turned on **per path, behind a flag**, and reverts instantly by
flag flip — no migration. Converge to Option A (flip the login role, drop the SET ROLE) only after 100% of
paths have soaked green under enforcement.
- **Pros:** **canary-able and reversible.** Enable owner paths (highest IDOR value) first, then courier,
  then anon reads — each behind `RLS_ENFORCE_<lane>` flags. A regression reverts by flag (seconds), never a
  migration. "Schema rich, runtime minimal" applied to enforcement itself: the policies are dark, the
  role exists dark, enforcement is a runtime toggle. Un-migrated worker paths keep functioning under the
  login role's bypass until each is wrapped — no forced simultaneity.
- **Cons:** adds `SET LOCAL ROLE` (one cheap statement) per enforced txn; requires the login role to be a
  member of `dowiz_app_rls` (a grant, benign); enforcement is **partial** until every path is wrapped — so
  defense-in-depth is incomplete during the ramp (honest: the un-wrapped path is exactly as (in)secure as
  today, no worse). Two role names to reason about. Raw-pool anon reads (not via `withTenant`) need an
  explicit wrap or a `SET LOCAL ROLE` helper before they enforce.

### Option C — "Shadow parallel-pool canary"
Concept: **a second connection pool** bound to a NOBYPASSRLS role, with a fraction of read-only/idempotent
endpoints (or one canary Fly machine) routed through it behind a flag.
- **Pros:** real production traffic validates enforcement with a bounded blast radius; the primary pool is
  untouched until confidence is high.
- **Cons:** **doubles the connection budget** (two pools competing for Supavisor slots — directly fights §2)
  and adds routing complexity + a second failure surface. Over-engineered for a small system. Rejected as
  primary; kept only as a fallback if Option B's per-txn role-switch proves problematic under load.

**Decision: Option B (txn-scoped role-switch ratchet), converging to Option A once soaked.** It is the most
reversible (flag, not migration), gives a genuine canary lane order (owner → courier → anon), and does not
double the connection budget. Concept named: **enforcement-as-runtime-flag over a dark NOBYPASSRLS role.**

> **[REVISED r1 — the flip is DEFERRED; the role model is corrected but not yet flip-safe.]**
> The breaker (C1) proved Option B as written locks everyone out: the enforcement role `dowiz_app_rls`
> is a *different* role from the policy-target `dowiz_app`, and the required grant does not exist (§3 even
> stated it backwards). **Corrected role model (proven on staging before any flip):** create
> `dowiz_app_rls NOLOGIN NOBYPASSRLS` **and `GRANT dowiz_app TO dowiz_app_rls` (membership)** — Postgres
> applies a `TO dowiz_app` policy to any role that is a *member* of `dowiz_app`, and membership inherits
> grants but **not** the `BYPASSRLS` attribute, so `dowiz_app_rls` both sees the RC2/RC4/RC6 policies and
> stays enforcing. This membership grant also fixes M2 (it inherits `dowiz_app`'s *post-080* grant set
> exactly — no blanket `GRANT ALL`, so TRUNCATE / platform_admins-write stay revoked). Correcting the
> role model does **not** make the flip shippable: it is still blocked by C2, H2 (money-writes on
> `app.current_tenant`), H3 (~123 raw DML paths outside the seam), M3 (DEFINER-fn owner), M4 (drift claim
> unverified), and ES-2 (silent-denial detection). See `resolution.md` §3–§4 for the gating program.

**C1 anti-orphan decision (fail-CLOSED for tenant rows).** A tenant-table row with `NULL`
owner/tenant/`location_id` does **not** match any location-scoped policy → it is **denied** (fail closed).
This is correct and intended. Pre-flip **audit requirement**: scan every FORCE-RLS tenant table for
`location_id IS NULL` (or null tenant key) rows; any legitimate orphan must be back-filled or explicitly
policy-covered **before** its lane is enforced, or it vanishes. Auth/global tables (`users`,
`auth_refresh_tokens`) are **role-restricted, not row-restricted** (RC2: `TO dowiz_app USING(true)`), so
they stay reachable — this is the deliberate carve-out that prevents the fail-closed rule from becoming a
**login lockout** (see §7).

**Schema-drift blocker (prod ≠ staging) — must resolve before Phase-1 runs on prod.** The Phase-1 migration
`1790000000077` re-keys `telegram_connect_tokens` with `owner_id = app_current_user()`. Base schema is
`owner_id`; **prod is claimed to key on `user_id`** (a rename migration applied on staging, skipped on prod).
If prod's column is `user_id`, `CREATE POLICY … USING (owner_id = …)` **throws at migrate time → the whole
prod deploy fails** (the 2026-06-20 schema-drift outage class). **Required forward-only pre-step:**
1. Introspect each environment: `SELECT column_name FROM information_schema.columns WHERE table_name='telegram_connect_tokens' AND column_name IN ('owner_id','user_id');`
2. Add a forward-only, idempotent **reconciliation migration** that converges prod to the canonical column
   *before* the policy migration: `ALTER TABLE telegram_connect_tokens RENAME COLUMN user_id TO owner_id`
   guarded so it is a no-op where already `owner_id` (check `IF EXISTS`/catalog before rename; node-pg-migrate
   `sql` with a `DO $$ … IF EXISTS … THEN … END $$` block).
3. Only then does `1790000000077` (which references `owner_id`) run safely on both environments.
Do **not** write the RLS policy against a column that may not exist on the target. Policies are written
against **prod's actual, reconciled schema** — verified by introspection, not by reading the base migration.

---

## 4. Recommendation & the other five items (each: priority · flag · named rollback · phase)

| # | Item | Priority | Feature flag | Named rollback | Phase |
|---|---|---|---|---|---|
| 1 | NOBYPASSRLS enforcement (Option B) | **HIGH-RISK** | `RLS_ENFORCE_OWNER` / `_COURIER` / `_ANON` (default off) | flip flag off (seconds); role/policies stay dark | after §6 wins, staged canary |
| 2 | JWT off WS URL | **NEEDS-STAGING-REHEARSAL** (client migration) | `WS_URL_TOKEN_ACCEPT` (default **on** until usage→0, then off) | re-enable flag; revert FE `status/ws.ts` build | after FE migrated + usage→0 |
| 3 | Central owner→tenant preHandler (C1) | **NEEDS-STAGING-REHEARSAL** | `CENTRAL_TENANT_PREHANDLER` (default off) | flag off → per-route checks remain (they are **kept**, not deleted) | incremental route adoption |
| 4 | **[REVISED r1]** Rotate credential (**decoupled → incident response**) + re-role (stays B3) | **HIGH-RISK** (live cred) | n/a (secret + role) | new clean role → cut over → disable leaked role (two distinct identities, NOT one-role dual-password) | **standalone, do FIRST, human-go recorded separately** |
| 5 | **[REVISED r1]** ~~Shared rate-limit store~~ → **DEFERRED**; keep in-memory | n/a (deferred) | `RATE_LIMIT_STORE` stays `memory` | in-memory `Map` (accept 2× fleet budget) | deferred (M5: pg store = DoS amplifier) |
| 6 | Auth-surface SAFE-NOW wins | **SAFE-NOW** | per-item (see §6) | per-item revert (each independent, deterministic) | first |

**Sequencing rationale.** Do the **SAFE-NOW** wins (§6) and the **credential rotation** (#4) first — they
are deterministic and reduce standing risk (a leaked cred in git is live exposure *right now*). Then the
structural preHandler (#3, shrinks IDOR surface *before* we rely on RLS). Then the shared limiter (#5).
The NOBYPASSRLS ramp (#1) is last and slowest because it has the widest blast radius; it rides on top of
everything else being solid. WS URL removal (#2) is gated purely on client-usage telemetry reaching zero.

---

## 5. Data / migrations (forward-only, atomic, RLS FORCE, integer)

All migrations forward-only, idempotent (`DROP … IF EXISTS`/`CREATE`, `DO $$ IF … $$`), no `down` behavior
change, integer-money untouched, RLS `ENABLE`+`FORCE` already present on every named table.

1. **Drift reconciliation** (before Phase-1, §3): idempotent `telegram_connect_tokens` column convergence.
   Verified by introspection on each env in the pre-flight (ci-pre-prod P1 preflight is the natural home).
2. **[REVISED r1] NOBYPASSRLS role** (Option B): `CREATE ROLE dowiz_app_rls NOLOGIN NOBYPASSRLS;` +
   **`GRANT dowiz_app TO dowiz_app_rls;` (role membership — NOT a blanket grant-mirror).** Membership makes
   `dowiz_app_rls` inherit `dowiz_app`'s *post-080* grant set exactly (M2: no re-opened TRUNCATE /
   platform_admins-write) and makes the `TO dowiz_app` policies apply to it (C1: no login lockout), while
   `BYPASSRLS` is a role attribute and is **not** inherited, so `dowiz_app_rls` stays enforcing. Prove on
   staging by impersonation (`SET ROLE dowiz_app_rls` + GUC → read `users` → expect rows). Still verify
   grants explicitly (remediation R-d: a *missing* grant is a hard deny under enforcement). This role is
   **dark** until a flag turns on the `SET LOCAL ROLE`.
   **DEFINER-fn ownership (M3):** `app_owner_location` (RC3) and the ~19 Phase-2 sweep fns must be owned by
   a **bypass-class** role (`dowiz_definer NOLOGIN BYPASSRLS`, or keep owned by `postgres`) so they keep
   bypassing after convergence flips `dowiz_app` to NOBYPASSRLS — else owner-resolve and worker sweeps
   silently return 0. Assert owner-is-bypass in `verify:rls`.
3. **Phase-1 policies** — already in-tree (`1790000000077`); keep, but re-verify each predicate against
   **prod's** reconciled schema (§3). RC2 stays `TO dowiz_app` — retarget the role name to whichever role
   the flip actually lands on (`dowiz_app` login role, since `SET LOCAL ROLE dowiz_app_rls` runs as that
   role's grantee — confirm `pg_has_role` mapping in the staging pre-flight).
4. **Phase-2 worker GUCs** — per the remediation plan §Phase-2 inventory (~15–17 workers): each sets
   `set_config('app.current_tenant',…,true)` (or `app.user_id`) inside its txn, per-location loop for
   cross-tenant sweeps. Never grant a worker BYPASSRLS as a shortcut. Ship dark, proven before any flip.
5. **Convergence to Option A** (post-soak): `ALTER ROLE <login-role> NOBYPASSRLS` + remove the
   `SET LOCAL ROLE` and the `WHERE`-nothing grant of the RLS role — a later, separate, operator-gated step.
6. **[REVISED r1 — M5] Rate-limit pg store: DEFERRED.** A per-attempt upsert on a single `(key,
   window_start)` hot row serializes under row-lock → self-inflicted pool starvation (the 2026-06-20 class),
   and degrade-to-in-memory removes cross-instance limiting exactly during DB stress → the control becomes a
   DoS amplifier. Boring wins: **keep the process-local `Map` limiter.** Accepted risk: at the current fleet
   (2 machines) the worst case is `N_machines × per-instance budget` = **2×** the auth budget — bounded,
   revisit only on material fleet growth or an observed distributed brute-force, and then with a *sloppy*
   per-instance-batched aggregate, not a synchronous per-attempt upsert.

7. **[REVISED r1 — C2] Provisioning-path context fix (Phase-0):** `onboarding.ts:75` +
   `spa-proxy.ts:771` use `set_config('app.user_id',…,false)` (session-scope) on a **transaction-mode**
   (`:6543`) client — the same backend is not guaranteed across autocommit statements → onboarding breaks
   or a stale GUC leaks cross-tenant under enforcement. Convert to an explicit `BEGIN…COMMIT` with
   `is_local=true` (or route through the canonical seam). Inert-neutral today (BYPASSRLS); a hard pre-flip
   prerequisite → ships in Phase-0 with its own proof.

**Idempotency of the operations themselves:** the role flip, grants, and `ALTER ROLE` are all naturally
idempotent.
**[REVISED r1 — H1]** The "overlap window (both passwords valid)" is **impossible for a single PG role** and
would keep the leaked credential exploitable for the whole window (contradicting the rotation's goal). The
correct mechanism is **two distinct role identities**: stand up a new clean role, cut the app over, then
disable/drop the leaked role. For the **postgres SUPERUSER** password (runtime does not use it) there is no
overlap need — rotate immediately + coordinated redeploy. Rotation is **decoupled from B3** (ES-1) → see
`resolution.md` §3 / P0-1.

**[REVISED r1 — M4] Drift reconciliation is introspection-FIRST.** The claim "prod keys
`telegram_connect_tokens` on `user_id`" has **no in-tree artifact** (`grep RENAME COLUMN` = 0) and §3 states
the drift direction inconsistently → treat it as **UNVERIFIED**. Step 1 is to introspect BOTH environments
and record the real column state; write no rename against a claim. The migration must be re-run-safe across
all three states: only `owner_id` → no-op; only `user_id` → rename; **both columns present** → a rename
*fails* (cannot rename into an existing column) → handle explicitly (operator-gated data merge + drop-stale,
never auto-rename).

---

## 6. Auth-surface SAFE-NOW wins (deterministic, ship first, separate from the risky items)

These are quick, high-certainty, independently-revertible. Each needs a red→green guardrail per the harness.

| Win | Current | Change | Proof |
|---|---|---|---|
| **W1 · Pool BYPASSRLS boot-guard [REVISED r1 — M1]** | boot-guard only rejects `postgres` | **two-mode:** during ramp the login role stays BYPASSRLS, so `current_user`-based checks are dead — instead assert the *enforcement* role via catalog: `SELECT rolbypassrls FROM pg_roles WHERE rolname='dowiz_app_rls'` must be false + the membership grant exists (gated on role existence → no-op until created). Post-convergence, assert `current_user` (now `dowiz_app`, NOBYPASSRLS). | unit test: guarded reject when the enforcement role bypasses |
| **W2 · argon2 params confirmed** | argon2id 64MB/t3/p4 | **N/A — already OWASP-solid**; add a guardrail test pinning the params so a future edit can't silently weaken them | test asserts `memoryCost>=65536,timeCost>=3` |
| **W3 · Refresh rotation confirmed** | rotation + reuse-detection + live re-derive present | **N/A — already correct**; add regression test for reuse-detection family-delete + the ≤24h access bound | existing/added E2E on `/auth/refresh` reuse |
| **W4 · Cookie flags / CSRF** | bearer-only, **zero cookies** | **N/A — documented**: no cookies → no CSRF surface on state-changing owner routes (they require `Authorization: Bearer`, not ambient credentials). Guardrail: a test/grep gate that fails if any route ever sets a cookie without an ADR | grep-gate `Set-Cookie` absent |
| **W5 · JWT kid rotation runbook** | single kid, RS256, dev-kid segregated | design-only: document a **two-kid overlap** rotation procedure (publish new kid in verifier's accept-set, sign with new, retire old after max-token-TTL 7d) — no code now, a runbook + the verifier already selects by kid | runbook doc; verifier already multi-kid capable |
| **W6 · Access-token TTL review** | access 24h, customer 7d, refresh 7d | **keep** 24h access (ADR-0004 blast-radius bound) + refresh rotation. Customer 7d is a bearer held client-side — acceptable (scoped to one order tuple, no PII). Flag only: consider shortening customer token to order-lifetime + short grace | ADR cross-ref; no change unless council wants it |

W1 is the one with teeth (it makes a future accidental re-grant of BYPASSRLS fail the boot); W2–W6 are
mostly "confirm + lock with a guardrail so it can't regress." These ship **before** any RLS ramp.

---

## 7. Failure & degradation (every external/DB call: timeout + fallback; no cascade; fail-closed vs lockout)

**The central tension: fail-CLOSED (correct for tenant data) must not become LOCKOUT (fatal for auth).**

| Scenario | Behavior | Design |
|---|---|---|
| Enforced path forgets its GUC/role → 0 rows | **fail closed** for tenant data (correct); for **auth tables** the RC2 role-restriction keeps them readable → **no login lockout** | RC2 `USING(true) TO dowiz_app` is the lockout firebreak; auth reads never depend on a tenant GUC |
| A whole lane regresses under enforcement | **flag off** (Option B) reverts in seconds — no migration, no redeploy required if flag is env/remote-config | `RLS_ENFORCE_<lane>` default off; ramp one lane at a time |
| Anti-orphan: tenant row with NULL location_id | **denied** (fail closed) | pre-flip audit back-fills/covers legit orphans; documented accepted deny for the rest |
| `getOwnerLocationId` pre-context read returns 0 under RLS | resolved by RC3 `app_owner_location()` DEFINER fn (pinned search_path) — **not** a broad membership read policy | RC3 fn already designed; adopt in the preHandler (§3 item) |
| Shared rate-limit store (pg) unavailable | **degrade to per-instance in-memory limiter** (still limits, just not cross-instance) — do **not** fail-open globally, do **not** fail-closed on auth (that = lockout) | `RATE_LIMIT_STORE` read with a try/catch fallback to the existing `Map` limiter; log + alert on fallback |
| WS URL-token removed but a cached PWA still sends `?token=` | with `WS_URL_TOKEN_ACCEPT` off → clean auth-timeout close (1008), FE reconnect falls back to message auth after redeploy | gate removal on telemetry `logTokenDeprecation` count → 0; keep dual-accept until then |
| Credential rotation mid-flight | **dual-valid overlap window**: create new password/role, deploy secret, verify, retire old — a redeploy during the window works with either | overlap = no cascade; rollback = repoint secret |
| Pool starvation from longer enforced txns | operational-pool `connectionTimeoutMillis` (5s) breach → 503 (existing degradation) + scaling-gate alert | §9 gate raises pool size / machine count before the next lane |

No new external network dependency is introduced except the (pg-backed, same-DB) shared limiter — which has
an explicit in-memory fallback, so it cannot cascade into an auth outage.

---

## 8. Security & tenant isolation

- **Defense-in-depth achieved:** post-ramp, a forgotten app-layer `WHERE location_id` no longer leaks
  cross-tenant — RLS denies at the row layer. This is the whole point of B3.
- **Tenant carve-outs are honest:** `users`/`auth_refresh_tokens` are global/pre-auth → **role-restricted**
  (`TO dowiz_app`), not row-restricted. That is a deliberate, documented reduction (protection = only the op
  role can reach them, non-tenant API surface already locked down in `1780421100065`), and it is what keeps
  fail-closed from becoming lockout. Council must sign this (remediation R-b).
- **Money red-line (RC4):** courier/webhook writes to `orders`/`courier_cash_ledger`/`delivery_trace` admit
  `app.current_tenant` via command-split policies (SELECT/UPDATE on orders; INSERT/SELECT on ledger/trace).
  This equals the courier's existing app-layer reach — defense-in-depth, not a widening. Council sign (R-c).
- **[REVISED r1 — H2] Money-writes are outside `withTenant`.** RC4 keys on `app.current_tenant`, but
  `withTenant` sets only `app.user_id`; the courier/webhook money paths set `app.current_tenant` on a **raw**
  client with no role switch. Fix: a single canonical seam `withTenantContext(pool,{userId?,tenantId?})`
  that opens the txn, applies the enforcement role, sets the correct GUC `is_local=true`, and for money
  tables **asserts `rowcount>0` → else throw** (a silent 0-row money write is a red-line failure, never a
  no-op). The money lane may not flip until every money path is on this seam. **DEFERRED.**
- **[REVISED r1 — H3] Half the DML surface is outside the seam.** ~123 raw `.db.query`/`pool.query` DML
  call-sites vs ~116 `withTenant` — webhooks, workers, funnel, courier, spa-proxy. Defense-in-depth is only
  real where the seam runs; the flip is blocked until a **complete checked-in enumeration** classifies each
  raw path as migrated-to-seam or explicitly-quarantined, plus a CI gate against new out-of-seam DML on
  FORCE-RLS tables. This enumeration is **MISSING** — it is the gating pre-flip work.
- **Transport leak closed (#2):** URL `?token=` leaks via access logs / Referer / SW caches. Moving WS auth
  to message/subprotocol-only removes a standing credential-in-logs vector. Telemetry-gated removal.
- **Zero cookies kept** → no CSRF surface (W4). No PII in tokens (confirmed: customer token carries no phone).
- **Credential hygiene (#4):** the leaked `deliveryos_api_user` password is live exposure. Rotate now; decide
  whether to promote `dowiz_app` to the sole LOGIN+NOBYPASSRLS operational role (staging's model) and retire
  `deliveryos_api_user` entirely — the cleaner end-state. Tie the canonical secret to the ci-pre-prod P2
  single-source-of-truth (Fly as SoT). No secret ever in git; the incident scrub is a hard precondition.
- **Central preHandler (#3)** shrinks the IDOR surface *structurally*: one audited resolve-and-assert of
  owner→location scoping (reusing RC3 `app_owner_location()`), replacing N scattered per-route reads.
  Per-route checks are **kept as belt-and-suspenders** during adoption (flag-gated), removed per-route only
  after each is proven covered — never a silent big-bang deletion of existing guards.

---

## 9. Operability

- **Health: degraded vs down.** Add an RLS-enforcement probe to the health surface / `verify:rls`: assert
  the *intended* role attribute for the current ramp state (pre-flip: bypass expected; post-flip: NOT bypass
  expected). A mismatch is **degraded**, not down. Distinguish DB-down (503) from RLS-misconfig (alert).
- **Observability < 1 min.** Emit a metric/log on the two anomaly classes that a silent RLS regression
  produces: (a) a spike in **0-row** responses on enforced owner/courier endpoints (the "empty data" tell),
  and (b) any `set_config`/`SET ROLE` failure in `withTenant`. Both must page within a minute — the
  2026-06-20 outage was slow to spot precisely because "empty" looks benign. Log the flag state on boot.
- **Rollback (named, per §4 table).** Every risky item reverts by **flag** (Option B RLS lanes, WS URL,
  preHandler) or **secret repoint** (credential) — none require an emergency migration **during the ramp**.
  **[REVISED r1 — L1] Honest end-state caveat:** the converged Option-A end-state (`ALTER ROLE dowiz_app
  NOBYPASSRLS` + removing `SET LOCAL ROLE`) is **not** flag-reversible — its rollback is another migration,
  not a flag. "Every risky item reverts by flag" scopes to the ramp only; convergence deliberately trades
  flag-reversibility for simplicity and happens only after a full soak + proof.
- **[REVISED r1 — ES-2] Silent-denial gates on any prod lane flip (hard preconditions, human-set):**
  (a) 0-row anomaly metric + per-lane flag-revert **proven live on prod**; (b) NULL-key orphan audit re-run
  **at the flip** + a **continuous** NULL-keyed-insert gate throughout the ramp (a snapshot cannot guard a
  live system); (c) a fail-closed tenant denial is **user-distinguishable** — the owner sees "temporarily
  unavailable / contact support", never a silent empty list that reads as "you have no orders". No prod flip
  in Phase-0.
- **Scaling-gate.** Do not widen the RLS ramp to the next lane while operational-pool `connectionTimeout`
  breaches are elevated (§2). Raise `OPERATIONAL_POOL_SIZE` or add a machine first. This gate is the direct
  learning from the pool-starvation outage.
- **Flag inventory** is explicit (§4) and defaults are conservative (all risky flags default **off**;
  `WS_URL_TOKEN_ACCEPT` defaults **on** to avoid locking out cached clients; `RATE_LIMIT_STORE` defaults
  `memory`). Dark-deploy to verify is fine; launching each is a separate explicit act (ship discipline).

---

## 10. Risks — open & accepted (with owner)

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R-1 | An un-enumerated FORCE-RLS path 0-rows silently after its lane flips | **accept w/ nets:** lifecycle E2E + per-tenant inverse worker tests + 0-row anomaly metric (§9) + flag revert. Option B's per-lane ramp bounds blast radius vs big-bang | Architect + DB owner |
| R-2 | `USING(true)` on `users`/`auth_refresh_tokens` (RC2) is role-restriction only | **NEEDS COUNCIL sign-off**; tighten to `TO dowiz_app` (already done in-tree) — accept as the lockout firebreak | Council + DB owner |
| R-3 | RC4 admits `app.current_tenant` writes to money tables | **NEEDS COUNCIL sign-off** (money red-line); command-split already narrows it; equals existing courier reach | Council + Architect |
| R-4 | Prod↔staging drift (`telegram_connect_tokens`) breaks Phase-1 on prod | **BLOCKER — must fix first:** introspect + forward-only reconciliation migration (§3) before Phase-1 runs on prod | DB owner |
| R-5 | Grant gaps: a *missing* grant on `dowiz_app_rls` becomes a hard deny under enforcement | pre-flip grant-mirror + explicit grant verification (§5.2, remediation R-d) | DB owner |
| R-6 | Pool starvation from longer enforced txns | scaling-gate (§9) + `connectionTimeout` alerting; ramp gated on headroom | Architect |
| R-7 | Leaked `deliveryos_api_user` credential remains live until rotated | **do #4 early**; gate open-source flip on rotation+scrub (existing incident memory) | Operator + Architect |
| R-8 | Shared limiter (pg) failure locks out auth if it fails-closed | mitigated: **degrade to in-memory**, never fail-closed on auth (§7) | Architect |
| R-9 | WS URL removal locks out cached PWA/SW clients | mitigated: telemetry-gated removal, dual-accept until usage→0 (#2) | Architect |
| R-10 | Central preHandler adoption misses a route → false sense of coverage | keep per-route checks during adoption; per-route removal only after proof; flag-gated | Architect |
| R-11 | Customer 7d bearer token lifetime | **accepted** (scoped to one order tuple, no PII); optional shortening flagged, not required | Architect |

**[REVISED r1] Council-round-1 additions (see `resolution.md` for full disposition):**

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R-12 | C1 role-grant chain → login lockout | **FIXED (design):** membership `GRANT dowiz_app TO dowiz_app_rls`; prove on staging; flip still DEFERRED | Architect + DB owner |
| R-13 | C2 `is_local=false` on txn-mode pool | **FIX Phase-0:** explicit-txn `is_local=true` on the two provisioning paths | Architect |
| R-14 | H1 impossible single-role overlap | **FIXED:** two role identities; superuser rotate-now; folded into decoupled rotation | Operator + Architect |
| R-15 | H2 money-writes outside `withTenant` | **FIX (design) + DEFER:** canonical seam + rowcount>0 guard; money lane deferred | Architect + Council |
| R-16 | H3 ~123 raw DML paths outside seam | **DEFER-FLAG (MISSING):** full enumeration + CI gate before flip | Architect + DB owner |
| R-17 | M3 DEFINER-fn owner unpinned | **FIX (design) + DEFER:** own via bypass-class role; assert in `verify:rls` | DB owner |
| R-18 | M4 drift claim unverified | **FIX:** introspection-first; handle both-columns state; claim marked UNVERIFIED | DB owner |
| R-19 | M5 pg limiter = DoS amplifier | **DEFER pg store + ACCEPT 2× fleet budget** at 2 machines | Architect |
| R-20 | L2 WS-URL removal has no forcing fn | **FIX:** dated deadline + migrate FE `client/status/ws.ts` + close socket on failed verify | Architect |
| R-21 | ES-1 leaked SUPERUSER unrotated + entangled | **REVISE:** decouple rotation (name superuser); standalone incident response, human-go recorded separately | Operator |
| R-22 | ES-2 silent-denial on prod flip | **REVISE → hard gates** (metric+revert live, orphan re-run, user-distinguishable denial); no prod flip in Phase-0 | Human sets flip |
| R-23 | Strategic horizon (counsel) | B3 is **open-source-/scale-blocking, not first-paid-order-blocking**; do it as an OSS precondition, do not let its size crowd out the cheap-certain leak closure | Operator + Architect |

**Operator-gated (protect-paths) implementation steps:** every `packages/db/migrations/**` file (drift
reconciliation, `dowiz_app_rls` role+grants, Phase-1 re-verify, the flip), the credential rotation + secret
repoint, `verify:rls`/boot-guard edits, and rate-limit store DDL. App-code edits (`withTenant` role-switch,
worker `set_config`, central preHandler, WS URL flag, limiter fallback) are normal code but ship dark and
are proven before any enforcement flag is turned on.
