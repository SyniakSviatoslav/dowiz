// admin/mcp.js: the console's per-role MCP sheet.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { body, roleTools, openMcp, TOURS, ROLES } from './mcp.js';
import { Document, fire } from '../lib/ui/dom-shim.mjs';

const DESC = { roles: { owner: [{ name: 'dashboard', description: 'd' }], waiter: [{ name: 'room', description: 'r' }],
  kitchen: [{ name: 'kitchen_ack', description: 'k' }], 'counter-manager': [], courier: [{ name: 'tasks', description: 't' }] } };
const KEYS = [{ id: 's1', holder: 'staff', role: 'waiter', person: 'p1', label: 'tablet', expiresMs: 0 },
  { id: 'c1', holder: 'courier', role: 'courier', person: 'c9', label: 'phone', expiresMs: 0 }];
const settle = () => new Promise(r => setTimeout(r, 0));

test('the sheet: every role to pick, the owner\'s tools first, every person\'s key, and never a key', () => {
  const h = body('en', { url: 'https://h/api/mcp', describe: DESC, keys: KEYS });
  for (const a of Object.values(TOURS)) assert.ok(h.includes(`data-tour="${a.tour}"`), a.tour);
  for (const r of ROLES) assert.ok(h.includes(`data-value="${r}"`), r);
  assert.ok(h.includes('>dashboard<') && !h.includes('>room<'));
  assert.ok(h.includes('waiter · p1') && h.includes('courier · c9'));
  assert.ok(h.includes('&lt;YOUR_KEY&gt;') && !/dowiz[sc]?_/.test(h));
  assert.ok(roleTools('en', { describe: DESC, role: 'kitchen' }).includes('kitchen_ack'));
  assert.ok(roleTools('en', { describe: DESC, role: 'counter-manager' }).includes('This role has no tools.'));
  assert.ok(body('sq', { url: 'u', describe: null, role: 'nonsense' }).includes('aria-checked="true" tabindex="0" data-value="owner"'));
});

test('openMcp: switching role redraws the tools; revoke names the key and its holder; API keys opens', async () => {
  const doc = new Document(), sheetIn = doc.createElement('div'), posts = [], toasts = [];
  doc.body.appendChild(sheetIn);
  let opened = 0, gets = 0;
  await openMcp({ lang: () => 'en', head: '<h2>MCP</h2>', origin: 'https://h', toast: m => toasts.push(m),
    sheet: html => { sheetIn.innerHTML = html; }, root: () => sheetIn,
    api: async p => { gets++; if (p === '/mcp') return DESC; return gets > 2 ? {} : { keys: KEYS }; },
    post: async (p, b) => { posts.push([p, b]); }, openKeys: () => { opened++; } });
  assert.equal(gets, 2);
  fire(sheetIn.querySelector('[data-value="courier"]'), 'click');
  assert.ok(sheetIn.querySelector('#mcpRoleTools').innerHTML.includes('tasks'));
  fire(sheetIn.querySelector('[data-revoke="c1"]'), 'click');
  await settle(); await settle();
  assert.deepEqual(posts, [['/owner/mcp/keys/revoke', { id: 'c1', holder: 'courier' }]]);
  assert.equal(toasts.at(-1), 'Key revoked');
  // The redraw after the revoke: a key list with no `keys` reads as none.
  assert.equal(sheetIn.querySelector('[data-revoke]'), null);
  fire(sheetIn.querySelector('#mcpKeys'), 'click');
  assert.equal(opened, 1);
});

test('openMcp: a refused revoke is said; the reads failing still draw the sheet', async () => {
  const doc = new Document(), sheetIn = doc.createElement('div'), toasts = [];
  doc.body.appendChild(sheetIn);
  await openMcp({ lang: () => 'uk', head: '', origin: 'o', toast: m => toasts.push(m),
    sheet: html => { sheetIn.innerHTML = html; }, root: () => sheetIn,
    api: async p => { if (p === '/mcp') throw new Error('x'); return { keys: KEYS }; },
    post: async () => { throw new Error('not found'); }, openKeys: () => {} });
  fire(sheetIn.querySelector('[data-revoke="s1"]'), 'click');
  await settle(); await settle();
  assert.deepEqual(toasts, ['not found']);
});

test('openMcp: the page\'s origin and clipboard by default; a nameless refusal says the panel\'s error; no key list reads as none', async () => {
  const doc = new Document(), sheetIn = doc.createElement('div'), toasts = [], clip = [];
  doc.body.appendChild(sheetIn);
  globalThis.location = { origin: 'https://page' };
  Object.defineProperty(globalThis, 'navigator', { value: { clipboard: { writeText: async t => { clip.push(t); if (clip.length > 1) throw new Error('denied'); } } }, configurable: true });
  try {
    let n = 0;
    await openMcp({ lang: () => 'en', head: '', toast: m => toasts.push(m),
      sheet: html => { sheetIn.innerHTML = html; }, root: () => sheetIn,
      api: async p => { if (p === '/mcp') return DESC; if (n++) throw new Error('offline'); return { keys: KEYS }; },
      post: async () => { if (n < 3) { n = 3; throw {}; } }, openKeys: () => {} });
    assert.equal(sheetIn.querySelector('#mcpUrl').textContent, 'https://page/api/mcp');
    fire(sheetIn.querySelector('[data-copy="mcpUrl"]'), 'click');
    await settle();
    fire(sheetIn.querySelector('[data-copy="mcpUrl"]'), 'click');
    await settle(); await settle();
    assert.deepEqual([clip.length, toasts], [2, ['Copied']]);
    fire(sheetIn.querySelector('[data-revoke="s1"]'), 'click');
    await settle(); await settle();
    assert.equal(toasts.at(-1), 'Something went wrong');
    // The second revoke lands; the key list read then fails, and the sheet says there are none.
    fire(sheetIn.querySelector('[data-revoke="s1"]'), 'click');
    await settle(); await settle(); await settle();
    assert.equal(toasts.at(-1), 'Key revoked');
    assert.equal(sheetIn.querySelector('[data-revoke]'), null);
  } finally { delete globalThis.location; delete globalThis.navigator; }
});
