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
  // Distinguish a REAL stall from the already-green case (where moved is also false): a stall
  // must have actually iterated until the breaker tripped at K=3, not terminated instantly.
  assert.equal(r.terminated, false);
  assert.equal(r.outcome, 'stall');
  assert.equal(r.iters, design().breaker.K, 'stall trips after K non-improving iterations, having actually run');
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

test('smokeTest — an ALREADY-GREEN scenario (startMetric 0) terminates instantly green', async () => {
  const r = await smokeTest(design(), { startMetric: 0, perIterDelta: 1, scopeClean: true });
  // The harness behaves correctly: terminal on entry, zero iterations, green outcome.
  assert.equal(r.terminated, true);
  assert.equal(r.outcome, 'green');
  assert.equal(r.iters, 0);
  // known-bug (smoke.ts:56,59): moved = (tests_fail_end < tests_fail_start) is false when
  // start===0, so a correctly already-green design is REJECTED as "stuck". Pin current
  // behavior; a future fix (treat 0-iter green as moved) must flip these two assertions RED.
  assert.equal(r.moved, false);
  assert.equal(r.pass, false);
});

test('smokeTest — noChurn is a pass-through of seed.scopeClean, NOT harness-measured', async () => {
  // known-bug (smoke.ts:58): noChurn === seed.scopeClean — the harness never measures actual
  // out-of-scope edits against carveOut. Proof: scopeClean:true yields noChurn:true even with
  // an EMPTY carve-out, and scopeClean:false yields noChurn:false even WITH a security carve-out.
  // A real scope-enforcement wiring (derive churn from edits vs carveOut) must make this go RED.
  const noCarve = await smokeTest(design({ carveOut: [] }), { startMetric: 4, perIterDelta: 1, scopeClean: true });
  assert.equal(noCarve.noChurn, true);
  const withCarve = await smokeTest(design({ carveOut: ['**/auth/**'] }), { startMetric: 4, perIterDelta: 1, scopeClean: false });
  assert.equal(withCarve.noChurn, false);
});

test('defaultSeed — keeps the scenario reachable within the design breaker', () => {
  const s = defaultSeed(design(), true);
  assert.ok(s.startMetric < design().breaker.maxIter, 'reachable before maxIter');
  assert.equal(s.perIterDelta, 1);
});
