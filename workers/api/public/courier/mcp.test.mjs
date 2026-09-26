// courier/mcp.js: the courier's "AI agent (MCP)" panel.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { body, openMcp, mcpButton, TOURS } from './mcp.js';
import { useTranslator } from '../lib/ui/core.js';
import { Document, fire } from '../lib/ui/dom-shim.mjs';

useTranslator(k => `«${k}»`);
const DESC = { roles: { courier: [{ name: 'tasks', description: 'mine' }], owner: [{ name: 'dashboard', description: 'x' }] } };
const settle = () => new Promise(r => setTimeout(r, 0));

test('the button carries panel.agent; the body is the courier\'s tools only', () => {
  const b = mcpButton();
  assert.ok(b.includes('data-tour="panel.agent"') && b.includes('id="mcpOpen"') && b.includes('«agent»'));
  const h = body('en', { url: 'u', describe: DESC });
  assert.ok(h.includes('>tasks<') && !h.includes('dashboard'));
  for (const a of Object.values(TOURS)) if (!['agent.key', 'agent.keyRow', 'agent.revoke'].includes(a.tour)) assert.ok(h.includes(`data-tour="${a.tour}"`), a.tour);
});

test('openMcp: draws in the app sheet, mints (JSON body), shows the key once, revokes', async () => {
  const doc = new Document(), app = doc.createElement('div'), calls = [], toasts = [], titles = [];
  doc.body.appendChild(app);
  const api = async (path, o = {}) => {
    calls.push([path, o.method || 'GET', o.body]);
    if (path === '/mcp') return DESC;
    if (path === '/courier/mcp/keys' && o.method === 'POST') return { key: 'dowizc_n.s' };
    if (path === '/courier/mcp/keys/revoke') throw new Error('not found');
    return { keys: [{ id: 'k1', label: 'phone', expiresMs: 0, holder: 'courier' }] };
  };
  await openMcp({ api, lang: () => 'sq', toast: m => toasts.push(m), origin: 'https://h',
    panel: async (title, html) => { titles.push(title); app.innerHTML = html; }, root: () => app });
  assert.equal(titles[0], 'Agjenti AI (MCP)');
  assert.equal(app.querySelector('#mcpUrl').textContent, 'https://h/api/mcp');
  app.querySelector('#mcpLabel').value = 'phone';
  fire(app.querySelector('#mcpMint'), 'click');
  await settle(); await settle();
  assert.deepEqual(calls.find(c => c[1] === 'POST'), ['/courier/mcp/keys', 'POST', '{"label":"phone"}']);
  assert.equal(app.querySelector('#mcpKey').textContent, 'dowizc_n.s');
  fire(app.querySelector('[data-revoke="k1"]'), 'click');
  await settle(); await settle();
  assert.equal(toasts.at(-1), 'not found');
});

test('openMcp: a failed mint is said; a revoke that lands redraws without the key', async () => {
  const doc = new Document(), app = doc.createElement('div'), toasts = [];
  doc.body.appendChild(app);
  const api = async (path, o = {}) => {
    if (path === '/courier/mcp/keys' && o.method === 'POST') throw new Error('bad');
    if (path === '/courier/mcp/keys/revoke') return { ok: true };
    if (path === '/mcp') throw new Error('offline');
    return { keys: [{ id: 'k2', label: 'x', expiresMs: 0 }] };
  };
  await openMcp({ api, lang: () => 'en', toast: m => toasts.push(m), origin: 'o', panel: async (t, h) => { app.innerHTML = h; }, root: () => app });
  fire(app.querySelector('#mcpMint'), 'click');
  await settle(); await settle();
  assert.deepEqual(toasts, ['bad']);
  fire(app.querySelector('[data-revoke="k2"]'), 'click');
  await settle(); await settle();
  assert.equal(toasts.at(-1), 'Key revoked');
  assert.equal(app.querySelector('#mcpKey'), null);
});

test('openMcp: the page\'s origin and clipboard by default; nameless failures say the panel\'s error; no root binds nothing', async () => {
  const doc = new Document(), app = doc.createElement('div'), toasts = [], clip = [];
  doc.body.appendChild(app);
  globalThis.location = { origin: 'https://page' };
  Object.defineProperty(globalThis, 'navigator', { value: { clipboard: { writeText: async t => { clip.push(t); if (clip.length > 1) throw new Error('denied'); } } }, configurable: true });
  try {
    let keysRead = 0;
    const api = async (path, o = {}) => {
      if (path === '/mcp') return DESC;
      if (o.method === 'POST') throw {};
      if (keysRead++ === 0) return { keys: [{ id: 'k3', label: 'x', expiresMs: 0 }] };
      return null;
    };
    await openMcp({ api, lang: () => 'en', toast: m => toasts.push(m), panel: async (t, h) => { app.innerHTML = h; }, root: () => app });
    assert.equal(app.querySelector('#mcpUrl').textContent, 'https://page/api/mcp');
    fire(app.querySelector('[data-copy="mcpUrl"]'), 'click');
    await settle();
    fire(app.querySelector('[data-copy="mcpUrl"]'), 'click');
    await settle(); await settle();
    assert.deepEqual([clip.length, toasts], [2, ['Copied']]);
    fire(app.querySelector('#mcpMint'), 'click');
    await settle(); await settle();
    fire(app.querySelector('[data-revoke="k3"]'), 'click');
    await settle(); await settle();
    assert.deepEqual(toasts.slice(1), ['Something went wrong', 'Something went wrong']);
    // The panel drawn where there is no root: nothing to bind, nothing thrown.
    await openMcp({ api: async () => { throw new Error('offline'); }, lang: () => 'en', toast: () => {}, origin: 'o', panel: async () => {}, root: () => null });
  } finally { delete globalThis.location; delete globalThis.navigator; }
});
