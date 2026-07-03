import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Pool } from 'pg';
import {
  createIsolatedPool, resetSchema, applyMoneyFix, seedPair, seedDeliveredCash, WATERMARK,
} from './money-spine-fixture.js';

// M-2 proofs (ADR-audit-fix-money §4, migration draft 1790000000085): P8 locked-row deferral,
// P9 missed-day catch-up, P9b pre-watermark rows never auto-swept, P10 paid-payout immutability,
// P11 aggregate-recompute idempotency, P13 operator backfill flow. Each is RED against the
// verbatim pre-fix fn (1790000000078): the old window scan loses P8/P9 rows forever, bumps paid
// payouts (P10), and phantom-counts (P11).
// Skips without MONEY_TEST_DATABASE_URL (throwaway DB — see money-spine-fixture.ts header).

const url = process.env.MONEY_TEST_DATABASE_URL;
const maybe = url ? test : test.skip;

let pool: Pool;
let teardown: (() => Promise<void>) | undefined;
before(async () => {
  if (!url) return;
  ({ pool, teardown } = await createIsolatedPool(url)); // per-process DB — parallel test files can't race
  await resetSchema(pool);
  await applyMoneyFix(pool);
});
after(async () => { if (teardown) await teardown(); });

// All money rows post-watermark unless a test says otherwise (watermark = 2026-07-10; test rows
// use fixed future timestamps — the fn compares delivered_at ranges only, wall clock irrelevant).
const D15 = '2026-07-15 12:00:00+00';
const D15_LATE = '2026-07-15 18:00:00+00';
const P1_START = '2026-07-15 00:00:00+00', P1_END = '2026-07-16 00:00:00+00';
const P2_START = '2026-07-16 00:00:00+00', P2_END = '2026-07-17 00:00:00+00';

async function generate(start: string, end: string) {
  await pool.query(`SELECT app_generate_settlements($1, $2)`, [start, end]);
}
async function itemFor(assignmentId: string) {
  const r = await pool.query(
    `SELECT si.*, cp.period_start, cp.period_end, cp.status AS payout_status
       FROM settlement_items si JOIN courier_payouts cp ON cp.id = si.payout_id
      WHERE si.assignment_id = $1`,
    [assignmentId],
  );
  return r.rows[0] ?? null;
}

maybe('P8: row locked during the run is DEFERRED (skip-locked), then caught up by the NEXT run', async () => {
  const pair = await seedPair(pool);
  const aId = await seedDeliveredCash(pool, pair, D15, 1500);

  const locker = await pool.connect();
  try {
    await locker.query('BEGIN');
    await locker.query(`SELECT * FROM courier_assignments WHERE id = $1 FOR UPDATE`, [aId]);
    await generate(P1_START, P1_END); // row locked → skipped
    assert.equal(await itemFor(aId), null, 'locked row must be skipped this run');
    await locker.query('ROLLBACK');
  } finally {
    locker.release();
  }

  await generate(P2_START, P2_END); // pre-fix fn: P2 window excludes the D15 row → lost forever (RED)
  const item = await itemFor(aId);
  assert.ok(item, 'catch-up scan must sweep the previously-skipped row into the next run');
  assert.equal(new Date(item.period_start).toISOString(), new Date(P2_START).toISOString(), 'lands in the NEXT period payout (period = label)');
});

maybe('P9: a whole day with no run is swept by the next successful run', async () => {
  const pair = await seedPair(pool);
  const aId = await seedDeliveredCash(pool, pair, D15, 2000);
  // No run for the [P1] period at all — first run ever is for [P2] (pre-fix: permanently lost, RED).
  await generate(P2_START, P2_END);
  const item = await itemFor(aId);
  assert.ok(item, 'missed-day row must enter the next generated payout');
  assert.equal(item.amount, 2000);
});

maybe('P9b: PRE-watermark rows are NEVER auto-swept by cron generation', async () => {
  const pair = await seedPair(pool);
  const preId = await seedDeliveredCash(pool, pair, '2026-07-01 12:00:00+00', 1200); // before 2026-07-10
  await generate(P1_START, P1_END);
  await generate(P2_START, P2_END);
  assert.equal(await itemFor(preId), null,
    `pre-watermark (${WATERMARK}) rows move ONLY through the operator backfill — auto-resurrection is the C2 double-pay`);
});

maybe('P10: a PAID payout is immutable — late items defer to the next period, totals never move', async () => {
  const pair = await seedPair(pool);
  const a1 = await seedDeliveredCash(pool, pair, D15, 1500);
  await generate(P1_START, P1_END);
  const first = await itemFor(a1);
  assert.ok(first, 'seed item settled into P1');
  await pool.query(`UPDATE courier_payouts SET status = 'paid' WHERE id = $1`, [first.payout_id]);

  const a2 = await seedDeliveredCash(pool, pair, D15_LATE, 700); // late item inside the paid period
  await generate(P1_START, P1_END); // pre-fix fn: bumps the paid payout (RED via immutability trigger or drifted totals)
  const paid = await pool.query(`SELECT deliveries_count, total_earned, status FROM courier_payouts WHERE id = $1`, [first.payout_id]);
  assert.equal(paid.rows[0].status, 'paid');
  assert.equal(paid.rows[0].deliveries_count, 1, 'paid payout deliveries_count must not move');
  assert.equal(paid.rows[0].total_earned, 1500, 'paid payout total_earned must not move');
  assert.equal(await itemFor(a2), null, 'late item is NOT forced into the closed period');

  await generate(P2_START, P2_END);
  const late = await itemFor(a2);
  assert.ok(late, 'late item lands in the NEXT period pending payout');
  assert.equal(late.payout_status, 'pending');
  assert.notEqual(late.payout_id, first.payout_id);
});

maybe('P11: totals are aggregate-recomputed — double runs and pre-existing items cannot inflate', async () => {
  const pair = await seedPair(pool);
  await seedDeliveredCash(pool, pair, D15, 1000);
  await seedDeliveredCash(pool, pair, D15_LATE, 900);
  await generate(P1_START, P1_END);
  await generate(P1_START, P1_END); // idempotent second run
  const payout = await pool.query(
    `SELECT cp.id, cp.deliveries_count, cp.total_earned,
            (SELECT count(*)::int FROM settlement_items si WHERE si.payout_id = cp.id) AS agg_count,
            (SELECT COALESCE(sum(si.amount),0)::int FROM settlement_items si WHERE si.payout_id = cp.id) AS agg_total
       FROM courier_payouts cp WHERE cp.courier_id = $1 AND cp.location_id = $2`,
    [pair.courierId, pair.locationId],
  );
  assert.equal(payout.rowCount, 1);
  const p = payout.rows[0];
  assert.equal(p.deliveries_count, p.agg_count, 'deliveries_count === count(settlement_items) exactly');
  assert.equal(p.total_earned, p.agg_total, 'total_earned === sum(settlement_items.amount) exactly');
  assert.equal(p.agg_count, 2);
  assert.equal(p.agg_total, 1900);
});

maybe('P13: operator backfill — pre-count matches created rows; pending + backfilled=true; cron never reaches it', async () => {
  const pair = await seedPair(pool);
  await seedDeliveredCash(pool, pair, '2026-07-02 10:00:00+00', 1100);
  await seedDeliveredCash(pool, pair, '2026-07-03 10:00:00+00', 1300);

  const pre = await pool.query(
    `SELECT * FROM app_backfill_precount_settlements() WHERE o_courier_id = $1 AND o_location_id = $2`,
    [pair.courierId, pair.locationId],
  );
  assert.equal(pre.rowCount, 1, 'pre-count reports the pair');
  assert.equal(Number(pre.rows[0].o_eligible_items), 2);
  assert.equal(Number(pre.rows[0].o_eligible_total), 2400);

  const created = await pool.query(
    `SELECT * FROM app_backfill_historical_settlements() WHERE o_courier_id = $1 AND o_location_id = $2`,
    [pair.courierId, pair.locationId],
  );
  assert.equal(created.rowCount, 1);
  assert.equal(created.rows[0].o_items_added, 2, 'backfill creates exactly the pre-counted rows');
  assert.equal(created.rows[0].o_total_added, 2400, 'backfill total equals the pre-count total');

  const items = await pool.query(
    `SELECT si.backfilled, cp.status FROM settlement_items si JOIN courier_payouts cp ON cp.id = si.payout_id
      WHERE si.location_id = $1`,
    [pair.locationId],
  );
  assert.equal(items.rowCount, 2);
  for (const row of items.rows) {
    assert.equal(row.backfilled, true, 'backfilled rows are flagged for the owner/courier "caught-up" display');
    assert.equal(row.status, 'pending', 'backfill NEVER auto-pays');
  }

  // idempotent re-run: nothing new
  const again = await pool.query(
    `SELECT * FROM app_backfill_historical_settlements() WHERE o_courier_id = $1`,
    [pair.courierId],
  );
  assert.equal(again.rowCount, 0, 'second backfill run creates nothing');
});

// P13 structural arm — no code path from cron/workers/routes reaches the backfill fn. Runs DB-free.
test('P13: app_backfill_historical_settlements has ZERO callers in runtime code (operator-only)', () => {
  const __dirname = path.dirname(fileURLToPath(import.meta.url));
  const roots = [
    path.resolve(__dirname, '../src'),
    path.resolve(__dirname, '../../worker/src'),
  ].filter((p) => fs.existsSync(p));
  const hits: string[] = [];
  const walk = (dir: string) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (/\.(ts|tsx|js|mjs)$/.test(entry.name)
        && fs.readFileSync(full, 'utf8').includes('app_backfill_historical_settlements')) {
        hits.push(full);
      }
    }
  };
  for (const r of roots) walk(r);
  assert.deepEqual(hits, [], `backfill fn must be operator-invoked only; found runtime references: ${hits.join(', ')}`);
});
