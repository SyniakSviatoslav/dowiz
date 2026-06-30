# B3 — NOBYPASSRLS remediation plan (the concrete *how* + phased rollout)

Status: DESIGN (no prod code, no migration files placed). Date: 2026-06-30.
Direction APPROVED: `docs/adr/ADR-pg-privilege-hardening.md`. Inputs:
`docs/security/RLS-NOBYPASSRLS-GAP-AUDIT-2026-06-30.md`, `docs/security/pg-privilege-hardening-OPERATOR-HANDOFF.md`.
Red-line: auth / RLS / money / `packages/db/migrations/**` → every migration here is **operator-gated** (protect-paths).

---

## 0. The correctness backbone (why all of Phase 1 is a safe no-op under today's bypass)

Postgres combines **PERMISSIVE** policies with **OR** — both for `USING` (read/visibility) and for the
combined `WITH CHECK` on INSERT/UPDATE. Every Phase-1 object below is a **new permissive policy** or a
**widening rewrite** of an existing one. Adding a permissive policy can therefore only ever *admit more
rows* — it never denies a row another policy already admits. While `dowiz_app` still has `BYPASSRLS`, no
policy is consulted at all, so Phase 1 is observably inert (proven by: full lifecycle E2E green before and
after the Phase-1 migration, identical). The behavior change happens at the **Phase-3 flip**, by which
point every gap is pre-covered. This is the "schema rich, runtime minimal" seam: the policies go in early
and dark; enforcement is switched on once, later, behind the E2E gate.

**Live-schema corrections to the audit (verified against migrations, not the prose):**
- `orders`, `order_items`, `customers`, `idempotency_keys` **already** carry
  `anonymous_insert FOR INSERT WITH CHECK (app_current_user() IS NULL)` (`1780315000000_customer-rls.ts`).
  The anon-checkout INSERT gap is therefore **exactly 3 tables**, not the whole `POST /orders` txn:
  `velocity_events`, `order_item_modifiers`, `customer_track_grants` (all added later, none got the sibling).
- `ops_worker_heartbeat` **already** has `allow_ops_heartbeat_all FOR ALL USING(true) WITH CHECK(true)`
  (`1780691408625`, applied *after* the lockdown). Root cause 2 reduces to **2 tables**: `users`,
  `auth_refresh_tokens`.
- `locations`/`products`/`categories` already have `public_select FOR SELECT USING(true)` — the anon
  storefront/checkout *reads* survive the flip. The storefront `organizations` JOIN does not (RC3).
- The **dual-context pattern already exists and is proven** in `1790000000066_sensor-bus-now.ts`
  (`tenant_dual` / `funnel_events`): `… IN (SELECT app_member_location_ids()) OR location_id =
  NULLIF(current_setting('app.current_tenant', true), '')::uuid`. Phase 1 reuses it verbatim — no novelty.

---

## Phase 1 — additive RLS policies (per root cause)

All names follow existing conventions; all are idempotent (`DROP POLICY IF EXISTS … ; CREATE POLICY …`).
Forward-only, atomic, integer-money untouched, RLS already `ENABLE+FORCE` on every table named.

### RC1 — anon-checkout INSERTs (3 policies)
Mirror the existing `anonymous_insert` sibling on `orders`/`customers` exactly. The anon checkout sets **no**
GUC, so `app_current_user()` is `NULL` → the predicate is the correct, minimal discriminator (an
authenticated member writing these tables goes through the member policy instead, never this one).

| Table | Policy | Cmd | Expr | Role |
|---|---|---|---|---|
| `velocity_events` | `anonymous_insert` | `FOR INSERT` | `WITH CHECK (app_current_user() IS NULL)` | (none → PUBLIC) |
| `order_item_modifiers` | `anonymous_insert` | `FOR INSERT` | `WITH CHECK (app_current_user() IS NULL)` | (none → PUBLIC) |
| `customer_track_grants` | `anonymous_insert` | `FOR INSERT` | `WITH CHECK (app_current_user() IS NULL)` | (none → PUBLIC) |

Why correct + minimal: `order_item_modifiers` has no `location_id` (it is reachable only via
`order_item_id → order_items → orders`, all already gated), so an anon-null check is exactly the orders/items
sibling and adds zero new tenant surface. `velocity_events`/`customer_track_grants` carry `location_id` but
the anon writer's `location_id` is FK-validated and the row is only ever read back under the member/courier
policies — the accepted residual (a forged `location_id` only pollutes that tenant's own advisory/grant) is
identical to the already-accepted `funnel_events` residual (audit §7). No `TO` clause → applies to whatever
role connects; protection on the read side stays member/courier-scoped.

### RC2 — zero-policy auth tables (2 policies) — **NEEDS HUMAN/COUNCIL**
`users` and `auth_refresh_tokens` are pre-auth (login/signup/refresh/logout/courier-activate run with **no**
`app.user_id` — you must read a user *by email* before you know who they are), and they are **not
tenant-scoped** (a user is global, not a location row). Row-restriction is therefore impossible here; the
correct, honest design is **role-restriction**:

| Table | Policy | Cmd | Expr | Role |
|---|---|---|---|---|
| `users` | `ops_all` | `FOR ALL` | `USING (true) WITH CHECK (true)` | (none → PUBLIC) |
| `auth_refresh_tokens` | `ops_all` | `FOR ALL` | `USING (true) WITH CHECK (true)` | (none → PUBLIC) |

Why safe: identical to the live, accepted `ops_worker_heartbeat` policy. The protection is **not** the RLS
predicate — it is that (a) `anon`/`authenticated`/`service_role` were stripped of all privileges *and*
schema `USAGE` on these tables in `1780421100065_lockdown-nontenant-api-surface.ts`, so the **only** role
that can reach them is the operational pool role, and (b) the operational role is becoming NOBYPASSRLS but
still needs to *function* for auth. A `USING(true)` policy on a table only the trusted app role can touch is
defense-in-depth-neutral, not a hole.

> **Council call:** is `USING(true)` on `users` acceptable, or do we want belt-and-suspenders — e.g. restrict
> the policy `TO dowiz_app` explicitly (so a future mis-granted role still can't read), and/or split into a
> `FOR SELECT`/`FOR INSERT`/`FOR UPDATE`-scoped set so `auth_refresh_tokens` rows are insert+select+delete but
> never bulk-updatable? Recommendation: keep `USING(true)` (matches heartbeat precedent) **but** add `TO
> dowiz_app` once Phase-3 confirms the role name, turning the role-restriction into an explicit grant rather
> than an implicit one. Flagged because it is an auth-table red-line.

### RC3 — `memberships`/`organizations` pre-context read (1 SECURITY DEFINER fn; recommended)
`getOwnerLocationId` (`apps/api/src/lib/get-owner-location.ts:13,21`) reads `memberships` on the **raw pool,
before any `withTenant`** → under RLS it sees 0 rows → returns null → every owner endpoint 401s. Two designs:

- **(A) — recommended — scoped SECURITY DEFINER resolver.** Add one fn that performs the live-owner-membership
  check itself (DEFINER runs as the fn owner, which bypasses `memberships` RLS internally — exactly how the
  existing `app_member_location_ids()` lynchpin already works post-flip):
  ```sql
  CREATE FUNCTION app_owner_location(p_user uuid, p_location uuid DEFAULT NULL)
    RETURNS uuid LANGUAGE sql STABLE SECURITY DEFINER
    SET search_path = pg_catalog, public, pg_temp AS $$
      SELECT location_id FROM memberships
       WHERE user_id = p_user AND role = 'owner' AND status = 'active'
         AND (p_location IS NULL OR location_id = p_location)
       LIMIT 1 $$;
  -- REVOKE ALL FROM PUBLIC; GRANT EXECUTE TO dowiz_app;
  ```
  `getOwnerLocationId` then calls this one fn (a Phase-2 code change). **Must pin `search_path` =
  `pg_catalog, public, pg_temp`** or it trips the ITEM1 DEFINER guardrail (`guardrail-definer-search-path.mjs`).
  Why preferred: zero new broad read policy on `memberships`/`organizations`; least-privilege; the resolver
  encodes the exact ADR-0004 insider-removal check (active owner membership) in one auditable place.

- **(B) — alternative — `app.user_id` GUC in a txn.** Wrap the resolver read in `BEGIN; set_config('app.user_id',
  userId, true); …; COMMIT` so the existing member policy (`location_id IN (SELECT app_member_location_ids())`)
  admits it. Also a code change, slightly more LOC, and turns a single autocommit read into a transaction.

The **other** pre-context read sites the audit names (auth-refresh role re-derivation; `order_messages` owner
checks; storefront `organizations` JOIN) are each resolved the same way (route through `app.user_id` where a
user exists, or a narrow DEFINER read), **not** by a blanket `organizations`/`memberships` ops-read policy —
that would re-expose cross-tenant membership rows on the hot read path and defeat the flip. *Storefront's
`organizations` JOIN: confirm which columns it actually needs (brand name lives on `locations`, which is
already public-readable) — likely droppable, otherwise a column-narrowed `public_select` on org name only.*

### RC4 — `orders` (+ cash-as-proof sibling writes) courier-aware policy (3 policies) — **NEEDS HUMAN/COUNCIL (money red-line)**
Today: courier status transitions (`courier/assignments.ts`) and `deliveryCompletion.ts` run under
`app.current_tenant` (couriers are **not** members — no `memberships` row — they live in their own table keyed
by tenant), and `telegram-webhook.ts` sets `app.current_tenant` too. `orders` has only a member
(`app.user_id`) policy + anon SELECT/INSERT → every courier/webhook `UPDATE orders` matches 0 rows → 409 on
every transition; and `completeDelivery`'s `INSERT delivery_trace` / `INSERT courier_cash_ledger` (both
member-only, no WITH CHECK arm for current_tenant) abort → **the whole cash-as-proof completion txn rolls
back → till-debt / lost HOLD**. Owner transitions go through `withTenant` (`app.user_id`) and are already fine.

**Decision: Option A — add a courier-context permissive policy (reuse the proven `tenant_dual` shape), NOT
run transitions under `app.user_id`.** Option B (run courier writes under `app.user_id`) is rejected:
couriers have no membership, so `app_member_location_ids()` returns nothing for them — Option B would require
inventing synthetic courier memberships, a far larger and riskier change that crosses the courier/owner
authz boundary.

| Table | Policy | Cmd | Expr (USING and WITH CHECK both) | Role |
|---|---|---|---|---|
| `orders` | `courier_tenant_write` | `FOR ALL` | `location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid` | (none) |
| `delivery_trace` | `courier_tenant_write` | `FOR ALL` | `location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid` | (none) |
| `courier_cash_ledger` | `courier_tenant_write` | `FOR ALL` | `location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid` | (none) |

Additive (new permissive policy alongside the existing `tenant_isolation`/anon ones — OR-combined, so owner
and anon paths are untouched). Against the money invariants: this does **not** widen what a courier can
already legitimately reach — the courier is authenticated and authorized at the app layer
(`courier-room-authz.ts`, assignment-ownership checks, `SELECT … FOR UPDATE` of the assignment), and
`app.current_tenant` is set from the courier's *verified active shift location*. RLS here is defense-in-depth
mirroring the app WHERE clauses, not the primary gate. The cash-as-proof coherence (`paid_full` ⇒
`cash_amount = total`, no-partial-handover, server-authoritative `payment_outcome`) is enforced in
`deliveryCompletion.ts` and is orthogonal to row visibility — unchanged.

> **Council call:** sign off that admitting `app.current_tenant` to write `orders`/`courier_cash_ledger`/
> `delivery_trace` is acceptable as defense-in-depth (it equals the courier's existing app-layer reach).
> Open sub-question: should `courier_tenant_write` on `orders` be `FOR UPDATE`+`FOR SELECT` only (couriers
> never INSERT or DELETE orders) to keep the surface minimal? Recommendation: yes — split to `FOR SELECT` +
> `FOR UPDATE`, since the only courier order-writes are status transitions; that keeps anon `INSERT` the sole
> insert path. (Same `FOR INSERT`/`FOR SELECT`-only split for the two ledger/trace tables, which couriers only
> insert+read.)

### RC5 — courier-table reads use non-missing-ok `current_setting(...)` (≈9 policy rewrites)
Every courier-table policy was written as `location_id = current_setting('app.current_tenant')::uuid`
(**no `, true`**). When the GUC is unset, `current_setting` *throws* (`unrecognized configuration parameter`)
→ a hard 500 on any read without the GUC, instead of a clean 0-row deny. Under bypass this predicate was
never evaluated; after the flip it is. Rewrite each to the missing-ok, null-guarded form:

```sql
DROP POLICY IF EXISTS <existing_name> ON <table>;
CREATE POLICY <existing_name> ON <table>
  USING (location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid);
```

Tables (existing policy names): `courier_assignments` (`isolate_courier_assignments`), `courier_shifts`
(`isolate_courier_shifts`), `courier_positions` (`isolate_courier_positions`), `courier_invites`
(`isolate_courier_invites`), `courier_audit_log` (`isolate_courier_audit_log`), `courier_payouts`
(`isolate_courier_payouts`), `courier_dispatch_queue` (`isolate_courier_dispatch_queue`), `courier_locations`
(`isolate_courier_locations`), `customer_track_grants` (`isolate_customer_track_grants`). Pure rewrite —
forward-only, behavior-identical under bypass, and turns a post-flip 500 into a correct deny.

### RC6 — `TO authenticated`-only policies re-keyed (3 policies)
`owner_notification_targets`, `telegram_connect_tokens`, `customer_devices` use the Supabase Data-API pattern
(`TO authenticated` + `current_setting('request.jwt.claim.sub', true)`). The operational pool role is not
`authenticated` and never sets `request.jwt.claim.*` — these policies are inapplicable to it, so the rows
vanish on flip. Re-key to the operational `app.*` GUC model:

| Table | New policy | Cmd | Expr | Why |
|---|---|---|---|---|
| `owner_notification_targets` | `tenant_isolation` | `FOR ALL` | `location_id IN (SELECT app_member_location_ids())` (USING+CHECK) | owner CRUD runs via `withTenant` (`app.user_id`). |
| `telegram_connect_tokens` | `tenant_dual` | `FOR ALL` | `location_id IN (SELECT app_member_location_ids()) OR location_id = NULLIF(current_setting('app.current_tenant', true), '')::uuid` | owner *creates* via `app.user_id`; the webhook *consumes* via `app.current_tenant`. |
| `customer_devices` | `customer_owns` | `FOR ALL` | `customer_id = app_current_user()` (USING+CHECK) | `customer/push.ts` sets `app.user_id`; drop `TO authenticated`. |

Each is `DROP POLICY IF EXISTS <old> ; CREATE POLICY <new>`. `telegram_connect_tokens` has no `location_id`?
— it does (`location_id uuid NOT NULL`), so the `tenant_dual` predicate is valid as written.

**Phase-1 total: 20 policy objects (RC1 3, RC2 2, RC4 3, RC5 9, RC6 3) + 1 SECURITY DEFINER resolver fn (RC3).**

---

## Phase 2 — worker/service GUC (set_config)

Background workers have **no request context**, so they must set the GUC themselves inside a transaction
(`set_config(..., true)` is txn-local; an autocommit `set_config` is reset before the next statement — this is
exactly the `telegram-webhook` autocommit-reset bug the audit flags). Two structural shapes:

- **Single-tenant job** (acts on one location it already knows): wrap its work in `BEGIN; set_config('app.
  current_tenant', locationId, true) [or app.user_id for member context]; …; COMMIT`.
- **Cross-tenant sweep** (scans *all* tenants): RLS fundamentally fights a single connection seeing every
  tenant. Resolve by **iterating per location** (outer query enumerates locations on a DEFINER/maintenance read,
  then a per-location txn sets the GUC and does that tenant's work), **or** route the scan through a narrow
  `SECURITY DEFINER` maintenance fn (pinned search_path). Do **not** grant the worker role BYPASSRLS as a
  shortcut — that re-opens B3.

| # | Worker/service | Tenant table(s) touched | GUC + placement |
|---|---|---|---|
| 1 | `workers/signal-raiser.ts` | velocity_events, customer_signals, customers, orders | per-location loop → `app.current_tenant` |
| 2 | `workers/order-timeout-sweep.ts` | orders | per-location loop → `app.current_tenant` (or `app.user_id` if it carries an owner) |
| 3 | `workers/courier-dispatch.ts` | orders, courier_assignments, courier_dispatch_queue, couriers | per-location → `app.current_tenant` |
| 4 | `workers/courier-offer-sweep.ts` | courier_assignments, courier_dispatch_queue | per-location → `app.current_tenant` |
| 5 | `workers/courier-cron.ts` | courier_shifts, courier_positions, courier_assignments | per-location → `app.current_tenant` |
| 6 | `workers/courier-events.ts` | courier_assignments, courier_shifts | per-location → `app.current_tenant` |
| 7 | `workers/settlement-cron.ts` | settlement_items, courier_cash_ledger, courier_payouts | per-location → `app.current_tenant` |
| 8 | `workers/reconciliation.ts` | orders, courier_assignments, courier_cash_ledger | per-location → `app.current_tenant` |
| 9 | `workers/dwell-monitor.ts` | orders, courier_positions, courier_assignments | per-location → `app.current_tenant` |
| 10 | `workers/dwell-escalation.ts` | orders, courier_assignments | per-location → `app.current_tenant` |
| 11 | `workers/liveness-checker.ts` | courier_shifts, courier_positions | per-location → `app.current_tenant` |
| 12 | `workers/lifecycle-handlers.ts` | orders | per-event `app.current_tenant` (carries locationId) |
| 13 | `workers/anonymizer-gdpr.ts` + `anonymizer-retention.ts` | customers, orders | DEFINER fn already (anonymize); confirm GUC for any direct reads |
| 14 | `workers/delivery-trace-retention.ts` | delivery_trace | per-location or DEFINER maintenance fn (retention delete) |
| 15 | `workers/access-request-retention.ts` + `access-request-notify.ts` | access_requests | per-location → `app.current_tenant` (verify access_requests RLS key) |
| 16 | `notifications/workers/index.ts` | orders, customers | **partially done** — sets `app.user_id` at L117; audit remaining reads |
| 17 | `notifications/workers/telegram.poll.ts` | owner_notification_targets, telegram tables | per-location → `app.current_tenant` |
| — | `workers/rates-refresh.ts`, `free-tier-watch.ts`, `acquisition-retention.ts`, `backup/*` | exchange_rates (global) / platform tables / pg_dump | **N/A or verify** — global/non-tenant or runs as a distinct role; confirm each is not a FORCE-RLS tenant table before assuming covered |

**Count: ~15-17 workers need explicit `set_config` (items 1-17 above; 16 is partially done; the last row is
verify/N-A).** Exact GUC + line placement per worker is a build task verified query-by-query against each
file — this table is the inventory + the structural rule, not the final patch.

---

## Phase 3 — the flip (staging)

1. **Retarget the staged migration.** `docs/security/OPERATIONAL-ROLE-nobypassrls.migration.ts` currently
   `ALTER ROLE deliveryos_api_user NOBYPASSRLS`. The **live operational role is `dowiz_app`**
   (`rolbypassrls=t`); `deliveryos_api_user` is legacy/nologin. Retarget to flip **`dowiz_app`** and **keep**
   the `deliveryos_api_user` line (idempotent, harmless) so the migration is correct on any environment.
   Update the `verify:rls` probe + boot-guard messages to name `dowiz_app`.
2. **Pre-flip checks (staging, RED before any flip):**
   - `SELECT rolname, rolbypassrls FROM pg_roles WHERE rolname = 'dowiz_app';` → expect `t` (still bypass).
   - **Is `dowiz_app ∈ authenticated`?** `SELECT pg_has_role('dowiz_app','authenticated','member');` — if
     **true**, any surviving `TO authenticated` policy (RC6) *would* apply to it and the re-key is still
     required but the old policy is not silently dead; if **false** (expected), confirms RC6 policies are
     genuinely inapplicable pre-fix. Record the answer either way.
   - **Retains DML grants?** `SELECT table_name, string_agg(privilege_type,',') FROM
     information_schema.role_table_grants WHERE grantee='dowiz_app' AND table_schema='public' GROUP BY 1;`
     → verify SELECT/INSERT/UPDATE/DELETE present on `orders`, `velocity_events`, `order_item_modifiers`,
     `customer_track_grants`, `delivery_trace`, `courier_cash_ledger`, and the courier tables. The
     grant-mirroring migrations copied grants *from `orders`' grantees*, so if `dowiz_app` writes orders it
     writes these — but `velocity_events`/`order_item_modifiers` predate some of that mirroring: **explicitly
     confirm** their grants, since NOBYPASSRLS does not change grants but a *missing* grant becomes a hard
     deny instead of a silent bypass.
3. **Apply order:** Phase-1 policy migration(s) → Phase-2 worker code deploy (dark, still bypass) → then the
   **flip** migration (ITEM2, `dowiz_app NOBYPASSRLS`) → then ITEM1 (DEFINER search_path) — exactly the
   operator-handoff order (ITEM2 before ITEM1 so ITEM1's gate is observable).
4. **Flip probe (red→green on staging):**
   `SELECT rolname FROM pg_roles WHERE rolname='dowiz_app' AND rolbypassrls;` → 1 row RED → 0 rows GREEN.
5. **The gate = the full lifecycle E2E under the now-NOBYPASSRLS role**, run against staging:
   anonymous checkout (`/s/:slug` → POST /orders, exercising RC1 + RC4 anon paths) → owner accept/confirm
   (`/admin`, RC3 owner-gate) → courier assign/pickup/deliver with cash-as-proof (RC4 + RC5) → telegram-webhook
   transition (RC4 current_tenant) → notifications fan-out (Phase-2 workers). Plus `pnpm verify:rls` (gains the
   two permanent probes) + `pnpm typecheck` + the worker unit/integration tests. **Any newly-failing query is a
   real isolation bug to fix — never re-grant BYPASSRLS.** Boot-guard (operator-handoff §2) now fail-fasts if
   the role ever bypasses again.

---

## Phase 4 — prod

Separate, explicit operator step after staging is green and stable. Migrations are idempotent and applied
**before** / independently of the code deploy that needs them (release_command runs migrations on boot;
Phase-2 worker code must already be deployed-dark on prod before the flip migration runs there). Order on
prod mirrors staging: confirm Phase-1 policies + Phase-2 worker code are live and dark → run flip migration →
flip probe 0 rows → smoke the lifecycle on prod → boot-guard confirms. No re-grant escape hatch.

---

## Per-phase verification (proving each phase red→green WITHOUT the global flip)

The whole point: you can prove every policy *before* flipping the role, in an isolated transaction, by
**impersonating the future state** — `SET LOCAL ROLE dowiz_app` + setting the GUC — then `ROLLBACK`. This
needs a superuser/owner session on staging (the migration session has it) and leaves nothing behind.

```sql
-- Template: prove a policy admits the intended row under the post-flip role, in a throwaway txn.
BEGIN;
  SET LOCAL ROLE dowiz_app;                              -- simulate the future NOBYPASSRLS hot path
  -- RC4 courier write proof:
  SELECT set_config('app.current_tenant', '<loc-uuid>', true);
  UPDATE orders SET status = status WHERE id = '<order-in-that-loc>';   -- expect rowCount = 1 (was 0 RED)
  INSERT INTO courier_cash_ledger (...) VALUES (...);                   -- expect success (was WITH-CHECK fail)
  -- RC1 anon proof (no GUC → app_current_user() IS NULL):
  RESET app.current_tenant;
  INSERT INTO velocity_events (location_id, client_ip_hash, kind, window_started_at) VALUES (...); -- success
ROLLBACK;                                                -- nothing persisted
```

- **RC1/RC4/RC6 (writes):** the `SET LOCAL ROLE` + GUC template above — assert the INSERT/UPDATE succeeds
  *post-policy* where it fails *pre-policy* (run the same txn before the Phase-1 migration → it errors; after
  → it succeeds; `ROLLBACK` both). Wire the representative cases into `verify:rls` so they stay green.
- **RC2:** under `SET LOCAL ROLE dowiz_app`, `SELECT 1 FROM users LIMIT 1` returns a row after the policy,
  zero before (with FORCE-RLS + no bypass).
- **RC3:** call `SELECT app_owner_location('<owner-uuid>')` under `SET LOCAL ROLE dowiz_app` with **no** GUC →
  returns the location (DEFINER bypasses `memberships` RLS); proves the owner-gate resolves pre-`withTenant`.
- **RC5:** under `SET LOCAL ROLE dowiz_app` with **no** GUC set, `SELECT … FROM courier_assignments` →
  **before** the rewrite it raises (non-missing-ok throw); **after** it returns 0 rows cleanly. That "throw →
  clean deny" delta is the red→green.
- **Phase 2:** each worker gets a unit/integration test that runs its query under a NOBYPASSRLS test role +
  the GUC it sets, asserting rows are returned/written; and an inverse test asserting 0 rows / no cross-tenant
  bleed when the GUC names a *different* tenant.
- **Phase 3:** the lifecycle E2E is the composite gate (above). `verify:rls` exits non-zero on any anon leak +
  the `rolbypassrls` probe.

This means Phases 1, 2, and the per-policy correctness are **all proven on staging before the role is ever
flipped** — the flip itself becomes a low-information confirmation, not a discovery event.

---

## Residual risks + ownership

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R-a | A FORCE-RLS tenant table touched by a path nobody enumerated 0-rows silently post-flip | Mitigated by lifecycle E2E + per-tenant inverse worker tests; **accept** the long-tail with boot-guard + `verify:rls` as the permanent net | Architect + DB owner |
| R-b | `USING(true)` on `users`/`auth_refresh_tokens` (RC2) — role-restriction only | **NEEDS COUNCIL** sign-off; recommend tightening to `TO dowiz_app` post Phase-3 | Council + DB owner |
| R-c | `orders`/`cash_ledger`/`trace` admit `app.current_tenant` writes (RC4) — money red-line | **NEEDS COUNCIL** sign-off; recommend `FOR SELECT`+`FOR UPDATE`/`FOR INSERT`-narrowed split | Council + Architect |
| R-d | `velocity_events`/`order_item_modifiers` grants may predate the orders-grant mirroring → hard deny on flip | Pre-flip grant check (Phase 3 step 2) is the gate; **must verify**, not assume | DB owner |
| R-e | Cross-tenant sweep workers can't see all tenants under RLS | Resolved by per-location iteration / DEFINER maintenance fn; **never** worker-role BYPASSRLS | Architect |
| R-f | New RC3 DEFINER fn unpinned would trip ITEM1 guardrail | Fn pins `search_path = pg_catalog, public, pg_temp` by design | DB owner |
| R-g | Storefront `organizations` JOIN columns unknown | Investigate before adding any org read policy; prefer dropping the JOIN (brand on `locations`, already public) | Architect |

**Operator-gated (protect-paths) steps:** every `packages/db/migrations/**` file (all Phase-1 policies, the
RC3 fn, the retargeted flip migration, ITEM1), the `verify:rls` + boot-guard edits, and `package.json`
guardrail wiring. The Phase-2 worker `set_config` edits and the `get-owner-location.ts` RC3 call are normal
app code (not protected) but ship dark and are proven before the flip.
