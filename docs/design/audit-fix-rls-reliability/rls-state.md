# RLS state fact sheet — verified 2026-07-03 (companion to proposal.md §2)

Source of truth: `packages/db/migrations/*.ts`, net effect in apply order. Key structural fact:
**`couriers` and `courier_sessions` (credential/session tables) never get RLS enabled anywhere** —
their only protection is that the pool role is BYPASSRLS-privileged, which is exactly what the flip removes.

## 1. The unscoped anonymous policies (§E-1)

- `1780338981783_anonymous_orders.ts:4-11`:
  `CREATE POLICY anonymous_select ON orders FOR SELECT USING (app_current_user() IS NULL);`
  (same on `order_items`). Fail-open: any no-GUC connection sees ALL rows, all tenants; permissive
  OR-combination nullifies `tenant_isolation`.
- `1780338981782_customer-anonymous-update.ts:5-12`:
  `anonymous_update ON customers FOR UPDATE USING (app_current_user() IS NULL)` — **no WITH CHECK**;
  plus `anonymous_select` same shape.
- `1780315000000_customer-rls.ts:5-29`: anon INSERTs — `orders WITH CHECK (app_current_user() IS NULL)`
  (no location scoping); `order_items WITH CHECK (EXISTS (SELECT 1 FROM orders o WHERE o.id =
  order_items.order_id))` (scope inherited from whatever the orders SELECT policy admits — today: everything);
  `customers` and `idempotency_keys` same unscoped shape; `idempotency_keys anonymous_select USING (key IS NOT NULL)` ≈ USING(true).
- Already tracked: `docs/regressions/REGRESSION-LEDGER.md` row 50 — "C1 anonymous RLS policies
  fail-open — DESIGNED-NOT-SHIPPED… latent today ONLY because dowiz_app is BYPASSRLS."

## 2. No-RLS credential tables (§E-2)

- `1780421029538_couriers.ts` — creates `couriers` with `password_hash text NOT NULL` (:13),
  `email/phone/full_name_encrypted` bytea (:7-11). No ENABLE/FORCE/policy here or in any later
  migration (later touches are columns only: `1790000000038:15-17`, `1790000000074:20-23`).
  Only sibling `courier_locations` gets RLS (:30-32, throw-on-unset form, rewritten missing-ok by 077 RC5).
- `1780421032856_courier-sessions.ts` — `token_hash text NOT NULL` (:9), `user_agent_hash`,
  `ip_hash` (:16-17). No RLS ever.

## 3. Over-broad public/ops policies (§E-3/E-5)

- `1780338909301_public-locations-rls.ts:6-9` — `public_select ON locations FOR SELECT USING (true)`.
- `1780421100048_backup-metadata.ts:21-23` — ENABLE (not FORCE) + `backup_metadata_owner_read FOR
  SELECT TO authenticated USING (true)`.
- `1780421100049_backup-audit-log.ts:15-17` — same shape.
- `1780421100050_backup-system-policy.ts:5-7` — `"System can do everything" … FOR ALL TO
  deliveryos_api_user USING (true) WITH CHECK (true)` (legacy NOLOGIN role).

## 4. FORCE inventory (net of NO FORCE)

FORCEd today (abridged; full list in the lane transcript): locations, memberships, organizations,
categories, products, modifier*, menu_versions, translations, customers, orders, order_items,
idempotency_keys, order_status_history, order_messages, order_ratings, delivery_tiers, reservations,
order_routes, courier_assignments, webhook_endpoints, api_keys, domain_verifications,
recurring_orders, promotions, location_alerts, courier_locations, courier_invites, courier_shifts,
courier_positions, courier_audit_log, courier_payouts, settlement_items, settlement_audit_log,
courier_dispatch_queue, customer_signals, velocity_events, customer_otp_sessions, phone_otp,
customer_devices, notification_prefs_audit, telegram_action_nonces, location_themes, theme_versions,
telegram_connect_tokens, owner_notification_targets, import_sessions, gdpr_erasure_requests,
anonymization_audit_log, payments, payment_events.

**ENABLE-but-not-FORCE** (owner-role bypass hole persists post-flip): `backup_metadata`,
`backup_audit_log`, `customer_track_grants` (token_hash), `courier_cash_ledger`, `delivery_trace`,
`ingredients`, `recipe_components`, `provision_grants` (token_hash), `claim_invites` (token_hash),
`access_requests`, `acquisition_sources`, `product_media`, `menu_schedules`, `order_sensor_events`,
`funnel_events`.

**Deliberate firebreak exceptions (keep):** `users`, `ops_worker_heartbeat`, `auth_refresh_tokens`.
**CORRECTION (RESOLVE round, 2026-07-03 — see `resolution.md §0.1`):** the v1 text here said
"ENABLE then `:69-73` NO FORCE" — that misread the migration's `down()`. The actual firebreak is
`1780421100065` up() STEP A2: ENABLE **+ FORCE** on all three tables, then `1790000000077:24-30`
RC2 adds the role-restricted permissive policy `ops_all FOR ALL TO dowiz_app USING (true) WITH
CHECK (true)`. So the convention = **FORCE + role-restricted-to-`dowiz_app` policy** (deny-by-default
for every other role; pre-context auth reads keep working post-flip). This is the pattern MIG-1
(v2) mirrors for `couriers`/`courier_sessions`.

## 5. GUC functions

- `app_current_user()` — `1780310071220_core-identity.ts:70-73`:
  `SELECT NULLIF(current_setting('app.user_id', true), '')::uuid` (STABLE, missing-ok).
- `app_member_location_ids()` — `:76-80`, SECURITY DEFINER, memberships lookup (any role, any status='active').
- `app_owner_location(p_user, p_location)` — `1790000000077:33-42`, DEFINER, owner-role-filtered.
- **`app_current_tenant()` does NOT exist.** Policies read the GUC inline; two dialects:
  throw-on-unset `current_setting('app.current_tenant')::uuid` (old, e.g. couriers.ts:32) vs
  missing-ok `NULLIF(current_setting('app.current_tenant', true), '')::uuid` (mig 077 convention, :49,:73).

## 6. Where the flip actually lives

- Mig `1790000000077_rls-nobypassrls-phase1-policies.ts` = Phase 1, **policies only, zero FORCE,
  zero role change**, provably inert under BYPASSRLS (header :1-11); RC1 anon INSERT siblings,
  RC2 role-restricted users/auth_refresh_tokens, RC3 app_owner_location, RC4 courier-context order
  policies, RC5 missing-ok rewrites, RC6 re-keys (incl. `customer_owns` on `customer_devices` —
  the policy that kills the notify worker's autocommit GUC read post-flip).
- The `ALTER ROLE dowiz_app NOBYPASSRLS` flip is staged **out-of-tree** as an approval-pending
  artifact: `docs/security/OPERATIONAL-ROLE-nobypassrls.migration.ts` (:38-46), sequenced as
  Phase 3 (staging) / Phase 4 (prod) in `docs/design/pg-privilege-hardening/remediation-plan.md:236,272`.
- Live app role: `dowiz_app` LOGIN **BYPASSRLS** today, provisioned outside migrations
  (`.github/workflows/ci.yml:101`; `docs/ops/PROD-UNBLOCK-RUNBOOK-2026-07-03.md:50`).
  `deliveryos_operational_user` (read pool) is already NOBYPASSRLS (`1790000000015:19`).
- B3 ramp mechanism (ADR-b3-deep-auth-hardening decision #1): dark `dowiz_app_rls` NOBYPASSRLS role +
  `SET LOCAL ROLE` inside the helper txn per lane flag — the txn my proposal's `withTenantTx` provides.

## 7. Credential-column map (what the flip must actually protect)

| Table | Secret columns | RLS today |
|---|---|---|
| `couriers` | password_hash, *_encrypted PII | **none** |
| `courier_sessions` | token_hash, family_id, ip/ua hashes | **none** |
| `auth_refresh_tokens` | token_hash | FORCE + `ops_all TO dowiz_app` (firebreak — corrected, see §4) |
| `customer_otp_sessions` | token_hash | FORCE ✓ |
| `customer_track_grants` | token_hash | ENABLE only |
| `provision_grants` | token_hash | ENABLE only + `FOR ALL USING(true)` (self-mint, §E) |
| `claim_invites` | token_hash | ENABLE only + `FOR ALL USING(true)` (self-mint, §E) |
| `users` | password_hash | FORCE + `ops_all TO dowiz_app` (firebreak — corrected, see §4) |
