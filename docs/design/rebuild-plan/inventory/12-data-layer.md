# 12 — Data Layer Inventory & Transition Map (Lane C)

- **Date:** 2026-07-04 · **Lane:** C (data layer) · **Frame:** per `06-complete-rebuild-stack.md` the
  schema is **UNCHANGED** and data **never migrates** — the rebuild is code-only (Rust/axum/sqlx)
  against the live Supabase Postgres. This map is what the Rust layer must **honor**, not a redesign.
- **Source of truth for this census:** `packages/db/migrations/` (157 forward-only node-pg-migrate
  files, 🔴 protected — nothing here touches them) + staged out-of-tree drafts in
  `docs/design/*/migration-drafts/`. Live-DB drift is NOT verified here (see §11 gaps;
  `scripts/ci-schema-drift.mjs` + `scripts/ci-migration-preflight.mjs` exist for that).
- **Method:** every count below is machine-extractable; the extraction command is given per census.
  `down()` bodies are excluded everywhere (forward-only: down never runs). Last-write-wins applied
  for re-`CREATE`d objects (policies via `DROP POLICY`+`CREATE POLICY`; no `ALTER POLICY` exists in
  the corpus — verified `grep -c "ALTER POLICY" packages/db/migrations/*.ts` → 0 files).

## 0. Headline counts

| Census | Count | Extraction (all scoped to `packages/db/migrations`, up() sections only) |
|---|---|---|
| Migrations | **157** | `ls packages/db/migrations \| wc -l` |
| Tables (public schema, live) | **84 migrated + 2 out-of-band = 86** (+ `pgboss.*` schema) | `CREATE TABLE` (82 raw uniq incl 2 recreates) + `pgm.createTable` (4) − 0 net drops (only `courier_invites` & `promotions` dropped in up(), both recreated) + `platform_admins`/`platform_admin_audit_log` applied manually from `docs/security/platform-admins-and-audit.migration.ts` (§6) |
| RLS: FORCE / ENABLE-only / NONE | **56 / 18 / 11** | `ALTER TABLE … FORCE\|ENABLE ROW LEVEL SECURITY` per table (dedup, `public.` normalized) |
| Live RLS policies | **103** (117 raw CREATE − drops/recreates) | `CREATE POLICY` last-write-wins after `DROP POLICY` replay |
| DB functions (live defs) | **45** (47 raw names incl down-only) | `CREATE [OR REPLACE] FUNCTION` last-def-wins |
| SECURITY DEFINER fns | **35** (5 with **no** search_path pin 🔴) | `SECURITY DEFINER` within fn span |
| Triggers | **13** live (15 raw names incl re-creates) | `CREATE TRIGGER` |
| Indexes (explicit) | **114** | `CREATE [UNIQUE] INDEX` + `pgm.createIndex` |
| Postgres enums | **10** | `CREATE TYPE … AS ENUM` + `pgm.createType`, `ALTER TYPE ADD VALUE` folded in |
| CHECK / UNIQUE / FK refs | **≥159 / ≥60 / ≥133** | regex refs (not catalog counts — see §11) |
| `current_setting(` refs | **46** in up() (47 incl down) — **0** initplan-wrapped | reconciles lane-4's "48 bare refs" (counting method: ±down/drafts); initplan wrap is still fully pending (REBUILD-PLAN A6, council-gated) |
| Staged draft migrations | **4** (085–088), all operator-gated, none applied | `find docs/design -path '*migration-drafts*'` |

Reproduction: `/tmp` scratchpad script `census.py` (regex parser over up() sections); every number
above restated from its JSON output. Re-derive with the greps in each section.

## 1. Table census (84 public tables)

Legend: **Tenant** = tenant-arm column on the table itself (`location_id`, or `-` = scoped via join/
none); **RLS** F=FORCE, E=ENABLE-only, N=none; **Pol** = live policy count; **Money** = integer
minor-unit columns (ALL money is `integer` + `CHECK (>= 0)` — verified, no numeric money survives);
**Soft** = soft-delete/anonymization columns. Purpose is 1-line from the creating migration.

| Table | Purpose | Tenant | RLS | Pol | Money cols | Soft |
|---|---|---|---|---|---|---|
| users | owner/admin identities (argon2 password_hash) | - | E | 1 | - | - |
| organizations | top-level tenant org | - | F | 2 | - | - |
| locations | venue = THE tenant unit (slug, settings, tz, delivery_paused, kitchen_busy) | - (is the tenant) | F | 3 | - | - |
| memberships | user↔location role (owner/courier/admin) + status — the RLS root | location_id | F | 1 | - | - |
| categories | menu categories | location_id | F | 3 | - | - |
| products | menu items (price integer, prep-time, source, publish state) | location_id | F | 3 | price | - |
| customers | per-location customer records (phone, PII) 🔴 GDPR N1 | location_id | F | 4 | - | anonymized_at |
| orders | THE order aggregate 🔴 (status enum, pricing fields, promised window) | location_id | F | 5 | cash_pay_with, delivery_fee, subtotal, tip_amount, total, discount_total | anonymized_at |
| order_items | line items, price_snapshot 🔴 | - (via orders EXISTS) | F | 3 | price_snapshot | - |
| idempotency_keys | idempotent order create; **PK (location_id, key)** since mig 029 | location_id | F | 3 | - | - |
| location_themes | branding/theme + fonts + google fields per tenant | location_id | F | 1 | - | - |
| webhook_endpoints | planned-feature scaffold | location_id | F | 1 | - | - |
| api_keys | planned-feature scaffold | location_id | F | 1 | - | - |
| domain_verifications | planned-feature scaffold | location_id | F | 1 | - | - |
| recurring_orders | planned-feature scaffold | location_id | F | 1 | - | - |
| promotions | promo rules (recreated by mig 017 promotions_full) | location_id | F | 1 | min_order_amount | - |
| location_alerts | ops alerts per tenant (escalation tiers) | location_id | F | 1 | - | - |
| ops_worker_heartbeat | worker liveness | - | E | 1 | - | - |
| auth_refresh_tokens | owner refresh tokens (ADR-0004 revocation) | - | E | 1 | - | - |
| courier_invites | courier onboarding invites (recreated 1780421031109) | location_id | F | 2 | - | - |
| modifier_groups / modifiers | menu modifiers (price_delta) | location_id | F | 1+1 | price_delta | - |
| product_modifier_groups | product↔modifier-group join (+loc column) | location_id | F | 1 | - | - |
| order_item_modifiers | snapshot of chosen modifiers 🔴 | - (via order_items) | F | 2 | price_delta_snapshot | - |
| product_translations / category_translations | i18n (al/en/uk) | - (via parent) | F | 1+1 | - | - |
| delivery_tiers | distance/fee tiers | location_id | F | 1 | fee | - |
| order_status_history | status transition audit 🔴 | location_id | F | 1 | - | - |
| reservations | scaffold | location_id | F | 1 | - | - |
| menu_versions | menu version counter (cache invalidation; bumped by 8 triggers) | location_id | F | 2 | - | - |
| theme_versions | theme version + public read | location_id | F | 2 | - | - |
| telegram_connect_tokens | owner TG connect flow | location_id | F | 1 | - | - |
| owner_notification_targets | notif targets (+user_id, locale, quiet hours, categories) | location_id | F | 1 | - | - |
| customer_devices | push subscription per customer | - | F | 1 | - | - |
| couriers | courier identities (cross-location) | - | **N** | 0 | - | - |
| courier_locations | legacy courier↔location (superseded by courier_positions, never dropped) | location_id | F | 1 | - | - |
| courier_sessions | courier auth sessions | - | **N** | 0 | - | - |
| courier_audit_log | courier action audit | location_id | F | 1 | - | - |
| courier_shifts | shift open/close (cash reconciliation) 🔴 | location_id | F | 1 | - | - |
| courier_assignments | order↔courier assignment 🔴 (cash_amount/cash_collected, immutability trigger) | location_id | F | 1 | cash_amount, cash_collected | - |
| courier_positions | GPS points (precision-gated trigger; purged by sweep) | location_id | F | 1 | - | - |
| courier_payouts | payout aggregate 🔴 (immutability trigger) | location_id | F | 1 | total_earned | - |
| courier_dispatch_queue | dispatch offers queue | location_id | F | 1 | - | - |
| settlement_items | payout line items 🔴 (throwing tenant policy) | location_id | F | 1 | amount | - |
| settlement_audit_log | settlement audit 🔴 (throwing tenant policy) | location_id | F | 1 | - | - |
| backup_metadata / backup_audit_log | backup worker records | - | E | 1+1 | - | - |
| phone_otp | OTP codes (code-immutable trigger; OTP_ENABLED off) | location_id | F | 1 | - | - |
| customer_signals / velocity_events / customer_otp_sessions | anti-fake seam | location_id | F | 1/2/1 | - | - |
| gdpr_erasure_requests | GDPR erase queue 🔴 (N1 — see §2) | location_id | F | 1 | - | - |
| anonymization_audit_log | GDPR audit | location_id | F | 1 | - | - |
| customer_contact_reveals | contact-reveal audit | location_id | **N** | 0 | - | - |
| upload_audit | upload audit (hardening seam) | location_id | **N** | 0 | - | - |
| free_tier_snapshots | Fly/Supabase free-tier watch | - | **N** | 0 | - | - |
| order_messages | courier↔customer↔owner chat | location_id | F | 3 | - | - |
| analytics_events / analytics_abuse_log / analytics_cwv | analytics (no RLS; grant-gated) | location_id | **N** | 0 | - | - |
| exchange_rates | currency rates (numeric rate! — see §9) | - | **N** | 0 | - | - |
| order_ratings | post-delivery ratings | location_id | F | 1 | - | - |
| customer_track_grants | tracking-page grants (claim-check) | location_id | E | 2 | - | - |
| delivery_trace | delivered-order trace (anonymized by fn) | location_id | E | 3 | total | - |
| courier_cash_ledger | cash custody ledger 🔴 (immutability trigger) | location_id | E | 3 | amount | - |
| telegram_login_tokens | owner TG login | - | **N** | 0 | - | - |
| order_routes | route polylines (throwing tenant policy) | location_id | F | 1 | - | - |
| access_requests | soft access gate requests | - | E | 1 | - | - |
| telegram_action_nonces | TG action nonces | location_id | F | 1 | - | - |
| notification_prefs_audit | notif prefs audit | location_id | F | 1 | - | - |
| product_media | media seam (ADR-0002; kind enum) | location_id | E | 2 | - | - |
| menu_schedules | availability schedules | location_id | E | 2 | - | - |
| order_sensor_events / funnel_events | sensor bus + funnel (dual-arm policies) | location_id | E | 1+1 | - | - |
| ingredients / recipe_components | BOM seams (orphan trigger) | location_id | E | 1+1 | - | - |
| acquisition_sources | demo-builder acquisition pipeline (state enum) | - | E | 1 | - | - |
| provision_grants / claim_invites | provisioning + claim flow (provision_token GUC) | - | E | 1+1 | - | - |
| payments | payment ledger 🔴 (ADR-0017; amount_minor trio) | location_id | F | 1 | amount_minor, captured_amount_minor, refunded_amount_minor | - |
| payment_events | payment event ledger 🔴 | location_id | F | 1 | amount_minor | - |
| import_sessions | menu import sessions — ⚠️ **FORCE RLS + 0 policies** | location_id | F | **0** | - | - |
| modifier_translations / modifier_group_translations | AI translations — ⚠️ **FORCE RLS + 0 policies** | - | F | **0** | - | - |
| notification_outbox_audit | notif outbox audit | location_id | **N** | 0 | - | - |
| `pgboss.job`, `pgboss.version` (+ pg-boss internals) | job queue schema — bootstrapped by mig 011 via pg-boss's own installer (`pgm.noTransaction()`); runtime role has DML-only grants, `migrate:false` | n/a | n/a | - | - | - |

Money reality check: **16 tables carry money columns; every one is `integer` minor units with
`CHECK (>= 0)`** (`orders.cash_pay_with` went numeric→integer in mig `1790000000000`). No
floating-point or numeric money anywhere. 🔴 Any Rust type other than a checked integer newtype is
a regression.

## 2. RLS policy census (103 live policies)

Class distribution (live): **member-fn 58** (`app_member_location_ids()` / `app_owner_location()` /
`app_current_user()` forms) · **tenant-GUC 17** (`current_setting('app.current_tenant')`) ·
**tenant-GUC+member-fn dual 5** · **open/service 15** (`USING (true)`, mostly role-restricted `TO
dowiz_app` or ops tables + 3 `public_select` for storefront) · **other 8** (anon-checkout
`app_current_user() IS NULL` arms, `provision_shadow` INSERT arms keyed on
`app.provision_token`, theme public read).

**The two RLS roots (the Rust layer must set BOTH GUC families):**
- `app.user_id` → `app_current_user()` (missing_ok `NULLIF(current_setting('app.user_id', true),'')`)
  → `app_member_location_ids()` = `SELECT location_id FROM memberships WHERE user_id =
  app_current_user() AND status='active'` (SECURITY DEFINER, **unpinned** 🔴). Owner/admin path.
- `app.current_tenant` → courier/worker/service path (22 policies incl duals).
- Other GUCs in policies/fns: `app.settlement_reversal` (3 refs — settlement reversal escape hatch),
  `app.provision_token` (2), `request.jwt.claim.sub` (3, Supabase-claims fallback in bootstrap fns).

**bare vs initplan:** all 46 `current_setting` refs are **bare** (0 wrapped in `(SELECT …)`
initplan form) — per-row re-evaluation cost on every policy scan. The initplan wrap is a recorded,
council-gated transition task (REBUILD-PLAN A6) — do it **before or during** Phase A parity
measurement, or Rust-vs-Node latency comparisons will be polluted by policy overhead.

**missing_ok vs throw-on-unset:** 20 policies use `current_setting(…, true)` (missing → NULL → row
invisible). **3 policies THROW when the GUC is unset**: `settlement_items.isolate_settlement_items`,
`settlement_audit_log.isolate_settlement_audit_log`, `order_routes.isolate_order_routes` (bare
`current_setting('app.current_tenant')::uuid`). 🔴 Rust consequence: any query touching these three
tables outside a `SET LOCAL app.current_tenant` transaction fails with `55000 unrecognized
configuration parameter`, not an empty result — sqlx error mapping must distinguish this from data
absence, and tests must cover the unset-GUC path.

**B3 NOBYPASSRLS — the open flip.** Runtime role `dowiz_app` currently holds **BYPASSRLS**; every
policy today is dormant for app traffic (defense = grants + app-layer withTenant). Migration 077
(`rls-nobypassrls-phase1-policies`) staged the additive policy set "provably INERT under today's
bypass"; the Phase-3 flip (revoke BYPASSRLS) is **not applied** (plan:
`docs/design/pg-privilege-hardening/remediation-plan.md`, state:
`docs/design/audit-fix-rls-reliability/rls-state.md`). Flip-blockers visible in this census:
- ⚠️ **3 tables with FORCE RLS + ZERO policies** — `import_sessions`, `modifier_translations`,
  `modifier_group_translations` → completely dark post-flip (grounding doc:
  `docs/design/error-contract-parse-token-economy/B-grounding-import-sessions-force-rls.migration.ts`).
- 🔴 **GDPR N1 (`customers`)**: `customers` has **no `app.current_tenant` arm** (member-fn +
  anon arms only; the 077 RC4 courier arm is orders-only) → post-flip the anonymizer's context-free
  connection sees ∅ and "erases" nothing. Structural fix = staged DEFINER draft **088** (below);
  fail-loud worker backstop already shipped (ledger #61).
- 13 RLS-on tables have **no tenant-scoped arm at all** (service/ops-class by design): users,
  ops_worker_heartbeat, auth_refresh_tokens, theme_versions, backup_metadata, backup_audit_log,
  access_requests, acquisition_sources, provision_grants, claim_invites + the 3 zero-policy tables.
- 11 tables have **no RLS**: couriers, courier_sessions, customer_contact_reveals, upload_audit,
  free_tier_snapshots, analytics_events, analytics_abuse_log, analytics_cwv, exchange_rates,
  telegram_login_tokens, notification_outbox_audit — protection is role grants only (mig 065
  lockdown + 080 grant-hardening). The Rust port must NOT assume RLS covers these.

Roles created in migrations: `anon`, `authenticated`, `service_role` (Supabase-compat),
`deliveryos_api_user`, `deliveryos_operational_user`; grants flow overwhelmingly `TO dowiz_app`
(29 grant refs) — `dowiz_app` itself is created out-of-band in Supabase (not in migrations). The
migration role is `dowiz_migrator`/`postgres` (per mig 011 header).

## 3. Functions / triggers / DEFINER census

**45 live functions** (last-def-wins), **35 SECURITY DEFINER**. Families:
- Hot-path readers 🔴: `read_public_menu` (redefined 11×, last in mig 072 c2-read-gate — serves
  EVERY storefront menu read), `read_public_menu_all_locales`, `read_preview_menu`,
  `payment_location_by_provider_ref`.
- RLS helpers: `app_current_user`, `app_member_location_ids`, `app_owner_location`,
  `app_is_shadow_location`.
- Sweep/recon workers (mig 078/079 phase2): `app_sweep_timeout_orders`, `app_sweep_expired_offers`,
  `app_sweep_stale_couriers`, `app_sweep_gps_purge`, `app_sweep_velocity_active_locations`,
  `app_recon_open_shifts`, `app_recon_delivered_cash_mismatch`,
  `app_recon_assignments_missing_courier`, `app_generate_settlements` 🔴,
  `app_velocity_phone_count`, `app_velocity_ip_count`.
- Alert/dwell: `app_alert_*` (5), `app_dwell_*` (2), `app_resolve_order_alerts`,
  `app_raise_customer_signal`, `app_count_active_dwell_alerts`, `app_active_notification_targets`.
- Lifecycle/misc: `bootstrap_owner`, `activate_courier`, `claim_transfer`, `erase_shadow_tenant`,
  `upsert_menu_version`, `bump_menu_version_trigger_fn`, `update_updated_at_column`,
  `product_available_now`, `menu_schedule_matches`, `anonymize_stale_delivery_trace`,
  `validate_location_timezone`, `orders_promised_window_set_once`, `prevent_cash_mutation`,
  `prevent_payout_mutation`, `phone_otp_code_immutable`, `recipe_components_orphan_on_product_delete`.

⚠️ **5 DEFINER fns have NO `SET search_path` pin in their live definition** (ledger #33 "option A"
guardrail exists for NEW fns; the backfill MIG is staged, not landed): `app_current_user`*,
`app_member_location_ids`, `upsert_menu_version`, `bump_menu_version_trigger_fn`, and 🔴
`read_public_menu` (its last redefinition in 072 re-created it WITHOUT the pin). 30 pins exist
elsewhere (mig 078 alone pins 21). *`app_current_user` is not DEFINER but is the security root.
**Recommendation: land the pin backfill BEFORE Phase A** so the Rust port never runs against
unpinned DEFINER hot paths.

**13 live triggers** (state machine + money immutability 🔴):
`trg_bump_menu_version_*` ×8 (categories, category_translations, locations, modifier_groups,
modifiers, product_modifier_groups, products, product_translations → menu cache invalidation),
`courier_assignments_cash_immutable`, `courier_payouts_immutable`, `phone_otp_code_immutable_trg`,
`trg_orders_promised_window_set_once`, `trg_recipe_orphan_product`, `trg_validate_location_timezone`,
`set_promotions_updated_at`. Convention: enforcement triggers **RAISE EXCEPTION** (immutability);
the staged 086 refund trigger deliberately inverts this (swallowing, per breaker N8) — do not
"normalize" it.

**Staged drafts (operator-gated, NOT applied — live in docs/, never in packages/db/migrations/):**
| Draft | What | Gate / hazard |
|---|---|---|
| `1790000000085_settlements-catchup.ts` | M-2: `app_generate_settlements` → catch-up, immutable-once-paid, idempotent + backfill pair | ⚠️ **literal watermark `2026-07-10 00:00:00+00` baked into fn bodies — MUST be re-checked/bumped at apply time (3 occurrences); early literal = double-pay hazard** |
| `1790000000086_refund-due-trigger.ts` | M-1: non-throwing AFTER UPDATE OF status trigger recording refund_due on CANCELLED/REJECTED with paid payment | rides money-audit ADR; deliberately swallowing (N8) |
| `1790000000087_refund-due-reconciler.ts` | M-3: `app_reconcile_refund_due()` — deterministic alarm of last resort, called by timeout-sweep worker each tick | worker call is try/caught; deploy order flexible |
| `1790000000088_gdpr-erase-definer.ts` | N1.1: SECURITY DEFINER erase fn for customers (visibility-safe post-flip) | rides LC4-MIG / GATE-FLIP-E2E |

🔴 Rust-port consequence: these 4 fns/triggers may land **during** the rebuild window — the Rust
money/orders surface must be written against BOTH shapes (pre/post 085–088), or sequenced after.

## 4. Index census (114 explicit indexes)

Hot-table detail (explicit indexes; PKs implicit):
| Table | Indexes | Assessment vs query patterns |
|---|---|---|
| orders | `(location_id,status)`, `(location_id,created_at DESC)`, partial anonymize `(id,location_id)` | ✅ covers admin listing + sweep scans. ⚠️ no index on `customer_id` (customer order-history lookups) |
| order_items | **none** beyond PK | ⚠️ `order_id` FK unindexed — every order fetch joins it; fine at current volume, first candidate when Rust parity-profiles |
| courier_assignments | 6: uniq `order_id`(×2 forms), uniq partial one-active per courier, `(courier_id,status)`, `offered_expires_at` | ✅ strongest-indexed table; encodes dispatch invariants (one active assignment) as partial uniques 🔴 |
| courier_shifts | `(location_id,status)` | ✅ dispatch check |
| courier_positions | `(courier_id,recorded_at DESC)`, `(location_id,recorded_at DESC)` | ✅ tracking reads |
| products / categories | `(location_id,external_key)` uniq (import), `(location_id,name)` uniq on categories | ✅ menu read goes via `read_public_menu` + `menu_versions` cache, not ad-hoc scans |
| payments / payment_events | `order_id`, `location_id` (+ provider_ref lookup via DEFINER fn) | ✅ |
| idempotency_keys | PK **(location_id, key)** (mig 029) — the idempotency contract 🔴 | ✅ composite PK IS the uniqueness guarantee; Rust create-order must keep `ON CONFLICT` semantics against exactly this key |
| courier_dispatch_queue | **none** | ⚠️ queue scans unindexed — same profile-then-add posture |

Gaps are recorded, NOT to be fixed during the port (schema unchanged); they are candidates for
post-parity, council-gated additions.

## 5. Enum / constraint census (10 enums)

| Enum | Values (order preserved; `(+)` = added later via ALTER TYPE) |
|---|---|
| **order_status** 🔴 | `PENDING, CONFIRMED, PREPARING, READY, IN_DELIVERY, DELIVERED, REJECTED, CANCELLED, SCHEDULED, PICKED_UP` |
| order_type | `delivery, pickup, scheduled` |
| payment_method | `cash, crypto(+), card(+)` |
| payment_outcome 🔴 | `pending, paid_full, paid_partial, refused_payment, refused_goods, customer_cancelled_on_door` |
| membership_role | `owner, courier, admin` |
| membership_status | `active, suspended, removed` |
| message_sender | `courier, customer, owner` |
| product_media_kind | `image, video, spin, model` |
| product_source | `owner, imported, ai_inferred, place` |
| acquisition_state | `SOURCED, PLACE_INGESTED, MENU_EXTRACTED, ENRICHED, PROVISIONED, VERIFIED, CLAIM_OFFERED, CLAIMED, MENU_NOT_FOUND, LOW_QUALITY, MANUAL_REVIEW, DISQUALIFIED, ABANDONED` |

🔴 **State-machine reality:** the DB enum stores the *vocabulary*; the *transition rules* live in
app code (orderStatusService) + partial triggers (`promised_window_set_once`, cash/payout
immutability) + sweep fns (`app_sweep_timeout_orders` cancels). The Rust port must re-encode
transitions as an exhaustive `match` on a `#[derive(sqlx::Type)] #[sqlx(type_name="order_status")]`
enum — any silent variant addition breaks decode at runtime, so the boot guard should also assert
enum cardinality (see §8). Constraints: ≥159 CHECK refs (all money `>= 0`, GPS precision, status
guards), ≥60 UNIQUE refs (idempotency PK, `courier_one_active_assignment` partial uniques,
`(location_id, external_key)` import keys, `(location_id,name)` categories), ≥133 FK refs (note
`order_items.product_id` is `ON DELETE SET NULL` per mig 023 — snapshot survives product delete).

## 6. Consumers map (table → API surface)

**Method/counts** (greps from `apps/api/src`): **1,171 `.query(` sites across 118 files**; 84
`withTenant()` invocations (shared helper); ~897 `client.query` inside held-connection transactions;
**274 raw-pool sites** (no shared wrapper). 925 SQL-literal table refs resolved against the
migration table list → **78 tables actually consumed**; false positives excluded (`FROM app_*(...)`
are FUNCTION calls, not tables).

**Hottest tables by access sites:** orders 96 · locations 90 · courier_assignments 71 · products 50
· courier_shifts 37 · memberships 31 · owner_notification_targets 30 · customers 29 · couriers 27 ·
categories 26.

**Strangler-sequencing view — tables each surface cutover touches** (rebuild phases per 06-doc §3):

| Cutover surface | Tables touched (R=read W=write) |
|---|---|
| **Phase A: storefront-read** (`spa-proxy.ts`, `public/menu.ts`, `storefrontService.ts`) | locations R, location_themes R, theme_versions R, products/categories R (via `read_public_menu` DEFINER fn — port = keep calling it), menu_versions R, delivery_tiers R, product_media R, menu_schedules R, telegram_action_nonces R/W, analytics_* W (telemetry), funnel_events W, exchange_rates R |
| **auth** (`auth/local.ts`, `auth.ts`, `plugins/auth.ts`) | users R/W, memberships R, auth_refresh_tokens R/W, organizations R, locations R, telegram_login_tokens R/W |
| **catalog/admin (owner)** | products, categories, modifiers, modifier_groups, product_modifier_groups, product_translations, category_translations, product_media, menu_schedules, menu_versions, import_sessions, modifier(-group)_translations, location_themes, theme_versions, promotions, owner_notification_targets, notification_prefs_audit, location_alerts, customer_signals, gdpr_erasure_requests, customer_contact_reveals, backup_metadata/audit R |
| 🔴 **orders/money** (`orders.ts`, `orderStatusService.ts`, `order-persistence.ts`, `payments-webhook.ts`, customer routes) | orders, order_items, order_item_modifiers, idempotency_keys, customers, payments, payment_events, order_status_history, order_ratings, order_messages, customer_track_grants, phone_otp, customer_otp_sessions, velocity_events, order_routes R |
| 🔴 **courier/realtime** (courier routes, dispatch, WS) | couriers, courier_sessions, courier_shifts, courier_assignments, courier_positions, courier_dispatch_queue, courier_invites, courier_audit_log, courier_locations(legacy), courier_payouts, settlement_items, settlement_audit_log, courier_cash_ledger, delivery_trace, order_sensor_events |
| **workers/jobs** | all sweep/recon/dwell/alert tables via the **RPC layer** (24 `app_*` DB functions — table access invisible to app-code grep; the Rust port keeps calling them or must re-council each), backup_*, free_tier_snapshots, upload_audit R, access_requests, gdpr_erasure_requests, exchange_rates W, pgboss.job/schedule R (5 direct health-check sites: `server.ts:269`, `order-timeout-sweep.ts:49`, `reconciliation.ts:235`, `delivery-trace-retention.ts:68`, `access-request-retention.ts:147`) |
| **platform-admin** (`lib/platform-admin.ts`) | platform_admins R, platform_admin_audit_log W — ⚠️ **BOTH OUT-OF-BAND tables** (see below) |
| **acquisition/provisioning** (`modules/acquisition/*`) | acquisition_sources, provision_grants, claim_invites, organizations W, locations W, products W, categories W, menu_versions W |

**Findings the Rust port must honor:**
- ⚠️ **Out-of-band tables**: `platform_admins` + `platform_admin_audit_log` exist in the LIVE DB but
  in NO migration — created via `docs/security/platform-admins-and-audit.migration.ts` (B4,
  applied manually); mig 080 revokes their writes from `dowiz_app` and marks them "managed
  out-of-band". Consequence: the sqlx offline-prepare shadow DB (§9) can NOT be built from
  migrations alone — it needs migrations + the out-of-band file(s), or `cargo sqlx prepare` fails
  on platform-admin queries. Live table count = **86** (84 migrated + 2 out-of-band) + pgboss.
- 🔴 **UUID-as-capability on orders/order_items**: the `anonymous_select` policy (`USING
  (app_current_user() IS NULL)`) grants full cross-tenant SELECT whenever no GUC is set — every
  raw-pool query against orders/order_items is deliberately DB-unrestricted; isolation for
  anon/customer/courier reads lives ONLY in app-code `WHERE id = $1` ownership checks. Porting
  "assume RLS covers it" breaks either functionality or security. The Rust orders surface must make
  the capability check a typed, mandatory parameter, not a convention.
- ⚠️ `order_messages`: FORCE RLS + member-fn policy, but the route runs raw-pool with a JS-side
  membership check (`routes/order-messages.ts:57-61`) — DB policy is dead weight on this path today.
- ⚠️ `upload_audit`: read by free-tier-watch worker, **no writer anywhere in apps/api** — locate the
  writer (infra/out-of-repo?) before porting or the sum-query reads a frozen table.
- **Zero-consumer tables (7)**: api_keys, domain_verifications, ingredients, recipe_components,
  recurring_orders, reservations, webhook_endpoints — scaffolds with no API consumer; **no Rust port
  work**, but they still exist in the live schema (offline-prepare must include them).
- Tenant-scoping is **Mixed** for most hot tables (both withTenant and raw-pool paths exist per
  table) — the per-site discipline, not the table, determines safety; see §7 GUC findings.

## 7. Connection / session reality → sqlx pool plan

(Verified with file:line by the Lane-C connections sweep; corroborates `04-data-infra-reliability.md`
§verified-facts and adds new findings.)

**URLs (no plain `DATABASE_URL` exists):** schema at `packages/config/src/index.ts:7-9` (+ optional
`DATABASE_URL_ADMIN` at `:133` for restore-sandbox only).
| Var | Port/mode | Node consumer | Purpose |
|---|---|---|---|
| `DATABASE_URL_OPERATIONAL` | Supavisor **transaction :6543** | `packages/db/src/index.ts:17-42` | hot-path CRUD; `statement_timeout='10s'` per connect; **hard-fails if `current_user='postgres'`** (superuser guard) |
| `DATABASE_URL_SESSION` | Supavisor **session :5432** | `packages/db/src/index.ts:48-63` | LISTEN/NOTIFY bus, advisory locks, DDL; `statement_timeout='30s'`, max 3 |
| `DATABASE_URL_MIGRATIONS` | session :5432 | `scripts/migrate-runner.ts:47`; reused as backup pool `apps/api/src/server.ts:213-217` | migrations (DDL role) + backup manifest queries |
| `DATABASE_URL_ADMIN` (opt) | admin | `apps/api/src/lib/restore-sandbox.ts:12-14` | create/drop restore-verify sandbox DBs |

**Six runtime pool instances today** (budget doc: `docs/connection-budget.md`): operational(API,
max 20 — budget doc says 8, drift), session/message-bus(API, max 3 — bus dedicates one checked-out
client to `LISTEN`, NOTIFYs go through the pool: `packages/platform/src/message-bus.ts:14-28,118-120`;
`RedisMessageBus` is a name-only alias of `PgMessageBus`), backup(API, max 2), pg-boss(API — clones
OPERATIONAL url and **force-rewrites port to 5432**, `server.ts:239-245`; `schema:'pgboss'`,
`migrate:false`, max 4), session(worker), pg-boss(worker — ⚠️ known bug: falls back to OPERATIONAL
:6543, wrong mode for LISTEN/advisory locks, `queue-provider.ts:95`; do NOT port this default).

**GUC discipline (the withTenant contract):** canonical helper
`packages/platform/src/auth/tenant.ts:3-21` = `BEGIN` → `SELECT set_config('app.user_id', $1,
true)` (is_local) → work → `COMMIT`/`ROLLBACK` → release. A second private clone keyed on
`app.current_tenant` lives in `apps/api/src/workers/courier-events.ts:26-40`. Ref volume:
`app.current_tenant` ~102 sites, `app.user_id` ~34, `app.provision_token` 1 writer,
`app.settlement_reversal` **no app writer** (trigger-read escape hatch, set manually by operator).

⚠️ **Six latent GUC-bug classes found — MUST NOT be ported as-is** (all currently masked by
`dowiz_app` BYPASSRLS; they detonate at the B3 flip):
1. `set_config(..., false)` (session-scoped) on the **transaction-pooled** pool, no RESET anywhere:
   `spa-proxy.ts:771`, `owner/onboarding.ts:75` — cross-tenant GUC leak on connection reuse.
2. `set_config(..., true)` with **no BEGIN** (dies in its own autocommit txn before the next
   statement): `courier/assignments.ts:111` (one outlier handler; the file's other 8 handlers are
   correct), `owner/couriers.ts:152`, `owner/signals.ts:207`, `customer/push.ts:35,72`,
   `notifications/workers/index.ts:122`, `telegram-webhook.ts:281,411,631` (whole file has zero
   BEGIN/COMMIT yet calls `updateOrderStatus` after the set_config).
3. `Pool.query()` set_config then `Pool.query()` data query — **different physical connections
   guaranteed** under txn pooling: `courier/settlements.ts:25,46,59,62,75,79`.
The Rust rebuild's single biggest data-layer win: there is exactly ONE way to touch a tenant table —
a `with_tenant(pool, TenantCtx)` combinator that owns the txn; make the raw pool unreachable from
route code (visibility, not convention).

**sqlx pool plan (port mapping):**
| Node reality | Rust/sqlx |
|---|---|
| operational pool :6543 | `PgPool` A; `after_connect`: `SET statement_timeout='10s'` + assert `current_user <> 'postgres'`. ⚠️ sqlx prepares+caches statements by default — Supavisor txn mode & cached statements conflict (the Node code avoids named statements deliberately): either `statement_cache_capacity(0)` / `persistent(false)` on this pool, or route through :5432 session with a larger max — **decide in Phase A spike, measure both** |
| session pool :5432 | `PgPool` B (small) for advisory locks + anything session-scoped |
| LISTEN/NOTIFY bus | `sqlx::postgres::PgListener` — **requires session mode (:5432)**; one listener task, NOTIFYs via pool B (mirrors the Node split) |
| pg-boss :5432 rewrite | new Rust queue (Lane A) gets its own pool on :5432; never inherit the worker's :6543 fallback bug |
| withTenant | `with_tenant`: `pool.begin()` → `query("SELECT set_config($1,$2,true)")` → closure → commit. GUC pair: `app.user_id` (owner path) AND `app.current_tenant` (courier/service path) — both roots exist in policies (§2) |
| backup/migrations pool | not needed as a standing pool in Rust; migrate subcommand opens its own :5432 connection |

**Boot schema-head guard (to replicate):** `apps/api/src/lib/schema-guard.ts:24-66` — compares
build-time constant `__EXPECTED_MIGRATION_HEAD__` (esbuild define = lexically-last migration
filename, stamped by `scripts/build-apps.ts:33-51`) against `SELECT name FROM pgmigrations ORDER BY
id DESC LIMIT 1`; missing-table ⇒ FATAL `process.exit(1)`; transient errors ⇒ warn-and-boot. Runs
against the session pool at `server.ts:224-226`. Rust equivalent in §8.3 (compiled-in
`MIGRATOR.migrations.last()` replaces the esbuild define — strictly better: the head travels inside
the binary).

**Migration execution today:** Fly `release_command = "dist/migrate/index.cjs"` (`fly.toml:12`) →
`scripts/migrate-runner.ts` with `{direction:'up' hardcoded, count:Infinity, migrationsTable:
'pgmigrations', singleTransaction:true, checkOrder:false (two out-of-order platform migrations,
documented), sslmode=no-verify appended}` + a prod/staging `FLY_APP_NAME`×`NODE_ENV` cross-check
that FATAL-exits (`migrate-runner.ts:32-45`) — port this cross-check into the Rust migrate
subcommand too. Forward-only is **process**, not file shape: all 157 files export `down()` (≈73 are
no-op stubs) but production only ever runs `up`. Runbook: `docs/runbooks/prod-db-migrations.md`.

## 8. DECISION — new-era migration tooling

**Verdict: `sqlx::migrate` (embedded), node-pg-migrate frozen read-only. Refinery rejected** (weaker
documented transaction/no-transaction/locking story; second dependency for zero gain when sqlx is
already the data layer).

Grounding (researched 2026-07-04):
- **Coexistence is clean by construction**: sqlx keeps its own `_sqlx_migrations` table
  (`version BIGINT PK, description, installed_on, success, checksum BYTEA, execution_time`); it
  never reads `pgmigrations`. The 157-row history stays applied and untouched — `pgmigrations`
  becomes a frozen historical record (no new rows, ever). Table name is configurable via `sqlx.toml`
  in current sqlx if a rename is ever wanted (launchbadge/sqlx #1835/#3766).
- **Forward-only preserved & enforced harder than today**: sqlx checksums every migration file and
  aborts on mismatch; the embedded `Migrator` (`sqlx::migrate!()`) only ever applies up — reverts
  require the CLI, which won't exist in the image.
- **Scratch-image compatible**: `sqlx::migrate!()` embeds SQL in the binary; **no sqlx-cli in the
  runtime image**. Fly: `release_command = "/app/server migrate"` (a subcommand calling
  `MIGRATOR.run(&pool)`), same binary, fits the 15–25 MB static image.
- **Locking/transactions**: pg advisory lock taken by default (toggleable, PR #2063); one
  transaction per migration; `-- no-transaction` header supports `CREATE INDEX CONCURRENTLY`
  (single-statement files only — issue #3693).
- **Supavisor pitfall**: run the migrate subcommand against **port 5432 (direct/session)** — never
  6543 transaction mode (advisory locks + prepared statements break across statement-level
  connection reassignment).
- **Frozen-but-alive node-pg-migrate rejected as the forward path**: Fly's release_command runs the
  same image as the app → keeping Node "only for migrations" either bloats the static image with a
  Node runtime or forks migrations into a separate out-of-band pipeline, losing the
  release_command/boot-guard coupling.

**Coexistence plan:**
1. `pgmigrations` frozen at 157 rows (or 157+N if operator lands 085–088 via node-pg-migrate before
   cutover — the freeze point is "whatever head exists at Rust cutover").
2. First Rust-era migration: `sqlx migrate add rust_era_baseline` → a **no-op assertion migration**
   that `SELECT`-verifies schema fingerprints (table count, enum cardinalities, the idempotency PK
   shape) and fails loudly on drift — belt-and-suspenders against a wrong-database target.
3. **Boot schema-head guard (Rust)**: at startup, read
   `SELECT version FROM _sqlx_migrations WHERE success ORDER BY version DESC LIMIT 1`, compare to
   the compiled-in head (`MIGRATOR.migrations.last()`), FATAL-exit on mismatch — the exact
   equivalent of today's guard. **Transition tripwire**: also assert `SELECT count(*) FROM
   pgmigrations` equals the frozen head-count, so any accidental Node-tool run after cutover is
   caught at boot. Extend with the enum-cardinality assert from §5.
4. 085–088 drafts: whichever are still unapplied at cutover get **re-authored as sqlx migrations**
   (content verbatim, watermark re-checked per the 085 header rule) — they must NOT land through
   node-pg-migrate after the freeze point.

## 9. Rust mapping (what sqlx must honor)

| Pg reality | sqlx mapping | Proof artifact |
|---|---|---|
| money = `integer` minor units, `CHECK >= 0` 🔴 | newtype `Minor(i32)` (`#[derive(sqlx::Type)] #[sqlx(transparent)]`), constructor rejects negatives; NEVER f32/f64/Decimal for money | existing money E2E + unit tests; new: proptest round-trip + `try_from` rejection test |
| `uuid` PKs everywhere | `uuid::Uuid` (sqlx `uuid` feature) | compile-time via `query_as!` |
| `timestamptz` | `chrono::DateTime<Utc>` (pick ONE of chrono/time repo-wide) | compile-time |
| 10 Pg enums 🔴 | `#[derive(sqlx::Type)] #[sqlx(type_name = "order_status")]` Rust enums, exhaustive match; boot-time cardinality assert (§8) | decode test per enum against live values |
| `jsonb` (read_public_menu result, settings, theme) | `sqlx::types::Json<T>` for known shapes; `serde_json::Value` at DEFINER-fn boundaries | golden-JSON byte-parity test (mig 072's own GOLDEN gate pattern) |
| `exchange_rates.rate` numeric | `rust_decimal::Decimal` — the ONE legitimate non-integer numeric | unit test |
| tenant GUCs | `let mut tx = pool.begin(); sqlx::query("SELECT set_config('app.current_tenant',$1,true)")…; work; tx.commit()` — set_config with `is_local=true` INSIDE the txn, mirroring withTenant verbatim; same for `app.user_id` | port of existing withTenant tests + rls-adversarial suite |
| throw-on-unset policies (§2, 3 tables) | error taxonomy: map SQLSTATE `55000` to a distinct internal error (config bug), never "not found" | new sqlx test: query settlement_items w/o GUC → assert typed error |
| RLS-invisible rows vs `query_as!` offline checks | **compile-time checks are schema-only** — `cargo sqlx prepare` needs a schema-identical DB (CI shadow DB from migrations), RLS/GUC state irrelevant at prepare time; runtime emptiness ≠ compile-time concern. BUT nullability inference on DEFINER-fn calls is weak → wrap in explicit `RETURNS`-typed views of the call | CI: `cargo sqlx prepare --check` gate (ports the schema-drift guard) |
| DEFINER fns as API (read_public_menu etc.) | call via `query_scalar!("SELECT read_public_menu($1,$2)")` — do NOT re-implement menu assembly in Rust in Phase A; byte-parity against Node output is the cutover oracle | mig 072 golden no-op discipline + Playwright storefront E2E |
| pg-boss schema | new Rust queue (Lane A decision) must NOT touch `pgboss.*`; drain-then-freeze pgboss.job at jobs cutover | queue cutover checklist (Lane A) |

**Existing proof net to reuse:** `pnpm verify:rls` (`packages/db/scripts/verify-rls.ts`),
`apps/api/tests/phase5/rls-adversarial.test.ts`, `apps/api/tests/provision-rls.test.ts`,
`apps/api/tests/claim-rls.test.ts`, Playwright E2E suite (the language-independent parity oracle per
06-doc). Each Rust surface cutover reruns the matching adversarial slice against the Rust binary.

## 10. 🔴 Red-line register (data layer)

| # | Item | Why red-line |
|---|---|---|
| R1 | B3 NOBYPASSRLS flip + 3 zero-policy FORCE tables + N1 customers | flip changes effective visibility for EVERY query the Rust port makes; port code written pre-flip must be tested post-flip |
| R2 | order_status transitions (app-enforced) + immutability triggers | money/state machine; exhaustive-match port + parity tests mandatory |
| R3 | 16 money tables, integer minor units, idempotency PK `(location_id,key)` | any drift = wrong charges; Minor newtype + ON CONFLICT parity |
| R4 | 35 DEFINER fns, 5 unpinned (incl hot-path read_public_menu) | privilege boundary; pin backfill before Phase A |
| R5 | GUC discipline (SET LOCAL in txn; 3 throwing policies; 0 initplan wraps; **6 latent broken-GUC site classes in Node — §7 — masked only by BYPASSRLS**) | wrong pool mode or missing txn = cross-tenant exposure or 55000 errors; porting the broken sites verbatim ships the leak into Rust |
| R6 | Staged drafts 085–088 (085 watermark 2026-07-10) | operator-gated money/GDPR changes may land mid-rebuild; dual-shape or sequencing required |
| R7 | `anonymous_select` UUID-as-capability on orders/order_items (§6) + out-of-band `platform_admins` pair | anon/customer isolation is app-code-only by design — the Rust port must encode the capability check in types, not assume RLS; shadow-DB builds must include out-of-band tables |

## 11. Census gaps (what this file could NOT verify)

1. **Live-DB reconciliation not run** — this census is migrations-derived (code SoT). Prod/staging
   catalog diff (information_schema vs this map) requires the staging proxy recipe
   (`DATABASE_URL_*`, memory: staging-db-access) and `scripts/ci-schema-drift.mjs`; run it as the
   Phase-A entry gate.
2. CHECK/UNIQUE/FK counts are regex reference-counts, not `pg_catalog` counts (inline pgm-object
   constraints undercounted; treat as lower bounds).
3. Column-level types for the 4 `pgm.createTable` tables (import_sessions, modifier_translations,
   modifier_group_translations, notification_outbox_audit) parsed only to column names.
4. `dowiz_app` role attributes (BYPASSRLS bit) asserted from mig 077 header + security docs, not
   from a live `pg_roles` read.
5. pgboss internal table list beyond `job`/`version` (installer-created; version-dependent).
6. **Out-of-band applied objects are only partially enumerable from the repo**: `platform_admins` +
   `platform_admin_audit_log` were found (via mig 080's comment + `lib/platform-admin.ts`
   consumers); `docs/security/OPERATIONAL-ROLE-nobypassrls.migration.ts` is staged-not-applied.
   There may be further manually-applied objects with no repo trace — only the live-DB catalog diff
   (gap #1) closes this definitively.
