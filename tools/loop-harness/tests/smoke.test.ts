import { test } from 'node:test';
import assert from 'node:assert/strict';
import { smokeTest, defaultSeed } from '../src/smoke.js';
import type { LoopDesign } from '../src/loop-builder.js';

const design = (over: Partial<LoopDesign> = {}): LoopDesign => ({
  id: 'x', goal: 'g', tags: [], oracle: 'failing ↓', tools: [], iterate: 'fix one', isTerminal: 'm<=0',
  reflect: 'harness fresh-context reviewer', scopeClass: 'A', carveOut: ['**/auth/**'],
  breaker: { K: 3, maxIter: 25, budgetUsd: null, timeCapMs: null }, reuseOf: null, ...over,
});

test('smokeTest — a sound design (metric moves, terminates in budget, scoped) PASSES', async () => {
  const d = design();
  const r = await smokeTest(d, { startMetric: 8, perIterDelta: 1, scopeClean: true });
  assert.equal(r.pass, true, r.detail);
  assert.equal(r.terminated, true);
  assert.equal(r.outcome, 'green');
});

test('smokeTest — a STUCK design (metric never moves) is REJECTED (breaker stall)', async () => {
  const r = await smokeTest(design(), { startMetric: 8, perIterDelta: 0, scopeClean: true });
  assert.equal(r.pass, false);
  assert.equal(r.moved, false);
  assert.match(r.detail, /stuck|move/i);
});

test('smokeTest — a breaker too tight for the scenario is REJECTED (does not terminate)', async () => {
  const d = design({ breaker: { K: 3, maxIter: 3, budgetUsd: null, timeCapMs: null } });
  const r = await smokeTest(d, { startMetric: 10, perIterDelta: 1, scopeClean: true }); // needs 10 iters, only 3 allowed
  assert.equal(r.pass, false);
  assert.equal(r.terminated, false);
  assert.match(r.detail, /terminate|maxIter/i);
});

test('smokeTest — out-of-scope churn is REJECTED', async () => {
  const r = await smokeTest(design(), { startMetric: 5, perIterDelta: 1, scopeClean: false });
  assert.equal(r.pass, false);
  assert.equal(r.noChurn, false);
  assert.match(r.detail, /scope|carve/i);
});

test('defaultSeed — keeps the scenario reachable within the design breaker', () => {
  const s = defaultSeed(design(), true);
  assert.ok(s.startMetric < design().breaker.maxIter, 'reachable before maxIter');
  assert.equal(s.perIterDelta, 1);
});
