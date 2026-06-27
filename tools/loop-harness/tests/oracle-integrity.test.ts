import { test } from 'node:test';
import assert from 'node:assert/strict';
import { checkOracleIntegrity, countTests, countAssertions, weakenersAdded } from '../src/oracle-integrity.js';

const T2 = "test('a', () => { expect(x).toBe(1); });\ntest('b', () => { assert.equal(y, 2); });\n";
const T1 = "test('a', () => { expect(x).toBe(1); });\n";

test('countTests / countAssertions', () => {
  assert.equal(countTests(T2), 2);
  assert.equal(countAssertions(T2), 2);
});

test('checkOracleIntegrity — a removed test trips (count dropped)', () => {
  const r = checkOracleIntegrity({ files: [{ path: 'x.test.ts', before: T2, after: T1 }] });
  assert.equal(r.ok, false);
  assert.ok(r.trips.some((t) => /test count dropped/.test(t)));
});

test('checkOracleIntegrity — weakening a test (.skip / expect(true)) trips no-fake-green', () => {
  const before = "test('a', () => { expect(x).toBe(1); });\n";
  const after = "test.skip('a', () => { expect(true); });\n";
  assert.deepEqual(weakenersAdded(before, after).sort(), ['expect(true)', 'skip/only/fixme/todo']);
  const r = checkOracleIntegrity({ files: [{ path: 'x.test.ts', before, after }] });
  assert.equal(r.ok, false);
  assert.ok(r.trips.some((t) => /WEAKENED/.test(t)));
});

test('checkOracleIntegrity — mutating the immutable benchmark scenario trips', () => {
  const r = checkOracleIntegrity({
    files: [{ path: 'bench/scenario.json', before: '{"n":100}', after: '{"n":1}' }],
    benchmarkPaths: ['bench/scenario'],
  });
  assert.equal(r.ok, false);
  assert.ok(r.trips.some((t) => /benchmark scenario MUTATED/.test(t)));
});

test('checkOracleIntegrity — reviewer change without verifiable fresh context trips', () => {
  const r = checkOracleIntegrity({ files: [{ path: 'src/reviewer.ts', before: 'a', after: 'b' }], reviewerFresh: false });
  assert.equal(r.ok, false);
  assert.ok(r.trips.some((t) => /reviewer/i.test(t)));
});

test('checkOracleIntegrity — a clean change (test ADDED, no weakening) is ok', () => {
  const r = checkOracleIntegrity({ files: [{ path: 'x.test.ts', before: T1, after: T2 }] });
  assert.equal(r.ok, true);
});
