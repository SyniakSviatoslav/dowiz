// node --test workers/api/public/admin/pf2-view.test.mjs
// Lane W-PF2: the import's semi-finished preview and the production act,
// drawn from the hub's answers (`/owner/recipes/import`, `/owner/stock/cooked`).
import test from 'node:test';
import assert from 'node:assert/strict';
import * as V from './pf2-view.js';

const fmt = { money: n => `L${n}`, t: k => `[${k}]` };

test('the import preview names each semi-finished product, new or updated; a flat file draws nothing', () => {
  const html = V.prepsBlock([{ id: 'spicy-mayo', name: 'Spicy <i>mayo</i>', unit: 'g', yield: 1000, k: 1000, new: true, lines: [{ item: 'mayo', name: 'Mayo', qty: 800 }, { item: 'sriracha', qty: 200 }] },
    { id: 'rice', name: 'Rice', unit: 'g', yield: 2100, k: null, new: false, lines: [] }]);
  assert.ok(html.includes('data-bk-preps="2"') && html.includes('data-bk-prep="spicy-mayo"'));
  assert.ok(html.includes('1000 g · K 100.0%') && html.includes('Mayo 800, sriracha 200'));
  assert.ok(html.includes('data-t="bulkNew"') && html.includes('data-t="bulkPrepUpdate"'));
  assert.ok(!html.includes('<i>mayo') && html.includes('&lt;i&gt;'), 'escaped');
  assert.equal(V.prepsBlock([]), '');
  assert.equal(V.prepsBlock(undefined), '');
});

test('the shelf of a semi-finished product: a batch on it, or the words for none', () => {
  assert.ok(V.shelfFact({ counted: true, onHand: 2050, available: 1920, unit: 'g' }, fmt.t).includes('data-pf-shelf="1920"'));
  assert.ok(V.shelfFact({ counted: true, onHand: 2050, unit: 'g' }, fmt.t).includes('<b>2050 g</b>'));
  assert.ok(V.shelfFact({ counted: false }, fmt.t).includes('[pf_notStocked]'));
  assert.ok(V.shelfFact(null, fmt.t).includes('data-pf-shelf="none"'));
});

test('the act: the live loss, the form, and the answer', () => {
  // A 2350 g card that makes 2100: cooking 2100 by the card and weighing 2050.
  assert.deepEqual(V.cookLive('2100', '2050', 2350, 2100), { gross: 2350, loss: 300, pm: 872 });
  assert.deepEqual(V.cookLive('1050', '', 2350, 2100), { gross: 1175, loss: 125, pm: 893 }, 'empty out: what the card says');
  assert.equal(V.cookLive('0', '', 2350, 2100), null);
  assert.equal(V.cookLive('100', '', 0, 2100), null, 'unknown gross: no live number');
  const form = V.cookForm({ id: 'rice', name: 'Rice', yield: 2100 }, { field: o => `<f ${o.id}=${o.value}>`, input: o => `<i ${o.id}>` });
  assert.ok(form.includes('<f ck-qty=2100>') && form.includes('<f ck-out=>') && form.includes('<i ck-exp>') && form.includes('id="ckSave"'));
  const r = V.cookResult({ act: 'pa_1', out: 2050, value: 266, lossG: 300, yieldPm: 872, cardPm: 893, lines: [{ item: 'rice-dry', name: 'Rice', qty: 1000, unit: 'g' }] }, fmt);
  assert.ok(r.includes('data-ck-done="pa_1"') && r.includes('L266') && r.includes('300 g (87.2% / 89.3%)') && r.includes('<td>1000 g</td>'));
  assert.ok(V.cookResult({ out: 5, lines: [] }, fmt).includes('[pf_cookLoss]: <b>-</b>'), 'no gross: no loss claimed');
});
// Every word in all four languages: tools/gates/langs.sh (the `keys` rule) reads pf2-i18n.js.
