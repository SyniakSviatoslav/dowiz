import test from 'node:test';
import assert from 'node:assert/strict';
import {
  historyInit,
  historyPush,
  historyUndo,
  historyRedo,
  DEFAULT_HISTORY_LIMIT,
  type HistoryState,
} from './use-history-stack.js';

// GUARDRAIL — undo/redo history core (useHistoryStack). The React hook is a thin
// stable-callback wrapper over these pure functions; the reducer logic (push /
// undo / redo / bound / branch-truncation) is what carries the correctness, so it
// is unit-tested here with plain values — no React/DOM needed (tsx --test runner).

test('historyInit: starts with empty past/future and the given present', () => {
  const s = historyInit('a');
  assert.deepEqual(s, { past: [], present: 'a', future: [] });
});

test('historyPush: moves present into past, sets new present, clears future', () => {
  let s = historyInit('a');
  s = historyPush(s, 'b');
  assert.deepEqual(s, { past: ['a'], present: 'b', future: [] });
  s = historyPush(s, 'c');
  assert.deepEqual(s, { past: ['a', 'b'], present: 'c', future: [] });
});

test('historyPush: pushing a value equal to present is a no-op that returns the SAME state object', () => {
  const s = historyPush(historyInit('a'), 'b');
  const same = historyPush(s, 'b');
  assert.equal(same, s, 'no-op push must return the identical state object (lets the hook skip re-renders)');
});

test('historyPush: custom isEqual drops value-equal (but not reference-equal) snapshots', () => {
  type Draft = { name: string };
  const eq = (a: Draft, b: Draft) => JSON.stringify(a) === JSON.stringify(b);
  const s = historyInit<Draft>({ name: 'x' });
  const same = historyPush(s, { name: 'x' }, { isEqual: eq });
  assert.equal(same, s, 'deep-equal snapshot must not create a history entry');
  const changed = historyPush(s, { name: 'y' }, { isEqual: eq });
  assert.equal(changed.present.name, 'y');
  assert.equal(changed.past.length, 1);
});

test('historyUndo: steps back, pushing the displaced present onto future', () => {
  let s = historyInit('a');
  s = historyPush(s, 'b');
  s = historyPush(s, 'c');
  s = historyUndo(s);
  assert.deepEqual(s, { past: ['a'], present: 'b', future: ['c'] });
  s = historyUndo(s);
  assert.deepEqual(s, { past: [], present: 'a', future: ['b', 'c'] });
});

test('historyUndo: with an empty past is a no-op returning the SAME state object', () => {
  const s = historyInit('a');
  assert.equal(historyUndo(s), s);
});

test('historyRedo: steps forward, restoring from future in order', () => {
  let s = historyInit('a');
  s = historyPush(s, 'b');
  s = historyPush(s, 'c');
  s = historyUndo(s);
  s = historyUndo(s);
  s = historyRedo(s);
  assert.deepEqual(s, { past: ['a'], present: 'b', future: ['c'] });
  s = historyRedo(s);
  assert.deepEqual(s, { past: ['a', 'b'], present: 'c', future: [] });
});

test('historyRedo: with an empty future is a no-op returning the SAME state object', () => {
  const s = historyPush(historyInit('a'), 'b');
  assert.equal(historyRedo(s), s);
});

test('branch truncation: a push after undo discards the redo branch', () => {
  let s = historyInit('a');
  s = historyPush(s, 'b');
  s = historyPush(s, 'c');
  s = historyUndo(s); // present 'b', future ['c']
  s = historyPush(s, 'd');
  assert.deepEqual(s, { past: ['a', 'b'], present: 'd', future: [] }, 'the "c" branch must be gone');
  assert.equal(historyRedo(s), s, 'redo after the truncating push has nothing to restore');
});

test('limit: past is bounded — oldest entries are dropped, newest survive', () => {
  let s: HistoryState<number> = historyInit(0);
  for (let i = 1; i <= 10; i++) s = historyPush(s, i, { limit: 3 });
  assert.equal(s.present, 10);
  assert.deepEqual(s.past, [7, 8, 9], 'only the 3 most recent undo steps are kept');
  // Undoing past the bound stops at the oldest retained entry.
  s = historyUndo(historyUndo(historyUndo(s)));
  assert.equal(s.present, 7);
  assert.equal(historyUndo(s), s, 'cannot undo beyond the bounded window');
});

test('limit: defaults to DEFAULT_HISTORY_LIMIT and never goes below 1', () => {
  let s: HistoryState<number> = historyInit(0);
  for (let i = 1; i <= DEFAULT_HISTORY_LIMIT + 10; i++) s = historyPush(s, i);
  assert.equal(s.past.length, DEFAULT_HISTORY_LIMIT);

  let tiny: HistoryState<number> = historyInit(0);
  tiny = historyPush(tiny, 1, { limit: 0 }); // clamped to 1
  tiny = historyPush(tiny, 2, { limit: 0 });
  assert.deepEqual(tiny.past, [1], 'limit is clamped to at least 1 so undo always has one step');
});

test('immutability: push/undo/redo never mutate the input state', () => {
  const s0 = historyPush(historyInit('a'), 'b');
  const frozen = Object.freeze({ past: Object.freeze([...s0.past]) as string[], present: s0.present, future: Object.freeze([...s0.future]) as string[] });
  // Throws under 'use strict' (node ESM) if any function mutates its input.
  const pushed = historyPush(frozen, 'c');
  const undone = historyUndo(frozen);
  assert.deepEqual(frozen, s0, 'input state unchanged');
  assert.equal(pushed.present, 'c');
  assert.equal(undone.present, 'a');
});
