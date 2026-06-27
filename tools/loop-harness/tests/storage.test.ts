import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  nextRunIndex, writeRunRecord, readRunRecord, appendIter, appendMetricsLine, readMetrics,
} from '../src/storage.js';
import type { RunRecord, MetricsLine, IterationTelemetry } from '../src/types.js';

function tmp(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'loopstore-'));
}

const recordFixture = (run_index: number): RunRecord => ({
  loop: 'convergence', run_index, outcome: 'green', breaker_reason: null,
  iter_from: 1, iter_to: 4, t_start: 'T0', t_end: 'T1', wall_s: 120,
  goal: 'g', what_done: 'd', issues: [], patterns: [],
  telemetry: {
    iterations: 4, tests_fail_start: 8, tests_fail_end: 0, edits: 9, loc_add: 80, loc_del: 30,
    slop_min: 92, fake_green_caught: 0, commits: 1, conflicts: 0, prs: 0, rss_peak_mb: 5000,
    agents: {}, skills_used: {}, skills_ghost: [], tokens_in: 100, tokens_out: 20, cache_read: 50,
    cost_usd: 0.5, per_resolved: 15, eco: { kwh: 0, gco2: 0, water_ml: 0, method: 'deferred', estimate: true },
  },
  carry_forward: { guards: [], watch: [] },
});

test('storage — nextRunIndex is append-only (never overwrites a prior run)', () => {
  const dir = tmp();
  assert.equal(nextRunIndex(dir, 'convergence'), 1, 'empty → 1');
  writeRunRecord(dir, 'convergence', 1, recordFixture(1));
  assert.equal(nextRunIndex(dir, 'convergence'), 2, 'after run 1 → 2');
  writeRunRecord(dir, 'convergence', 2, recordFixture(2));
  assert.equal(nextRunIndex(dir, 'convergence'), 3, 'after run 2 → 3');
  // both records still present — nothing cleaned
  assert.deepEqual(readRunRecord(dir, 'convergence', 1).run_index, 1);
  assert.deepEqual(readRunRecord(dir, 'convergence', 2).run_index, 2);
});

test('storage — writeRunRecord/readRunRecord is a lossless gzip round-trip', () => {
  const dir = tmp();
  const rec = recordFixture(7);
  writeRunRecord(dir, 'convergence', 7, rec);
  const back = readRunRecord(dir, 'convergence', 7);
  assert.deepEqual(back, rec); // gzip is lossless — exact equality
});

test('storage — iteration trace + metrics index are append-only', () => {
  const dir = tmp();
  const iter = (i: number): IterationTelemetry => ({
    loop: 'convergence', run_index: 1, iteration: i, t_start: 'a', t_end: 'b', dur_s: 1,
    breaker: { state: 'running', stall_count: 0 }, progress_metric: 10 - i, progress_delta: -1,
  });
  appendIter(dir, 'convergence', 1, iter(1));
  appendIter(dir, 'convergence', 1, iter(2));
  const traceLines = fs.readFileSync(path.join(dir, 'convergence', '1.iters.jsonl'), 'utf8').split('\n').filter(Boolean);
  assert.equal(traceLines.length, 2);

  const ml = (run_index: number): MetricsLine => ({
    loop: 'convergence', run_index, ts: 'T', outcome: 'green', iters: 4, wall_s: 1,
    tokens_in: 1, tokens_out: 1, cost_usd: 1, kwh: 0, gco2: 0, water_ml: 0,
    fail_start: 8, fail_end: 0, per_resolved: 1, slop_min: 90, conflicts: 0, recurring_flags: [],
  });
  appendMetricsLine(dir, ml(1));
  appendMetricsLine(dir, ml(2));
  assert.equal(readMetrics(dir).length, 2);
  assert.equal(readMetrics(dir, 'convergence').length, 2);
  assert.equal(readMetrics(dir, 'other').length, 0);
});
