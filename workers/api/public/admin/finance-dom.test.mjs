// The money sheet in node: the DOM half of admin/finance.js (W-FINANCE).
// `node --test workers/api/public/admin/finance-dom.test.mjs`
import { page, tick } from './dom_test.harness.mjs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const F = await import('./finance.js');

const answer = over => ({ month: '2026-11', today: '2026-11-08', currency: 'ALL',
  expenses: [{ id: 'e1', date: '2026-11-03', name: 'ПДВ', amount: 500, kind: 'tax' }, { id: 'e2', date: '2026-11-05', name: 'Бензин', amount: 3200, kind: 'other' }],
  tax: { total: 500 }, other: { total: 3200 },
  profit: { plus: { total: 1800, rows: [{ key: 'orders', amount: 1300 }, { key: 'till', amount: 500 }] },
    minus: { total: 4900, rows: [{ key: 'purchases', amount: 1200, receipts: 2, uncosted: 1 }, { key: 'taxes', amount: 500 }, { key: 'other', amount: 3200 }, { key: 'payroll', amount: 0 }] }, net: -3100 },
  history: [{ month: '2026-10', plus: 10, minus: 4, net: 6 }, { month: '2026-11', plus: 1800, minus: 4900, net: -3100 }], ...over });

const value = (root, sel, v) => root.querySelector(sel).setAttribute('value', v);

test('the sheet opens on the month, one row per source, and the net carries its sign', async () => {
  const { f, box } = page({ 'GET /owner/finance': answer() });
  await F.open();
  await tick();
  assert.equal(f.calls[0][1], '/owner/finance?location_id=V1');
  const text = box.textContent;
  for (const w of ['fn_src_orders', 'fn_src_till', 'fn_src_purchases', 'fn_src_taxes', 'fn_src_payroll']) assert.ok(box.innerHTML.includes(`data-t="${w}"`), w);
  for (const w of ['-3100', '1800', '4900']) assert.ok(text.includes(w), w);
  assert.equal(box.querySelectorAll('[data-month]').length, 2, 'the history, month by month');
  assert.ok(box.querySelector('.stat.warn'), 'a negative net is marked');
  const nav = box.querySelector('.fn-month');
  assert.deepEqual(nav.children.map(c => c.getAttribute('id')), ['fnPrev', 'fnMonth', 'fnNext'], 'back, the month and forward share one row of their own');
  assert.ok(box.innerHTML.includes('data-t="fn_uncosted"'), 'an unpriced delivery says so');
  assert.ok(!box.innerHTML.includes('data-t="fn_undated"'), 'no undated delivery, no sentence about one');
});

test('a tax is entered through the Taxes tab and its body is whole minor units', async () => {
  let sent = null;
  const { f, box } = page({ 'GET /owner/finance': answer(), 'POST /owner/finance/expenses': (b, p) => { sent = [b, p]; return { ok: true, id: 'e9' }; } });
  await F.open();
  await tick();
  box.querySelector('[data-mode="tax"]').onclick();
  await tick();
  assert.equal(box.querySelectorAll('[data-exp]').length, 1, 'only the taxes are listed');
  value(box, '#feDate', '2026-11-08');
  value(box, '#feName', 'ПДВ листопад');
  value(box, '#feAmount', '1 500');
  await box.querySelector('#feSave').onclick();
  await tick();
  const { client_id, ...typed } = sent[0];
  assert.deepEqual(typed, { date: '2026-11-08', name: 'ПДВ листопад', amount: 1500, kind: 'tax' });
  assert.match(client_id, /^[a-z0-9]{8,40}$/, 'a new entry carries the form id the hub files it under');
  assert.ok(sent[1].includes('location_id=V1'));
  assert.ok(f.calls.filter(c => c[0] === 'GET').length >= 2, 'the sheet re-reads the hub, it does not add up');
});

test('a form with no amount is named and nothing is sent; its twin edits and deletes', async () => {
  let saved = null, deleted = null;
  const { f, box } = page({ 'GET /owner/finance': answer(),
    'POST /owner/finance/expenses/': (b, p) => { saved = [b, p]; return { ok: true }; },
    'DELETE /owner/finance/expenses/': (b, p) => { deleted = p; return { ok: true }; } });
  await F.open();
  await tick();
  box.querySelector('[data-mode="other"]').onclick();
  await tick();
  value(box, '#feName', 'Бензин');
  await box.querySelector('#feSave').onclick();
  assert.deepEqual(f.toast, ['<fn_noAmount>'], 'named');
  assert.equal(f.calls.filter(c => c[0] === 'POST').length, 0, 'nothing sent');
  box.querySelector('[data-exp="e2"]').onclick();
  await tick();
  assert.equal(box.querySelector('#feAmount').getAttribute('value'), '3200', 'a lek has no decimals');
  value(box, '#feAmount', '700');
  await box.querySelector('#feSave').onclick();
  await tick();
  assert.deepEqual(saved[0], { date: '2026-11-05', name: 'Бензин', amount: 700, kind: 'other' }, 'an edit names its entry in the path and carries no form id');
  assert.ok(saved[1].startsWith('/owner/finance/expenses/e2?'));
  box.querySelector('[data-exp="e2"]').onclick();
  await tick();
  await box.querySelector('#feDel').onclick();
  await tick();
  assert.ok(deleted.startsWith('/owner/finance/expenses/e2?'), String(deleted));
});

test('a past month can be left and the next arrow stops at the present', async () => {
  let asked = '';
  const { box } = page({ 'GET /owner/finance': (b, p) => { asked = p; return answer(p.includes('month=2026-10') ? { month: '2026-10' } : {}); } });
  await F.open();
  await tick();
  assert.ok(box.querySelector('#fnNext').hasAttribute('disabled'), 'this month is the last');
  box.querySelector('#fnPrev').onclick();
  await tick();
  assert.ok(asked.includes('month=2026-10'), asked);
  assert.ok(!box.querySelector('#fnNext').hasAttribute('disabled'), 'a past month can go forward');
});

const openMode = async (routes, mode = 'other') => { const p = page(routes); await F.open(); await tick(); p.box.querySelector(`[data-mode="${mode}"]`).onclick(); await tick(); return p; };
const fill = (box, name = 'Бензин', amount = '500') => { value(box, '#feName', name); value(box, '#feAmount', amount); };

test('a double tap on Save adds the expense once', async () => {
  let posts = 0;
  const { box } = await openMode({ 'GET /owner/finance': answer(), 'POST /owner/finance/expenses': () => { posts++; return { ok: true, id: 'e9' }; } });
  fill(box);
  const save = box.querySelector('#feSave');
  await Promise.all([save.onclick(), save.onclick()]);
  await tick();
  assert.equal(posts, 1);
});

test('a save whose answer was lost is sent again under the same id, and the next entry under a new one', async () => {
  const ids = [];
  let lose = true;
  const { box, f } = await openMode({ 'GET /owner/finance': answer(), 'POST /owner/finance/expenses': b => { ids.push(b.client_id); if (lose) { lose = false; throw new Error('offline'); } return { ok: true, id: 'e9' }; } });
  fill(box);
  await box.querySelector('#feSave').onclick();
  await tick();
  assert.deepEqual(f.toast, ['offline']);
  assert.equal(box.querySelector('#feName').getAttribute('value'), 'Бензин', 'what was typed is still there');
  await box.querySelector('#feSave').onclick();
  await tick();
  fill(box, 'Прикраси на Хеловін', '3200');
  await box.querySelector('#feSave').onclick();
  await tick();
  assert.equal(ids.length, 3);
  assert.equal(ids[0], ids[1], 'the retry is the same entry');
  assert.notEqual(ids[2], ids[1], 'the next entry is another');
});

test('a date that has not happened is named and nothing is sent', async () => {
  const { box, f } = await openMode({ 'GET /owner/finance': answer() });
  fill(box);
  value(box, '#feDate', '2026-11-09');
  await box.querySelector('#feSave').onclick();
  assert.deepEqual(f.toast, ['<fn_futureDate>']);
  assert.equal(f.calls.filter(c => c[0] === 'POST').length, 0);
});

test('a new expense in a past month starts in that month, and the sheet follows the saved date', async () => {
  let asked = '';
  const { box } = await openMode({ 'GET /owner/finance': (b, p) => { asked = p; return answer(p.includes('month=2026-10') ? { month: '2026-10' } : {}); }, 'POST /owner/finance/expenses': { ok: true, id: 'e9' } });
  box.querySelector('#fnPrev').onclick();
  await tick();
  assert.equal(box.querySelector('#feDate').getAttribute('value'), '2026-10-01');
  fill(box);
  value(box, '#feDate', '2026-11-02');
  await box.querySelector('#feSave').onclick();
  await tick();
  assert.ok(asked.includes('month=2026-11'), asked);
});

test('a failed first load says so; a failed month keeps the month on screen', async () => {
  const dead = page({ 'GET /owner/finance': new Error('boom') });
  await F.open();
  await tick();
  assert.ok(dead.box.innerHTML.includes('loadFail'), 'the failure is said');
  assert.equal(dead.box.querySelector('.ui-skeleton, .skeleton'), null, 'no skeleton left up');
  assert.ok(dead.box.querySelector('#fnRetry'), 'and there is a way to ask again');
  const { box, f } = page({ 'GET /owner/finance': (b, p) => { if (p.includes('month=2026-10')) throw new Error('boom'); return answer(); } });
  await F.open();
  await tick();
  box.querySelector('#fnPrev').onclick();
  await tick();
  assert.ok(box.querySelector('#fnPrev'), 'the arrows are back');
  assert.ok(box.querySelector('.stats'), 'the month is still drawn');
  assert.equal(f.toast.length, 1);
});

test('a delete that fails closes the form; a cancelled delete keeps it and reads nothing', async () => {
  const gone = await openMode({ 'GET /owner/finance': answer(), 'DELETE /owner/finance/expenses/': new Error('404') });
  gone.box.querySelector('[data-exp="e2"]').onclick();
  await tick();
  await gone.box.querySelector('#feDel').onclick();
  await tick();
  assert.equal(gone.box.querySelector('#feDel'), null, 'no form for an entry that is gone');
  const kept = await openMode({ 'GET /owner/finance': answer() });
  // As in the console: the question takes the sheet's place, and "no" leaves it empty.
  Object.defineProperty(kept.f, 'confirm', { get(){ kept.box.innerHTML = '<p>asked</p>'; return false; } });
  kept.box.querySelector('[data-exp="e2"]').onclick();
  await tick();
  const reads = kept.f.calls.filter(c => c[0] === 'GET').length;
  await kept.box.querySelector('#feDel').onclick();
  await tick();
  assert.ok(kept.box.querySelector('#feDel'), 'still editing');
  assert.equal(kept.f.calls.filter(c => c[0] === 'GET').length, reads);
});

test('deliveries with no date are counted on screen', async () => {
  const base = answer();
  base.profit.minus.rows[0].undated = 2;
  const { box } = page({ 'GET /owner/finance': base });
  await F.open();
  await tick();
  assert.ok(box.innerHTML.includes('data-t="fn_undated"'));
  assert.match(box.textContent, /2/, 'with how many');
});
