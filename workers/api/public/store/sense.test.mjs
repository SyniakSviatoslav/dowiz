// `node --test workers/api/public/store/sense.test.mjs`
// A dish's taste, texture and aroma on the storefront, and the guest on the same axes (W-SENSE).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as S from './sense.js';
import * as V from './sense-view.js';
import * as T from './taste.js';
import { SENSE_WORDS, wordKey } from '../lib/sense-words.js';

const D = 20_000;

test('the vocabulary is the hub\'s: six axes 0..5, nine textures, twelve aromas 1..3', () => {
  assert.deepEqual(S.TASTE, ['sweet', 'sour', 'salty', 'bitter', 'umami', 'spicy']);
  assert.equal(S.TEXTURE.length, 9);
  assert.equal(S.AROMA.length, 12);
  const s = S.senseOf({ sense: { taste: { spicy: 6, sweet: 0, sour: 2.5, umami: '3', salty: 4 }, texture: { crispy: 3, slimy: 2, soft: 0 }, aroma: { smoky: 1 } } });
  assert.deepEqual(s.taste, { sweet: 0, salty: 4 }, 'out of range, fractional and string values are dropped; a 0 axis is kept');
  assert.deepEqual(s.texture, { crispy: 3 });
  assert.deepEqual(s.aroma, { smoky: 1 });
});

test('the old taste field reads at x5/3 and nothing at all is null (no fake zeros)', () => {
  assert.deepEqual([1, 2, 3].map(S.legacyLevel), [2, 3, 5]);
  assert.deepEqual(S.senseOf({ taste: { spicy: 3, richness: 2 } }).taste, { spicy: 5 });
  assert.equal(S.senseOf({ name: 'Water' }), null);
  assert.equal(S.senseOf({ sense: { v: 1, taste: {}, texture: {}, aroma: {} } }), null);
  assert.equal(V.cardSense({ name: 'Water' }), '', 'a card with nothing declared draws nothing');
  assert.equal(V.sheetSense({ name: 'Water' }), '');
});

test('the vector is per mille of each scale and cosine is 1 for the same dish', () => {
  const v = S.vectorOf(S.senseOf({ sense: { taste: { spicy: 4, sweet: 0 }, texture: { crispy: 3 }, aroma: { smoky: 1 } } }));
  assert.deepEqual(v, { 't:spicy': 800, 'x:crispy': 1000, 'a:smoky': 333 });
  assert.ok(Math.abs(S.cosine(v, v) - 1) < 1e-9);
  assert.equal(S.cosine(v, {}), 0);
  assert.equal(S.cosine({ 't:sweet': 1 }, v), 0);
});

test('every declared axis is drawn with its figure as text, and only the declared ones', () => {
  const p = { sense: { taste: { spicy: 4, sweet: 0 }, texture: { crispy: 3, creamy: 1 }, aroma: { smoky: 2 } } };
  const h = V.sheetSense(p);
  assert.match(h, /4\/5/);
  assert.match(h, /0\/5/, 'a declared 0 is shown as 0');
  assert.doesNotMatch(h, /sx_t_bitter/, 'an axis nobody declared is not drawn');
  for (const k of ['sx_t_spicy', 'sx_t_sweet', 'sx_x_crispy', 'sx_x_creamy', 'sx_a_smoky']) assert.match(h, new RegExp(`data-t="${k}"`));
  const c = V.cardSense(p);
  assert.match(c, /aria-label="spicy 4\/5, sweet 0\/5"/, 'the compact strip reads as text');
  assert.equal((c.match(/class="sx-tag"/g) || []).length, 2, 'at most two chips on a card');
});

test('filters: spicy, not spicy (declared only), textures and aromas; all chips must pass', () => {
  const menu = [
    { id: 'a', sense: { taste: { spicy: 4 }, texture: { crispy: 3 } } },
    { id: 'b', sense: { taste: { spicy: 0 }, texture: { creamy: 2 }, aroma: { smoky: 2 } } },
    { id: 'c', name: 'undeclared' },
  ];
  const ids = S.filterChips(menu);
  assert.deepEqual(ids.slice(0, 5), ['spicy', 'not-spicy', 'x:crispy', 'x:creamy', 'a:smoky']);
  const attr = Object.fromEntries(menu.map(p => [p.id, S.cardAttr(S.senseOf(p))]));
  const on = (...f) => menu.filter(p => S.passes(attr[p.id], new Set(f))).map(p => p.id);
  assert.deepEqual(on(), ['a', 'b', 'c']);
  assert.deepEqual(on('spicy'), ['a']);
  assert.deepEqual(on('not-spicy'), ['b'], 'an undeclared dish is not "not spicy"');
  assert.deepEqual(on('x:creamy', 'a:smoky'), ['b']);
  assert.deepEqual(on('spicy', 'a:smoky'), []);
  assert.match(V.filterRow(ids, new Set(['spicy'])), /aria-pressed="true"/);
});

test('every word exists in four languages', () => {
  const keys = Object.keys(SENSE_WORDS.en);
  for (const l of ['sq', 'uk', 'ru']) assert.deepEqual(Object.keys(SENSE_WORDS[l]).sort(), [...keys].sort(), l);
  for (const k of [...S.TASTE.map(x => 't:' + x), ...S.TEXTURE.map(x => 'x:' + x), ...S.AROMA.map(x => 'a:' + x)]) assert.ok(SENSE_WORDS.en[wordKey(k)], k);
  for (const m of S.MOOD_IDS) assert.ok(SENSE_WORDS.uk['sx_mood_' + m], m);
});

// ── the guest on the same axes ───────────────────────────────────────────────
const dish = (id, sense, extra = {}) => ({ id, categoryId: 'cat-' + id, tags: [], available: true, sense: { v: 1, ...sense }, ...extra });
const fold = (evs, day = D) => evs.reduce((p, e) => T.record(p, e, day), T.empty({ day }));

test('a NEW dish with the guest\'s axes is predicted from them, and the strip says why', () => {
  const menu = [
    dish('eel', { aroma: { smoky: 3 }, texture: { crispy: 3 } }),
    dish('ebi', { aroma: { smoky: 2 }, texture: { crispy: 3 } }),
    dish('mochi', { taste: { sweet: 5 }, texture: { soft: 3 } }),
    dish('new-smoky', { aroma: { smoky: 3 }, texture: { crispy: 2 } }),
    dish('new-sweet', { taste: { sweet: 4 }, texture: { chewy: 2 } }),
  ];
  const p = fold([{ kind: 'order', items: [{ id: 'eel', qty: 2 }, { id: 'ebi', qty: 1 }] }]);
  const s = T.strip(menu, p, D);
  const rest = s.filter(x => x.why === 'taste').map(x => x.id);
  assert.equal(rest[0], 'new-smoky', JSON.stringify(s));
  assert.ok(!rest.includes('new-sweet'), 'nothing in common, nothing to say');
  assert.deepEqual(T.because(T.senseVec(p, menu, D)), ['x:crispy', 'a:smoky']);
  assert.match(V.becauseLine(['a:smoky', 'x:crispy']), /sx_because.*sx_a_smoky.*sx_x_crispy/);
});

test('the context ranks what was ordered in it, and the mood is this session only', () => {
  const menu = [dish('ramen', { taste: { umami: 5 }, aroma: { 'spice-warm': 3 } }), dish('poke', { taste: { sour: 3 }, aroma: { citrus: 3 } }),
    dish('new-warm', { taste: { umami: 4 }, aroma: { 'spice-warm': 2 } }), dish('new-fresh', { aroma: { citrus: 3, herbal: 2 } })];
  const p = fold([{ kind: 'order', items: [{ id: 'ramen', qty: 1 }], ctx: ['wx:rain'] }, { kind: 'order', items: [{ id: 'poke', qty: 1 }], ctx: ['wx:hot'] }]);
  const top = o => T.strip(menu, p, D, o).filter(x => x.why === 'taste')[0]?.id;
  assert.equal(top({ ctx: ['wx:rain'] }), 'new-warm');
  assert.equal(top({ ctx: ['wx:hot'] }), 'new-fresh');
  assert.equal(top({ mood: 'light' }), 'new-fresh');
  assert.equal(top({ mood: 'cosy' }), 'new-warm');
  assert.deepEqual(Object.keys(p.ctx), ['wx:rain', 'wx:hot']);
  const bad = T.record(T.empty({ day: D }), { kind: 'order', items: [{ id: 'x', qty: 1 }], ctx: ['x'.repeat(40)] }, D);
  assert.equal(bad.ctx, undefined, 'an over-long context key is not kept');
});

test('a monthly snapshot is kept for the year, and only aggregated keys may leave', () => {
  const menu = [dish('eel', { aroma: { smoky: 3 } })];
  let p = T.empty({ day: D });
  for (let i = 0; i < 16; i++) p = T.snapshot(T.record(p, { kind: 'order', items: [{ id: 'eel', qty: 1 }] }, D + i * 31), menu, D + i * 31);
  assert.equal(Object.keys(p.months).length, T.MONTHS_KEEP);
  assert.deepEqual(Object.values(p.months).at(-1), { 'a:smoky': 1000 });
  assert.match(V.yearMarkup(p.months), /sx_a_smoky/);
  const v = T.syncVector(p, menu, D + 15 * 31);
  assert.deepEqual(v.sense, { 'a:smoky': 1000 });
  assert.deepEqual(Object.keys(v).sort(), ['cats', 'sense', 'tags', 'v'], 'no events, no months, no ctx leave the phone');
});

// ── MEASURED: does a new dish with matching axes rank in the top 3? ─────────────
function rng(seed){ let x = seed >>> 0; return () => ((x = (x * 1664525 + 1013904223) >>> 0) / 2 ** 32); }
test('measured on 300 synthetic guests: a new dish built on the guest\'s axes ranks in the top 3', () => {
  const r = rng(20261004);
  const pick = (arr, n) => { const a = [...arr]; const out = []; while (out.length < n) out.push(a.splice(Math.floor(r() * a.length), 1)[0]); return out; };
  const tags = [...S.TEXTURE.map(x => ['texture', x]), ...S.AROMA.map(x => ['aroma', x])];
  const randomSense = () => {
    const s = { taste: {}, texture: {}, aroma: {} };
    for (const ax of pick(S.TASTE, 2)) s.taste[ax] = 1 + Math.floor(r() * 5);
    for (const [d, id] of pick(tags, 3)) s[d][id] = 1 + Math.floor(r() * 3);
    return s;
  };
  const menu = Array.from({ length: 40 }, (_, i) => dish('d' + i, randomSense()));
  const N = 300; let top3 = 0, top1 = 0, fresh3 = 0;
  for (let g = 0; g < N; g++) {
    const latent = randomSense();
    const lv = S.vectorOf(latent);
    const near = [...menu].sort((a, b) => S.cosine(lv, S.vectorOf(S.senseOf(b))) - S.cosine(lv, S.vectorOf(S.senseOf(a))) || a.id.localeCompare(b.id));
    const ordered = near.slice(0, 4).map((d, i) => ({ id: d.id, qty: 1 + (i === 0 ? 1 : 0) }));
    const p = fold([{ kind: 'order', items: ordered }]);
    const fresh = dish('new', latent);
    const ranked = T.strip([...menu, fresh], p, D).filter(x => x.why === 'taste').map(x => x.id);
    const at = ranked.indexOf('new');
    if (at >= 0 && at < 3) top3++;
    if (at === 0) top1++;
    const never = ranked.filter(id => !ordered.some(o => o.id === id));
    if (never.indexOf('new') >= 0 && never.indexOf('new') < 3) fresh3++;
  }
  console.log(`MEASURED sense ranking (300 guests, 40-dish menu, 4 dishes ordered each): the new dish is in the strip's top 3 for ${top3}/${N} (${(100 * top3 / N).toFixed(1)}%), first for ${top1}/${N}; top 3 among never-ordered dishes for ${fresh3}/${N} (${(100 * fresh3 / N).toFixed(1)}%)`);
  assert.ok(top3 / N >= 0.8, `${top3}/${N}`);
});

test('W-TASTE: the weather is credited to open-meteo.com (CC BY 4.0) exactly where it shaped the strip', () => {
  const h = V.weatherCredit(['band:evening', 'wx:rain']);
  assert.match(h, /data-t="sx_weather"/);
  assert.match(h, /href="https:\/\/open-meteo\.com\/"[^>]*>open-meteo\.com<\/a>/);
  assert.equal(V.weatherCredit(['band:evening', 'day:weekday']), '', 'no weather key, no weather credit');
  assert.equal(V.weatherCredit(null), '');
  for (const l of ['sq', 'en', 'uk', 'ru']) assert.ok(SENSE_WORDS[l].sx_weather, l);
  // The strip itself carries it: taste-device.js appends weatherCredit(momentKeys()).
  const dev = readFileSync(new URL('./taste-device.js', import.meta.url), 'utf8');
  assert.match(dev, /\$\{weatherCredit\(momentKeys\(\)\)\}/, 'the "For you" strip draws the credit');
});

test('W-TASTE2: the owner segment builder credits open-meteo.com beside its weather filter', () => {
  const line = V.weatherSourceLine();
  assert.match(line, /data-t="sx_weather"/);
  assert.match(line, /href="https:\/\/open-meteo\.com\/"[^>]*>open-meteo\.com<\/a>/);
  assert.equal(V.weatherCredit(['wx:rain']), line, 'the storefront credit is the same line');
  const b = readFileSync(new URL('../admin/sense-builder.js', import.meta.url), 'utf8');
  const at = b.indexOf("sel('sbWx'");
  assert.ok(at > 0, 'the builder has its weather filter');
  assert.match(b.slice(at, at + 400), /\$\{weatherSourceLine\(\)\}/, 'the credit is drawn right after the weather filter');
});
