import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { Pool } from 'pg';
import {
  createSource,
  advance,
  flagManualReview,
  flagTerminal,
  getById,
} from '../src/modules/acquisition/service.js';

// Integration test — proves the dedup anchor + transition/reason invariants against a REAL
// Postgres. Skips cleanly when ACQ_TEST_DATABASE_URL is unset (so the pure suite stays green
// without infra). The P6-1 proof run sets it to a throwaway docker PG with the table created.
const url = process.env.ACQ_TEST_DATABASE_URL;
const maybe = url ? test : test.skip;

let pool: Pool;
before(() => {
  if (url) pool = new Pool({ connectionString: url });
});
after(async () => {
  if (pool) await pool.end();
});

maybe('createSource is idempotent — same place_id → exactly one row', async () => {
  const placeId = 'ChIJ_test_dedup_' + Math.random().toString(36).slice(2);
  const a = await createSource(pool, placeId);
  const b = await createSource(pool, placeId);
  assert.equal(a.id, b.id, 'second createSource must return the same row');
  const { rows } = await pool.query('SELECT count(*)::int AS n FROM acquisition_sources WHERE place_id = $1', [placeId]);
  assert.equal((rows[0] as { n: number }).n, 1, 'exactly one row per place_id');
  assert.equal(a.state, 'SOURCED');
});

maybe('advance follows legal edges and rejects illegal ones', async () => {
  const src = await createSource(pool, 'ChIJ_test_adv_' + Math.random().toString(36).slice(2));
  const ingested = await advance(pool, src.id, 'PLACE_INGESTED', { website_url: 'https://example.test' });
  assert.equal(ingested.state, 'PLACE_INGESTED');
  assert.equal(ingested.website_url, 'https://example.test');
  await assert.rejects(() => advance(pool, src.id, 'VERIFIED'), /illegal acquisition transition/);
});

maybe('a terminal/exit state requires a non-empty failure_reason', async () => {
  const src = await createSource(pool, 'ChIJ_test_reason_' + Math.random().toString(36).slice(2));
  await assert.rejects(() => advance(pool, src.id, 'MANUAL_REVIEW', { failure_reason: '  ' }), /non-empty failure_reason/);
  const flagged = await flagManualReview(pool, src.id, 'menu page returned 404');
  assert.equal(flagged.state, 'MANUAL_REVIEW');
  assert.equal(flagged.failure_reason, 'menu page returned 404');
  const dead = await flagTerminal(pool, src.id, 'ABANDONED', 'operator closed');
  assert.equal(dead.state, 'ABANDONED');
  const reread = await getById(pool, src.id);
  assert.equal(reread?.state, 'ABANDONED');
});
