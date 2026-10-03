// The owner's AI sheet in node: the DOM half of admin/ai.js (W-AI).
// `node --test workers/api/public/admin/ai-dom.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { Document, Element } from '../lib/ui/dom-shim.mjs';

const PUBLIC = new URL('../', import.meta.url).href;
const FAKES = {
  '/admin/core.js': `const F = () => globalThis.__ai;
    import * as ui from '/lib/ui/index.js';
    export const esc = ui.esc, lang = 'uk';
    export const $ = (s, r = document) => r.querySelector(s), $$ = (s, r = document) => [...r.querySelectorAll(s)];
    export const t = k => '<' + k + '>';
    export const toast = m => F().toast.push(m);
    export const post = (path, body) => F().call('post', path, body);
    export const api = (path, o) => F().call('api', path, o && o.body);
    export const busy = (el, fn) => fn();
    export const retranslate = () => {}, hydrate = () => {};
    export const switchEl = (id, on, key, hint, tour) => '<label data-tour="' + tour + '"><input id="' + id + '" type="checkbox"' + (on ? ' checked' : '') + '></label>';
    export function sheet(html){ let s = document.querySelector('#sheetIn'); if (!s) { s = document.createElement('div'); s.id = 'sheetIn'; document.body.appendChild(s); } s.innerHTML = html; }`,
  '/admin/i18n.js': `export const T = { sq: {}, en: {}, uk: {}, ru: {} };`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));
const AI = await import('./ai.js');

Object.defineProperty(Element.prototype, 'value', {
  get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });
Object.defineProperty(Element.prototype, 'checked', {
  get(){ return this.hasAttribute ? this.hasAttribute('checked') : this.getAttribute('checked') != null; }, configurable: true });
Element.prototype.focus ||= function (){};
const tick = async (n = 6) => { for (let i = 0; i < n; i++) await new Promise(r => setTimeout(r, 0)); };

const STATUS = { contract: 'ai.owner.v1', enabled: true, mode: 'auto', endpoint: 'https://openrouter.ai/api/v1', model: 'google/gemma-4-26b-a4b-it:free', keySet: true,
  workersAi: { available: true, model: '@cf/qwen/qwen3-30b-a3b-fp8' }, budget: { used: 120, cap: 300, left: 180, account: 10000, resetsAtUtcDay: 1 },
  plan: ['own', 'workers-ai'], skipped: [] };
const EXPLAIN = { cards: [{ kind: 'trend', text: 'Revenue 1800 ALL', template: 'Revenue 1800 ALL', reworded: false, source: '/api/owner/analytics?days=7&v=2',
  numbers: [{ value: 1800, unit: 'money', source: '/api/owner/analytics?days=7&v=2', pointers: ['/revenue'], trace: [] }] }] };

function page(answers){
  globalThis.document = new Document();
  const log = { calls: [], toast: [] };
  globalThis.__ai = { toast: log.toast, call: async (kind, path, body) => {
    log.calls.push([kind, path, body]);
    const a = typeof answers === 'function' ? answers(kind, path, body) : answers.shift();
    if (a instanceof Error) throw a;
    return a;
  } };
  return log;
}

test('ai dom: the sheet draws the question box, the connection and the meter, and never a key', async () => {
  const log = page([STATUS, { values: { 'ai.token': '•••• set', 'ai.model': 'google/gemma-4-26b-a4b-it:free' } }, EXPLAIN]);
  await AI.openAi(); await tick();
  const s = document.querySelector('#sheetIn');
  for (const tour of ['ai.starter', 'assistant.question', 'assistant.ask', 'ai.provider', 'ai.openrouter', 'assistant.endpoint', 'assistant.model', 'assistant.token', 'assistant.save', 'ai.test', 'ai.budget', 'ai.explain', 'ai.reword'])
    // The shim reads a dot in a selector as a class, so anchors are matched by attribute value.
    assert.ok(s.querySelectorAll('[data-tour]').some(e => e.getAttribute('data-tour') === tour), tour);
  assert.match(s.innerHTML, /120 \/ 300/);
  assert.ok(!/sk-or/.test(s.innerHTML.replace('sk-or-v1-...', '')), 'no key is drawn');
  assert.deepEqual(log.calls.map(c => c[1].split('?')[0]), ['/owner/ai', '/owner/settings', '/owner/ai/explain']);
});

test('ai dom: a question is asked in the console language and each number opens its cell', async () => {
  const answer = { understood: true, answer: 'In the last 7 days: revenue 1800 ALL from 1 orders.', template: 'x', reworded: false, pickedBy: 'lexicon',
    source: '/api/owner/analytics?days=7&v=2', numbers: [{ value: 1800, unit: 'money', source: '/api/owner/analytics?days=7&v=2', pointers: ['/revenue'], trace: ['/api/owner/analytics?trace=2026-10-01'] }] };
  const log = page((kind, path) => (path === '/owner/ai' ? STATUS : path === '/owner/settings' ? { values: {} } : path.startsWith('/owner/ai/explain') ? EXPLAIN
    : path === '/owner/ai/ask' ? answer : path === '/owner/analytics?days=7&v=2' ? { revenue: 1800 } : { orders: [] }));
  await AI.openAi(); await tick();
  document.querySelector('#ai-q').value = '  revenue   this week ';
  await document.querySelector('#aiAsk').onclick(); await tick();
  const ask = log.calls.find(c => c[1] === '/owner/ai/ask');
  assert.deepEqual(ask[2], { question: 'revenue this week', lang: 'uk' });
  const out = document.querySelector('#aiOut');
  assert.match(out.textContent, /revenue 1800 ALL/);
  const num = out.querySelector('[data-ainum="0"]');
  assert.ok(num && num.getAttribute('data-tour') === 'ai.number');
  await num.onclick(); await tick();
  assert.ok(log.calls.some(c => c[1] === '/owner/analytics?days=7&v=2'), 'the source route was read');
  assert.match(document.querySelector('#aiTrail').textContent, /\/revenue = 1800/);
});

test('ai dom: save writes the settings in order and sends the key once, then forgets it', async () => {
  const log = page((kind, path) => (path === '/owner/ai' ? { ...STATUS, keySet: false } : path === '/owner/settings' ? { values: {} } : path.startsWith('/owner/ai/explain') ? { cards: [] } : { ok: true }));
  await AI.openAi(); await tick();
  document.querySelector('#aiOr').onclick();
  assert.equal(document.querySelector('#ai-endpoint').value, 'https://openrouter.ai/api/v1');
  document.querySelector('#ai-token').value = 'sk-or-v1-NEW';
  await document.querySelector('#aiSave').onclick(); await tick();
  const writes = log.calls.filter(c => c[0] === 'post' && c[1] === '/owner/settings').map(c => c[2].key);
  assert.deepEqual(writes, ['ai.enabled', 'ai.provider', 'ai.endpoint', 'ai.model', 'ai.token']);
  assert.equal(log.calls.filter(c => JSON.stringify(c[2] || '').includes('sk-or-v1-NEW')).length, 1, 'the key went out once');
});

test('ai dom: the explain card draws nothing for a member of staff (the hub refuses)', async () => {
  page([new Error('403 forbidden')]);
  const host = document.createElement('div'); document.body.appendChild(host);
  await AI.explainCard(host, 'kitchen', 7);
  assert.equal(host.innerHTML, '');
});
