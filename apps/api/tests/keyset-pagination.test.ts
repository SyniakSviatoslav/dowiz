import { test } from 'node:test';
import assert from 'node:assert/strict';
import pg from 'pg';

// A4 (ADR-0010, B3/B13) — proof that the strict composite `(sort, id)` keyset never drops
// same-timestamp rows, and that the OLD naive `sort < $cursor` comparator DOES (the live
// drop-bug). Self-contained: a session-local TEMP table mirroring the (sort, id) shape used by
// owner/dashboard.ts (orders), owner/alerts.ts (location_alerts), owner/signals.ts
// (customer_signals) — no migration, no app schema, no protected path. Runs against any
// Postgres via KEYSET_TEST_DB_URL (e.g. the staging DB over `flyctl proxy`).
//
// Skips (does not fail) when no DB URL is supplied, so it never blocks a no-DB CI lane; the
// real red→green is recorded by running it against staging.

const DB_URL = process.env.KEYSET_TEST_DB_URL;

// Page through `kt` with the given cursor-comparator, limit 2, mirroring the route loop:
// ORDER BY sort DESC, id DESC; cursor = last (sort,id) of the previous page.
async function pageAll(
  client: pg.PoolClient,
  comparator: 'naive' | 'composite',
): Promise<string[]> {
  const limit = 2;
  const seen: string[] = [];
  let cursor: { sort: string; id: string } | null = null;
  for (let guard = 0; guard < 100; guard++) {
    let where = '';
    const params: any[] = [];
    if (cursor) {
      if (comparator === 'naive') {
        params.push(cursor.sort);
        where = `WHERE sort < $1`;
      } else {
        params.push(cursor.sort, cursor.id);
        where = `WHERE (sort, id) < ($1, $2)`;
      }
    }
    params.push(limit + 1);
    const { rows } = await client.query(
      `SELECT sort, id FROM kt ${where} ORDER BY sort DESC, id DESC LIMIT $${params.length}`,
      params,
    );
    const hasMore = rows.length > limit;
    const page = hasMore ? rows.slice(0, limit) : rows;
    if (page.length === 0) break;
    for (const r of page) seen.push(r.id);
    if (!hasMore) break;
    const last = page[page.length - 1];
    cursor = { sort: last.sort, id: last.id };
  }
  return seen;
}

test('A4 keyset pagination (composite (sort,id) never drops a same-timestamp tie)', async (t) => {
  if (!DB_URL) {
    t.skip('KEYSET_TEST_DB_URL not set — run against staging DB for the red→green proof');
    return;
  }
  const pool = new pg.Pool({ connectionString: DB_URL, max: 1 });
  const client = await pool.connect();
  try {
    // 5 rows; THREE share the exact same `sort` (a same-millisecond burst) — the tie the naive
    // comparator drops when the page boundary lands inside the tie group.
    // Session-scoped temp table (dropped when the connection closes at pool.end()). No
    // ON COMMIT DROP — under autocommit that would drop it the instant CREATE returns.
    await client.query('CREATE TEMP TABLE kt (sort timestamptz NOT NULL, id uuid NOT NULL)');
    await client.query(`
      INSERT INTO kt (sort, id) VALUES
        ('2026-06-26T10:00:03Z', gen_random_uuid()),
        ('2026-06-26T10:00:02Z', gen_random_uuid()),
        ('2026-06-26T10:00:02Z', gen_random_uuid()),
        ('2026-06-26T10:00:02Z', gen_random_uuid()),
        ('2026-06-26T10:00:01Z', gen_random_uuid())
    `);

    const composite = await pageAll(client, 'composite');
    const naive = await pageAll(client, 'naive');

    // Composite: all 5 distinct rows returned, no duplicates, no drops.
    assert.equal(composite.length, 5, `composite returned ${composite.length}/5`);
    assert.equal(new Set(composite).size, 5, 'composite returned duplicates');

    // Naive: drops at least one tie row (the bug). This asserts the comparator we REPLACED is
    // genuinely broken — i.e. the test is a real red→green, not a tautology.
    assert.ok(naive.length < 5, `naive should DROP a tie row, but returned ${naive.length}/5`);
  } finally {
    client.release();
    await pool.end();
  }
});
