// The bag insert's pure parts, in node (W-QR): the form body the hub takes, the
// line printed on the card, the card's rows. `node --test workers/api/public/admin/bag-logic.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { formBody, whole, bonusText, cardLine, campaign, statRows, svgSrc } from './bag-logic.js';
import { format } from '../lib/money.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const EN = { bag_line: 'Next time order direct', bag_fixed: '{value} off', bag_fixedMin: '{value} off from {min}', bag_giftOf: '{dish} on the house', bag_stamps: 'a double stamp' };
const t = k => EN[k] ?? k;
const lek = n => format(n, 'ALL', 'en');

test('the body sends whole money and only the fields the kind uses', () => {
  assert.deepEqual(formBody({ kind: 'fixed', value: '1 500', min: '2000', product: 'x', pct: ' 30 ' }),
    { offer: { kind: 'fixed', value: 1500, min: 2000 }, commission_pct: '30' });
  assert.deepEqual(formBody({ kind: 'gift', value: '9', product: 'edamame' }), { offer: { kind: 'gift', product: 'edamame' }, commission_pct: '' });
  assert.deepEqual(formBody({ kind: 'stamps' }).offer, { kind: 'stamps' });
  assert.deepEqual(formBody({ kind: 'percent' }).offer, { kind: 'off' }, 'a kind the hub does not know is no offer');
  assert.equal(whole('15.5'), null, 'not money here');
  assert.equal(formBody({ kind: 'fixed', value: '15.5' }).offer.value, 0, 'refused by the hub, never rounded here');
});

test('the line on the card is the venue currency, never a second money copy', () => {
  const s = cardLine({ kind: 'fixed', value: 300, min: 2000 }, t, lek);
  assert.match(s, /^Next time order direct - /);
  assert.ok(s.includes(lek(300)) && s.includes(lek(2000)), s);
  assert.ok(!s.includes('3.00') && !s.includes('$'), 'lek has no minor unit: 300 is 300, not 3.00');
  assert.equal(bonusText({ kind: 'gift', product: 'p1' }, t, lek, () => 'Edamame'), 'Edamame on the house');
  assert.equal(bonusText({ kind: 'stamps' }, t, lek), 'a double stamp');
  assert.equal(cardLine(null, t, lek), 'Next time order direct');
  const src = fs.readFileSync(path.join(HERE, 'bag-logic.js'), 'utf8').replace(/\/\/[^\n]*/g, '');
  assert.ok(!/Intl\.NumberFormat|\/ ?100\b/.test(src), 'no money formatting of its own');
});

test('scans are never counted and the saving is shown only with a typed percent', () => {
  const rows = statRows({ orders: 3, guests: 2, repeat: 1, saved: null }, lek);
  assert.deepEqual(rows[0], ['bag_scans', null, 'bag_scansNone']);
  assert.deepEqual(rows[4], ['bag_saved', null, 'bag_savedNone']);
  assert.equal(statRows({ saved: 630 }, lek)[4][1], lek(630));
});

test('a campaign is cleaned as typed, and the QR data URL escapes #', () => {
  assert.equal(campaign('Spring 2026!'), 'spring2026');
  assert.equal(campaign('x'.repeat(40)).length, 24);
  assert.ok(!svgSrc('<rect fill="#fff"/>').includes('#'));
});
