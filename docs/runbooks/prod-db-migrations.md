# Runbook — Production DB migrations (schema-drift prevention)

## What changed (the permanent fix)
`fly.toml` now has a `[deploy] release_command = "node dist/migrate/index.cjs"`. On
**every** deploy (CI push-to-`main` or a manual `flyctl deploy`), Fly runs the bundled
migrator in a one-off machine **before** the new image receives traffic. node-pg-migrate
applies only pending migrations (it records each by name in `pgmigrations`), so this is
idempotent. If it fails, Fly **aborts the rollout** — the old code keeps serving (fail-safe),
instead of the new code crash-looping against an old schema (the earlier outage).

The migrator is bundled by `scripts/build-apps.ts` to `dist/migrate/` (runner + every
migration compiled to `.cjs`). `scripts/migrate-runner.ts` uses `checkOrder:false` (prod
intentionally never recorded two platform migrations — Supabase-managed roles; pg-boss
bootstrapped out-of-band) and appends `sslmode=no-verify` when the URL has no sslmode
(Supabase poolers require TLS).

## Prerequisite (verify ONCE before relying on it)
The prod app (`dowiz`) must have the **`***REDACTED***`** secret set (session
pooler / port 5432, the DDL role). If absent, `release_command` exits 1 and the deploy
aborts. Check / set:

```
flyctl secrets list -a dowiz | grep ***REDACTED***
# if missing:
flyctl secrets set ***REDACTED***="postgres://postgres:<pw>@<host>:5432/postgres" -a dowiz
```
(Same for `dowiz-staging` — its secret already exists.) Setting a secret triggers a deploy;
that deploy will itself run the release_command.

## Merging this branch (feat/golive-remediation → main) — migrations 041–044
With the release_command in place, **the merge deploy auto-applies 041–044** before the new
code boots. No manual step is required **if** the prerequisite secret exists. Sequence on
push to `main`:
1. CI builds the image (bundles migrator + migrations 001–044, stamps schema head = 044).
2. Fly runs `release_command` → applies pending 041–044 to the prod DB.
3. Rollout proceeds; new code boots against the now-current schema.

Watch the deploy: `flyctl logs -a dowiz` — look for `[migrate] applied N migration(s)` then
the normal boot. If `release_command` fails, the deploy stops; fix and re-deploy (old code
unaffected).

## Manual fallback (if you must migrate prod out-of-band)
Only needed if you deploy without the release_command, or to pre-migrate. From a machine
with repo access:
```
# 1. Proxy to the prod DB is NOT needed — use the prod session-pooler URL directly.
export ***REDACTED***="$(grep ^***REDACTED***= .env | cut -d= -f2-)?sslmode=no-verify"
# 2. Apply pending migrations (no-check-order: prod baseline gaps).
node_modules/.bin/node-pg-migrate up -d ***REDACTED*** -j ts \
  -m packages/db/migrations --tsx --tsconfig tsconfig.migrations.json --no-check-order
```
If node-pg-migrate errors that a platform migration "is preceding already run …", baseline
it (verify the object exists on prod first), then re-run with `--no-check-order`:
```
psql "$***REDACTED***" -c \
  "INSERT INTO pgmigrations(name,run_on) VALUES ('1780310044711_create-supabase-roles', now()), ('1790000000011_pgboss-bootstrap-schema', now()) ON CONFLICT DO NOTHING;"
```

## Connection gotchas (Supabase)
- Password may contain `%` (percent-encoded). `pg` (and node-pg-migrate via connectionString)
  URL-decodes it correctly; `psql` and discrete `password:` fields do NOT. Always use the
  connectionString form.
- URLs carry no `sslmode`; the pooler requires TLS → append `?sslmode=no-verify` for migrate
  runs (the bundled runner does this automatically).
- Load `.env` via node `--env-file`/dotenv, not shell `cut` into argv, for anything with
  special characters.
