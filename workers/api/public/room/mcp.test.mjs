// room/mcp.js: the room's "AI agent (MCP)" sheet -- this person's tools, their
// own key minted and shown once, revoked by id.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { body, openMcp, TOURS } from './mcp.js';
import { Document, fire } from '../lib/ui/dom-shim.mjs';

const DESC = { roles: { waiter: [{ name: 'room', description: 'tables' }], kitchen: [{ name: 'kitchen_ack', description: 'saw it' }] } };
const tours = html => [...html.matchAll(/data-tour="([^"]+)"/g)].map(m => m[1]);
const settle = () => new Promise(r => setTimeout(r, 0));

test('the body is this role\'s tools, and carries the lesson W11 anchors', () => {
  const k = body('en', { url: 'https://h/api/mcp', role: 'kitchen', describe: DESC, keys: [{ id: 'k1', label: 'l', expiresMs: 0 }], key: 'dowizs_a.b' });
  assert.ok(k.includes('kitchen_ack') && !k.includes('>room<'));
  for (const a of Object.values(TOURS)) assert.ok(tours(k).includes(a.tour), a.tour);
  // No role word yet: the waiter's list; no key: the placeholder.
  const w = body('sq', { url: 'u', describe: DESC });
  assert.ok(w.includes('>room<') && !w.includes('dowizs_'));
});

test('openMcp: loads, mints a key for THIS person and shows it once, revokes by id', async () => {
  const doc = new Document(), calls = [], toasts = [], copied = [];
  const api = async (path, o = {}) => {
    calls.push([path, o.method || 'GET', o.body]);
    if (path === '/mcp') return DESC;
    if (path === '/staff/mcp/keys' && o.method === 'POST') return { key: 'dowizs_new.secret' };
    if (path === '/staff/mcp/keys') return { keys: [{ id: 'k1', label: 'old', expiresMs: 0, holder: 'staff' }] };
    if (path === '/staff/mcp/keys/revoke') return { ok: true };
    throw new Error('unexpected ' + path);
  };
  const sheet = await openMcp({ api, lang: () => 'en', toast: m => toasts.push(m), role: 'waiter', closeLabel: 'Close', doc, origin: 'https://h', copy: async t => copied.push(t) });
  const el = sheet.el;
  assert.equal(el.querySelector('#mcpUrl').textContent, 'https://h/api/mcp');
  assert.equal(el.querySelector('#mcpKey'), null);
  el.querySelector('#mcpLabel').value = 'laptop';
  fire(el.querySelector('#mcpMint'), 'click');
  await settle(); await settle();
  assert.deepEqual(calls.find(c => c[1] === 'POST'), ['/staff/mcp/keys', 'POST', { label: 'laptop' }]);
  assert.equal(el.querySelector('#mcpKey').textContent, 'dowizs_new.secret');
  assert.ok(el.querySelector('#mcpSnip-claudeCode').textContent.includes('dowizs_new.secret'));
  fire(el.querySelector('[data-copy="mcpKey"]'), 'click');
  await settle();
  assert.deepEqual(copied, ['dowizs_new.secret']);
  assert.deepEqual(toasts, ['Copied']);
  fire(el.querySelector('[data-revoke="k1"]'), 'click');
  await settle(); await settle();
  assert.deepEqual(calls.filter(c => c[0] === '/staff/mcp/keys/revoke'), [['/staff/mcp/keys/revoke', 'POST', { id: 'k1' }]]);
  // Revoked: the fresh key is not shown again.
  assert.equal(el.querySelector('#mcpKey'), null);
  assert.equal(toasts.at(-1), 'Key revoked');
});

test('openMcp: a refused mint is said, and the panel stays', async () => {
  const doc = new Document(), toasts = [];
  const api = async (path, o = {}) => {
    if (path === '/staff/mcp/keys' && o.method === 'POST') throw new Error('say what this key is for');
    if (path === '/mcp') throw new Error('offline');
    if (path === '/staff/mcp/keys/revoke') throw new Error('not found');
    return { keys: [{ id: 'k9', label: 'x', expiresMs: 0 }] };
  };
  const sheet = await openMcp({ api, lang: () => 'uk', toast: m => toasts.push(m), role: 'waiter', doc, origin: 'o' });
  fire(sheet.el.querySelector('#mcpMint'), 'click');
  await settle(); await settle();
  assert.deepEqual(toasts, ['say what this key is for']);
  assert.ok(sheet.el.querySelector('#mcpUrl'));
  fire(sheet.el.querySelector('[data-revoke="k9"]'), 'click');
  await settle(); await settle();
  assert.deepEqual(toasts.at(-1), 'not found');
});

test('openMcp: the page\'s own origin, document and clipboard by default; a nameless failure says the panel\'s error', async () => {
  const doc = new Document(), toasts = [], clip = [];
  globalThis.location = { origin: 'https://page' };
  globalThis.document = doc;
  Object.defineProperty(globalThis, 'navigator', { value: { clipboard: { writeText: async t => { clip.push(t); if (clip.length > 1) throw new Error('denied'); } } }, configurable: true });
  try {
    const api = async (path, o = {}) => {
      if (path === '/staff/mcp/keys' && o.method === 'POST') throw {};
      if (path === '/staff/mcp/keys/revoke') throw {};
      if (path === '/staff/mcp/keys') return { keys: [{ id: 'k7', label: 'x', expiresMs: 0 }] };
      return DESC;
    };
    const sheet = await openMcp({ api, lang: () => 'en', toast: m => toasts.push(m), role: 'kitchen', closeLabel: 'Close' });
    const el = sheet.el;
    assert.equal(el.querySelector('#mcpUrl').textContent, 'https://page/api/mcp');
    assert.ok(el.querySelector('#mcpHost').innerHTML.includes('kitchen_ack'));
    fire(el.querySelector('[data-copy="mcpUrl"]'), 'click');
    await settle();
    assert.deepEqual([clip, toasts], [['https://page/api/mcp'], ['Copied']]);
    // A refused clipboard is silent.
    fire(el.querySelector('[data-copy="mcpUrl"]'), 'click');
    await settle(); await settle();
    assert.deepEqual(toasts, ['Copied']);
    fire(el.querySelector('#mcpMint'), 'click');
    await settle(); await settle();
    assert.equal(toasts.at(-1), 'Something went wrong');
    fire(el.querySelector('[data-revoke="k7"]'), 'click');
    await settle(); await settle();
    assert.deepEqual(toasts.slice(1), ['Something went wrong', 'Something went wrong']);
  } finally { delete globalThis.location; delete globalThis.document; delete globalThis.navigator; }
});

test('openMcp: a sheet closed before the reads land draws nothing', async () => {
  const doc = new Document();
  let release;
  const gate = new Promise(r => { release = r; });
  const api = async path => { await gate; if (path === '/mcp') return DESC; throw new Error('offline'); };
  const p = openMcp({ api, lang: () => 'en', toast: () => {}, role: 'waiter', doc, origin: 'o' });
  await settle();
  const host = doc.body.querySelector('#mcpHost');
  host.remove();
  release();
  const sheet = await p;
  assert.equal(sheet.el.querySelector('#mcpUrl'), null);
});
