// The agent panel in node: the DOM half of admin/assistant.js.
// `node --test workers/api/public/admin/assistant-dom.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { Document, Element, XSS, injected } from '../lib/ui/dom-shim.mjs';

const PUBLIC = new URL('../', import.meta.url).href;
const FAKES = {
  '/admin/core.js': `const F = () => globalThis.__af;
    import * as ui from '/lib/ui/index.js';
    export const esc = ui.esc, lang = 'uk';
    export const store = { loc: 'V1', get t(){ return F().token; } };
    export const $ = (s, r = document) => r.querySelector(s), $$ = (s, r = document) => [...r.querySelectorAll(s)];
    export const t = k => '<' + k + '>';
    export const toast = m => F().toast.push(m);
    export const post = (path, body) => F().call('post', path, body);
    export const api = (path, o) => F().call('api', path, o.body);
    export function sheet(html){ let s = document.querySelector('#sheetIn'); if (!s) { s = document.createElement('div'); s.id = 'sheetIn'; document.body.appendChild(s); } s.innerHTML = html; }`,
  '/admin/i18n.js': `export const T = { sq: {}, en: {}, uk: {} };`,
  '/room/mcp.js': `export async function openMcp(c){ globalThis.__af.mcp.push({ role: c.role, lang: c.lang(), close: c.closeLabel }); if (c.role === 'boom') throw new Error('x'); }`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));
const A = await import('./assistant.js');

Object.defineProperty(Element.prototype, 'firstElementChild', { get(){ return this.children[0] || null; }, configurable: true });
Object.defineProperty(Element.prototype, 'value', {
  get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });
Element.prototype.insertBefore ||= function (n, ref){
  if (!ref) return this.appendChild(n);
  n.parentNode = this; this.childNodes.splice(this.childNodes.indexOf(ref), 0, n); return n;
};

const jwt = c => `h.${Buffer.from(JSON.stringify(c)).toString('base64url')}.s`;
const KITCHEN = jwt({ role: 'staff', caps: 'advance,catalog,stock' });
const tick = async (n = 4) => { for (let i = 0; i < n; i++) await new Promise(r => setTimeout(r, 0)); };

function page({ token = KITCHEN, answers = [] } = {}){
  const doc = new Document();
  globalThis.document = doc;
  Object.defineProperty(globalThis, 'navigator', { value: { vibrate(){} }, configurable: true, writable: true });
  const log = { calls: [], toast: [], shown: [], refreshed: 0 };
  globalThis.__af = { token, toast: log.toast, mcp: log.mcp = [],
    call: async (kind, path, body) => { log.calls.push([kind, path, body]); const a = answers.shift(); if (a instanceof Error) throw a; return a; } };
  A.reset();
  const header = doc.createElement('div'), prefs = doc.createElement('button');
  header.appendChild(prefs); doc.body.appendChild(header);
  A.mountAssistant(header, prefs, { show: async s => { log.shown.push(s); }, refresh: async () => { log.refreshed++; } });
  return { doc, log, header };
}
const bubbles = doc => doc.body.querySelectorAll('.as-msg').map(b => b.textContent);

test('assistant dom: one button on every screen opens the panel with the starters this role may use', () => {
  const p = page();
  const b = p.header.querySelector('#asstBtn');
  assert.equal(p.header.children[0], b);
  assert.equal(b.getAttribute('data-tour'), 'hud.assistant');
  A.mountAssistant(p.header, null, {});
  assert.equal(p.header.querySelectorAll('#asstBtn').length, 1, 'mounted once');
  A.mountAssistant(null);
  b.onclick();
  const sug = p.doc.body.querySelectorAll('[data-sug]').map(c => c.getAttribute('data-sug'));
  assert.deepEqual(sug, ['asStatus', 'asShowKitchen', 'asShowStock', 'asLow']);
  assert.ok(p.doc.body.querySelector('#asQ') && p.doc.body.querySelector('#asGo'));
  const w = page({ token: jwt({ role: 'staff', caps: 'take_orders' }) });
  w.header.querySelector('#asstBtn').onclick();
  assert.deepEqual(w.doc.body.querySelectorAll('[data-sug]').map(c => c.getAttribute('data-sug')), ['asStatus']);
});

test('assistant dom: a typed line goes to the hub as the mic\'s words; a count is answered at once', async () => {
  const p = page({ answers: [{ understood: true, action: 'status', open: 3, waiting: 1 }] });
  A.open();
  const q = p.doc.body.querySelector('#asQ'); q.value = '  скільки   чекає ';
  await p.doc.body.querySelector('#asGo').onclick(); await tick();
  assert.deepEqual(p.log.calls, [['post', '/voice', { transcript: 'скільки чекає', confidence: 1, is_final: true, lang: 'uk' }]]);
  assert.deepEqual(bubbles(p.doc), ['скільки чекає', '<voiceStatus>']);
  await A.send('   '); assert.equal(p.log.calls.length, 1, 'an empty line is not sent');
});

test('assistant dom: "show me" opens the screen; a starter chip sends its own words', async () => {
  const p = page({ answers: [{ understood: true, action: 'show', screen: 'stock' }] });
  A.open();
  p.doc.body.querySelector('[data-sug="asShowStock"]').onclick(); await tick();
  assert.deepEqual(p.log.calls[0][2].transcript, '<asShowStock>');
  assert.deepEqual(p.log.shown, ['stock']);
});

test('assistant dom: a question goes to the KITCHEN\'s assistant for staff, the owner\'s for the owner', async () => {
  const p = page({ answers: [{ understood: true, action: 'ask', question: 'low?' }, { answer: 'Salmon: 200 g left' }] });
  A.open(); await A.send('low?');
  assert.deepEqual(p.log.calls[1], ['api', '/staff/assist?location_id=V1', { question: 'low?' }]);
  assert.deepEqual(bubbles(p.doc).slice(-2), ['<asThinking>', 'Salmon: 200 g left']);
  const o = page({ token: jwt({ role: 'owner' }), answers: [{ understood: true, action: 'ask', question: 'q' }, new Error('assistant off')] });
  A.open(); await A.send('q');
  assert.equal(o.log.calls[1][1], '/owner/assist');
  assert.equal(bubbles(o.doc).at(-1), 'assistant off');
  const e = page({ answers: [{ understood: true, action: 'ask', question: 'q' }, {}] });
  A.open(); await A.send('q');
  assert.equal(bubbles(e.doc).at(-1), '<asNotUnderstood>');
});

test('assistant dom: a refusal says why and what was heard; a lost hub is said', async () => {
  const p = page({ answers: [{ understood: false, say: 'Which ingredient?', heard: 'received 5 kg' }, new Error('offline'), null] });
  A.open();
  await A.send('received 5 kg'); await A.send('x'); await A.send('y');
  assert.deepEqual(bubbles(p.doc).filter((_, i) => i % 2), ['Which ingredient? · «received 5 kg»', 'offline', '<asNotUnderstood>']);
});

const PROPOSE = { understood: true, needsConfirmation: true, verb: 'receive', readback: 'прихід на склад: 4000 g Salmon', token: 'tok' };

test('assistant dom: a change is a read-back with ONE big "yes"; yes sends the token, then the button\'s route', async () => {
  const p = page({ answers: [PROPOSE, { understood: true, verb: 'receive', args: { itemId: 'salmon', qty: 4000 } }, { ok: true }] });
  A.open(); await A.send('прийшло 4 кг лосось');
  const yes = p.doc.body.querySelector('[data-yes]');
  assert.ok(yes.className.includes('ui-btn--lg') && yes.className.includes('ui-btn--primary'));
  assert.equal(p.log.calls.length, 1, 'nothing is done from the words alone');
  await yes.onclick(); await tick();
  assert.deepEqual(p.log.calls.slice(1), [['post', '/voice', { confirm: 'tok', lang: 'uk' }],
    ['post', '/owner/stock/received?location_id=V1', { item: 'salmon', qty: 4000 }]]);
  assert.deepEqual([p.log.toast, p.log.refreshed], [['<saved>'], 1]);
  assert.ok(p.doc.body.querySelector('.as-state').textContent.includes('<asDone>'));
  assert.equal(p.doc.body.querySelector('[data-yes]'), null, 'a proposal answers once');
  await A.confirmAt(1); await A.confirmAt(99);
  assert.equal(p.log.calls.length, 3);
});

test('assistant dom: "no" cancels and sends nothing', async () => {
  const p = page({ answers: [PROPOSE] });
  A.open(); await A.send('x');
  p.doc.body.querySelector('[data-no]').onclick();
  assert.ok(p.doc.body.querySelector('.as-state').textContent.includes('<asCancelled>'));
  assert.equal(p.log.calls.length, 1);
  assert.equal(A.conversation()[1].state, 'cancelled');
});

test('assistant dom: a refusal of an order asks for the reason and sends it', async () => {
  const p = page({ answers: [{ ...PROPOSE, verb: 'reject', readback: 'відхилити 4821' }, { understood: true, verb: 'reject', orderId: 'o1' }, {}] });
  A.open(); await A.send('відхили 4821');
  const yes = p.doc.body.querySelector('[data-yes]');
  assert.ok(yes.className.includes('ui-btn--danger'));
  await yes.onclick();
  assert.deepEqual([p.log.calls.length, p.log.toast], [1, ['<kReasonNeeded>']]);
  p.doc.body.querySelector('#asWhy1').value = 'no salmon';
  await p.doc.body.querySelector('[data-yes]').onclick(); await tick();
  assert.deepEqual(p.log.calls[2], ['post', '/owner/orders/o1/action', { location_id: 'V1', action: 'reject', reason: 'no salmon' }]);
});

test('assistant dom: an expired, unplanned or refused confirmation is said and marked not done', async () => {
  for (const [rest, said] of [[[{ understood: false, say: 'Expired' }], 'Expired'], [[{ understood: true, verb: 'fly' }], '<asFailed>'],
                               [[null], '<asFailed>'], [[{ understood: true, verb: 'receive', args: { itemId: 's', qty: 1 } }, new Error('403 role')], '403 role']]) {
    const p = page({ answers: [PROPOSE, ...rest] });
    A.open(); await A.send('x');
    await p.doc.body.querySelector('[data-yes]').onclick(); await tick();
    assert.equal(p.log.toast.at(-1), said);
    assert.equal(A.conversation()[1].state, 'failed');
    assert.equal(p.log.refreshed, 0);
  }
});

test('assistant dom: what the hub or a person wrote is text, never markup', async () => {
  const p = page({ answers: [{ understood: false, say: XSS }, { ...PROPOSE, readback: XSS }] });
  A.open(); await A.send(XSS); await A.send('y');
  assert.deepEqual(injected(p.doc.body.querySelector('#asLog').innerHTML), []);
});

test('assistant dom: Enter sends; other keys do not', async () => {
  const p = page({ answers: [{ understood: true, action: 'status', open: 0, waiting: 0 }] });
  A.open();
  const q = p.doc.body.querySelector('#asQ');
  q.value = 'status';
  q.onkeydown({ key: 'a', preventDefault(){} });
  assert.equal(p.log.calls.length, 0);
  q.onkeydown({ key: 'Enter', preventDefault(){} }); await tick();
  assert.equal(p.log.calls.length, 1);
});

test('assistant dom: a member of staff opens their OWN agent key here; the owner has it under More', async () => {
  const p = page();
  A.open();
  await p.doc.body.querySelector('#asKey').onclick();
  assert.deepEqual(p.log.toast, []);
  assert.deepEqual(p.log.mcp, [{ role: 'kitchen', lang: 'uk', close: '<close>' }]);
  const w = page({ token: jwt({ role: 'staff', caps: 'take_orders,advance' }) });
  A.open(); await w.doc.body.querySelector('#asKey').onclick();
  assert.equal(w.log.mcp[0].role, 'waiter');
  const o = page({ token: jwt({ role: 'owner' }) });
  A.open();
  assert.equal(o.doc.body.querySelector('#asKey'), null);
});
