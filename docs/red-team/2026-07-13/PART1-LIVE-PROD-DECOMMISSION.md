# Part 1 — Live prod credential + the legacy host deploy decommission (OPERATOR ACTION)

> Status: **NOT EXECUTABLE FROM THIS HOST.** No legacy-host CLI, no `DATABASE_URL` /
> Supabase / prod-DB credentials are present in the agent environment, and the
> live legacy host deploy is a production system touching the auth/money
> red-line (operator-confirmed gating). Per the operator's own precedence and
> the red-line rule, this is documented as an exact runbook for the operator to
> run — it is **not** faked from here.

## What this repo already did (D1, verified 2026-07-13)
- The **build** no longer produces the centralized server: `Dockerfile` is a
  static nginx SPA, `scripts/build-apps.ts` assembles only `dist/public`, and
  `attic/<legacy-deploy-manifest>` + `scripts/migrate-runner.ts` were deleted.
- CI has **no deploy job** — `git grep -nE 'release_command'`
  over the live branch returns only Dockerfile *comments* describing the drop.
- Therefore this repo **cannot rebuild or redeploy** the old prod. The live
  the legacy host is orphaned by design; it keeps running only because the
  operator has not yet torn down the legacy host app + DB. That teardown is the action.

## Operator runbook (run from a host with `psql`/Supabase access and the legacy host console)

### A. Rotate the compromised owner credential (P0, do first)
The red-team synthesis confirmed `test@dowiz.com` / `test123456` is live and
owner-privileged. Rotate + disable before decommission so a teardown window
can't be abused.

```bash
# Supabase SQL (or Dashboard → SQL Editor). Replace the email as needed.
-- Disable the seeded test owner account
update auth.users set banned_until = '2099-01-01', encrypted_password = null
  where email = 'test@dowiz.com';
-- Force-rotate any sessions / revoke refresh tokens
delete from auth.refresh_tokens where user_id in (
  select id from auth.users where email = 'test@dowiz.com'
);
-- Confirm no other seeded test creds remain
select email from auth.users where email like '%@dowiz.com' and email like 'test%';
```

If the prod still uses the bebop `owner-token` model (ADR-0004), also rotate
the owner token via the running app's admin endpoint and revoke old tokens.

### B. Decommission the legacy host app + DB (P0)
- Stop the legacy app's machines (freeze ingress), suspend the app, then detach and destroy its attached Postgres in the provider console.
- If the DB is Supabase (not the legacy host Postgres), rotate the DB password in the
  Supabase dashboard and delete the project after a backup, then remove the
  `DATABASE_URL` / `SUPABASE_*` secrets from the legacy host + any CI.

### C. Remove any remaining deploy secrets
- List the legacy app's secrets and unset `DATABASE_URL`, `SUPABASE_URL` and `SUPABASE_SERVICE_ROLE_KEY` on it.

### D. Verify teardown
- Request the legacy host URL (`<legacy-host-url>`) -> expect connection refused / 404 (app suspended).
- Confirm no scale events on the legacy app after the freeze in its logs.

## After teardown
- The architecture replacement (ADR-0007 self-certifying node identity,
  ADR-0008 local SQLite PQ-at-rest, ADR-0009 SSRF-safe IP canonicalization)
  is the durable fix for the synthesized C2/H1–H4 classes — there is no
  centralized prod to seed-cred or cross-tenant anymore.
- Re-run `pnpm verify:secrets` and `pnpm run build` on this branch to confirm
  the repo itself stays green post-D1.
