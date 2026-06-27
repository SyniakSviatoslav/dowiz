import { test } from 'node:test';
import assert from 'node:assert/strict';
import { evaluate, type OracleHooks } from '../src/oracle.js';

interface Spy { applies: number; reverts: number; }
function hooks(over: Partial<OracleHooks> & { beforeAfter?: [number, number] }, spy: Spy): OracleHooks {
  const [before, after] = over.beforeAfter ?? [100, 90];
  let measured = 0;
  return {
    reversible: over.reversible ?? true,
    measure: () => (measured++ === 0 ? before : after),
    apply: over.apply ?? (() => { spy.applies++; }),
    revert: over.revert ?? (() => { spy.reverts++; }),
    green: over.green ?? (() => true),
    security: over.security ?? (() => true),
  };
}

test('oracle — KEEP when reversible + green + secure + speedup ≥ threshold (revert NOT called)', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ beforeAfter: [100, 90] }, spy)); // 10% faster
  assert.equal(v.decision, 'kept');
  assert.equal(v.kept, true, 'decision=kept must agree with kept flag');
  assert.equal(v.speedup_pct, 10);
  assert.equal(spy.applies, 1);
  assert.equal(spy.reverts, 0, 'kept change must not be reverted');
});

test('oracle — exact threshold (5%) keeps', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ beforeAfter: [100, 95] }, spy));
  assert.equal(v.decision, 'kept');
  assert.equal(v.kept, true, 'decision=kept must agree with kept flag');
});

test('oracle — NOT reversible → refuse to apply at all', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ reversible: false }, spy));
  assert.equal(v.decision, 'rolled_back');
  assert.equal(v.kept, false, 'decision=rolled_back must agree with kept flag');
  assert.match(v.reason, /reversible/i);
  assert.equal(spy.applies, 0, 'must not apply an irreversible change');
  assert.equal(spy.reverts, 0);
});

test('oracle — tests RED → atomic rollback (revert called)', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ green: () => false }, spy));
  assert.equal(v.decision, 'rolled_back');
  assert.equal(v.kept, false, 'decision=rolled_back must agree with kept flag');
  assert.equal(v.green, false);
  assert.match(v.reason, /RED/);
  assert.equal(spy.reverts, 1);
});

test('oracle — security regression → atomic rollback (revert called)', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ security: () => false }, spy));
  assert.equal(v.decision, 'rolled_back');
  assert.equal(v.kept, false, 'decision=rolled_back must agree with kept flag');
  assert.equal(v.green, true);
  assert.equal(v.security_ok, false);
  assert.match(v.reason, /security/i);
  assert.equal(spy.reverts, 1);
});

test('oracle — passes tests but no speedup → rolled back (added risk for nothing)', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ beforeAfter: [100, 98] }, spy)); // 2% < 5%
  assert.equal(v.decision, 'rolled_back');
  assert.equal(v.kept, false, 'decision=rolled_back must agree with kept flag');
  assert.equal(v.green, true);
  assert.equal(v.security_ok, true);
  assert.match(v.reason, /no proven speedup/);
  assert.equal(spy.reverts, 1);
});

test('oracle — a regression (slower) is rolled back', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ beforeAfter: [100, 110] }, spy)); // slower
  assert.equal(v.decision, 'rolled_back');
  assert.equal(v.kept, false, 'decision=rolled_back must agree with kept flag');
  assert.ok(v.speedup_pct! < 0);
  assert.equal(spy.reverts, 1);
});

// --- failure-of-a-hook itself (apply/green/security rejecting), not just a false verdict ---
// These pin the CURRENT behavior: oracle.ts has NO try/catch around apply()/green()/
// security(), so a thrown/rejected hook propagates out of evaluate() and revert() is
// NEVER called → the worktree/config is left half-applied. The assertions go RED if the
// oracle ever changes that contract.
// TODO(escalate, finding #1/#2 — red-line: reversibility/rollback): oracle.evaluate should
//   wrap apply()→security() in try/catch and revert() on any rejection, then surface a
//   rolled_back verdict. When that lands, flip these to assert decision==='rolled_back'
//   AND spy.reverts===1. Tracked in needs_staging.

test('oracle — apply() throws → propagates; revert NOT called (half-applied) — KNOWN GAP', async () => {
  const spy = { applies: 0, reverts: 0 };
  await assert.rejects(
    () => evaluate(hooks({ apply: () => { throw new Error('apply boom'); } }, spy)),
    /apply boom/,
  );
  // known-bug: no try/catch → no atomic rollback on apply failure.
  assert.equal(spy.reverts, 0, 'documents the missing rollback-on-apply-throw');
});

test('oracle — green() rejects (runner down) → propagates; revert NOT called — KNOWN GAP', async () => {
  const spy = { applies: 0, reverts: 0 };
  await assert.rejects(
    () => evaluate(hooks({ green: () => Promise.reject(new Error('runner down')) }, spy)),
    /runner down/,
  );
  assert.equal(spy.applies, 1, 'apply ran before green failed');
  assert.equal(spy.reverts, 0, 'documents the missing rollback-on-green-rejection');
});

test('oracle — security() rejects → propagates; revert NOT called — KNOWN GAP', async () => {
  const spy = { applies: 0, reverts: 0 };
  await assert.rejects(
    () => evaluate(hooks({ security: () => Promise.reject(new Error('sec runner down')) }, spy)),
    /sec runner down/,
  );
  assert.equal(spy.applies, 1, 'apply ran before security failed');
  assert.equal(spy.reverts, 0, 'documents the missing rollback-on-security-rejection');
});
