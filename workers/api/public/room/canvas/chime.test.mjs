// The new-ticket chime, pure: `node --test workers/api/public/room/canvas/chime.test.mjs` (CV8a).
// The AudioContext is a counting fake, so "no sound before a gesture" is a count, not a claim.
import test from 'node:test';
import assert from 'node:assert/strict';
import { arrivals, makeChime, KEY } from './chime.js';

function fakeAudio() {
  const made = { ctors: 0, starts: 0, resumes: 0 };
  class Ctx {
    constructor() { made.ctors++; this.state = 'running'; this.currentTime = 0; this.destination = {}; }
    createOscillator() { return { connect() {}, start() { made.starts++; }, stop() {}, frequency: {} }; }
    createGain() { return { connect() {}, gain: { setValueAtTime() {}, exponentialRampToValueAtTime() {} } }; }
    resume() { made.resumes++; }
  }
  return { Ctx, made };
}
const memory = (init = {}) => {
  const m = { ...init, sets: [] };
  return { get: k => m[k] ?? null, set: (k, v) => { m[k] = v; m.sets.push(v); }, m };
};

test('no AudioContext before the first pointer or key event', () => {
  const { Ctx, made } = fakeAudio();
  const vib = [];
  const c = makeChime({ AudioCtx: Ctx, vibrate: n => vib.push(n), store: memory() });
  c.ring(2);
  assert.equal(made.ctors, 0, 'a ring before any gesture creates no context');
  assert.deepEqual(vib, [], 'and does not vibrate');
  assert.equal(c.armed, false);
});

test('after a gesture the first ring makes the context, then one chime and one buzz', () => {
  const { Ctx, made } = fakeAudio();
  const vib = [];
  const c = makeChime({ AudioCtx: Ctx, vibrate: n => vib.push(n), store: memory() });
  c.gesture();
  assert.equal(made.ctors, 0, 'the gesture alone creates nothing');
  c.ring(1);
  c.ring(1);
  assert.equal(made.ctors, 1, 'one context, reused');
  assert.equal(made.starts, 4, 'two notes per chime');
  assert.deepEqual(vib, [12, 12]);
});

test('on by default; the stored choice "0" silences it and the button flips it back', () => {
  const { Ctx, made } = fakeAudio();
  const vib = [];
  const st = memory();
  const c = makeChime({ AudioCtx: Ctx, vibrate: n => vib.push(n), store: st });
  assert.equal(c.on, true, 'the default is on');
  c.gesture();
  assert.equal(c.toggle(), false);
  assert.equal(st.m[KEY], '0', 'the choice is kept on this device');
  c.ring(3);
  assert.equal(made.ctors, 0, 'off: no context');
  assert.deepEqual(vib, [], 'off: no buzz either');
  assert.equal(c.toggle(), true);
  assert.equal(st.m[KEY], '1');
  c.ring(1);
  assert.equal(made.ctors, 1);
  const off = makeChime({ AudioCtx: Ctx, store: memory({ [KEY]: '0' }) });
  assert.equal(off.on, false, 'a stored 0 starts silent');
});

test('a refused store keeps the choice for this page only, and nothing throws', () => {
  const broken = { get() { throw new Error('denied'); }, set() { throw new Error('denied'); } };
  const c = makeChime({ AudioCtx: null, store: broken });
  assert.equal(c.on, true);
  assert.equal(c.toggle(), false);
  c.gesture();
  c.ring(1);
});

test('a ring of nothing, or with no audio at all, is silent', () => {
  const { Ctx, made } = fakeAudio();
  const c = makeChime({ AudioCtx: Ctx, store: memory() });
  c.gesture();
  c.ring(0);
  assert.equal(made.ctors, 0);
  const none = makeChime({ AudioCtx: null, store: memory() });
  none.gesture();
  none.ring(1);
});

test('arrivals: the first snapshot rings for nothing; a new NEW-column id rings once', () => {
  const o = (id, status) => ({ id, status });
  const first = arrivals(null, [o('a', 'PENDING'), o('b', 'PREPARING')]);
  assert.equal(first.fresh, 0, 'the first snapshot is the baseline');
  const next = arrivals(first.ids, [o('a', 'CONFIRMED'), o('b', 'PREPARING'), o('c', 'PENDING'), o('d', 'READY')]);
  assert.equal(next.fresh, 1, 'c is new; a only moved on; b and d are not in the NEW column');
  assert.deepEqual([...next.ids].sort(), ['a', 'c']);
  assert.equal(arrivals(next.ids, [o('a', 'CONFIRMED'), o('c', 'PENDING')]).fresh, 0, 'nothing new');
  assert.equal(arrivals(null, null).fresh, 0);
});
