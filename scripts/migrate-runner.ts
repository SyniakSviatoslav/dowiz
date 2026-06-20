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
 * Requires ***REDACTED*** (the DDL/session role) in the environment.
 */
import { runner } from 'node-pg-migrate';
import { join } from 'node:path';

async function main(): Promise<void> {
  const databaseUrl = process.env.***REDACTED***;
  if (!databaseUrl) {
    console.error('[migrate] ***REDACTED*** is not set — cannot run migrations');
    process.exit(1);
  }

  const dir = join(__dirname, 'migrations');
  console.log(`[migrate] applying pending migrations from ${dir}`);

  const applied = await runner({
    databaseUrl,
    dir,
    direction: 'up',
    count: Infinity,
    migrationsTable: 'pgmigrations',
    singleTransaction: true, // mirrors CLI default: all-or-nothing
    checkOrder: true,
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
