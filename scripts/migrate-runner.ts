/**
 * Bundled migration runner → built to `dist/migrate/index.cjs`.
 *
 * This is the SINGLE source of truth for applying migrations in production:
 * `fly.toml`'s `release_command` invokes it on EVERY deploy (CI or a manual
 * `flyctl deploy`), in a one-off machine, BEFORE new code receives traffic. So
 * new code can never boot against an un-migrated schema — closing the gap where
 * a manual deploy shipped code ahead of the database.
 *
 * It runs the same migrations as `pnpm migrate:up`, with the same node-pg-migrate
 * options (singleTransaction / checkOrder / lock / pgmigrations table), so its
 * behaviour is identical to local and CI runs. The only difference is the source:
 * compiled `./migrations/*.mjs` (emitted next to this file by build-apps.ts),
 * loaded by node-pg-migrate via dynamic import. node-pg-migrate records each
 * migration by basename-without-extension, so `.mjs` and the source `.ts` produce
 * identical names — a migration applied locally as `.ts` is never re-applied here.
 *
 * Requires DATABASE_URL_MIGRATIONS (the DDL/session role) in the environment.
 */
import { runner } from 'node-pg-migrate';
import { join } from 'node:path';

async function main(): Promise<void> {
  // ADR-0003 pre-traffic gate: the release_command runs in a one-off machine BEFORE the
  // new code receives traffic, and a nonzero exit aborts the rollout (old code keeps
  // serving). This is the only place we can catch the INVERSE misconfig that boot-guard D
  // cannot — a prod app whose NODE_ENV is NOT 'production' (so D never fires and the
  // dev-auth fail-closed defaults silently flip on). Keyed on FLY_APP_NAME so it is inert
  // on staging (which legitimately runs 'development') and on local runs (no FLY_APP_NAME).
  // R-13: Fly injects FLY_APP_NAME into the release machine; if a future Fly change drops
  // it, set an explicit DEPLOY_TARGET=prod secret and key on that instead.
  const flyApp = process.env.FLY_APP_NAME;
  if (flyApp === 'dowiz' && process.env.NODE_ENV !== 'production') {
    console.error(
      `[migrate] FATAL: prod app 'dowiz' requires NODE_ENV='production', got '${process.env.NODE_ENV ?? '(unset)'}'. ` +
        `Aborting release so the dev-auth backdoor cannot silently re-open (ADR-0003).`,
    );
    process.exit(1);
  }
  if (flyApp === 'dowiz-staging' && process.env.NODE_ENV === 'production') {
    console.error(
      `[migrate] FATAL: staging app 'dowiz-staging' must NOT run NODE_ENV='production' (it would disable dev-login + E2E). Aborting release.`,
    );
    process.exit(1);
  }

  const raw = process.env.DATABASE_URL_MIGRATIONS;
  if (!raw) {
    console.error('[migrate] DATABASE_URL_MIGRATIONS is not set — cannot run migrations');
    process.exit(1);
  }

  // Supabase poolers REQUIRE TLS but the connection URLs carry no `sslmode`, and
  // node-pg-migrate's runner does not add one — so append a permissive sslmode when
  // absent (the cert is Supabase's; we verify the host via the pooler, not the chain).
  // Mirrors the documented manual procedure. Local/non-TLS URLs already set sslmode=disable.
  const databaseUrl = /sslmode=/.test(raw) ? raw : `${raw}${raw.includes('?') ? '&' : '?'}sslmode=no-verify`;

  const dir = join(__dirname, 'migrations');
  console.log(`[migrate] applying pending migrations from ${dir}`);

  const applied = await runner({
    databaseUrl,
    dir,
    direction: 'up',
    count: Infinity,
    migrationsTable: 'pgmigrations',
    singleTransaction: true, // mirrors CLI default: all-or-nothing
    // checkOrder MUST be false on prod: it intentionally never recorded two platform
    // migrations (Supabase-managed roles; pg-boss bootstrapped out-of-band), so the
    // order check would error "X precedes already-run Y" and abort every deploy. Pending
    // migrations still apply in (timestamp-sorted) filename order regardless. See the
    // prod schema-drift outage notes.
    checkOrder: false,
    verbose: true,
  });

  if (applied.length === 0) {
    console.log('[migrate] schema already up to date — no migrations to run');
  } else {
    console.log(`[migrate] applied ${applied.length} migration(s):`);
    for (const m of applied) console.log(`[migrate]   + ${m.name}`);
  }
}

main()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error('[migrate] FAILED:', err instanceof Error ? err.stack : err);
    process.exit(1);
  });
