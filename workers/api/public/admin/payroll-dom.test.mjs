// The payroll tab in node: the month as a table, one entry saved through the
// form, the entry-needs-a-rate state, and the employee's page.
// `node --test workers/api/public/admin/payroll-dom.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { page, tick } from './dom_test.harness.mjs';
const P = await import('./payroll.js');

const MONTH = { month: '2026-11', today: '2026-11-08', dates: ['2026-11-07', '2026-11-08'], totalMinutes: 930, totalMinor: 3875,
  staff: [{ id: 'a', name: 'Ira', role: 'waiter', active: true, rateMinor: 250 }, { id: 'b', name: 'Leo', role: 'kitchen', active: true, rateMinor: null }],
  rows: [{ staffId: 'a', name: 'Ira', role: 'waiter', active: true, rateMinor: 250, minutes: 930, payMinor: 3875, days: { '2026-11-07': { minutes: 450, note: '', rateMinor: 250 }, '2026-11-08': { minutes: 480, note: 'late', rateMinor: 250 } } }] };

const opened = async (routes = {}) => {
  const p = page({ 'GET /owner/payroll': MONTH, 'POST /owner/payroll/entry': { ok: true }, 'POST /owner/payroll/rate': { ok: true }, 'GET /owner/payroll/history': { months: [{ month: '2026-11', minutes: 930, payMinor: 3875 }] }, ...routes });
  await P.open(); await tick();
  return p;
};
const sheetIn = p => p.box;

test('the month is a table: employees by rows, entered dates by columns, totals', async () => {
  const p = await opened();
  const grid = sheetIn(p).querySelector('.pay-grid');
  assert.ok(grid, 'a table is drawn');
  const head = sheetIn(p).querySelector('thead').textContent;
  assert.match(head, /07/); assert.match(head, /08/);
  const body = sheetIn(p).querySelector('tbody').textContent;
  assert.match(body, /Ira/); assert.match(body, /7\.5/); assert.match(body, /15\.5/); assert.match(body, /3875/);
  assert.match(sheetIn(p).querySelector('tfoot').textContent, /3875/);
  const cells = sheetIn(p).querySelector('tbody tr').querySelectorAll('td');
  assert.deepEqual(cells.slice(0, 2).map(c => c.textContent.trim()), ['15.5', '3875'], 'the hours and pay of the month come right after the name, before the days');
  assert.deepEqual(sheetIn(p).querySelectorAll('thead th').map(h => h.getAttribute('data-t') || h.textContent), ['pay_employee', 'pay_sumHours', 'pay_sumPay', '07', '08']);
});

test('saving an entry posts integer minutes and redraws the month', async () => {
  const p = await opened();
  sheetIn(p).querySelector('#payAdd').onclick(); await tick();
  sheetIn(p).querySelector('#pe-staff').value = 'a'; sheetIn(p).querySelector('#pe-staff').onchange();
  sheetIn(p).querySelector('#pe-date').value = '2026-11-06'; sheetIn(p).querySelector('#pe-date').onchange();
  const hours = sheetIn(p).querySelector('#pe-hours'); hours.value = '7,25'; hours.oninput();
  assert.match(sheetIn(p).querySelector('#pe-pay').textContent, /7\.25 x 250 = 1813/);
  await sheetIn(p).querySelector('#peSave').onclick(); await tick();
  const c = p.f.calls.find(x => x[0] === 'POST' && x[1].startsWith('/owner/payroll/entry'));
  assert.ok(c, 'posted');
  assert.equal(c[2].minutes, 435);
  assert.equal(c[2].staff_id, 'a');
  assert.equal(c[2].date, '2026-11-06');
  assert.ok(sheetIn(p).querySelector('.pay-grid'), 'back on the month');
});

test('a bad hours value is refused with a word and posts nothing', async () => {
  const p = await opened();
  sheetIn(p).querySelector('#payAdd').onclick(); await tick();
  sheetIn(p).querySelector('#pe-hours').value = '7.3';
  await sheetIn(p).querySelector('#peSave').onclick(); await tick();
  assert.equal(p.f.calls.filter(x => x[0] === 'POST').length, 0);
  assert.deepEqual(p.f.toast, ['<pay_badHours>']);
});

test('an employee with no rate cannot be entered until a rate is set', async () => {
  const p = await opened();
  sheetIn(p).querySelector('#payAdd').onclick(); await tick();
  const s = sheetIn(p).querySelector('#pe-staff'); s.value = 'b'; s.onchange(); await tick();
  assert.ok(sheetIn(p).querySelector('#peSave').disabled, 'save is off');
  assert.ok(sheetIn(p).querySelector('#peRate'), 'the way to the rate is right there');
});

test('a tapped cell opens that entry with its hours, and delete is offered', async () => {
  const p = await opened();
  const cell = sheetIn(p).querySelector('.pay-cell');
  sheetIn(p).querySelector('#payBody').onclick({ target: cell }); await tick();
  assert.equal(sheetIn(p).querySelector('#pe-hours').value, '7.5');
  assert.ok(!sheetIn(p).querySelector('#peDel').hidden, 'an existing entry can be deleted');
});

test('the employee page saves a rate in minor units and lists pay by month', async () => {
  const p = await opened();
  const name = sheetIn(p).querySelector('.pay-name');
  sheetIn(p).querySelector('#payBody').onclick({ target: name }); await tick();
  assert.match(sheetIn(p).querySelector('#payHist').textContent, /3875/);
  sheetIn(p).querySelector('#pay-rate').value = '300';
  await sheetIn(p).querySelector('#payRateGo').onclick(); await tick();
  const c = p.f.calls.find(x => x[0] === 'POST' && x[1].startsWith('/owner/payroll/rate'));
  assert.deepEqual([c[2].staff_id, c[2].rate_minor], ['a', 300]);
});

test('a load failure says so instead of an empty table', async () => {
  const p = page({ 'GET /owner/payroll': new Error('boom') });
  await P.open(); await tick();
  assert.ok(sheetIn(p).textContent.includes('<loadFail>') || sheetIn(p).innerHTML.includes('loadFail'));
  assert.equal(sheetIn(p).querySelector('.pay-grid'), null, 'nothing of an earlier visit is shown as if it were now');
  assert.ok(sheetIn(p).querySelector('#payRetry'), 'and there is a way to ask again');
});

test('a month that cannot be read leaves the month on screen and says why', async () => {
  const p = await opened({ 'GET /owner/payroll': (b, path) => { if (path.includes('month=2026-10')) throw new Error('boom'); return MONTH; } });
  await sheetIn(p).querySelector('#payPrev').onclick(); await tick();
  assert.deepEqual(p.f.toast, ['boom']);
  assert.ok(sheetIn(p).querySelector('.pay-grid'), 'November is still drawn');
  assert.ok(sheetIn(p).querySelector('#payPrev'), 'with its arrows');
  await sheetIn(p).querySelector('#payPrev').onclick(); await tick();
  assert.equal(p.f.calls.filter(c => c[1].includes('month=2026-10')).length, 2, 'the next tap asks for October again, not September');
});

test('every request names the venue the owner has open', async () => {
  const p = await opened();
  sheetIn(p).querySelector('#payAdd').onclick(); await tick();
  sheetIn(p).querySelector('#pe-staff').value = 'a'; sheetIn(p).querySelector('#pe-staff').onchange();
  const hours = sheetIn(p).querySelector('#pe-hours'); hours.value = '8'; hours.oninput();
  await sheetIn(p).querySelector('#peSave').onclick(); await tick();
  const name = sheetIn(p).querySelector('.pay-name');
  sheetIn(p).querySelector('#payBody').onclick({ target: name }); await tick();
  sheetIn(p).querySelector('#pay-rate').value = '300';
  await sheetIn(p).querySelector('#payRateGo').onclick(); await tick();
  await sheetIn(p).querySelector('#payEmpBack').onclick(); await tick();
  const cell = sheetIn(p).querySelector('.pay-cell');
  sheetIn(p).querySelector('#payBody').onclick({ target: cell }); await tick();
  await sheetIn(p).querySelector('#peDel').onclick(); await tick();
  const seen = new Set(p.f.calls.map(c => `${c[0]} ${c[1].split('?')[0]}`));
  for (const route of ['GET /owner/payroll', 'GET /owner/payroll/history', 'POST /owner/payroll/entry', 'POST /owner/payroll/entry/delete', 'POST /owner/payroll/rate']) assert.ok(seen.has(route), route);
  for (const c of p.f.calls) assert.match(c[1], /[?&]location_id=V1(&|$)/, c[1]);
  for (const c of p.f.calls.filter(c => c[0] === 'POST')) assert.equal('location_id' in c[2], false, 'the venue rides in the query, where the hub reads it');
});

test('the form stays inside the month on screen, because that is the month it knows', async () => {
  const p = await opened();
  sheetIn(p).querySelector('#payAdd').onclick(); await tick();
  const date = sheetIn(p).querySelector('#pe-date');
  assert.equal(date.getAttribute('min'), '2026-11-01');
  sheetIn(p).querySelector('#pe-staff').value = 'a';
  date.value = '2026-10-30'; date.onchange();
  const hours = sheetIn(p).querySelector('#pe-hours'); hours.value = '8'; hours.oninput();
  await sheetIn(p).querySelector('#peSave').onclick(); await tick();
  assert.deepEqual(p.f.toast, ['<pay_dateOutside>']);
  assert.equal(p.f.calls.filter(c => c[0] === 'POST').length, 0);
});

test('moving from a filled date to an empty one clears the old hours', async () => {
  const p = await opened();
  sheetIn(p).querySelector('#payAdd').onclick(); await tick();
  sheetIn(p).querySelector('#pe-staff').value = 'a';
  const date = sheetIn(p).querySelector('#pe-date');
  date.value = '2026-11-07'; date.onchange();
  assert.equal(sheetIn(p).querySelector('#pe-hours').value, '7.5');
  date.value = '2026-11-06'; date.onchange();
  assert.equal(sheetIn(p).querySelector('#pe-hours').value, '');
});

test('a row whose person left the staff list opens as that person, not the first employee', async () => {
  const ghost = { staffId: 'gone', name: '', role: '', active: null, minutes: 60, payMinor: 100, days: { '2026-11-07': { minutes: 60, note: '', rateMinor: 100 } } };
  const p = await opened({ 'GET /owner/payroll': { ...MONTH, rows: [...MONTH.rows, ghost] } });
  const box = sheetIn(p);
  const cell = box.querySelector('[data-edit="gone|2026-11-07"]');
  assert.ok(cell);
  box.querySelector('#payBody').onclick({ target: cell }); await tick();
  const own = box.querySelector('#pe-staff option[value="gone"]');
  assert.ok(own, 'the row keeps its own person');
  assert.ok(own.hasAttribute('selected'), 'and that person is the one chosen');
  assert.equal(box.querySelector('#pe-hours').value, '1');
  assert.ok(box.querySelector('#peSave').disabled, 'hours of a person who left are not changed');
  assert.match(box.querySelector('#pe-pay').textContent, /pay_gone/);
  assert.ok(!box.querySelector('#peDel').hidden, 'and the day can still be deleted');
  await box.querySelector('#peDel').onclick(); await tick();
  const sent = p.f.calls.find(c => c[0] === 'POST');
  assert.deepEqual([sent[1].split('?')[0], sent[2]], ['/owner/payroll/entry/delete', { staff_id: 'gone', date: '2026-11-07' }], 'the delete names that person, not the first employee');
  assert.ok(box.querySelector('.pay-grid'), 'back on the month');
});

test('a cancelled delete leaves the entry on screen and sends nothing', async () => {
  const p = await opened();
  const cell = sheetIn(p).querySelector('.pay-cell');
  sheetIn(p).querySelector('#payBody').onclick({ target: cell }); await tick();
  // As in the console: the question takes the sheet's place, and "no" leaves it empty.
  Object.defineProperty(p.f, 'confirm', { get(){ p.box.innerHTML = '<p>asked</p>'; return false; } });
  await sheetIn(p).querySelector('#peDel').onclick(); await tick();
  assert.equal(p.f.calls.filter(c => c[0] === 'POST').length, 0);
  assert.equal(sheetIn(p).querySelector('#pe-hours').value, '7.5', 'the same entry, drawn again');
});

test('a month with staff and no hours says so under the table', async () => {
  const p = await opened({ 'GET /owner/payroll': { ...MONTH, dates: [], rows: [{ staffId: 'a', name: 'Ira', active: true, rateMinor: 250, minutes: 0, payMinor: 0, days: {} }], totalMinutes: 0, totalMinor: 0 } });
  assert.ok(sheetIn(p).innerHTML.includes('data-t="pay_noEntries"'));
  assert.ok(sheetIn(p).querySelector('.pay-name'), 'the people are still the way to their rate');
  const full = await opened();
  assert.ok(!sheetIn(full).innerHTML.includes('data-t="pay_noEntries"'));
});

test('an earlier month is opened with the arrow and shows the pay of each person for that month', async () => {
  const OCT = { ...MONTH, month: '2026-10', dates: ['2026-10-30'], totalMinutes: 600, totalMinor: 2700,
    rows: [{ staffId: 'a', name: 'Ira', active: true, rateMinor: 250, minutes: 480, payMinor: 2000, days: { '2026-10-30': { minutes: 480, note: '', rateMinor: 250 } } },
      { staffId: 'b', name: 'Leo', active: true, rateMinor: 350, minutes: 120, payMinor: 700, days: { '2026-10-30': { minutes: 120, note: '', rateMinor: 350 } } }] };
  const p = await opened({ 'GET /owner/payroll': (b, path) => (path.includes('month=2026-10') ? OCT : MONTH) });
  assert.ok(sheetIn(p).querySelector('#payNext').hasAttribute('disabled'), 'nothing after the present month');
  await sheetIn(p).querySelector('#payPrev').onclick(); await tick();
  assert.ok(p.f.calls.some(c => c[1].includes('month=2026-10')), 'October was asked for');
  const rows = sheetIn(p).querySelectorAll('tbody tr').map(r => r.textContent);
  assert.equal(rows.length, 2);
  assert.match(rows[0], /Ira\s+8\s*2000\s*8/, 'the hours of the month, the pay, then the day');
  assert.match(rows[1], /Leo\s+2\s*700\s*2/);
  assert.match(sheetIn(p).querySelector('tfoot').textContent, /10\s*2700/);
  assert.ok(!sheetIn(p).querySelector('#payNext').hasAttribute('disabled'), 'and the way forward is open');
  // Hours added while October is on screen start in October, and stay there.
  sheetIn(p).querySelector('#payAdd').onclick(); await tick();
  const date = sheetIn(p).querySelector('#pe-date');
  assert.deepEqual([date.value, date.getAttribute('min'), date.getAttribute('max')], ['2026-10-01', '2026-10-01', '2026-10-31']);
});

test('the staff card opens that person: the rate as stored, read fresh for the present month', async () => {
  const before = await opened({ 'GET /owner/payroll': (b, path) => ({ ...MONTH, month: path.includes('month=2026-10') ? '2026-10' : '2026-11' }) });
  await sheetIn(before).querySelector('#payPrev').onclick(); await tick();
  const p = page({ 'GET /owner/payroll': MONTH, 'GET /owner/payroll/history': { months: [] } });
  await P.openEmployee('a'); await tick();
  assert.equal(p.f.calls[0][1], '/owner/payroll?location_id=V1', 'the present month, not the one visited last');
  assert.equal(sheetIn(p).querySelector('#pay-rate').value, '250');
  assert.ok(sheetIn(p).innerHTML.includes('pay_noHistory'));
});
