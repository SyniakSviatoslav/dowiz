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
    apply: () => { spy.applies++; },
    revert: () => { spy.reverts++; },
    green: over.green ?? (() => true),
    security: over.security ?? (() => true),
  };
}

test('oracle — KEEP when reversible + green + secure + speedup ≥ threshold (revert NOT called)', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ beforeAfter: [100, 90] }, spy)); // 10% faster
  assert.equal(v.decision, 'kept');
  assert.equal(v.speedup_pct, 10);
  assert.equal(spy.applies, 1);
  assert.equal(spy.reverts, 0, 'kept change must not be reverted');
});

test('oracle — exact threshold (5%) keeps', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ beforeAfter: [100, 95] }, spy));
  assert.equal(v.decision, 'kept');
});

test('oracle — NOT reversible → refuse to apply at all', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ reversible: false }, spy));
  assert.equal(v.decision, 'rolled_back');
  assert.match(v.reason, /reversible/i);
  assert.equal(spy.applies, 0, 'must not apply an irreversible change');
  assert.equal(spy.reverts, 0);
});

test('oracle — tests RED → atomic rollback (revert called)', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ green: () => false }, spy));
  assert.equal(v.decision, 'rolled_back');
  assert.equal(v.green, false);
  assert.match(v.reason, /RED/);
  assert.equal(spy.reverts, 1);
});

test('oracle — security regression → atomic rollback (revert called)', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ security: () => false }, spy));
  assert.equal(v.decision, 'rolled_back');
  assert.equal(v.green, true);
  assert.equal(v.security_ok, false);
  assert.match(v.reason, /security/i);
  assert.equal(spy.reverts, 1);
});

test('oracle — passes tests but no speedup → rolled back (added risk for nothing)', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ beforeAfter: [100, 98] }, spy)); // 2% < 5%
  assert.equal(v.decision, 'rolled_back');
  assert.equal(v.green, true);
  assert.equal(v.security_ok, true);
  assert.match(v.reason, /no proven speedup/);
  assert.equal(spy.reverts, 1);
});

test('oracle — a regression (slower) is rolled back', async () => {
  const spy = { applies: 0, reverts: 0 };
  const v = await evaluate(hooks({ beforeAfter: [100, 110] }, spy)); // slower
  assert.equal(v.decision, 'rolled_back');
  assert.ok(v.speedup_pct! < 0);
  assert.equal(spy.reverts, 1);
});
