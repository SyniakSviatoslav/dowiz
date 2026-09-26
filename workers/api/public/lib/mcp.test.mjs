// lib/mcp.js: the "AI agent (MCP)" panel every app shows.
// `node --test workers/api/public/lib/mcp.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { snippets, PLACEHOLDER, CLIENTS, renderPanel, toolList, keyList, clientTabs, toolsOf, bindPanel } from './mcp.js';
import { XSS, injected, render, fire } from './ui/dom-shim.mjs';

const URL = 'https://dubin-sushi.dowiz.org/api/mcp';
const KEY = 'dowizs_0f3a.9b1c2d';
const W = new Proxy({}, { get: (_, k) => k === 'placeholderNote' ? 'use {k}' : `«${String(k)}»` });
const TOURS = { url: { tour: 'agent.url' }, copyUrl: { tour: 'agent.copyUrl' }, label: { tour: 'agent.label' }, mint: { tour: 'agent.mint' },
  key: { tour: 'agent.key' }, keyRow: { tour: 'agent.keyRow' }, revoke: { tour: 'agent.revoke' }, tools: { tour: 'agent.tools' },
  clients: { tour: 'agent.clients' }, snippet: { tour: 'agent.snippet' }, copySnippet: { tour: 'agent.copySnippet' } };
const TOOLS = [{ name: 'room', description: 'Every open table' }, { name: 'take_payment', description: 'Record a payment' }];

test('every client gets a setup naming the URL; the key only when one is given', () => {
  const none = snippets(URL);
  const withKey = snippets(URL, KEY);
  assert.deepEqual(Object.keys(none), CLIENTS);
  for (const c of CLIENTS) {
    assert.ok(none[c].includes(URL), c);
    assert.ok(none[c].includes(PLACEHOLDER) && !none[c].includes('dowizs_'), c);
    assert.ok(withKey[c].includes(KEY) && !withKey[c].includes(PLACEHOLDER), c);
  }
  assert.equal(withKey.claudeCode, `claude mcp add --transport http dowiz ${URL} --header "Authorization: Bearer ${KEY}"`);
  const desk = JSON.parse(withKey.claudeDesktop).mcpServers.dowiz;
  assert.deepEqual(desk.args, ['-y', 'mcp-remote', URL, '--header', 'Authorization:${DOWIZ_AUTH}']);
  assert.equal(desk.env.DOWIZ_AUTH, `Bearer ${KEY}`);
  assert.match(withKey.codex, /\[mcp_servers\.dowiz\]\nurl = "https:\/\/dubin-sushi\.dowiz\.org\/api\/mcp"\nbearer_token_env_var = "DOWIZ_MCP_KEY"/);
  assert.match(withKey.codex, /export DOWIZ_MCP_KEY="dowizs_0f3a\.9b1c2d"/);
  assert.match(none.generic, /Authorization: Bearer <YOUR_KEY>/);
});

test('the panel never shows a key it was not handed; a fresh one is shown once, with the warning', () => {
  const quiet = renderPanel({ w: W, url: URL, tools: TOOLS, keys: [], tours: TOURS, mint: true });
  assert.ok(!quiet.includes('dowizs_'));
  assert.ok(quiet.includes('use &lt;YOUR_KEY&gt;'));
  const fresh = renderPanel({ w: W, url: URL, tools: TOOLS, key: KEY, keys: [], tours: TOURS, mint: true });
  const { root } = render(fresh);
  assert.equal(root.querySelector('#mcpKey').textContent, KEY);
  assert.ok(root.querySelector('.ui-alert').textContent.includes('«shownOnce»'));
  assert.ok(root.querySelector('#mcpSnip-codex').textContent.includes(KEY));
});

test('the panel carries every anchor the lessons name, once each where it matters', () => {
  const { root } = render(renderPanel({ w: W, url: URL, tools: TOOLS, key: KEY, tours: TOURS, mint: true,
    keys: [{ id: 'k1', label: 'laptop', expiresMs: 0, holder: 'staff' }] }));
  // (the shim reads a '.' in a selector as a class, so the anchors are collected, not selected)
  const got = root.querySelectorAll('[data-tour]').map(e => e.getAttribute('data-tour'));
  for (const a of Object.values(TOURS)) assert.equal(got.filter(x => x === a.tour).length, 1, a.tour);
  // No minting (the owner mints in API keys): no label, no mint button.
  const { root: o } = render(renderPanel({ w: W, url: URL, tools: TOOLS, tours: TOURS }));
  assert.equal(o.querySelectorAll('#mcpMint').length, 0);
  assert.equal(o.querySelectorAll('#mcpKeys').length, 0);
});

test('the tool list is this role\'s, and says so when it is empty', () => {
  const { root } = render(toolList(TOOLS, W, TOURS));
  assert.deepEqual(root.querySelectorAll('.ui-row-title').map(x => x.textContent), ['room', 'take_payment']);
  assert.ok(toolList([], W).includes('«noTools»'));
  const d = { roles: { waiter: TOOLS, courier: [] } };
  assert.equal(toolsOf(d, 'waiter'), TOOLS);
  assert.deepEqual(toolsOf(d, 'owner'), []);
  assert.deepEqual(toolsOf(null, 'waiter'), []);
});

test('server strings are escaped everywhere', () => {
  const html = renderPanel({ w: W, url: XSS, key: XSS, tools: [{ name: XSS, description: XSS }], tours: TOURS, mint: true,
    keys: [{ id: XSS, label: XSS, expiresMs: 0, holder: XSS, name: XSS }] })
    + keyList([{ id: XSS, label: XSS, expiresMs: 0, holder: 'courier', role: XSS, name: XSS }], W, TOURS, true)
    + clientTabs(XSS, null, W, TOURS);
  assert.deepEqual(injected(html), []);
  assert.ok(keyList([], W).includes('«noKeys»'));
});

test('bindPanel: copy copies the block it names, mint reads the label, revoke names the key', () => {
  const { root } = render(renderPanel({ w: W, url: URL, tools: TOOLS, tours: TOURS, mint: true,
    keys: [{ id: 'k1', label: 'laptop', expiresMs: 0, holder: 'courier' }] }));
  const seen = [];
  bindPanel(root, { copy: t => seen.push(['copy', t]), onMint: l => seen.push(['mint', l]), onRevoke: (id, h) => seen.push(['revoke', id, h]) });
  fire(root.querySelector('[data-copy="mcpUrl"]'), 'click');
  root.querySelector('#mcpLabel').value = '  my laptop ';
  fire(root.querySelector('#mcpMint'), 'click');
  fire(root.querySelector('[data-revoke="k1"]'), 'click');
  assert.deepEqual(seen, [['copy', URL], ['mint', 'my laptop'], ['revoke', 'k1', 'courier']]);
  // The client tabs switch panels.
  fire(root.querySelector('#tab-mcp-codex'), 'click');
  assert.equal(root.querySelector('#mcp-codex-panel').hidden, false);
  assert.equal(root.querySelector('#mcp-claudeCode-panel').hidden, true);
  fire(root.querySelector('[data-copy="mcpSnip-codex"]'), 'click');
  assert.ok(seen.at(-1)[1].includes('[mcp_servers.dowiz]'));
  bindPanel(null);
});

test('no anchors, no description, no handlers: the panel still draws and a stray click does nothing', () => {
  const html = renderPanel({ w: W, url: URL, tools: [{ name: 'bare' }], keys: [{ id: 'k2', label: 'x', expiresMs: 0 }], mint: true });
  assert.ok(!html.includes('data-tour='));
  const { root } = render(html);
  assert.ok(root.querySelectorAll('.ui-row-title').map(x => x.textContent).includes('bare'));
  // Handlers absent: nothing throws and nothing happens.
  bindPanel(root);
  fire(root.querySelector('[data-copy="mcpUrl"]'), 'click');
  fire(root.querySelector('#mcpMint'), 'click');
  fire(root.querySelector('[data-revoke="k2"]'), 'click');
  // With handlers: a click off any button, a copy of a missing block, a revoke with no holder.
  const seen = [];
  bindPanel(root, { copy: t => seen.push(['copy', t]), onRevoke: (id, h) => seen.push(['revoke', id, h]), onMint: l => seen.push(['mint', l]) });
  fire(root.querySelector('.mcp'), 'click');
  const b = root.querySelector('[data-copy="mcpUrl"]');
  b.setAttribute('data-copy', 'nowhere');
  fire(b, 'click');
  fire(root.querySelector('[data-revoke="k2"]'), 'click');
  root.querySelector('#mcpLabel').remove();
  fire(root.querySelector('#mcpMint'), 'click');
  assert.deepEqual(seen, [['revoke', 'k2', ''], ['mint', '']]);
  // A label whose value was never set reads as empty.
  const { root: r2 } = render(renderPanel({ w: W, url: URL, tools: [], mint: true }));
  const got = [];
  bindPanel(r2, { onMint: l => got.push(l) });
  r2.querySelector('#mcpLabel').value = undefined;
  fire(r2.querySelector('#mcpMint'), 'click');
  assert.deepEqual(got, ['']);
});
