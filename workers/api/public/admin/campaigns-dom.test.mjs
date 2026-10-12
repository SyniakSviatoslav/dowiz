// The campaign editor in node (W-TKUI sweep c, 2026-10-12): the WhatsApp template and the promo
// code are optional and sit under one remembered fold. `node --test workers/api/public/admin/campaigns-dom.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { page, tick, fakeStorage } from './dom_test.harness.mjs';
const C = await import('./campaigns.js');

const openEditor = async (routes = {}) => {
  const p = page({ 'GET /owner/campaigns': { campaigns: [], tags: ['vip'] }, ...routes });
  await C.open(); await tick();
  p.box.querySelector('#cpNew').onclick(); await tick();
  return p;
};

test('sweep c: template and promo fold away, closed by default; name, text and the audience stay in the open', async () => {
  globalThis.localStorage = fakeStorage();
  const { box } = await openEditor();
  const fold = box.querySelector('details#cpTplFold');
  assert.ok(fold, 'no fold around the WhatsApp template group (sweep c)');
  assert.equal(fold.hasAttribute('open'), false);
  for (const id of ['cp-tpl', 'cp-tpl-lang', 'cp-tpl-params', 'cp-promo']) assert.ok(fold.querySelector('#' + id), `${id} is inside the fold`);
  for (const id of ['cp-name', 'cp-text', 'cp-seg', 'cpSave']) assert.equal(box.querySelector('#' + id).closest('details'), null, `${id} stays visible`);
});

test('sweep c twin: a device that opened it sees it open, and the save still reads the folded fields', async () => {
  globalThis.localStorage = fakeStorage({ 'dw_fold_cpTplFold': 'open' });
  const { box, f } = await openEditor({ 'POST /owner/campaigns': { campaign: { id: 'c9', name: 'x', segment: { kind: 'everyone_consented' } } }, 'GET /owner/campaigns/c9': { campaign: { id: 'c9', name: 'x', segment: { kind: 'everyone_consented' }, text: '-' }, sent: 0 } });
  assert.equal(box.querySelector('details#cpTplFold').hasAttribute('open'), true);
  box.querySelector('#cp-name').value = 'Spring'; box.querySelector('#cp-text').value = 'Hi'; box.querySelector('#cp-promo').value = 'SPRING'; box.querySelector('#cp-tpl').value = 'promo_v1';
  await box.querySelector('#cpSave').onclick(); await tick();
  const sent = f.calls.find(c => c[0] === 'POST' && c[1] === '/owner/campaigns')[2];
  assert.equal(sent.promo, 'SPRING'); assert.equal(sent.template.name, 'promo_v1');
});
