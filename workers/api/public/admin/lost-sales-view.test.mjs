// node --test workers/api/public/admin/lost-sales-view.test.mjs
// The "orders lost to stock-outs" card (A13, W-LOST), from a block shaped
// exactly like the hub's (`services/analytics/kitchen/lost/tests.rs`,
// stock.refused.v1): per dish and per day, lek through `money`, no markup
// injected, and a kitchen token's answer draws no revenue column.
import test from 'node:test';
import assert from 'node:assert/strict';
import { drawLost, hasRevenue } from './lost-sales-view.js';
import { WORDS, merge } from './lost-sales-words.js';
import { LANGS } from '../lib/langs.js';

const lost = {
  contract: 'stock.refused.v1', rows: 3, portions: 4, revenue: 2800, rateLimitedMinutes: 10,
  dishes: [{ id: 'maki', name: 'Maki <b>salmon</b>', rows: 2, portions: 3, revenue: 2100, byDay: [{ rows: 1, revenue: 1400 }, { rows: 1, revenue: 700 }] },
    { id: 'roll', name: 'Philadelphia', rows: 1, portions: 1, revenue: 700, byDay: [{ rows: 0, revenue: 0 }, { rows: 1, revenue: 700 }] }],
  byDay: [{ day: '2026-09-25', rows: 1, portions: 2, revenue: 1400 }, { day: '2026-09-26', rows: 2, portions: 2, revenue: 1400 }],
};
const money = n => `${n} L`;

test('the card draws refusals, portions and lek per dish and per day', () => {
  const html = drawLost({ lost }, { money });
  for (const k of ['ls_title', 'ls_refused', 'ls_portions', 'ls_revenue', 'ls_dish', 'ls_day', 'ls_limit']) assert.ok(html.includes(`data-t="${k}"`), k);
  for (const v of ['2800 L', '2100 L', '700 L', '2026-09-25', 'Philadelphia']) assert.ok(html.includes(v), v);
  assert.ok(html.includes('data-tour="kitchen.lost"'));
  assert.ok(!html.includes('<b>salmon</b>') && html.includes('&lt;b&gt;salmon'), 'a dish name is text, not markup');
});

test('a kitchen token sees counts and no lek; the twin with revenue draws it', () => {
  const strip = JSON.parse(JSON.stringify(lost), (k, v) => (k === 'revenue' ? undefined : v));
  const html = drawLost({ lost: strip, scope: 'kitchen' }, { money });
  assert.equal(hasRevenue(strip), false);
  assert.ok(!html.includes('ls_revenue') && !html.includes(' L<'), 'no revenue column, no money');
  assert.ok(html.includes('>3<'), 'the refusals are still counted');
  assert.equal(hasRevenue(lost), true);
});

test('no refusal says so; an older hub with no block draws nothing', () => {
  const html = drawLost({ lost: { ...lost, rows: 0, portions: 0, revenue: 0, dishes: [], byDay: [] } }, { money });
  assert.ok(html.includes('ls_none') && !html.includes('<table'));
  assert.equal(drawLost({}, { money }), '');
  assert.equal(drawLost(null), '');
});

test('every word in every language, and the table merges without overwriting', () => {
  const keys = Object.keys(WORDS.en);
  for (const l of LANGS) for (const k of keys) assert.ok(WORDS[l] && WORDS[l][k], `${l} ${k}`);
  for (const l of LANGS) for (const v of Object.values(WORDS[l])) assert.ok(!/[\u2018\u2019\u201c\u201d]/.test(v), `typographic quote in ${l}: ${v}`);
  const T = { en: { ls_title: 'kept' } };
  merge(T);
  assert.equal(T.en.ls_title, 'kept', 'a word the console has wins');
  assert.equal(T.en.ob_title, 'Option recipes');
});
