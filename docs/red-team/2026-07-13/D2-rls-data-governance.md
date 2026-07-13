# D2 — RLS, Money-Integrity & Data-Governance Red-Team Audit

- **Target:** `/root/dowiz` @ branch `feat/decentralized-pq-protocol`
- **Date:** 2026-07-13
- **Lens:** strictest red-line invariants — auth / money / RLS / PII / migrations
- **Auditor:** database-security & data-governance red-team (D2)
- **Method:** read migration SQL, DB access layer, anonymizer, GDPR/OTP routes, live Rust kernel + web glue; every claim cited `file:line`; CONFIRMED vs SUSPECTED distinguished.

---

## 0. GROUND TRUTH — READ FIRST (reframes every finding below)

The task brief assumes a live "Supabase/Postgres with Row-Level Security" stack. **On this branch that is no longer the deployed reality.** Commit `e1505e1d` ("chore(declutter C2): quarantine retired Supabase/Fly/RLS stack") **moved the entire Postgres/Supabase data tier into `attic/`**: `apps-api`, `apps-worker`, `packages-db` (all 140 migrations), and `fly.toml`. Those packages are still git-tracked and therefore reactivatable, but they are **not installed, not built, and not deployed** on `feat/decentralized-pq-protocol`.

What is actually **LIVE** on this branch:

| Live surface | Path | Data role |
|---|---|---|
| Rust money/order kernel → WASM | `kernel/src/{money,domain,order_machine,wasm}.rs` | pricing + order state machine, executed **client-side** in the browser |
| Astro/Svelte storefront | `web/src/` | calls the WASM kernel; no server API |
| Postgres LISTEN/NOTIFY bus | `packages/platform/src/message-bus.ts` | pub/sub only — touches **no tenant rows**; not instantiated by any live server on this branch |

There is **no live server tier** (`grep` for a `server/` crate, `axum`, `tokio_postgres`, `sqlx`, `rusqlite`, or any `CREATE TABLE` outside `attic/` → 0 hits), **no live migrations**, and therefore **no live RLS-enforced multi-tenant datastore** on this branch.

**Two consequences that govern severity throughout this report:**

1. **The quarantined RLS design, even when it was live, was never the tenant boundary.** The runtime app connects as a **BYPASSRLS** role (`deliveryos_api_user` is granted `BYPASSRLS` at `attic/packages-db/migrations/1780691681296_ops-location-alerts-policy.ts:8`; the session/migration pool is superuser). For a BYPASSRLS role every `ENABLE`/`FORCE`/policy is inert. The real tenant boundary is (a) the GRANT/role layer (`anon`/`authenticated`/`service_role` are fully revoked from schema `public`, `1780421100065_lockdown-nontenant-api-surface.ts:11-18,51`) plus (b) application-level tenant scoping via `set_config('app.user_id' | 'app.current_tenant')` feeding policy predicates. RLS is **defense-in-depth**, not the boundary.

2. **The RLS gaps below are exploitable by the one NOBYPASSRLS role** — `deliveryos_operational_user` (`LOGIN NOBYPASSRLS`, granted `SELECT ON ALL TABLES`, `1790000000015_operational-pool-role.ts:19,33`) — **and by any app-code path that connects NOBYPASSRLS and forgets its tenant filter**, and by a resurrected Supabase Data API. They are **not** reachable by a rival tenant through the normal Fastify API, which injects the caller's own tenant context. This is a genuine second-layer weakness, reported conservatively as such — not an "any authenticated user reads all tenants over HTTP" gate.

Bottom line of the framing: **this audit describes the security posture of a data tier that is currently dormant.** The findings matter (a) as a reactivation gate — this stack must not be un-quarantined without fixing them — and (b) because the live replacement (client-side WASM pricing) has its **own** distinct, and arguably worse, money-authority problem (§4).

---

## 1. BOTTOM LINE

**Is customer/tenant data actually isolated?**

- **On this branch: N/A — there is no live multi-tenant datastore to isolate.** The retired stack held the data; it is dormant.
- **In the quarantined stack (reactivation posture): PARTIALLY.** RLS is `ENABLE + FORCE` with sound tenant predicates on the large majority of tenant tables, but **11 tables ship with no RLS at all or a `USING(true)`/fail-open policy** — including `couriers` (holds `password_hash` + encrypted PII) and `telegram_login_tokens` (owner-login auth tokens). Because the app runs BYPASSRLS, these are second-layer gaps that bite the NOBYPASSRLS read pool and app-filter bugs — but two of them (`couriers`, `telegram_login_tokens`) are credential/account-takeover material and must be treated as HIGH. The isolation model is **fragile-by-construction**: a single forgotten `WHERE` clause on the BYPASSRLS pool, or one misgrant, converts every `USING(true)` table into a full cross-tenant read.

**Is money tamper-proof?**

- **Arithmetic: YES.** Money is integer minor units end-to-end — `i64` in the kernel (`kernel/src/money.rs`), `integer … CHECK (>= 0)` on every DB money column. Zero float on any monetary value (the only `f64`s are the tax *rate* and FX *rate*, quantized to integer micro-units before any money multiply). Negativity is guarded.
- **Authority: NO (on the live path).** The authoritative pricing engine runs **client-side as WASM**, and it prices from a **caller-supplied `unit_price`** (`kernel/src/domain.rs:54-55`) with no server re-computation from a trusted catalog. It is only not-currently-exploitable because the web glue hard-codes `unit_price: 0` (`web/src/lib/kernel.js:26`), so every live order stamps `total = 0`. The red-line "server/kernel is authoritative; client must not supply trusted prices" is **NOT satisfied** on this branch. The correct server-authoritative pricer exists only in the quarantined oracle (`attic/apps-api/src/routes/orders.ts`).

**Is PII protected / is GDPR erasure real?**

- **RLS on core PII tables: HELD** (`customers`, `orders`, `phone_otp`, `customer_otp_sessions`, `gdpr_erasure_requests` all `ENABLE + FORCE` with sound predicates). No IDOR on customer/owner order-read or the GDPR routes.
- **Anonymizer / erasure: REAL but INCOMPLETE.** Erasure is a genuine one-way in-place `UPDATE` (not a soft flag), but it **forgets** order geolocation (`delivery_lat`/`delivery_lng`), free-text PII (`delivery_instructions`, `preferences`), the `messenger_handle` on customers, and leaves **raw phone** behind in `phone_otp` and `gdpr_erasure_requests.subject_phone`; it also **never cascades** a customer erasure to that customer's orders. A "forgotten" customer remains re-identifiable by phone and locatable by GPS. **GDPR erasure does not achieve right-to-be-forgotten.**

---

## 2. RLS COVERAGE TABLE (quarantined `attic/packages-db/migrations`)

Legend: **E** = `ENABLE ROW LEVEL SECURITY`, **F** = `FORCE`. "policy sound?" = tenant/owner-scoped predicate present. Reader for any gap = the NOBYPASSRLS `deliveryos_operational_user` read pool, a future NOBYPASSRLS reporting role, or a resurrected Data API. 76 app tables; OK rows collapsed, all gates listed individually.

| table | created-in (file id) | E? | F? | policy sound? | verdict |
|---|---|---|---|---|---|
| users | 1780310071220 (+lockdown 65:26) | ✅ | ✅ | none → deny-all | LOCKED (fail-closed) |
| organizations | 1780310071220 | ✅ | ✅ | sound (:101) | OK |
| locations | 1780310071220 | ✅ | ✅ | sound (:87) + `public_select USING(true)` (1780338909301:8) | OK (public menu by design) |
| memberships | 1780310071220 | ✅ | ✅ | sound (:94) | OK |
| categories / products | 1780310072731 | ✅ | ✅ | sound + `public_select USING(true)` (1780338741329) | OK (public menu by design) |
| **customers** | 1780310074262 | ✅ | ✅ | sound (:76) **+ anonymous_select `USING(app_current_user() IS NULL)`** (1780338981782:11) **+ anonymous_update** (:7) | ⚠ **GATE #8 (fail-open read+update)** |
| **orders** | 1780310074262 | ✅ | ✅ | sound (:83) **+ anonymous_select `USING(app_current_user() IS NULL)`** (1780338981783:6) | ⚠ **GATE #8 (fail-open read)** |
| **order_items** | 1780310074262 | ✅ | ✅ | sound (:91) **+ anonymous_select `USING(app_current_user() IS NULL)`** (1780338981783:10) | ⚠ **GATE #8** |
| **idempotency_keys** | 1780310074262 | ✅ | ✅ | sound (:98) **+ anonymous_select `USING(key IS NOT NULL)`≈true** (1780315000000:28) | ⚠ GATE #11 (low) |
| location_themes / webhook_endpoints / api_keys / domain_verifications / recurring_orders / promotions / location_alerts | 1780310075801 / 077400 / 1790…017 | ✅ | ✅ | sound | OK |
| ops_worker_heartbeat | 1780313199559 → ENABLE lockdown 65:28 | ✅ | ✅ | `USING(true)` FOR ALL (1780691408625:7) | OK (infra, no tenant data) |
| auth_refresh_tokens | 1780314625706 → ENABLE lockdown 65:30 | ✅ | ✅ | none → deny-all | LOCKED |
| courier_invites | 1780314627951 / 1780421031109 | ✅ | ✅ | sound (app.current_tenant) | OK |
| menu_modifiers family (modifier_groups, modifiers, product_modifier_groups, order_item_modifiers) | 1780338982010 | ✅ | ✅ | sound (via FK chain) | OK |
| product/category/modifier translations | 1780338982011 / 027 | ✅ | ✅ | sound (via parent) | OK |
| delivery_tiers / order_status_history / reservations / menu_versions / order_ratings / order_messages | 1780338982014–1790…025/002 | ✅ | ✅ | sound | OK |
| import_sessions | 1780338982025:26 | ✅ | ❌ | sound (:31) | OK-ish (**F missing**) |
| theme_versions | 1780338982030:30 | ✅ | ❌ | sound (:36) | OK-ish (**F missing**) |
| telegram_connect_tokens | 1780348982031:18 | ✅ | ❌ | sound (:24 owner_id=jwt.sub) | OK-ish (**F missing**) |
| owner_notification_targets | 1780348982032:20 | ✅ | ❌ | sound (:26) | OK-ish (**F missing**) |
| order_routes | 1790000000036:25 | ✅ | ❌ | sound (:28 app.current_tenant) | OK-ish (**F missing**) |
| customer_devices | 1780348982033 → **DISABLE** 1780421100059:42 | ❌ | ❌ | policy DROPPED (:22) | ⚠ **GATE (RLS disabled, PII)** |
| **couriers** | **1780421029538** | **❌** | **❌** | **NONE** | 🔴 **OPEN GATE #1 (password_hash + PII)** |
| courier_locations / courier_audit_log / courier_shifts / courier_assignments / courier_positions / courier_payouts / courier_dispatch_queue / courier_cash_ledger / courier_sessions* | 1780421029538–1790…028 | ✅ | ✅ | sound (force-rls 1790000000051:5-14) | OK *(except courier_sessions — gate #3)* |
| **courier_sessions** | **1780421032856** | **❌** | **❌** | **NONE** | 🔴 **OPEN GATE #3** |
| settlement_items / settlement_audit_log | 1780421100045/046 | ✅ | ✅ | sound | OK |
| backup_metadata | 1780421100048:21 | ✅ | ❌ | `USING(true)` (:23 + system 50:5) | ⚠ **GATE #9 (USING(true), no F)** |
| backup_audit_log | 1780421100049:15 | ✅ | ❌ | `USING(true)` (:17 + system 50:6) | ⚠ **GATE #9** |
| phone_otp / customer_signals / velocity_events / customer_otp_sessions | 1780421100054/057 | ✅ | ✅ | sound | OK |
| gdpr_erasure_requests / anonymization_audit_log | 1780421100060 | ✅ | ✅ | sound | OK |
| **customer_contact_reveals** | **1780421100062** | **❌** | **❌** | **NONE** | 🔴 **OPEN GATE #4 (customer PII)** |
| **upload_audit** | **1780421100063** | **❌** | **❌** | **NONE** | 🔴 **OPEN GATE #7** |
| free_tier_snapshots | 1780421100064 | ❌ | ❌ | NONE | ⚠ low (ops metrics, no tenant data) |
| **notification_outbox_audit** | **1790000000007** | **❌** | **❌** | **NONE** | 🔴 **OPEN GATE #5 (payloads)** |
| **analytics_events / analytics_abuse_log / analytics_cwv** | **1790000000012** | **❌** | **❌** | **NONE (by design :7-9)** | 🔴 **OPEN GATE #6** |
| exchange_rates | 1790000000013 | ❌ | ❌ | NONE (by design) | OK (global ref data, no tenant col) |
| customer_track_grants / delivery_trace | 1790000000026/027 | ✅ | ✅ | sound (app.current_tenant) | OK |
| **telegram_login_tokens** | **1790000000031** | **❌** | **❌** | **NONE** | 🔴 **OPEN GATE #2 (owner auth tokens)** |
| access_requests | 1790000000041:47 | ✅ | ✅ | `USING(true) WITH CHECK(true)` (:50) | ⚠ **GATE #10 (USING(true), by-design ops, PII)** |
| telegram_action_nonces / notification_prefs_audit | 1790000000050/051 | ✅ | ✅ | sound | OK |
| product_media / menu_schedules | 1790000000054/062 | ✅ | ✅ | sound (+WITH CHECK) + `public_select USING(true)` | OK (public menu by design) |

Also flagged for completeness (out of RLS scope): the **`pgboss`** job-queue schema grants `SELECT,INSERT,UPDATE,DELETE … TO PUBLIC` (`1790000000009:35`) and `GRANT CREATE ON SCHEMA public TO PUBLIC` (`1790000000006:50`) — blanket `TO PUBLIC` on infra, broad but not tenant tables.

---

## 3. FINDINGS — RLS / ROLES / MIGRATIONS

### R1 · `couriers` table has NO RLS and stores `password_hash` + encrypted PII · **HIGH** · CONFIRMED
- `attic/packages-db/migrations/1780421029538_couriers.ts:5-19` — table created; RLS never enabled anywhere (0 `ROW LEVEL SECURITY` mentions for `couriers` repo-wide). Columns include `email_encrypted`, `phone_encrypted`, `full_name_encrypted`, `email_hash`, `phone_hash`, **`password_hash`**. No `location_id` (global entity; tenancy is via `courier_locations`).
- **Exploit (NOBYPASSRLS reader):** `SELECT id, email_hash, phone_hash, password_hash FROM couriers;` → every courier credential hash + PII across all owners.
- **Fix:** `ENABLE + FORCE ROW LEVEL SECURITY`; policy `EXISTS (SELECT 1 FROM courier_locations cl WHERE cl.courier_id = couriers.id AND cl.location_id IN (SELECT app_member_location_ids()))`. Independently: `password_hash` for couriers should not be readable by the operational pool at all.

### R2 · `telegram_login_tokens` has NO RLS — owner-login auth tokens exposed · **HIGH** · CONFIRMED
- `1790000000031_telegram-owner-login.ts:16` — no RLS; grants mirror `orders` (:34-38). Holds owner-login nonces/tokens (account-takeover material).
- **Exploit:** `SELECT * FROM telegram_login_tokens;` → live owner login tokens for every tenant → owner account takeover.
- **Fix:** `ENABLE + FORCE`; scope by the token's owner/`location_id` (`= app.current_tenant` / `app_member_location_ids()`). Treat as auth red-line.

### R3 · Fail-OPEN anonymous policies on `orders`/`order_items`/`customers` · **HIGH** · CONFIRMED
- `1780338981783_anonymous_orders.ts:5-6` (`orders FOR SELECT USING (app_current_user() IS NULL)`), `:9-10` (`order_items`); `1780338981782_customer-anonymous-update.ts:10-11` (`customers FOR SELECT`) **and `:6-7` (`customers FOR UPDATE`)**. The predicate `app_current_user() IS NULL` is **session-level, not row-scoped** — when a NOBYPASSRLS session has not called `set_config('app.user_id', …)`, `app_current_user()` returns NULL and the permissive policy matches **every row** (OR'd with the sound `tenant_isolation`).
- **Exploit (NOBYPASSRLS session that never set `app.user_id` — e.g. a reporting query or a code path that forgot the scoping call):** `SELECT customer_id, total, delivery_address FROM orders;` → all orders across every tenant; on `customers` an unset session can also **UPDATE any row**. Row isolation here rests 100% on the app adding `WHERE id = $token`; RLS provides none.
- **Fix:** replace the `app_current_user() IS NULL` seam with a token/order-id-scoped predicate (e.g. match on a claim-check token column), never a session-wide `IS NULL` that degrades to table-wide `true`.

### R4 · Six additional tenant tables with NO RLS (`courier_sessions`, `customer_contact_reveals`, `notification_outbox_audit`, `analytics_{events,abuse_log,cwv}`, `upload_audit`) · **MEDIUM–HIGH** · CONFIRMED
- `courier_sessions` `1780421032856:5-11` (live sessions, `active_location_id`) — `SELECT * FROM courier_sessions;`.
- `customer_contact_reveals` `1780421100062:15-20` (cross-tenant customer contact PII) — `SELECT * FROM customer_contact_reveals;`.
- `notification_outbox_audit` `1790000000007:12-16` (`location_id` + `payload_json`) — `SELECT location_id, payload_json FROM notification_outbox_audit;` → every tenant's outbound notification contents.
- `analytics_events/abuse_log/cwv` `1790000000012:14,37,55` — RLS **intentionally** omitted (`:7-9` "Do NOT add FORCE"); each carries `location_id` → full cross-tenant behavioural analytics + abuse-log IP/fingerprint.
- `upload_audit` `1780421100063:9-11` (lower sensitivity).
- **Fix:** `ENABLE + FORCE` + `location_id IN (SELECT app_member_location_ids())` on each. The analytics "by design no RLS" decision should be revisited — it is a cross-tenant read for the operational pool.

### R5 · `customer_devices` RLS DISABLED after the fact · **MEDIUM** · CONFIRMED
- Created with `ENABLE + FORCE` + policy (`1780348982033`), then `1780421100059_push-notifications.ts:22,42` **drops the policy and `ALTER TABLE customer_devices DISABLE ROW LEVEL SECURITY`**. Final state: RLS off. Table holds `customer_id`, `token_encrypted`, `fingerprint` (plaintext), **no `location_id`** (no tenant key to scope on). Not cascaded on erasure.
- **Exploit:** `SELECT customer_id, fingerprint FROM customer_devices;` → cross-tenant device fingerprints of every customer, incl. erased ones.
- **Fix:** add `location_id`, re-`ENABLE + FORCE` with a tenant policy, delete on erasure.

### R6 · `USING(true)` policies on `backup_metadata` / `backup_audit_log` / `access_requests` · **MEDIUM (access_requests) / LOW (backups)** · CONFIRMED
- `1780421100048:23` & `1780421100049:17`: `FOR SELECT TO authenticated USING(true)` (also ENABLE-only, **no FORCE**); `1780421100050:5-6`: `FOR ALL TO deliveryos_api_user USING(true) WITH CHECK(true)`. Infra/backup descriptors — `SELECT * FROM backup_metadata;`.
- `1790000000041_access-requests.ts:50`: `FOR ALL USING(true) WITH CHECK(true)` on a table holding prospective-owner PII (email/contact). Documented "Pattern A2" — the real boundary is the GRANT (`:52` revokes anon/authenticated/service_role). Still a literal full-table policy: `SELECT * FROM access_requests;` for any granted NOBYPASSRLS role.
- **Fix:** these are the clearest illustration of the systemic risk (R9). At minimum add `FORCE`; prefer a real scoped predicate over `USING(true)` even for "ops-only" tables.

### R7 · BYPASSRLS escalation hidden in a misnamed migration with swallow-all error handling · **MEDIUM** · CONFIRMED
- `1780691681296_ops-location-alerts-policy.ts:8` — a file named *ops-location-alerts-policy* actually runs `ALTER ROLE deliveryos_api_user BYPASSRLS`, wrapped in `EXCEPTION WHEN OTHERS THEN` (silent no-op on failure).
- **Impact:** the single most security-relevant statement in the schema (it makes RLS inert for the app) is invisible in review (name implies a table policy) and silent on failure → environment drift where migration success does not tell you whether `api_user` is BYPASSRLS.
- **Fix:** isolate the grant into a truthfully-named migration, drop the blanket exception, and assert `rolbypassrls` post-state.

### R8 · RLS verification gate is dead; no runtime RLS boot-guard · **HIGH (gate regression) / LOW (live impact this branch)** · CONFIRMED
- `scripts/verify-all.ts:16` and `scripts/verify-launch.ts:207` invoke `pnpm verify:rls`, but **no `verify:rls` script exists** in `package.json` (0 hits) and `@deliveryos/db` (which held `attic/packages-db/scripts/verify-rls.ts`) is uninstalled/quarantined. The only startup FATAL-guard on this branch (`packages/config/src/index.ts:207`) checks **dev-auth env only**, not `relforcerowsecurity`. No boot-guard aborts on a table shipped without FORCE.
- **Impact:** if the Supabase/TS stack is reactivated from `attic/`, a table without RLS/FORCE (see R1–R6) is caught neither in CI nor at boot. Low live impact only because this branch has no live Postgres.
- **Fix (reactivation gate):** restore `verify:rls` as a real, wired script and add a runtime boot-guard asserting `relforcerowsecurity` on every tenant table.

### R9 · Architectural: tenant isolation depends on app-code discipline, not RLS · **HIGH (systemic)** · CONFIRMED
- Root evidence: `1780691681296:8` (api_user BYPASSRLS) + `1790000000015:19,33` (only NOBYPASSRLS role is read-only) + `1780421100065:8,21` ("app roles use BYPASSRLS"). RLS is inert for the hot path; isolation is the GRANT layer + `set_config('app.user_id'|'app.current_tenant')` + explicit `WHERE` clauses.
- **Impact:** any single missing tenant `WHERE` on the BYPASSRLS pool leaks cross-tenant data with no DB backstop; and the fail-open (R3) / `USING(true)` (R6) / no-RLS (R1–R5) tables mean the NOBYPASSRLS read pool is **also** unprotected on those tables. The design is one bug away from a cross-tenant breach on many tables.
- **Fix:** either move the runtime to a NOBYPASSRLS role with real per-tenant policies on every table (make RLS the boundary), or formally document GRANT+app-layer as the boundary **and** eliminate every fail-open/`USING(true)`/no-RLS table so the second layer is not silently absent.

### R10 · `message-bus.ts` — raw-SQL string interpolation for LISTEN/NOTIFY · **LOW** · SUSPECTED (latent)
- `packages/platform/src/message-bus.ts:88` (`LISTEN "${channel}"`), `:131` (`NOTIFY "${channel}", '${payload}'`), `:200`, `:226`. Payload single-quotes are escaped by hand (`:129`, `.replace(/'/g,"''")`) and the local `no-raw-sql` lint rule is disabled (`:225`), but the **channel identifier** is interpolated into a `"…"` quoted identifier with no escaping of embedded `"`.
- **Impact:** if a channel name ever derives from user-controlled input (e.g. a venue slug), a `"` breaks out of the identifier → SQL injection on the bus connection. On this branch all channel producers are in `attic/` and ID-derived (`orderChannel(orderId)`, `dashboardChannel(locationId)`), so it is latent.
- **Fix:** validate channel against `^[A-Za-z0-9_]+$`; use `pg_notify($1,$2)` with bound params for the payload.

### R11 · `message-bus.ts` session pool has no pinned connection string · **LOW** · CONFIRMED
- `message-bus.ts:7,33` — `createSessionPool()` calls `new Pool()` with **no opts** (the retired `@deliveryos/db` factory used `***REDACTED***`). node-postgres falls back to libpq env (`PGUSER`/`PGHOST`/`PGPASSWORD`), so the configured `***REDACTED***` is bypassed and the bus connects as whatever `PGUSER`/OS user resolves to — possibly a superuser on a misconfigured host. `loadEnv` is imported (`:2`) but unused. Blast radius limited: the bus only issues LISTEN/NOTIFY/UNLISTEN and reads no tenant rows.
- **Fix:** pass `{ connectionString: env.***REDACTED*** }` explicitly.

### R12 · "De-privileged" operational role can't serve its workload → likely still BYPASSRLS in prod · **MEDIUM** · SUSPECTED (config-dependent)
- `1790000000015:33` grants `deliveryos_operational_user` **SELECT-only**, but `attic/apps-api/src/plugins`/`queue-provider` run pg-boss on `***REDACTED***`, which needs INSERT/UPDATE/DELETE on the `pgboss` schema. A NOBYPASSRLS SELECT-only role cannot serve pg-boss, so `***REDACTED***` in practice must still point at a BYPASSRLS/superuser role — the de-privilege migration is inert.
- **Fix:** grant the operational role exactly the DML it needs (or split read/write pools), repoint the env var, and assert `rolbypassrls = false` on the connected role at boot.

### R13 · No service-role/admin key in client code · CLEAN · CONFIRMED
- Grep of `web/`, `packages/ui/` for `service_role|SERVICE_ROLE_KEY|supabaseAdmin|createClient|SUPABASE_*` → **0 hits**. No Supabase admin/anon key embedded in shipped client code. (Positive finding.)

---

## 4. MONEY-INTEGRITY FINDINGS

Verdict: **arithmetic PASS, authority FAIL on the live path.** DB money columns all clean: `orders.subtotal/total/cash_pay_with integer … CHECK (>= 0)` (`1780310074262_orders.ts:32,33,36`), `order_items.price_snapshot integer CHECK (>=0)` (:55), `money_breakdown.{delivery_fee,discount_total,tax_total} integer … CHECK (>= 0)` (`1780338982013:6-8`). No `FLOAT`/`REAL`/`DECIMAL`/`double` on any money column (the only non-integer numerics are `tax_rate numeric` (a rate), `max_distance_km numeric` (distance), and `delivery_lat/lng double precision` (geo) — none are money).

### M1 · Authoritative pricing engine executes in the browser; no server re-price · **HIGH** · CONFIRMED
- `web/src/components/Storefront.svelte:131-136` dynamically imports and runs the WASM kernel client-side; `kernel` builds as `cdylib` → WASM. There is no `server/` dir, no `POST /api/orders`, no `axum`/`fetch('/api/orders')` on this branch.
- **Exploit (once real prices flow):** open devtools, call `k.place_order_js(undefined, '[{"product_id":"…","quantity":1,"unit_price":1}]', 'storefront')` with any `unit_price`; the returned order is whatever the client computed — no server authority to reject it.
- **Fix:** compute/charge the total on a real server from trusted catalog state (`product_id`+`modifier_ids`+`quantity` only); never accept a client `unit_price`/`subtotal`/`total`. The kernel may run in-browser for UX, but the persisted/charged total must be server-recomputed.

### M2 · Kernel `place_order` trusts caller-supplied `unit_price` (no catalog lookup) · **HIGH** · CONFIRMED
- `kernel/src/domain.rs:102-122` (`place_order`) → `Order::compute_subtotal` (`domain.rs:54-55`, `items.iter().map(|i| i.unit_price * i.quantity).sum()`). `unit_price` is a caller-populated field (`domain.rs:25-30`); the kernel holds no product/modifier price table.
- **Fix:** the price boundary must sit where the trusted catalog lives (server). If the kernel is to price, hand it a trusted catalog and take `product_id`+`modifier_ids` only.

### M3 · Web glue stubs `unit_price: 0` → every live order totals 0 · **MEDIUM** · CONFIRMED
- `web/src/lib/kernel.js:21-28` sets `unit_price: 0` ("0 until the server prices the line"); the on-screen subtotal (`Storefront.svelte:99-117`) is display-only from a mock menu (`:27-45`) and never persisted. Live path produces `subtotal=0, total=0` (proven by `web/src/lib/kernel/kernel.test.mjs`). Not an exploit — a half-built gap that is also the only reason M1/M2 are not a live theft vector today.
- **Fix:** wire real server pricing (M1). Until then the "money engine" prices nothing.

### M4 · Unchecked integer arithmetic in the kernel (silent wrap on overflow in release) · **LOW** · SUSPECTED
- `kernel/src/money.rs:62-64` (`unit += m; unit * quantity`), `:18` (`value * 10`), `domain.rs:54-55` (`unit_price * quantity … .sum()`), `domain.rs:88` (`subtotal + tax + fee`) use raw `*`/`+` with no `checked_mul`/`checked_add`. `kernel/Cargo.toml` release profile does not set `overflow-checks`, so release/WASM **wraps** instead of panicking. A wrap-to-negative is caught by `assert_non_negative` (`money.rs:68-73`), but a wrap to a smaller **positive** would pass. Unreachable today (`unit_price=0`); theoretical.
- **Fix:** use `checked_mul`/`checked_add` returning `Err` on overflow, or set `overflow-checks = true` for release.

### M5 · Kernel has no currency field / no mixed-currency guard · **LOW** · SUSPECTED
- `to_minor_unit` ignores its `_currency` arg (`money.rs:8`); `apply_tax`/`compute_line_total`/`compute_subtotal` and the `Order` aggregate (`domain.rs:37-50`) carry no currency. Single-currency deployments are fine; there is no structural guard against adding two different-currency amounts once multi-currency exists (the quarantined oracle threads `location.currency_minor_unit`, `attic/apps-api/src/routes/orders.ts:563`).
- **Fix:** attach a currency to the money type / Order and reject cross-currency addition.

### M6 · Dead defensive checks in `money.rs` (non-exploitable, but misleading) · **INFO** · CONFIRMED
- `money.rs:9` `if amount != amount` (NaN test on an `i64` — never true) and `money.rs:37` `if subtotal % 1 != 0` (`i64 % 1` is always 0) are no-ops carried over from the TS port. They imply validation that does not occur. Harmless but should be removed or replaced with real bounds checks (M4).

**Note — legacy oracle is correct but dormant:** `attic/apps-api/src/routes/orders.ts` prices server-side from the DB catalog (`:460,:485,:506-565`), client sends only `product_id`/`modifier_ids`/`quantity`, integer `money.ts` with BigInt tax math. This is the correct pattern — but it is quarantined and not wired.

---

## 5. PII / GDPR FINDINGS

Verdict: **RLS on core PII tables HELD; no IDOR; anonymizer is a REAL one-way redactor but INCOMPLETE; no DSAR/export endpoint exists.**

### P1 · GDPR erasure does NOT cascade to the customer's orders · **HIGH** · CONFIRMED
- `attic/apps-api/src/workers/anonymizer-gdpr.ts:62-65` calls `anonymize({ scope:'gdpr', subject:{ customerId } })`. In `attic/apps-api/src/lib/anonymizer/index.ts:73-84`, a `subject.customerId` runs **only** `anonymizeCustomer`; orders are touched only by the age-based retention sweep (`:95-100`, gated on `scope==='retention'`).
- **Leak:** after a completed GDPR erasure, every order of that customer keeps `delivery_address`, `delivery_lat`, `delivery_lng`, `delivery_instructions` until it independently ages past `retention_days` (default **365**, max 2555) — the request is marked `completed` while this PII is intact.
- **Fix:** in the gdpr scope, enumerate `SELECT id FROM orders WHERE customer_id=$1 AND anonymized_at IS NULL` and `anonymizeOrder` each in the same transaction.

### P2 · `anonymizeOrder` forgets most order-level PII (incl. precise geolocation) · **HIGH** · CONFIRMED
- `anonymizer/index.ts:210-217` nulls only `client_ip_hash` and `delivery_address`. It leaves `delivery_lat`, `delivery_lng` (`orders` cols `1780310074262_orders.ts:29-31` — precise home GPS; reverse-geocodes to the address just "erased"), `delivery_instructions` (free-text), `preferences` jsonb (`:37`), `delivery_photo_key`, `pickup_code`. The seam migration comment (`1780421100060:72`) claims "PII fields set to NULL" — **false** for lat/lng and instructions.
- **Fix:** extend the `UPDATE orders SET …` to null `delivery_lat`, `delivery_lng`, `delivery_instructions`, redact `preferences`, and purge `delivery_photo_key` from storage (as the avatar path does).

### P3 · Raw phone survives erasure in `phone_otp` and `gdpr_erasure_requests.subject_phone` · **HIGH** · CONFIRMED
- `phone_otp` stores plaintext phone (`routes/customer/otp.ts:78-82` inserts raw `phone`); `gdpr_erasure_requests.subject_phone` stores raw phone (`anonymization-seam.ts:16`, inserted `routes/owner/gdpr.ts:82-85`). The anonymizer touches neither table.
- **Leak:** an "erased" person remains identifiable by raw phone in (i) unexpired/consumed `phone_otp` rows and (ii) the `subject_phone` of their own erasure request.
- **Fix:** on erasure `DELETE FROM phone_otp WHERE location_id=$1 AND phone=$2` and null `subject_phone` once `status='completed'`; longer term store only `phone_hash` in `phone_otp` (the pattern `customer_otp_sessions` already uses).

### P4 · `messenger_handle` on customers never redacted · **MEDIUM** · CONFIRMED
- `anonymizeCustomer` (`index.ts:133-141`) redacts `phone`, `name`, `marketing_opt_in`, avatar — but a Telegram/messenger handle is written to the customers row (`routes/orders.ts:580-584`) and never nulled. A messenger handle is direct contact PII.
- **Fix:** null `messenger_kind`/`messenger_handle` in `anonymizeCustomer`.

### P5 · Raw phone/address written to logs on some paths · **LOW–MEDIUM** · CONFIRMED
- `attic/apps-api/src/index.ts:10` logs raw `input.customer.phone` (stub `planOrder`, likely dead but unmasked); `attic/apps-api/src/notifications/workers/index.ts:438` logs `address=${target.address}`; `attic/apps-api/src/notifications/adapters/push.ts:10` logs `target.address`.
- **Good/contrast:** the OTP path masks correctly (`routes/customer/otp.ts:100` `maskPhone`); `message-bus.ts:54-57` deliberately logs channel+byte-length, never the raw NOTIFY payload; `routes/customer/track.ts` never logs the raw code.
- **Fix:** remove/mask the `index.ts` stub log; route `target.address` through a masker before logging.

### P6 · OTP-session verify lookup is not tenant-scoped (relies on token secrecy) · **LOW** · SUSPECTED
- `routes/customer/otp.ts:158-162` looks up `customer_otp_sessions WHERE token_hash=$1` with **no** `location_id` predicate, on the BYPASSRLS `db` pool. Safe in practice (`token_hash` = sha256 of a random 32-byte token), but isolation rests on token unguessability rather than the RLS policy the migration defines (`anti-fake-signals.ts:92-95`).
- **Fix:** add `AND location_id = $loc` so tenant scoping is defense-in-depth, not token-only.

### Anonymizer assessment (mechanism)
Real, one-way, in-place: `UPDATE` inside a `FOR UPDATE` transaction, idempotent via `anonymized_at` (`index.ts:114-141,191-217`); phone overwritten with `'anon_' || gen_random_uuid()`, name set NULL — original destroyed, not recoverable. Audit log stores only subject UUID + counts, no PII (`:273-290`). The mechanism is sound; the **field set is under-specified** (P1–P4). The most damaging misses: order `delivery_lat/lng` + `delivery_instructions`, customer `messenger_handle`, and raw-phone residue in `phone_otp` / `gdpr_erasure_requests.subject_phone`.

### PII positives (checked, PASS)
- OTP codes hashed with **argon2id** (`attic/apps-api/src/lib/otp.js` / `routes/customer/otp.ts:68,168`), never stored raw; rate-limited (send 3/15min, verify 5/15min + attempts lockout, `otp.ts:36,114,152-174`); no user-enumeration (generic 400/410 regardless of phone existence). OTP globally disabled until an SMS gateway exists (`otp.ts:9`).
- No IDOR: `GET /orders/:id` forces `user.orderId === id` for customers + `withTenant` for owner/courier, explicit 401 for anonymous (`routes/orders.ts:810-822`); `customer/orders.ts:46` scopes by `o.customer_id = token.sub`; `owner/gdpr.ts` masks `customerId`/`subjectId` (`maskName`, :151,:204,:214) and wraps all reads in `withTenant`, three-layer auth (`verifyAuth` + `requireRole(['owner'])` + `requireLocationAccess`, :28-30), parameterized cursor.
- `owner/reveal-contact.ts:69-74` returns full name+phone but is owner-only, rate-limited (10/min), and audited — by-design, acceptable.
- **No customer-facing DSAR/data-export (Art. 15) endpoint exists** — the "export" surface in the brief is absent. Informational gap (no leak, but a compliance hole): erasure is owner-initiated only (`gdpr_erasure_requests.requested_by_owner_id`), a data-controller model.

---

## 6. PRIORITIZED REMEDIATION (reactivation gate for the quarantined stack)

| # | Finding | Severity | Class |
|---|---|---|---|
| 1 | R1 `couriers` no RLS (password_hash) | HIGH | reactivation-blocker |
| 2 | R2 `telegram_login_tokens` no RLS (owner auth) | HIGH | reactivation-blocker |
| 3 | R3 fail-open `orders`/`order_items`/`customers` anonymous policies | HIGH | reactivation-blocker |
| 4 | M1/M2 client-side authoritative pricing (LIVE) | HIGH | live — do not launch pricing |
| 5 | P1/P2/P3 GDPR erasure incomplete (orders, geo, raw phone) | HIGH | compliance-blocker |
| 6 | R4 six no-RLS tenant tables | MED–HIGH | reactivation-blocker |
| 7 | R8 dead `verify:rls` gate + no boot-guard | HIGH (gate) | restore before reactivation |
| 8 | R9 RLS-is-not-the-boundary (systemic) | HIGH | architectural decision |
| 9 | R5/R6/R7/R12 disabled RLS, USING(true), hidden BYPASSRLS, inert de-priv role | MED | reactivation-blocker |
| 10 | R10/R11, M4/M5/M6, P4/P5/P6 | LOW–MED | hardening |

---

## 7. METHODOLOGY / EVIDENCE BASE

- Enumerated all 76 app tables across 140 migrations (`grep CREATE TABLE`), cross-referenced `ENABLE`/`FORCE`/`CREATE POLICY`/`DISABLE`/`DROP POLICY` per table (latest-migration-wins), read the crux migrations firsthand (`1780421100065` lockdown, `1780310044711` roles, `1790000000015` operational role, `1780315000000`/`1780338981782`/`1780338981783` anonymous policies, `1780310074262` orders/idempotency, `1780421100048` backups, `1790000000041` access_requests).
- Read the live money engine (`kernel/src/money.rs`) and the live pg touchpoint (`packages/platform/src/message-bus.ts`) firsthand; read the anonymizer (`attic/apps-api/src/lib/anonymizer/index.ts`), GDPR route (`attic/apps-api/src/routes/owner/gdpr.ts`) and OTP route (`attic/apps-api/src/routes/customer/otp.ts`) firsthand.
- Money/web-glue, role/boot-guard, and PII/GDPR lanes corroborated by independent sub-audits; every asserted `file:line` above was verified against source. CONFIRMED = grep/read-verified; SUSPECTED = inferred from schema/config without live row/deploy confirmation.
