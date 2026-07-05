import test from 'node:test';
import assert from 'node:assert/strict';
import { canon, shc, diffPaths, integrityGate } from '../src/integrity.mjs';

const ORDER = { id: 'o_1', status: 'READY', courier_id: null, total: 1250 };

test('shc is key-order-insensitive (same state ⇒ same hash)', () => {
  const shuffled = { total: 1250, courier_id: null, status: 'READY', id: 'o_1' };
  assert.equal(shc(ORDER), shc(shuffled));
});

test('shc changes when state changes', () => {
  assert.notEqual(shc(ORDER), shc({ ...ORDER, status: 'IN_DELIVERY' }));
});

test('shc field subset ignores out-of-contract fields', () => {
  const fields = ['id', 'status', 'courier_id'];
  assert.equal(shc({ ...ORDER, total: 9999 }, fields), shc(ORDER, fields));
  assert.notEqual(shc({ ...ORDER, status: 'PICKED_UP' }, fields), shc(ORDER, fields));
});

test('canon sorts nested keys deterministically', () => {
  assert.equal(canon({ b: { y: 2, x: 1 }, a: 0 }), '{"a":0,"b":{"x":1,"y":2}}');
});

test('diffPaths pinpoints exactly the diverged paths', () => {
  const actual = { ...ORDER, status: 'IN_DELIVERY', courier_id: 'c_7' };
  assert.deepEqual(diffPaths(ORDER, actual).sort(), ['courier_id', 'status']);
  assert.deepEqual(diffPaths(ORDER, { ...ORDER }), []);
});

test('gate passes on identical state with zero drift', () => {
  const g = integrityGate(ORDER, { ...ORDER });
  assert.equal(g.pass, true);
  assert.equal(g.drift, 0);
  assert.deepEqual(g.mismatches, []);
});

test('gate circuit-breaks on divergence (strict default corridor)', () => {
  const g = integrityGate(ORDER, { ...ORDER, status: 'CANCELLED' });
  assert.equal(g.pass, false);
  assert.equal(g.inCorridor, false);
  assert.deepEqual(g.mismatches, ['status']);
  assert.ok(g.drift > 0, `drift must be > 0 on divergence, got ${g.drift}`);
  assert.notEqual(g.shcExpected, g.shcActual);
});

test('safety corridor: young mismatch is IN-FLIGHT (pass+flag), stale mismatch fails', () => {
  const actual = { ...ORDER, status: 'PICKED_UP' };
  const young = integrityGate(ORDER, actual, { ageMs: 800, corridorMs: 3000 });
  assert.equal(young.pass, true);
  assert.equal(young.inCorridor, true);
  assert.deepEqual(young.mismatches, ['status']); // still reported, never hidden
  const stale = integrityGate(ORDER, actual, { ageMs: 9000, corridorMs: 3000 });
  assert.equal(stale.pass, false);
  assert.equal(stale.inCorridor, false);
});

test('gate honors field-subset contract', () => {
  const g = integrityGate({ ...ORDER, total: 1 }, { ...ORDER, total: 2 }, { fields: ['id', 'status'] });
  assert.equal(g.pass, true);
});
