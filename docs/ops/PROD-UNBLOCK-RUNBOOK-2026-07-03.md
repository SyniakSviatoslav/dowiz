# Prod unblock runbook — 2026-07-03

Gets the 275-commit batch (main `2fded223`) deployed to prod + closes the credential leak, in one
coordinated pass. Prod is currently SAFE (old image, `/livez` 200; DB partially migrated to ~076,
additive so tolerated). Two independent problems solved together here:
- **Leak:** `postgres` superuser + `deliveryos_api_user` DB passwords were committed (see
  docs/security/pre-opensource-secrets-audit.md). postgres pw already reset by operator.
- **Deploy blocker:** migrations 077–082 `GRANT … TO dowiz_app`, a role that exists on staging/CI but
  **not on prod** → `migrate:up` fails → deploy aborts.

## 🔴 THE ACTUAL CI-MIGRATE BLOCKER (root cause, found 2026-07-03) — update the GITHUB secret
CI's `deploy → Migrate Database` step (`ci.yml:150-153`) runs `pnpm migrate:up` on the GitHub runner and
reads **`${{ secrets.DATABASE_URL_MIGRATIONS }}` — a GitHub Actions secret, NOT the Fly secret.** That GH
secret is stale (set 2026-06-05: old postgres pw, no sslmode) → every deploy fails with `ESSLREQUIRED`.
The agent CANNOT update it (the PAT lacks `secrets: write` — `gh secret set` silently no-ops). VERIFIED the
value works with node-pg-migrate ("No migrations to run" on an empty dir; `sslmode=no-verify` is honored,
`require` is now aliased to verify-full and fails on the self-signed pooler cert). OPERATOR — update TWO
GitHub Actions secrets (repo Settings → Secrets and variables → Actions, or `gh secret set` with a
secrets-write token):
- **DATABASE_URL_MIGRATIONS** =
  `postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db`
- **DATABASE_URL_SESSION** = same value.
Then re-run the CI `deploy` job → migrate applies 066..084 (dowiz_app exists) → new image ships. The Fly
runtime secrets are already correct (agent set them; OPERATIONAL restored prod). Also: the PAT can't
close issue #9 (resolved by the merge) — close it manually.

## ⚡ (already handled) Fly runtime secrets + dowiz_app role
`dowiz_app` role is created (agent did it, bare/inert). postgres pw `REDACTED_DB_PASSWORD` confirmed working.
The only remaining fix is the connection string's SSL param — VERIFIED working string is `?sslmode=no-verify`
(no-param → ESSLREQUIRED; `require` → self-signed-cert-chain). Run:
```bash
~/.fly/bin/flyctl secrets set -a dowiz \
  DATABASE_URL_MIGRATIONS='postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db' \
  DATABASE_URL_SESSION='postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db'
```
(No `--stage` → sets + deploys; migrate:up now succeeds → new image ships. NOTE: a broken no-sslmode
secret was staged earlier by the agent — this command overwrites it.) Then tell the agent to verify prod.
OPERATIONAL (deliveryos_api_user) is separate — rotate that leaked pw (Step 1b) + set it (Step 2) when ready.

## Already done (agent)
- ✅ `DATABASE_URL_MIGRATIONS` + `DATABASE_URL_SESSION` **staged** with the new postgres password
  (`flyctl secrets set --stage`), exact string:
  `postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db`
  — applies on the next deploy (no failing-release churn). Not yet live on VMs.

## Step 1 — Supabase SQL editor (as postgres superuser)
```sql
-- (a) create the operational role the migrations require (match staging; BYPASSRLS for now,
--     B3 later flips it NOBYPASSRLS via POOL_REQUIRE_NOBYPASSRLS). Pick a STRONG fresh pw.
CREATE ROLE dowiz_app LOGIN BYPASSRLS PASSWORD 'REDACTED_DB_PASSWORD' INHERIT;

-- (b) rotate the LEAKED api-user password (it was committed; still live until you do this).
ALTER ROLE deliveryos_api_user PASSWORD '<NEW_APIUSER_PW>';
```
Verify the pg-privilege-hardening role model if unsure: docs/design/pg-privilege-hardening/remediation-plan.md.

## Step 2 — set the OPERATIONAL Fly secret (agent could not: needs the new api-user/dowiz_app pw)
The runtime pool. To MATCH STAGING, point it at `dowiz_app` (the role the grants target). Confirm the
exact port from your Supabase dashboard → Connection string (transaction pooler is usually `:6543`):
```bash
flyctl secrets set --stage -a dowiz \
  DATABASE_URL_OPERATIONAL='postgresql://REDACTED:REDACTED@REDACTED-supabase-host/db'
```
(If you prefer to keep OPERATIONAL on `deliveryos_api_user` for now, use that role + `<NEW_APIUSER_PW>`
instead — but then the migrations' grants to dowiz_app are inert until a later switch. Matching staging =
dowiz_app is cleaner.)

## Step 3 — deploy (applies all staged secrets + finishes migrations)
Re-trigger the prod deploy (push a trivial commit to main, or re-run the failed CI `deploy` job). The
release runs `migrate:up` (now `dowiz_app` exists → 077..084 succeed) → deploys the new image with the
rotated secrets. Watch `dowiz.fly.dev/livez` (200) + `/health` (the `fallback`=degraded is the known-dark
R2 media, separate — see docs/security/product-media-OPERATOR-ENABLEMENT.md).

## Step 4 — close the leak fully (after passwords rotated)
Run `scripts/secrets-history-scrub.sh` (mirror-clone history rewrite → verify 0 occurrences → your
force-push). Rotation (steps 1–2) is the real fix; the scrub removes the traces.

## Ordering note
postgres pw is already reset in Supabase → the OLD postgres pw is DEAD now, so prod's MIGRATIONS/SESSION
connections (postgres role) are currently failing on the old cached secret; the OPERATIONAL pool
(deliveryos_api_user, not yet rotated) still works → `/livez` stays 200. Do steps 1–3 promptly so the
staged new postgres pw goes live and postgres-role connectivity is restored.
