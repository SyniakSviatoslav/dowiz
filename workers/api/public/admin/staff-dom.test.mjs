// Staff and courier cards in node (W-TKUI sweep a, 2026-10-12): a switch flipped on the card
// keeps the card open, and the list behind it is redrawn the moment the card closes --
// `rerender` skips a screen under a sheet, so without this the list stayed stale (an F5).
// `node --test workers/api/public/admin/staff-dom.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { page, tick } from './dom_test.harness.mjs';
const ST = await import('./staff.js');
const CO = await import('./couriers.js');

const openFirstRow = async (mod, p) => {
  const host = p.doc.createElement('div'); p.doc.body.appendChild(host);
  await mod.render(host); await tick();
  const row = host.querySelector('.ui-row'); assert.ok(row, 'a row to open');
  host.onclick({ target: row }); await tick();
  return host;
};

test('sweep a: a staff role changed on the card: the list is redrawn when the card closes', async () => {
  const staff = [{ id: 's1', name: 'Ira', role: 'kitchen', active: true }];
  const p = page({ 'GET /owner/staff': { staff, invites: [] }, 'POST /owner/staff/s1': { ok: true } }, { staff });
  await openFirstRow(ST, p);
  const sel = p.box.querySelector('#s-role'); assert.ok(sel, 'the card is open');
  sel.value = 'waiter'; await sel.onchange(); await tick();
  assert.equal(p.f.calls.filter(c => c[0] === 'POST').length, 1);
  const before = p.f.rerenders;
  (await import('/admin/core.js')).closeSheetFor();
  assert.ok(p.f.rerenders > before, 'the staff list behind the card is redrawn on close (sweep a)');
});

test('sweep a: a courier deactivated on the card: the list is redrawn when the card closes', async () => {
  const couriers = [{ id: 'c1', name: 'Bes', phone: '+355', active: true }];
  const p = page({ 'GET /owner/couriers': { couriers, invites: [] }, 'POST /owner/couriers/c1/active': { ok: true }, 'GET /owner/couriers/c1': { onShift: false } }, { couriers });
  await openFirstRow(CO, p);
  const sw = p.box.querySelector('#c-active'); assert.ok(sw, 'the card is open');
  sw.checked = false; await sw.onchange({ target: sw }); await tick();
  assert.equal(p.f.calls.filter(c => c[0] === 'POST').length, 1);
  const before = p.f.rerenders;
  (await import('/admin/core.js')).closeSheetFor();
  assert.ok(p.f.rerenders > before, 'the courier list behind the card is redrawn on close (sweep a)');
});
