// The customer card in node (W-TKUI sweep a, 2026-10-12): a link made in the card shows on the card
// at once, without closing and reopening it. `node --test workers/api/public/admin/customers-dom.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { page, tick } from './dom_test.harness.mjs';
const CU = await import('./customers.js');

test('sweep a: a key linked from the card appears in its linked list at once, with its own unlink', async () => {
  const { box, f } = page({ 'GET /owner/customers?sort=recent': { customers: [{ key: 'k3', name: 'Ana', phone: '+355' }] }, 'POST /owner/customers/k1/link': { ok: true } });
  CU.openCard({ key: 'k1', linked: ['k2'] }, null, null); await tick();
  box.querySelector('#cd-lreason').value = 'same person, said so';
  await box.querySelector('#cdLink').onclick(); await tick();
  const pick = box.querySelector('[data-to="k3"]'); assert.ok(pick, 'the other row is offered');
  await pick.onclick(); await tick();
  assert.deepEqual(f.calls.filter(c => c[0] === 'POST').map(c => c[1]), ['/owner/customers/k1/link']);
  assert.ok(box.querySelector('[data-unlink="k3"]'), 'the new link is on the card without a reopen (sweep a)');
  assert.ok(box.querySelector('[data-unlink="k2"]'), 'and the old one still is');
  assert.equal(typeof box.querySelector('[data-unlink="k3"]').onclick, 'function', 'and it can be unlinked');
});
