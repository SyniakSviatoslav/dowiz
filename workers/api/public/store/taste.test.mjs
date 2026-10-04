// `node --test workers/api/public/store/taste.test.mjs`
// The guest's taste on the phone (W-MR0 row MR7): what is recorded, how it fades, what the strip
// may do with it (add, re-order; never hide, never price), and the one vector that may leave.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as T from './taste.js';

const D = 20_000; // a day number
const menu = [
  { id: 'maki', categoryId: 'rolls', tags: ['salmon'], available: true },
  { id: 'nigiri', categoryId: 'rolls', tags: ['salmon', 'tuna'], available: true },
  { id: 'ramen', categoryId: 'soups', tags: ['hot'], available: true, allergens: ['gluten'] },
  { id: 'udon', categoryId: 'soups', tags: ['hot'], available: true, allergens: ['gluten'] },
  { id: 'cola', categoryId: 'drinks', tags: [], available: false },
];
const fold = (evs, day = D) => evs.reduce((p, e) => T.record(p, e, day), T.empty({ day, device: 'phone' }));

test('every stored time is a day; an hour is a 24-slot count only', () => {
  const p = fold([{ kind: 'visit', hour: 21 }, { kind: 'open', id: 'maki' }, { kind: 'open', id: 'maki' }]);
  assert.deepEqual(p.dishes.maki.open, [[D, 2]], 'two opens on one day are one entry');
  assert.equal(p.hours[21], 1);
  assert.equal(JSON.stringify(p).match(/\d{12,}/), null, 'no millisecond timestamp anywhere');
  assert.equal(T.dayOf(Date.UTC(2026, 9, 4, 23, 30), -120), T.dayOf(Date.UTC(2026, 9, 5, 0, 0)), 'local day, by the offset');
});

test('a signal fades by half in sixty days, and a year-old day is dropped on the next write', () => {
  assert.equal(T.decay(D, D), 1);
  assert.equal(T.decay(D, D + T.HALF_LIFE_DAYS), 0.5);
  let p = fold([{ kind: 'order', items: [{ id: 'maki', qty: 2 }] }], D);
  p = T.record(p, { kind: 'order', items: [{ id: 'maki', qty: 1 }] }, D + T.KEEP_DAYS + 1);
  assert.deepEqual(p.dishes.maki.order, [[D + T.KEEP_DAYS + 1, 1]]);
});

test('behaviour counts: opens, dwell (capped), adds; an add then a remove counts against', () => {
  const p = fold([{ kind: 'open', id: 'ramen' }, { kind: 'dwell', id: 'ramen', sec: 9999 }, { kind: 'add', id: 'udon' }, { kind: 'drop', id: 'udon' }]);
  const w = T.weights(p, menu, D);
  assert.equal(p.dishes.ramen.dwell[0][1], T.DWELL_CAP_S);
  assert.ok(w.dish.get('ramen') > 0);
  assert.ok(Math.abs(w.dish.get('udon') - (T.W.add + T.W.drop)) < 1e-9);
  assert.ok(w.tag.get('hot') > 0 && w.cat.get('soups') > 0, 'a dish passes its weight to its tags and category');
});

test('the strip: "again" first (at most two), then taste; only dishes on sale; at most six', () => {
  const p = fold([{ kind: 'order', items: [{ id: 'maki', qty: 3 }, { id: 'cola', qty: 9 }] }, { kind: 'open', id: 'ramen' }]);
  const s = T.strip(menu, p, D);
  assert.deepEqual(s[0], { id: 'maki', why: 'again' });
  assert.ok(!s.some(x => x.id === 'cola'), 'a dish off sale is not offered');
  assert.ok(s.length <= T.STRIP_MAX && s.filter(x => x.why === 'again').length <= T.AGAIN_MAX);
  assert.ok(s.some(x => x.id === 'nigiri' && x.why === 'taste'), 'the salmon the guest ordered lifts the other salmon dish');
});

test('an INFERRED allergy only moves a dish down the strip; it never hides one', () => {
  const p = fold([{ kind: 'open', id: 'ramen' }, { kind: 'open', id: 'ramen' }, { kind: 'open', id: 'maki' }]);
  const plain = T.strip(menu, p, D).map(x => x.id);
  const guessed = T.strip(menu, p, D, { avoidGuess: ['gluten'] }).map(x => x.id);
  assert.deepEqual([...guessed].sort(), [...plain].sort(), 'the same dishes');
  assert.ok(guessed.indexOf('ramen') > guessed.indexOf('maki'), 'the guessed one goes after');
  const before = JSON.stringify(menu);
  T.strip(menu, p, D, { avoidGuess: ['gluten'] });
  assert.equal(JSON.stringify(menu), before, 'the menu itself is never touched');
});

test('intent: browsing, ready to order, a group order -- with the rule that decided', () => {
  assert.deepEqual(T.intent({}), { kind: 'browsing', why: 'none' });
  assert.equal(T.intent({ adds: 1 }).kind, 'ready');
  assert.equal(T.intent({ cartQty: 5, cartLines: 2 }).kind, 'group');
});

test('first visit keeps a referrer HOST and the utm words; the device is a class', () => {
  assert.deepEqual(T.firstVisit('https://www.instagram.com/p/xyz?igsh=1', '?utm_source=ig&utm_campaign=autumn&x=1'),
    { ref: 'www.instagram.com', utm: { source: 'ig', campaign: 'autumn' } });
  assert.deepEqual(T.firstVisit('', ''), { ref: null, utm: null });
  assert.equal(T.deviceClass(390, true), 'phone');
  assert.equal(T.deviceClass(800, true), 'tablet');
  assert.equal(T.deviceClass(1440, false), 'desktop');
});

test('what may leave (only with consent): integer tag and category weights, nothing else', () => {
  const p = fold([{ kind: 'order', items: [{ id: 'maki', qty: 3 }] }, { kind: 'open', id: 'ramen' }, { kind: 'visit', hour: 20 }]);
  p.first = T.firstVisit('https://ref.example/a', '?utm_source=x');
  const v = T.syncVector(p, menu, D);
  assert.deepEqual(Object.keys(v).sort(), ['cats', 'tags', 'v']);
  assert.equal(v.tags.salmon, T.SYNC_SCALE);
  for (const n of [...Object.values(v.tags), ...Object.values(v.cats)]) assert.ok(Number.isInteger(n) && n >= 0 && n <= T.SYNC_SCALE);
  const s = JSON.stringify(v);
  for (const leak of ['maki', 'ramen', 'ref.example', 'phone', String(D)]) assert.ok(!s.includes(leak), `${leak} must not leave: ${s}`);
});

test('what this phone remembers, as plain counts', () => {
  const p = fold([{ kind: 'order', items: [{ id: 'maki', qty: 2 }] }, { kind: 'add', id: 'udon' }, { kind: 'drop', id: 'udon' }, { kind: 'visit', hour: 12 }]);
  const r = T.remembered(p);
  assert.equal(r.ordered, 1); assert.equal(r.portions, 2); assert.equal(r.added, 1); assert.equal(r.removed, 1); assert.equal(r.visits, 1);
});

test("the venue's profile (prior) adds to the phone's weights and ranks the strip; junk adds nothing", () => {
  const p = T.empty({ day: D });
  assert.deepEqual(T.strip(menu, p, D), [], 'an empty phone and no prior: no strip');
  const s = T.strip(menu, p, D, { prior: { tags: [{ key: 'hot', w: 3000 }], cats: [{ key: 'rolls', w: 1000 }] } });
  assert.deepEqual(s.map(x => x.id).slice(0, 2).sort(), ['ramen', 'udon'], 'hot (3 portions) outranks rolls (1)');
  assert.ok(s.every(x => x.why === 'taste'));
  assert.ok(!s.some(x => x.id === 'cola'), 'never an unavailable dish');
  assert.deepEqual(T.strip(menu, p, D, { prior: { tags: [{ key: 'hot', w: 'x' }, null, { w: 5 }], cats: 'no' } }), []);
});
