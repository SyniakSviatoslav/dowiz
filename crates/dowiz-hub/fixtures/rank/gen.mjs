// The SHARED FIXTURE of W-TASTE row 1: 1000 guests x 4 menus, ranked by the phone's integer strip.
//
//   node crates/dowiz-hub/fixtures/rank/gen.mjs            writes strip.json beside this file
//
// GENERATED ONCE (2026-10-05) and committed; regenerate ONLY on a deliberate change of the rule,
// in the same commit as both rankers. Two readers hold it:
//   workers/api/public/store/taste-int.test.mjs   the phone's taste.js must give `want`, 1000/1000
//   crates/dowiz-hub/src/rank/tests.rs            the hub's rank::strip must give `want`, 1000/1000
// so the phone and the hub agree with each other through the same file. The inputs are seeded
// (one 32-bit LCG), cover every path of the rule on purpose (future days, ages past the table,
// negative weights, unknown dishes, a v2 record, junk prior rows, legacy `taste` dishes, empty
// and absent contexts, unknown moods, ids that differ only in case) and are plain JSON.

import { writeFileSync } from 'node:fs';
import * as T from '../../../../workers/api/public/store/taste.js';
import * as I from '../../../../workers/api/public/store/taste-int.js';
import { TASTE, TEXTURE, AROMA, MOOD_IDS } from '../../../../workers/api/public/store/sense.js';

const SEED = 0x7a57e;
let x = SEED >>> 0;
const next = () => (x = (Math.imul(x, 1664525) + 1013904223) >>> 0);
const r = n => Math.floor(next() / 2 ** 32 * n);
const pick = arr => arr[r(arr.length)];
const some = (arr, n) => { const a = [...arr], out = []; while (out.length < n && a.length) out.push(a.splice(r(a.length), 1)[0]); return out; };

const TAGS = ['salmon', 'tuna', 'eel', 'veggie', 'hot', 'fried', 'raw', 'rice', 'noodle', 'sweet', 'kids', 'new'];
const EU14 = ['gluten', 'crustaceans', 'eggs', 'fish', 'peanuts', 'soy', 'milk', 'nuts', 'celery', 'mustard', 'sesame', 'sulphites', 'lupin', 'molluscs'];
const CTX = ['band:evening', 'band:midday', 'day:weekend', 'wx:rain', 'wx:hot'];

function sense(){
  const k = r(10);
  if (k < 2) return {};
  if (k === 2) return { taste: Object.fromEntries([...some(['spicy', 'sweet', 'salty', 'sour'], 1 + r(3)).map(a => [a, 1 + r(3)]), ['richness', 2]]) };
  const s = { v: 1, taste: {}, texture: {}, aroma: {} };
  for (const a of some(TASTE, r(4))) s.taste[a] = r(6);
  for (const t of some(TEXTURE, r(3))) s.texture[t] = 1 + r(3);
  for (const a of some(AROMA, r(3))) s.aroma[a] = 1 + r(3);
  return { sense: s };
}
function dish(mi, i){
  const id = (i % 7 === 3 ? 'M' : 'm') + `${mi}-d${String(i).padStart(3, '0')}`;
  const d = { id, name: `Dish ${i}`, price: 300 + 50 * r(40), available: r(10) !== 0, tags: some(TAGS, r(4)), ...sense() };
  const c = r(10);
  if (c > 0) d.categoryId = c === 1 ? '' : `c${r(8)}`;
  if (r(4)) d.allergens = some(EU14, r(3));
  return d;
}
const menus = [24, 60, 110, 165].map((n, mi) => Array.from({ length: n }, (_, i) => dish(mi, i)));

function list(day, n, maxAge, maxN){
  return Array.from({ length: n }, () => {
    const k = r(20);
    const d = k === 0 ? day + 1 + r(3) : k === 1 ? day - 2400 - r(2000) : day - r(maxAge);
    return [d, maxN === 0 ? r(121) : 1 + r(maxN)];
  });
}
function profile(menu, day){
  const p = { v: r(40) === 0 ? 2 : 1, since: day - 400, device: 'phone', first: null, dishes: {}, cats: {}, hours: Array(24).fill(0) };
  const ids = [...some(menu.map(d => d.id), r(6)), ...(r(5) === 0 ? ['gone-' + r(9)] : [])];
  for (const id of ids) {
    const e = {};
    if (r(2)) e.order = list(day, 1 + (r(4) === 0), 400, 3);
    if (r(2)) e.open = list(day, 1, 200, 4);
    if (r(3) === 0) e.dwell = list(day, 1, 100, 0);
    if (r(3) === 0) e.add = list(day, 1, 60, 2);
    if (r(3) === 0) e.drop = list(day, 1, 60, 3);
    p.dishes[id] = e;
  }
  for (const c of some(['c0', 'c1', 'c2', 'c3', 'c4', 'c5', 'c6', 'c7'], r(4))) p.cats[c] = { seen: list(day, 1, 90, 3) };
  if (r(2)) {
    p.ctx = {};
    for (const c of some(CTX, 1 + r(3))) p.ctx[c] = Object.fromEntries(some(ids, 1 + r(2)).map(id => [id, list(day, 1, 300, 2)]));
  }
  return p;
}
function opts(menu){
  const o = {};
  if (r(3) === 0) o.avoidGuess = some(EU14, 1 + r(2));
  if (r(3) === 0) {
    const rows = keys => some(keys, 1 + r(3)).map(key => ({ key, w: 1 + r(5000) }));
    o.prior = { tags: [...rows(TAGS), { key: 'x', w: '5' }, { w: 3 }, { key: 'y', w: 2.5 }], cats: rows(['c0', 'c1', 'c2', 'c3']),
      sense: rows(['t:umami', 'a:smoky', 'x:crispy', 't:sweet', 'a:citrus']) };
  }
  const c = r(4);
  if (c === 1) o.ctx = [];
  if (c >= 2) o.ctx = some(CTX, 1 + r(2));
  if (r(2)) o.mood = r(8) === 0 ? 'unknown' : pick(MOOD_IDS);
  return o;
}

const cases = Array.from({ length: 1000 }, () => {
  const mi = r(menus.length);
  const day = 20_500 + r(400);
  const p = profile(menus[mi], day);
  const o = opts(menus[mi]);
  const want = T.scored(menus[mi], p, day, o).map(x => [x.id, x.why, x.guessed ? 1 : 0, x.s]);
  return { menu: mi, day, profile: p, opts: o, want };
});

// Spot checks of the arithmetic alone, so a disagreement names its layer.
const fade = Array.from({ length: 300 }, (_, i) => {
  const w = i % 10 === 0 ? (r(2) ? 1 : -1) * (2 ** 36 + r(2 ** 37)) : r(10_000_000) - 5_000_000;
  const d = i % 13 === 0 ? -r(5) : r(4000);
  return [w, d, I.fade(w, d)];
});
const vec = () => Object.fromEntries(some([...TASTE.map(a => 't:' + a), ...TEXTURE.map(a => 'x:' + a), ...AROMA.map(a => 'a:' + a)], r(10)).map(k => [k, r(2001) - 1000]));
const cos = Array.from({ length: 150 }, () => { const a = vec(), b = vec(); return [a, b, I.cosPm(a, b), I.perMille(a)]; });

const head = { rule: 'W-TASTE row 1, taste.js scored() = dowiz_hub::rank::strip::strip()', seed: SEED, half: I.HALF };
const out = `${JSON.stringify(head).slice(0, -1)},\n"fade":${JSON.stringify(fade)},\n"cos":${JSON.stringify(cos)},\n"menus":[\n${menus.map(m => JSON.stringify(m)).join(',\n')}\n],\n"cases":[\n${cases.map(c => JSON.stringify(c)).join(',\n')}\n]}\n`;
writeFileSync(new URL('./strip.json', import.meta.url), out);
const n = cases.reduce((a, c) => a + c.want.length, 0);
console.log(`strip.json: ${cases.length} cases, ${n} ranked dishes, ${cases.filter(c => c.want.length === 0).length} empty strips, ${out.length} bytes`);
