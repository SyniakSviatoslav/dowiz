// W-NOM: the select-mode row, the bar, the packs editor and the pack chips,
// rendered in node; the ingredient row's owner-only Delete.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as V from './nom-view.js';
import { rowMarkup } from './ingredients-view.js';

const t = k => `[${k}]`;

test('a select-mode row says whether it is ticked and toggles by its id', () => {
  const on = V.pickRow('salmon', 'Salmon <b>', 'Fish', true), off = V.pickRow('rice', 'Rice', '', false);
  assert.ok(on.includes('data-pick="salmon"') && on.includes('aria-pressed="true"') && on.includes('ti-circle-check') && on.includes('nom-on'));
  assert.ok(off.includes('aria-pressed="false"') && off.includes('ti-plus') && !off.includes('nom-on'));
  assert.ok(!on.includes('<b>'), 'a name is text');
});

test('the bar counts the ticked and its Delete is off at zero', () => {
  const none = V.barMarkup({ ids: new Set() }, t), two = V.barMarkup({ ids: new Set(['a', 'b']) }, t);
  assert.ok(none.includes('<b>0</b> [nom_selected]') && /id="nomDel"[^>]*disabled/.test(none));
  assert.ok(two.includes('<b>2</b>') && !/id="nomDel"[^>]*disabled/.test(two) && two.includes('ui-btn--danger'));
  for (const id of ['nomAll', 'nomDel', 'nomDone']) assert.ok(two.includes(`id="${id}"`), id);
});

test('the packs editor shows every pack and one empty row', () => {
  const html = V.packsMarkup([{ name: 'box 5 kg', qty: 5000 }]);
  assert.equal((html.match(/class="nom-pack"/g) || []).length, 2);
  assert.ok(html.includes('value="box 5 kg"') && html.includes('value="5000"') && html.includes('id="nomPackAdd"'));
  assert.equal((V.packsMarkup(null).match(/class="nom-pack"/g) || []).length, 1, 'no packs: just the row to type one');
  assert.ok(V.packRow().includes('data-pkn="1"') && V.packRow().includes('data-pkq="1"'));
});

test('a delivery offers one chip per pack, and none without packs', () => {
  const html = V.packChips({ unit: 'g', packs: [{ name: 'box', qty: 5000 }, { name: 'kg', qty: 1000 }] });
  assert.ok(html.includes('data-pack="5000"') && html.includes('data-pack="1000"') && html.includes('+ box (5000 g)'));
  assert.equal(V.packChips({ unit: 'g' }), '');
  assert.equal(V.packChips(null), '');
});

test('an ingredient row carries the red Delete only for the owner', () => {
  const sup = { id: 'salmon', name: 'Salmon', unit: 'g', kind: 'food_ingredient', counted: true, onHand: 1, reserved: 0, available: 1 };
  const fmt = { money: n => `L${n}`, t, warnDays: 2 };
  assert.ok(rowMarkup(sup, { ...fmt, del: true }).includes('data-del="salmon"'));
  assert.ok(!rowMarkup(sup, fmt).includes('data-del='), 'staff: no Delete');
});
