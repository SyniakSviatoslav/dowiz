import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { runLoop } from '../src/harness.js';
import { readRunRecord, readMetrics } from '../src/storage.js';
import type { Loop, IterationOutcome } from '../src/types.js';

function tmp(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'loopharness-'));
}
// deterministic clock advancing 1000ms per read
function fakeClock() {
  let t = 0;
  return () => (t += 1000);
}

interface S { failing: number }

// A loop that fixes one failing test per iteration.
const convergingLoop = (perIterCost = 0.1): Loop<S, unknown> => ({
  id: 'convergence',
  goal: () => 'reach green',
  iterate: (_ctx, s: S): IterationOutcome<S> => ({
    state: { failing: Math.max(0, s.failing - 1) },
    telemetry: { code: { tests_fail_after: Math.max(0, s.failing - 1), edits: 2 }, tokens: { in: 100, out: 20, cost_usd: perIterCost } },
    reflection: { changed: ['fixed one test'], verified: ['one more green'], not_verified: [], risks: [], confidence: 0.8 },
  }),
  progressMetric: (s: S) => s.failing,
  isTerminal: (s: S) => s.failing === 0,
});

test('harness — happy path: converges to GREEN, prints report, persists losslessly', async () => {
  const dir = tmp();
  const printed: string[] = [];
  const rec = await runLoop(convergingLoop(), { failing: 3 }, {
    baseDir: dir, ctx: {}, clockMs: fakeClock(), print: (s) => printed.push(s),
  });

  assert.equal(rec.outcome, 'green');
  assert.equal(rec.iter_to, 3, '3 iterations to clear 3 failing');
  assert.equal(rec.telemetry.tests_fail_start, 3);
  assert.equal(rec.telemetry.tests_fail_end, 0);

  // report ALWAYS printed, in full
  assert.equal(printed.length, 1);
  assert.match(printed[0]!, /LOOP REPORT · convergence · run #1 .* GREEN ✓/);

  // persisted + lossless round-trip
  assert.deepEqual(readRunRecord(dir, 'convergence', 1), rec);
  // metrics index appended
  const m = readMetrics(dir, 'convergence');
  assert.equal(m.length, 1);
  assert.equal(m[0]!.outcome, 'green');
  assert.equal(m[0]!.fail_start, 3);
});

test('harness — second run on the same store gets run #2 (append-only)', async () => {
  const dir = tmp();
  const clock = () => { let t = 0; return () => (t += 1000); };
  await runLoop(convergingLoop(), { failing: 2 }, { baseDir: dir, ctx: {}, clockMs: clock(), print: () => {} });
  const rec2 = await runLoop(convergingLoop(), { failing: 2 }, { baseDir: dir, ctx: {}, clockMs: clock(), print: () => {} });
  assert.equal(rec2.run_index, 2);
  assert.equal(readMetrics(dir, 'convergence').length, 2);
});

test('harness — stall path: no-progress loop trips the breaker and reports STALL', async () => {
  const dir = tmp();
  const printed: string[] = [];
  // never improves: failing stays at 5
  const stuck: Loop<S, unknown> = {
    id: 'convergence',
    goal: () => 'reach green',
    iterate: (_c, s: S) => ({ state: { failing: s.failing }, telemetry: { tokens: { cost_usd: 0.1 } } }),
    progressMetric: (s: S) => s.failing,
    isTerminal: (s: S) => s.failing === 0,
  };
  const rec = await runLoop(stuck, { failing: 5 }, {
    baseDir: dir, ctx: {}, breaker: { K: 3 }, clockMs: fakeClock(), print: (s) => printed.push(s),
  });

  assert.equal(rec.outcome, 'stall');
  assert.equal(rec.breaker_reason, 'stall');
  assert.equal(rec.iter_to, 3, 'stops at K=3, does not run away');
  assert.match(printed[0]!, /STALL ✗/); // partial report still printed on trip
});

test('harness — budget cap aborts a long-but-progressing run', async () => {
  const dir = tmp();
  const rec = await runLoop(convergingLoop(5), { failing: 100 }, {
    baseDir: dir, ctx: {}, breaker: { budgetUsd: 12 }, clockMs: fakeClock(), print: () => {},
  });
  assert.equal(rec.outcome, 'abort');
  assert.equal(rec.breaker_reason, 'budget');
});
