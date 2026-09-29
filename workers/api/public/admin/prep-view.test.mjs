// node --test workers/api/public/admin/prep-view.test.mjs
// A semi-finished product's row, card, editor lines and "one sale takes",
// drawn from the hub's `GET /api/owner/preps` and `/takes` shapes.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as V from './prep-view.js';

const fmt = { money: n => `L${n}`, t: k => `[${k}]` };
const rice = {
  id: 'rice-seasoned', name: 'Rice <b>seasoned</b>', unit: 'g', kind: 'prep', category: 'Prep', yield: 2100, k: 894, costPer: 112, batchCost: 235, active: true,
  lines: [{ item: 'rice-dry', name: 'Rice', unit: 'g', qty: 1000, cost: 200, kind: 'food_ingredient' }, { item: 'water', name: 'Water', unit: 'ml', qty: 1100, cost: 0, untracked: true },
    { item: 'mitsukan', name: 'Mitsukan', unit: 'g', qty: 250, cost: 35, kind: 'prep' }],
  uses: { preps: [], dishes: [{ id: 'philadelphia', name: 'Philadelphia' }] },
};
const bare = { id: 'sauce', name: 'Sauce', unit: 'ml', kind: 'prep', yield: 250, k: null, costPer: null, batchCost: null, lines: [{ item: 'gone', qty: 1, missing: true }], uses: { preps: [{ id: 'x', name: 'X' }], dishes: [] } };

test('a row: yield, K, cost per kg, where used; a name is text', () => {
  const html = V.prepRowMarkup(rice, { ...fmt, del: true });
  assert.ok(html.includes('[pf_yield] 2100 g') && html.includes('K 89.4%') && html.includes('L112 [pf_costPerKg]'));
  assert.ok(html.includes('[pf_usedIn]: 1 [pf_dishes]'));
  assert.ok(html.includes('data-s="rice-seasoned"') && html.includes('data-prep="1"') && html.includes('data-pedit="rice-seasoned"') && html.includes('data-del="rice-seasoned"'));
  assert.ok(!html.includes('<b>seasoned') && html.includes('&lt;b&gt;'), 'escaped');
  const b = V.prepRowMarkup(bare, fmt);
  assert.ok(b.includes('[pf_costUnknown]') && b.includes('[pf_usedNowhere]') === false && b.includes('1 [pf_preps]'));
  assert.ok(!b.includes('data-del'), 'staff: no delete');
});

test('the card: the four numbers, the lines with cost, where used with one-sale-takes, the three buttons', () => {
  const html = V.cardMarkup(rice, { ...fmt, owner: true });
  assert.ok(html.includes('<b>2100</b>') && html.includes('<b>89.4%</b>') && html.includes('<b>L112</b>') && html.includes('<b>L235</b>'));
  assert.ok(html.includes('Rice</td>') && html.includes('1000 g') && html.includes('L200'));
  assert.ok(html.includes('([pf_untracked])'), 'water is marked');
  assert.ok(html.includes('data-takes="philadelphia"'));
  for (const id of ['pfEdit', 'pfRetire', 'pfDelete']) assert.ok(html.includes(`id="${id}"`), id);
  const staff = V.cardMarkup(rice, { ...fmt, owner: false });
  assert.ok(!staff.includes('id="pfDelete"'));
  const b = V.cardMarkup(bare, fmt);
  assert.ok(b.includes('[pf_kUnknown]') && b.includes('data-t="pf_costUnknown"') && b.includes('ui-badge--danger'), 'unknown K, no cost, a missing item');
});

test('editor lines and the live numbers', () => {
  const l = V.editorLine({ item: 'mitsukan', qty: 250, sup: { name: 'Mitsukan', unit: 'g', kind: 'prep' } }, 2);
  assert.ok(l.includes('data-plq="2"') && l.includes('data-plx="2"') && l.includes('value="250"') && l.includes('chef-hat'));
  const live = V.liveMarkup({ k: 894, batch: 235, per: 112, unit: 'g' }, fmt);
  assert.ok(live.includes('<b>89.4%</b>') && live.includes('<b>L235</b>') && live.includes('<b>L112</b> [pf_costPerKg]'));
  const none = V.liveMarkup({ k: null, batch: null, per: null, unit: 'ml' }, fmt);
  assert.ok(none.includes('[pf_kUnknown]') && !none.includes('pf_costPerL'));
});

test('one sale takes off the shelf: three decimals per leaf, the total, and the two other answers', () => {
  const tk = { lines: 1, cost: 21, leaves: [{ supply: 'rice-dry', name: 'Rice', unit: 'g', uq: 61904762, cost: 19 }, { supply: 'salt', name: 'Salt', unit: 'g', uq: 773810, cost: 0 }] };
  const html = V.takesMarkup(tk, fmt);
  assert.ok(html.includes('61.905 g') && html.includes('0.774 g') && html.includes('L19') && html.includes('L21'));
  assert.ok(V.takesMarkup({ lines: 1, leaves: [], cost: null }, fmt).includes('data-t="pf_takesTotal"'), 'a recipe with no leaf still has a total row');
  assert.ok(V.takesMarkup({ lines: 0, leaves: [] }, fmt).includes('data-t="pf_takesNone"'), 'no recipe');
  assert.ok(V.takesMarkup({ leaves: [], refused: 'unknown supply nobody' }, fmt).includes('[pf_refused]: unknown supply nobody'));
  assert.ok(V.takesMarkup(null, fmt).includes('data-t="pf_takesNone"'));
  assert.equal(V.usesMarkup({ preps: [], dishes: [] }, fmt.t), '<p class="hint" data-t="pf_usedNowhere"></p>');
});
