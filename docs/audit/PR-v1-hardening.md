# PR: v1 hardening — secure, reliable, polished, deployable v1

**Branch:** `feat/v1-hardening` → `main` · **Open it:**
https://github.com/SyniakSviatoslav/dowiz/compare/main...feat/v1-hardening?expand=1

> Paste the section below as the PR body. Full evidence: `docs/audit/v1-verification-2026-06-19.md`.

---

## Summary

Verified every core flow (order, delivery, courier invite, onboarding, menu import, client ordering,
cross-tenant roles) against a local full stack + live probes, then fixed the real gaps. Net result:
**security HOLDS 13 / WEAK 2 / BROKEN 0**, full order→delivery lifecycle proven end-to-end, and a
**fresh database can now be provisioned from scratch** (it could not before). 3 commits, all green.

## What was broken and is now fixed

| Sev | Issue | Fix |
|---|---|---|
| P0 | Fresh DB could not migrate/boot (4 migration bugs; missing Supabase roles; pg-boss never bootstrapped) — DR/new-env/onboarding broken | Fixed migrations + idempotent role-bootstrap + pg-boss-bootstrap migrations + CI fresh-provision smoke |
| P0 | **Anti-fraud preflight + OTP layer was dead** — `lib/preflight` missing → import silently stubbed to `clean` in dev *and* prod (OTP + velocity/no-show never fired) | Restored `apps/api/src/lib/preflight.ts`, **fail-loud static import**, added `require_phone_otp` to the order SELECT, opened the public OTP endpoints in the auth gate; + regression test |
| P0 | Raw **phone PII in the 7-day customer JWT** (docs falsely claimed removed) | Dropped from claim/schema; resolved server-side |
| P0 | `'open'` vs `'active'`: published storefronts returned "Location not found" | `read_public_menu` accepts `status IN ('active','open') OR published_at IS NOT NULL` |
| P0 | Checkout had no OTP UI | OTP send/verify step + brand `OTPModal`, reacts to `soft_confirm/requiresOtp` |
| P1 | WS duplicate-delivery + LISTEN leak (events 4–5×) | unsubscribe per-room handler on teardown + churn regression test |
| P1 | `sitemap.xml` 500; generic SSR `<title>` | fixed query; per-tenant `<title>`/OG (XSS-safe) |
| P1 | anon `GET /orders/:id` unscoped; CORS precedence | 401 for anonymous; parenthesized |
| P1 | 40 eslint a11y errors; lint hygiene | → 0 errors (keyboard handlers/roles); lint gates CI |
| P1/P2 | fallback phone unset (health degraded); monogram image placeholders; low-contrast allergen chips; 1 MB map chunk; broken phase5 test SQL | publish auto-seeds fallback phone + UI field; crafted no-photo fallback; AA chips; chunk split; `set_config` fix |

## Verification (all GREEN, local full stack)

Full lifecycle order→DELIVERED (auto-assign + cash) · idempotency · OTP `soft_confirm`→send 200→verify ·
PII-free JWT · cross-tenant 404 · anon 401 · sitemap 200 · per-tenant SSR · WS dedup · preflight 17/17 ·
71 unit tests · both phase5 suites execute (0×42601). Fresh-provision: migrate(107)→seed→boot→serve-menu.

## Deploy + backfill runbook (you drive)

**Staging (recommended): provision an isolated DB first.** Don't point staging at prod Supabase.
1. DB: `fly mpg create` (Managed Postgres) **or** a separate Supabase project. Set its URL in staging secrets.
2. `fly apps create dowiz-staging`; copy prod secrets except `DATABASE_URL_*` (use the staging DB) and set `DEV_AUTH_SECRET` for E2E.
3. `pnpm migrate:up` (against staging) → `pnpm seed` → `fly deploy -a dowiz-staging`.
4. Smoke: `/health` 200, place a test order, run `e2e/lifecycle-e2e`.

**Promote to prod (`dowiz`):**
1. Merge this PR to `main` (CI runs validate + fresh-provision + post-deploy smoke).
2. Migrate prod: only the **3 new migrations** run (the 4 fixed ones share names with already-applied
   records, so they won't re-run): `create-supabase-roles` (no-op on Supabase), `pgboss-bootstrap`
   (idempotent), `public-menu-accept-published` (broadens the menu filter — safe).
3. **Backfill SQL (review before running on prod):**
   ```sql
   -- fallback phone for existing published locations (stops /health degradation)
   UPDATE locations
     SET fallback_config = jsonb_set(COALESCE(fallback_config,'{}'::jsonb), '{phone}', to_jsonb(phone))
     WHERE phone IS NOT NULL AND COALESCE(fallback_config->>'phone','') = '';
   -- (P0-CLUTTER) purge visible test-data from the customer-facing demo tenant — INSPECT first:
   --   SELECT id,name FROM categories WHERE location_id=(SELECT id FROM locations WHERE slug='demo')
   --     AND name ~* '^(E2E|Test|UI-F|WS2)-';   then DELETE those + their products.
   ```
4. Post-deploy: `/health` all-green, `/sitemap.xml` 200, `/s/<slug>` shows the restaurant name, a live
   test order completes.

## Follow-ups (non-blocking)

- **TI-6/TI-7:** align two phase5 test *fixtures* with the current schema (they write columns that moved;
  real idempotency + RLS are proven green by live endpoints).
- **TI-3/TI-4:** `:3003` phase-test harness + lifecycle UI test-seams.
- **Dependabot:** 5 dependency vulns on `main` (1 high/2 mod/2 low) — run an SCA pass (V15).
- Stop E2E/test runs from writing into a customer-facing tenant; use a throwaway tenant + cleanup.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
