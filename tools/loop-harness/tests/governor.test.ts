import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { aggregate, checkGovernor, isHalted, masterHalt, clearHalt, staleLoops, DEFAULT_GOVERNOR } from '../src/governor.js';
import { appendMetricsLine } from '../src/storage.js';
import type { MetricsLine } from '../src/types.js';

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'gov-'));
const NOW = Date.parse('2026-06-27T12:00:00Z');
const line = (over: Partial<MetricsLine>): MetricsLine => ({
  loop: 'x', run_index: 1, ts: new Date(NOW).toISOString(), outcome: 'green', iters: 1, wall_s: 1,
  tokens_in: 0, tokens_out: 0, cost_usd: 0, kwh: 0, gco2: 0, water_ml: 0, fail_start: 0, fail_end: 0,
  per_resolved: null, slop_min: null, conflicts: 0, recurring_flags: [], ...over,
});

test('aggregate — sums cost + churn within the day/hour windows', () => {
  const d = tmp();
  appendMetricsLine(d, line({ cost_usd: 10, edits: 30 }));
  appendMetricsLine(d, line({ cost_usd: 5, edits: 10, ts: new Date(NOW - 2 * 3600_000).toISOString() })); // 2h ago
  const a = aggregate(d, NOW);
  assert.equal(a.costDay, 15);
  assert.equal(a.churnDay, 40);
  assert.equal(a.costHour, 10); // only the recent one
});

test('checkGovernor — within ceilings → allowed', () => {
  const d = tmp();
  appendMetricsLine(d, line({ cost_usd: 5 }));
  assert.equal(checkGovernor(d, { nowMs: NOW }).allowed, true);
});

test('checkGovernor — cost/day breach → NOT allowed AND AUTO-HALTS (resume manual)', () => {
  const d = tmp();
  appendMetricsLine(d, line({ cost_usd: DEFAULT_GOVERNOR.costPerDayUsd + 1 }));
  const r = checkGovernor(d, { nowMs: NOW });
  assert.equal(r.allowed, false);
  assert.ok(r.breached.some((b) => /cost\/day/.test(b)));
  assert.equal(isHalted(d).halted, true, 'breach auto-halts');
  // a subsequent check stays blocked until a human clears it
  assert.equal(checkGovernor(d, { nowMs: NOW }).allowed, false);
  clearHalt(d);
  assert.equal(isHalted(d).halted, false);
});

test('checkGovernor — RAM + concurrency ceilings', () => {
  const d = tmp();
  assert.equal(checkGovernor(d, { nowMs: NOW, freeRamMb: 200 }).allowed, false); // < 1024
  clearHalt(d);
  assert.equal(checkGovernor(d, { nowMs: NOW, concurrentLoops: 5 }).allowed, false); // >= 2
});

test('masterHalt — manual halt blocks; clearHalt is the only resume', () => {
  const d = tmp();
  masterHalt(d, 'operator stop');
  const r = checkGovernor(d, { nowMs: NOW });
  assert.equal(r.allowed, false);
  assert.match(r.reason, /MASTER HALT|manual/i);
  clearHalt(d);
  assert.equal(checkGovernor(d, { nowMs: NOW }).allowed, true);
});

test('staleLoops — loops silent beyond the window are reaped', () => {
  const stale = staleLoops({ a: NOW - 1000, b: NOW - 120_000 }, NOW, 60_000);
  assert.deepEqual(stale, ['b']);
});
