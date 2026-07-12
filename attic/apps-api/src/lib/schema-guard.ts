/**
 * Fail-fast schema guard.
 *
 * Refuses to boot the API when the database is BEHIND the migration head this
 * build expects. This converts the silent-500 failure mode — new code querying
 * columns/tables that an old schema lacks — into a loud, immediate crash-loop,
 * which is the safe outcome for data integrity and is trivially visible in ops.
 *
 * The expected head (newest migration's basename, no extension) is stamped into
 * the bundle at build time by `scripts/build-apps.ts` via esbuild `define`.
 * In unbundled/dev runs the define is absent, so the guard is a no-op — local
 * databases are managed by hand and we never want the guard to block dev.
 *
 * With `release_command` applying migrations before every deploy, a behind
 * schema should be impossible; this guard is the backstop for the cases
 * release_command can't cover (a misconfiguration, a hand-rolled deploy).
 */
import type { Pool } from 'pg';

// Replaced at build time by esbuild `define`. Absent (and thus `undefined`) in
// dev/tsx runs — the `typeof` check below keeps that a no-op rather than a crash.
declare const __EXPECTED_MIGRATION_HEAD__: string | undefined;

export async function assertSchemaCurrent(pool: Pool): Promise<void> {
  const expectedHead =
    typeof __EXPECTED_MIGRATION_HEAD__ !== 'undefined' ? __EXPECTED_MIGRATION_HEAD__ : null;

  // Dev / unbundled: nothing stamped → nothing to assert.
  if (!expectedHead) return;

  let appliedHead: string | null = null;
  try {
    const headRes = await pool.query<{ name: string }>(
      'SELECT name FROM pgmigrations ORDER BY id DESC LIMIT 1'
    );
    appliedHead = headRes.rows[0]?.name ?? null;

    // The expected head migration being present means the schema is current (or
    // ahead, which is fine — extra migrations never remove what this build needs).
    const hasHead = await pool.query(
      'SELECT 1 FROM pgmigrations WHERE name = $1 LIMIT 1',
      [expectedHead]
    );
    if ((hasHead.rowCount ?? 0) > 0) {
      console.log(`[API] ✅ schema current (head: ${expectedHead})`);
      return;
    }
  } catch (err) {
    const msg = (err as Error).message || '';
    // A missing pgmigrations table means migrations never ran → genuinely behind,
    // fall through to the fatal exit. Any other (transient) error must NOT take
    // down a healthy app, so warn and allow boot.
    if (!/pgmigrations.*does not exist|does not exist.*pgmigrations/i.test(msg)) {
      console.warn('[API] ⚠️  schema guard could not verify migrations (continuing):', msg);
      return;
    }
  }

  console.error(
    `[API] ❌ FATAL: database schema is BEHIND. This build expects migration ` +
      `"${expectedHead}", but the database head is "${appliedHead ?? '(none)'}". ` +
      `Migrations did not run before boot — refusing to start to avoid serving ` +
      `errors against a stale schema. Apply migrations (release_command / pnpm migrate:up) and redeploy.`
  );
  process.exit(1);
}
