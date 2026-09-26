import test from 'node:test';
import assert from 'node:assert/strict';
import { ind, unverified, judge, parseBaseline, formatBaseline, groupOf, median, pct, RULES } from './rules.mjs';

const I = (rule, value, extra = {}) => ind('g.x', value, 'u', rule, 's', extra);

test('ind refuses an unknown rule and keeps every field', () => {
  assert.throws(() => ind('a', 1, 'u', 'nope', 's'), /unknown rule nope/);
  assert.deepEqual(ind('a', 1, 'u', 'zero', 's', { note: 'n' }), { id: 'a', value: 1, unit: 'u', rule: 'zero', source: 's', note: 'n' });
  assert.equal(RULES.length, 8);
});

test('an unverified indicator is never judged, whatever its rule', () => {
  const u = unverified('a', 'u', 'zero', 's', 'why');
  assert.equal(u.value, null);
  assert.equal(u.unverified, 'why');
  assert.deepEqual(judge(u, 5), { status: 'unverified' });
  assert.deepEqual(judge(I('zero', undefined)), { status: 'unverified' });
});

test('trend never fails; zero, max and min judge against the value or the limit', () => {
  assert.equal(judge(I('trend', 1e9)).status, 'ok');
  assert.equal(judge(I('zero', 0)).status, 'ok');
  assert.deepEqual(judge(I('zero', 2)), { status: 'breach', why: '2 != 0' });
  assert.equal(judge(I('max', 800, { limit: 800 })).status, 'ok');
  assert.deepEqual(judge(I('max', 801, { limit: 800 })), { status: 'breach', why: '801 > limit 800' });
  assert.equal(judge(I('min', 1, { limit: 1 })).status, 'ok');
  assert.deepEqual(judge(I('min', 0, { limit: 1 })), { status: 'breach', why: '0 < limit 1' });
});

test('a first run with no baseline is new and proposes the value', () => {
  for (const b of [undefined, null, '']) assert.deepEqual(judge(I('ratchet', 7), b), { status: 'new', next: 7 });
});

test('ratchet: higher breaches, lower improves and lowers, equal is ok', () => {
  assert.deepEqual(judge(I('ratchet', 11), '10'), { status: 'breach', why: '11 > baseline 10' });
  assert.deepEqual(judge(I('ratchet', 9), '10'), { status: 'improved', next: 9, why: 'lowered 10 -> 9' });
  assert.deepEqual(judge(I('ratchet', 10), '10'), { status: 'ok' });
});

test('floor: lower breaches, higher raises, equal is ok', () => {
  assert.deepEqual(judge(I('floor', 9), 10), { status: 'breach', why: '9 < baseline 10' });
  assert.deepEqual(judge(I('floor', 11), 10), { status: 'improved', next: 11, why: 'raised 10 -> 11' });
  assert.deepEqual(judge(I('floor', 10), 10), { status: 'ok' });
});

test('plus25 allows a quarter over the baseline and no more', () => {
  assert.equal(judge(I('plus25', 125), 100).status, 'ok');
  assert.deepEqual(judge(I('plus25', 126), 100), { status: 'breach', why: '126 > 100 x 1.25' });
});

test('exact breaches on any move', () => {
  assert.equal(judge(I('exact', 5), 5).status, 'ok');
  assert.deepEqual(judge(I('exact', 6), 5), { status: 'breach', why: '6 != baseline 5' });
});

test('baselines parse the tools/gates shape and format back sorted', () => {
  const b = parseBaseline('# c\n\nb.y=2\n a.x=1 \nbroken\n=nokey\n');
  assert.deepEqual(b, { 'b.y': '2', 'a.x': '1' });
  assert.equal(formatBaseline(b, 'hdr'), '# hdr\na.x=1\nb.y=2\n');
  assert.equal(groupOf('wasm.section.code'), 'wasm');
});

test('median of three, of an even list, and of nothing', () => {
  assert.equal(median([30, 10, 20]), 20);
  assert.equal(median([1, 2, 3, 5]), 3);
  assert.equal(median([NaN]), null);
  assert.equal(pct([5, 1, 4, 2, 3], 0.9), 5);
  assert.equal(pct([5, 1, 4, 2, 3], 0.5), 3);
  assert.equal(pct([], 0.5), null);
});
