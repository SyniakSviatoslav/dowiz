import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DEFAULT_BREAKER, initBreaker, stepBreaker, breakerReasonText } from '../src/breaker.js';
import type { BreakerReason } from '../src/types.js';

const cfg = { ...DEFAULT_BREAKER, K: 3, maxIter: 25, budgetUsd: 10, timeCapMs: 60_000 };
const step = (s: ReturnType<typeof initBreaker>, delta: number, iteration: number, cost = 0, ms = 0) =>
  stepBreaker(s, { delta, iteration, cumulativeCostUsd: cost, elapsedMs: ms }, cfg);

test('breaker — trips on stall after K non-improving iterations', () => {
  let s = initBreaker();
  s = step(s, 0, 1); assert.equal(s.tripped, false, 'iter1 no trip');
  s = step(s, 0, 2); assert.equal(s.tripped, false, 'iter2 no trip');
  s = step(s, 1, 3); // delta>=0 third time → stall reaches K=3
  assert.equal(s.tripped, true);
  assert.equal(s.reason, 'stall');
  assert.equal(s.stallCount, 3);
});

test('breaker — a strictly-improving iteration (delta<0) resets the stall counter', () => {
  let s = initBreaker();
  s = step(s, 0, 1);
  s = step(s, 0, 2);
  assert.equal(s.stallCount, 2);
  s = step(s, -3, 3); // progress!
  assert.equal(s.stallCount, 0);
  assert.equal(s.tripped, false);
});

test('breaker — trips on max_iter cap', () => {
  // improving every time so stall never fires, but iteration hits the cap
  let s = initBreaker();
  s = step(s, -1, 25);
  assert.equal(s.tripped, true);
  assert.equal(s.reason, 'max_iter');
});

test('breaker — trips on budget cap', () => {
  let s = initBreaker();
  s = step(s, -1, 2, 10.5);
  assert.equal(s.tripped, true);
  assert.equal(s.reason, 'budget');
});

test('breaker — trips on time cap', () => {
  let s = initBreaker();
  s = step(s, -1, 2, 0, 61_000);
  assert.equal(s.tripped, true);
  assert.equal(s.reason, 'time_cap');
});

test('breaker — stall takes precedence when multiple caps hit', () => {
  let s = initBreaker();
  s = step(s, 0, 1); s = step(s, 0, 2);
  s = step(s, 0, 25, 999, 999_999); // stall AND max_iter AND budget AND time all true
  assert.equal(s.reason, 'stall');
});

test('breakerReasonText — every reason maps to a distinct, non-empty, descriptive string', () => {
  // expected substring that proves the text actually describes THIS cap, not a generic blob
  const cases: Array<[BreakerReason, string]> = [
    ['stall', 'progress'],
    ['max_iter', 'MAX_ITER'],
    ['budget', 'BUDGET'],
    ['time_cap', 'TIME_CAP'],
  ];
  const seen = new Set<string>();
  for (const [reason, mustMention] of cases) {
    const text = breakerReasonText(reason);
    assert.equal(typeof text, 'string', `${reason} → string`);
    assert.ok(text.trim().length > 0, `${reason} → non-empty`);
    assert.ok(text.includes(mustMention), `${reason} → mentions "${mustMention}" (got: ${text})`);
    assert.equal(seen.has(text), false, `${reason} → distinct text (collision: ${text})`);
    seen.add(text);
  }
  assert.equal(seen.size, cases.length, 'all four reasons produce distinct text');
});
