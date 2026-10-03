// node --test workers/api/public/admin/ingredients-loss-view.test.mjs
// The Losses sheet drawn from an `avt` block shaped exactly like
// `services/analytics/kitchen/avt.rs` answers (avt/tests.rs).
import test from 'node:test';
import assert from 'node:assert/strict';
import * as V from './ingredients-loss-view.js';

const fmt = { money: n => `L${n}`, t: k => `[${k}]`, when: ms => `@${ms}` };
const salmon = {
  id: 'salmon', name: 'Salmon <b>', unit: 'g', from: 20, to: 60, opening: 1000, received: 500, sold: 80, wasted: 20, prep: 0, closing: 1300,
  actual: 200, unexplained: 100, value: 200, revenuePm: 40, flags: ['over30pm'], windowsBefore: 0,
  records: [{ at: 30, kind: 'received', qty: 500 }, { at: 50, kind: 'wasted', qty: 20, reason: 'spoiled', value: 40 }, { at: 60, kind: 'stocktake', qty: 1300 }],
};
const gain = { ...salmon, id: 'rice', name: 'Rice', unexplained: -10, value: -30, revenuePm: null, flags: [], records: [] };

test('a loss: its money, its share of sales, its flag, the sum and the records', () => {
  const html = V.rowMarkup(salmon, fmt);
  assert.ok(html.includes('100 g') && html.includes('L200') && html.includes('4.0%'), html);
  assert.ok(html.includes('data-t="ls_over"'), 'above 3% of sales is flagged');
  assert.ok(html.includes('+500 g') && html.includes('-80 g') && html.includes('-20 g') && html.includes('1300 g'), 'the sum that closes');
  assert.ok(html.includes('@50') && html.includes('[inv_mv_wasted] · [spoiled]') && html.includes('L40'), 'the records');
  assert.ok(!html.includes('<b>Salmon <b>') && html.includes('Salmon &lt;b&gt;'), 'a name is text');
  const g = V.rowMarkup(gain, fmt);
  assert.ok(g.includes('[ls_found]') && g.includes('10 g') && g.includes('class="pos"'), 'more found than expected is said so, never a loss');
});

test('the sheet: the top losses first, then who must count twice and what is not tracked', () => {
  const html = V.lossesMarkup({ rows: [salmon, gain], top: ['salmon'], countTwice: [{ id: 'nori', name: 'Nori' }], notTracked: [{ id: 'cola', name: 'Cola' }] }, fmt);
  assert.ok(html.indexOf('[ls_top]') < html.indexOf('data-loss="salmon"') && html.indexOf('data-loss="salmon"') < html.indexOf('data-loss="rice"'));
  assert.ok(html.includes('[ls_twice]') && html.includes('Nori'));
  assert.ok(html.includes('[ls_untracked]') && html.includes('Cola'));
  assert.ok(!html.includes('[ls_none]'));
  const none = V.lossesMarkup({ rows: [], top: [], countTwice: [], notTracked: [] }, fmt);
  assert.ok(none.includes('[ls_none]') && !none.includes('[ls_top]'), 'no window: it says how to get one, and shows no zero');
  assert.equal(V.pct(42), '4.2%');
});
