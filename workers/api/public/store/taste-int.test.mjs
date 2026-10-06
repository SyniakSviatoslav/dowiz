// `node --test workers/api/public/store/taste-int.test.mjs`
// W-TASTE row 1: the phone ranks in integers, and ranks EXACTLY as the hub does. The shared fixture
// (crates/dowiz-hub/fixtures/rank/strip.json, 1000 guests x 4 menus, from gen.mjs beside it) is read
// here by taste.js and in crates/dowiz-hub/src/rank/tests.rs by rank::strip: both must give `want`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as T from './taste.js';
import * as I from './taste-int.js';
import { MOODS } from './sense.js';

const FIX = JSON.parse(readFileSync(new URL('../../../../crates/dowiz-hub/fixtures/rank/strip.json', import.meta.url), 'utf8'));

test('the half-life table is the hub\'s, sixty days is exactly half, and nothing fades past 2^53', () => {
  assert.deepEqual(I.HALF, FIX.half);
  assert.equal(I.HALF[0], 65536);
  assert.equal(I.fade(1000, 60), 500);
  assert.equal(I.fade(-1000, 60), -500, 'symmetric for a removal');
  assert.equal(I.fade(1000, 0), 1000);
  assert.equal(I.fade(1000, -3), 1000, 'a day after today does not grow a weight');
  assert.equal(I.fade(1000, 60 * 37), 0, 'past the table: nothing');
  let bad = 0;
  for (const [w, d, want] of FIX.fade) if (I.fade(w, d) !== want) bad++;
  assert.equal(bad, 0, `${bad} of ${FIX.fade.length} fade spot checks differ`);
});

test('per-mille cosine: exact integers, truncated toward zero', () => {
  assert.equal(I.isqrt(0), 0); assert.equal(I.isqrt(15), 3); assert.equal(I.isqrt(16), 4); assert.equal(I.isqrt(2 ** 52 + 1), 2 ** 26);
  const v = { 't:spicy': 800, 'x:crispy': 1000 };
  assert.equal(I.cosPm(v, v), 1000);
  assert.equal(I.cosPm(v, {}), 0);
  assert.equal(I.cosPm({ 't:sweet': 3 }, v), 0);
  let bad = 0;
  for (const [a, b, want, pm] of FIX.cos) if (I.cosPm(a, b) !== want || JSON.stringify(I.perMille(a)) !== JSON.stringify(pm)) bad++;
  assert.equal(bad, 0, `${bad} of ${FIX.cos.length} cosine spot checks differ`);
});

test('the moods are the hub\'s (rank/strip.rs mood())', () => {
  for (const c of FIX.cases.filter(c => c.opts.mood && MOODS[c.opts.mood])) for (const x of Object.values(MOODS[c.opts.mood])) assert.ok(Number.isInteger(x));
  assert.deepEqual(Object.keys(MOODS).sort(), ['cosy', 'light', 'quick', 'treat']);
});

test('1000/1000: the phone ranks every fixture guest exactly as the shared file says (= the hub)', () => {
  let same = 0;
  const first = [];
  for (const [i, c] of FIX.cases.entries()) {
    const got = T.scored(FIX.menus[c.menu], c.profile, c.day, c.opts).map(x => [x.id, x.why, x.guessed ? 1 : 0, x.s]);
    for (const x of got) assert.ok(Number.isInteger(x[3]), `case ${i}: score ${x[3]} is not an integer`);
    if (JSON.stringify(got) === JSON.stringify(c.want)) same++;
    else if (first.length < 3) first.push(`case ${i}: got ${JSON.stringify(got)} want ${JSON.stringify(c.want)}`);
  }
  console.log(`MEASURED device strip vs shared fixture: ${same}/${FIX.cases.length} equal rankings (ids, reasons and integer scores)`);
  assert.equal(same, FIX.cases.length, first.join('\n'));
  assert.ok(FIX.cases.filter(c => c.want.length > 0).length >= 700, 'the fixture is not a file of empty strips');
});
