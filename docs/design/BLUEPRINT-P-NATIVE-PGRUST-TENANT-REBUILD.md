# BLUEPRINT — Native pgrust Tenant-Schema Rebuild (red-line, /council-gated)

Status: **SCOPE / PROPOSAL** — not implemented. Requires operator `/council` + per-change-confirm
(auth/money/RLS/migrations are red-line). This document is the blueprint for review; no code
is written until the council approves and a server tier is reintroduced (per
`docs/kernel-upgrade/RLS-SECURITY-GAP-ANALYSIS.md` plan B).

Ground truth this blueprint is built on (all cited):
- `docs/red-team/2026-07-13/D2-rls-data-governance.md` §0–§3 — the 11 fail-open tables + columns + seams.
- `kernel/src/retrieval/memory_store.rs` — the existing W13 `PgStore` sqlx adapter (the pattern to reuse).
- `kernel/Cargo.toml` — `pgrust = ["dep:sqlx","dep:tokio"]` feature, sqlx 0.8 cached offline.
- `deploy/pgrust.{service,toml,env}` — native Postgres process; `rls.cross_tenant = "deny"` is the
  enforced boundary today; the repo ships NO pgrust binary source, only deploy config.
- `docs/kernel-upgrade/RLS-SECURITY-GAP-ANALYSIS.md` — RLS is a SEPARATE red-line domain, out of
  kernel-autopilot scope; R1–R13 are reactivation gates, not live breaches (attic deleted, 0 exposure).

## 0. What this blueprint is NOT
- NOT a TS/Supabase migration (operator: "ніякого ts"). The old `attic/packages-db` 140 migrations
  are quarantined and dropped; we do not revive them.
- NOT an `ALTER ROLE dowiz_app NOBYPASSRLS` flip against a non-existent role/schema. The
  `dowiz_app` role and the 11 tables do not exist in this repo. A flip with no schema is fabrication.
- NOT a kernel-only change. The kernel's `pgrust` feature today is a `kv` store
  (`PgStore`). Tenant tables need a real server/adapter tier — which does not exist on this branch
  yet (it is a roadmap item: "when a server tier is reintroduced").

## 1. The architectural inversion (the actual "фліп")
The old `attic/` stack connected as a **BYPASSRLS** role (`deliveryos_api_user`, D2 §0/R7/R9) so
RLS was inert defense-in-depth and isolation rested 100% on app-code `WHERE` clauses + GRANTs. The
native pgrust rebuild **inverts** this:

- The app/service role connects **NOBYPASSRLS** → RLS becomes the REAL tenant boundary.
- Every tenant table carries `FORCE ROW LEVEL SECURITY` + a sound `location_id IN
  (SELECT app_member_location_ids())` predicate (or FK-chain equivalent).
- `deploy/pgrust.toml` `rls.cross_tenant = "deny"` is the app-level backstop; RLS is the DB-level
  backstop. Defense-in-depth, both directions.
- This is what makes the NOBYPASSRLS reactivation gate (D2 R8/R12) coherent: RLS is the boundary,
  so the gate asserts `rolbypassrls = false` + `relforcerowsecurity` on boot.

## 2. Scope — the tenant schema, rebuilt natively
Grounded in D2 §2/R1–R6 actual columns. All under `#[cfg(feature = "pgrust")]`, never compiled by
default, DDL only via explicit `migrate()` (W13 discipline — never auto-run against prod).

### 2.0 RESOLVED open questions (Q1–Q3) — iterated 2026-07-18
- **Q1 (location):** Lives in **`kernel/src/pgrust_tenant.rs`** behind the EXISTING `pgrust`
  feature. Reuses `PgStore`'s exact boundary (sqlx + captured tokio `Handle` + `block_on`,
  explicit `migrate()`), adds NO new crate, leaves the default pure-std build untouched. The
  future server/API tier calls the kernel's `pgrust` feature. (Rationale: `agent-adapters/` would
  force a new dependency edge and a second sqlx pool; keeping it in-kernel mirrors `PgStore`.)
- **Q2 (tenant context mechanism):** **Session GUC** — `set_tenant_context()` runs
  `SET LOCAL app.member_location_ids = $1` (array) on every pooled connection checkout; policies
  read it via `WHERE location_id = ANY (current_setting('app.member_location_ids')::int[])`. This
  mirrors the old `set_config('app.user_id'…)` seam BUT fixes R3: an unset context makes
  `current_setting(…)` raise/empty → the policy DENIES (no row matches), instead of the old
  `app_current_user() IS NULL` which matched ALL rows. The boot-guard (§4.3) + probe (§4.4) prove
  deny-on-unset. (Rejected bound-param-per-query: Postgres policies cannot reference query
  params, so a param would still need a session array — GUC is the only clean path.)
- **Q3 (analytics RLS):** **YES — scope `analytics_events` / `analytics_abuse_log` /
  `analytics_cwv`.** D2 R4 omitted RLS "by design" but these tables carry `location_id` + abuse-log
  IP/fingerprint = cross-tenant PII. Keep the inversion consistent: every tenant table is
  `FORCE` + `location_id IN (SELECT app_member_location_ids())`.

### 2.1 Adapter module `kernel/src/pgrust_tenant.rs`
Reuse the W13 `MemoryStore` boundary style: a `TenantStore` trait + `PgTenantStore` sqlx impl.
- `migrate(&self)` — idempotent `CREATE TABLE IF NOT EXISTS` for every tenant table below, then
  `ENABLE + FORCE ROW LEVEL SECURITY` + tenant predicate per table. EXPLICIT, never called by default.
- `set_tenant_context(&self, location_ids: &[i64])` — `SET LOCAL app.member_location_ids = $1` on
  the checked-out connection (replaces the dead `set_config('app.user_id' …)` seam; denies on unset).
- Typed CRUD for each aggregate (orders, couriers, customers, tokens).

### 2.2 Tables to (re)create natively — the 11 fail-open from D2, with their real columns
| table | D2 | key columns (from D2) | RLS predicate |
|---|---|---|---|
| `couriers` | R1 | `id`, `email_encrypted`, `phone_encrypted`, `full_name_encrypted`, `email_hash`, `phone_hash`, `password_hash`; NO `location_id` (scope via `courier_locations`) | `EXISTS (SELECT 1 FROM courier_locations cl WHERE cl.courier_id = couriers.id AND cl.location_id IN (SELECT app_member_location_ids()))` |
| `telegram_login_tokens` | R2 | owner login nonces/tokens; `location_id` | `location_id IN (SELECT app_member_location_ids())` |
| `orders` | R3 | `id`, `location_id`, `customer_id`, `subtotal`, `total`, `delivery_address`, `delivery_lat`, `delivery_lng`, `delivery_instructions` | `location_id IN (SELECT app_member_location_ids())` |
| `order_items` | R3 | `order_id`, `product_id`, `price_snapshot`, `quantity` | `order_id IN (SELECT id FROM orders WHERE location_id IN (SELECT app_member_location_ids()))` |
| `customers` | R3 | `id`, `location_id`, `messenger_handle`, … | `location_id IN (SELECT app_member_location_ids())` |
| `courier_sessions` | R4 | `id`, `courier_id`, `active_location_id` | via `courier_locations` FK chain |
| `customer_contact_reveals` | R4 | cross-tenant PII; `location_id` | `location_id IN (…)` |
| `notification_outbox_audit` | R4 | `location_id`, `payload_json` | `location_id IN (…)` |
| `analytics_events` / `analytics_abuse_log` / `analytics_cwv` | R4/R6 | `location_id` | `location_id IN (…)` — revisit "by design no RLS" (D2 R4) |
| `upload_audit` | R4 | `location_id` | `location_id IN (…)` |
| `customer_devices` | R5 | `customer_id`, `token_encrypted`, `fingerprint`; NO `location_id` → **add `location_id`** | add `location_id`, then `location_id IN (…)` |
| `backup_metadata` / `backup_audit_log` | R6 | `USING(true)`, no FORCE | add `FORCE` + scoped predicate (infra, but no fail-open) |
| `access_requests` | R6 | prospective-owner PII | replace `USING(true)` with scoped predicate |

### 2.3 The fail-open seams we must NOT recreate (D2 R3/R7/R9)
- **NO `app_current_user() IS NULL` session-wide predicate** (R3) — replace with token/order-id
  scoped predicate. A forgotten `set_tenant_context` must DENY, not match all rows.
- **NO `ALTER ROLE … BYPASSRLS`** in any migration (R7) — the app role is NOBYPASSRLS by construction.
- **NO `USING(true)`** policy (R6) — every table gets a real predicate or deny-all.
- **`customer_devices` gets `location_id`** (R5) so it is scorable at all.

## 3. Roles / migrations (the red-line core)
- One migration wave (idempotent SQL in `pgrust-store/migrations/` or a `migrate()` fn), executed
  ONLY by an explicit operator command against a known DB — never at boot, never by default build.
- Creates the NOBYPASSRLS app role + grants least-privilege (SELECT/INSERT/UPDATE/DELETE on tenant
  tables only; NO superuser, NO `BYPASSRLS`).
- Per D2 R12: assert `rolbypassrls = false` on the connected role at the end of `migrate()` and at
  service boot (R8 boot-guard).

## 4. Verification (falsifiable done-checks — no vibes)
1. `cargo build -p pgrust-store --features pgrust` → exit 0 (adapter compiles, default kernel untouched).
2. `cargo test -p pgrust-store` → offline tests green (struct/parse/SQL-string unit tests; DB-gated
   roundtrip `#[ignore]`d unless `DATABASE_URL` set — W13 pattern).
3. **Boot-guard (R8):** on connect, the store queries `pg_roles.rolbypassrls` for current_user → if
   true, `panic!`/FATAL before serving. Unit-test this branch by mocking the row.
4. **RLS-enforced probe (the real NOBYPASSRLS gate):** spin a throwaway NOBYPASSRLS role with NO
   tenant context set; `SELECT count(*) FROM couriers` → MUST return 0 (RLS denies). If >0 →
   migration is fail-open → FATAL. This is the RED→GREEN gate that replaces the dead `verify:rls`.
5. **Cross-tenant denial test:** set context A, insert row in tenant A; connect as tenant B context →
   row invisible. Deterministic, no live multi-tenant needed if the probe role is used.

## 5. DECART note (Integration Decart Rule)
| candidate | fit | verdict |
|---|---|---|
| Revive `attic/packages-db` TS migrations | forbidden (operator: no TS); also BYPASSRLS-by-design (R9) | REJECTED |
| Native Rust/sqlx adapter behind kernel `pgrust` feature (W13 pattern) | reuses shipped, cached, compiling adapter; RLS-is-boundary inversion; zero TS | **CHOSEN** |
| `diesel` / other Rust ORM | extra dep, same SQL surface, no upside over sqlx already in tree | REJECTED |

DECISION: native sqlx adapter behind the existing `pgrust` feature, reusing `PgStore`'s
`MemoryStore`-boundary discipline. Rationale: smallest correct surface, no new deps, RLS becomes
the boundary (fixes R9), falsifiable boot-guard + probe.

## 6. Resolved questions (Q1–Q4)
Q1–Q3 resolved (§2.0). Q4 decided by operator 2026-07-18: **option (i) — Rust subcommand.**

**Q4 (migrate execution):** `cargo run -p dowiz-kernel --features pgrust --bin pgrust-migrate -- --db $DATABASE_URL`.
A small `#[cfg(feature="pgrust")]` binary next to the existing `lm` / `markov_attractor` bins in
`kernel/Cargo.toml`. It:
1. runs `PgTenantStore::migrate()` (idempotent DDL + `ENABLE + FORCE RLS` + predicates),
2. asserts `pg_roles.rolbypassrls = false` for the connected app role,
3. asserts `relforcerowsecurity` on every tenant table,
4. runs the §4.4 probe (throwaway NOBYPASSRLS role with no context → 0 rows),
5. exits non-zero on ANY failure → no "serve" flag is flipped until green.

Operator runs it by hand against staging; the boot-guard (§4.3) re-asserts `rolbypassrls=false` at
service start so prod cannot serve with a BYPASSRLS role even if migration drifted.

This blueprint is the REACTIVATION GATE for `attic/` revival. It must NOT be applied while the
server tier is absent.

## 7. Ceiling (innovate:)
- pgrust binary source is NOT in this repo (only deploy config). If native pgrust itself needs
  schema-management features (migrations inside the binary), that is a separate `pgrust` upstream
  task, not this adapter's concern. Upgrade trigger: pgrust ships native migration runner.
- This blueprint is the REACTIVATION GATE for `attic/` revival. It must NOT be applied while the
  server tier is absent.
