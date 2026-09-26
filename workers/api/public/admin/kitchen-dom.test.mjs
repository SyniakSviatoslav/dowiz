// The kitchen board in node: the DOM half of admin/kitchen.js.
// `node --test workers/api/public/admin/kitchen-dom.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { Document, Element, XSS, injected } from '../lib/ui/dom-shim.mjs';

// THE BROWSER SEAM, as voice-dom.test.mjs does it: the console's core, its
// shell and its shared dictionary are answered by fakes reading globalThis.__kf;
// every other /admin/ or /lib/ path is the real file beside this one.
const PUBLIC = new URL('../', import.meta.url).href;
const FAKES = {
  '/admin/core.js': `const F = () => globalThis.__kf;
    import * as ui from '/lib/ui/index.js';
    export const esc = ui.esc, store = { loc: 'V1' };
    export const $ = (s, r = document) => r.querySelector(s), $$ = (s, r = document) => [...r.querySelectorAll(s)];
    export const icon = n => '<i class="ti ti-' + n + '"></i>';
    export const t = k => '<' + k + '>';
    export const S = new Proxy({}, { get: (_, k) => F().S[k], set: (_, k, v) => { F().S[k] = v; return true; } });
    export const toast = m => F().toast.push(m);
    export const post = (path, body) => F().call(path, body);`,
  '/admin/app.js': `const F = () => globalThis.__kf;
    export const rerender = async () => { F().rerendered++; };
    export const loadOrders = async () => { F().loaded++; };
    export const loadVenue = async () => { F().venue++; };`,
  '/admin/i18n.js': `export const T = { sq: {}, en: {}, uk: {} };`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));
const K = await import('./kitchen.js');

Object.defineProperty(Element.prototype, 'value', {
  get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });

const MIN = 60_000;
const PHONE = '+355691234567';
const tick = async (n = 3) => { for (let i = 0; i < n; i++) await new Promise(r => setTimeout(r, 0)); };

function page({ orders = [], products = [], answers = [] } = {}){
  const doc = new Document();
  globalThis.document = doc;
  Object.defineProperty(globalThis, 'navigator', { value: { vibrate(){} }, configurable: true, writable: true });
  const now = Date.now();
  globalThis.__kf = { S: { tab: 'kitchen', orders: orders.map(o => ({ ...o, created_at_ms: now - (o.age || 0) * MIN })), products, venue: null },
    toast: [], calls: [], rerendered: 0, loaded: 0, venue: 0,
    call: async (path, body) => { globalThis.__kf.calls.push([path, body]); const a = answers.shift(); if (a instanceof Error) throw a; return a || {}; } };
  const host = doc.createElement('div'); doc.body.appendChild(host);
  return { doc, host, f: globalThis.__kf };
}
const ticket = (id, status, age, items, extra = {}) => ({ id, status, age, items, fulfilment: { kind: 'pickup' }, ...extra });
const line = (name, quantity, station) => ({ product_id: name, name, quantity, ...(station ? { station } : {}) });

test('kitchen dom: three columns, oldest first, each ticket with ONE big bump', async () => {
  const p = page({ orders: [
    ticket('ord_new1', 'PENDING', 2, [line('Miso', 1)]),
    ticket('ord_cook', 'PREPARING', 12, [line('Dragon', 3, 'sushi')]),
    ticket('ord_rdy1', 'READY', 25, [line('Beer', 2, 'bar')], { fulfilment: { kind: 'dine_in', table: '4' } }),
    ticket('ord_gone', 'DELIVERED', 1, [line('Old', 1)]),
  ] });
  await K.render(p.host);
  const cols = p.host.querySelectorAll('section.kds-col');
  assert.equal(cols.length, 3);
  assert.deepEqual(cols.map(c => c.querySelectorAll('article').length), [1, 1, 1]);
  const bumps = p.host.querySelectorAll('[data-bump]').map(b => b.getAttribute('data-act'));
  assert.deepEqual(bumps, ['confirm', 'ready', 'collected']);
  for (const b of p.host.querySelectorAll('[data-bump]')) assert.ok(b.className.includes('kds-bump') && b.className.includes('ui-btn--lg'));
  // Age colours: 2 min ok, 12 amber, 25 red.
  const ages = p.host.querySelectorAll('article').map(a => a.className.match(/kds-age-(\w+)/)[1]);
  assert.deepEqual(ages.sort(), ['late', 'ok', 'warn']);
  assert.equal(p.host.querySelector('[data-o="ord_gone"]'), null, 'a delivered order is off the pass');
  // (The shim reads a dot in a selector as a class, so anchors are found by attribute.)
  const tours = p.host.querySelectorAll('[data-tour]').map(e => e.getAttribute('data-tour'));
  for (const a of ['kitchen.board', 'kitchen.allday', 'kitchen.station', 'kitchen.seen', 'kitchen.bump', 'kitchen.reject', 'kitchen.stoplist']) assert.ok(tours.includes(a), a);
  assert.ok(p.host.textContent.includes('<kTable> 4'), 'a table is named by its number');
});

test('kitchen dom: a planted phone or address on an owner\'s copy is never drawn', async () => {
  const p = page({ orders: [ticket('o1', 'PENDING', 1, [line('Miso', 1)], {
    contact: { name: 'Arta', phone: PHONE }, fulfilment: { kind: 'delivery', address: { line: 'Rruga 14' }, note: 'pa susam' } })] });
  await K.render(p.host);
  const html = p.host.innerHTML;
  for (const leak of [PHONE, 'Arta', 'Rruga 14']) assert.ok(!html.includes(leak), leak);
  assert.ok(html.includes('pa susam'), 'the kitchen note is the kitchen\'s');
});

test('kitchen dom: a station chip filters; an empty pass says so', async () => {
  const p = page({ orders: [ticket('o1', 'PENDING', 1, [line('Miso', 1), line('Beer', 1, 'bar')])] });
  await K.render(p.host);
  const chip = p.host.querySelector('[data-st="bar"]');
  assert.equal(chip.textContent, '<st_bar> 1');
  await K.act(chip, p.host);
  assert.equal(K.station(), 'bar');
  assert.equal(p.f.rerendered, 1);
  await K.render(p.host);
  assert.deepEqual(p.host.querySelectorAll('.kds-line').map(l => l.textContent.includes('Beer')), [true]);
  K.setStation('grill'); assert.equal(K.station(), 'bar', 'an unknown station is refused');
  K.setStation('all');
  const q = page(); await K.render(q.host);
  assert.ok(q.host.querySelector('[data-t="kNoTickets"]'), 'the empty pass says so');
  assert.equal(q.host.querySelector('section.kds-col'), null);
});

test('kitchen dom: a bump sends the console\'s own action and re-reads the pass', async () => {
  const p = page({ orders: [ticket('o/1', 'CONFIRMED', 1, [line('Miso', 1)])] });
  await K.render(p.host);
  await K.act(p.host.querySelector('[data-bump]'), p.host);
  assert.deepEqual(p.f.calls, [['/owner/orders/o%2F1/action', { location_id: 'V1', action: 'preparing' }]]);
  assert.deepEqual([p.f.toast, p.f.loaded], [['<saved>'], 1]);
});

test('kitchen dom: "seen" is a tap on the ticket head, once', async () => {
  const p = page({ orders: [ticket('o1', 'PENDING', 1, [line('Miso', 1)])] });
  await K.render(p.host);
  const head = p.host.querySelector('[data-seen]');
  assert.equal(head.getAttribute('aria-pressed'), 'false');
  await K.act(head, p.host);
  assert.deepEqual(p.f.calls, [['/staff/orders/o1/kitchen-ack', { location_id: 'V1' }]]);
  assert.equal(head.getAttribute('aria-pressed'), 'true');
  await K.act(head, p.host);
  assert.equal(p.f.calls.length, 1, 'a second tap sends nothing');
  const q = page({ orders: [ticket('o2', 'PENDING', 1, [line('Miso', 1)], { kitchen: { seen: { by: 's', at: 1 } } })] });
  await K.render(q.host);
  assert.equal(q.host.querySelector('[data-seen]').getAttribute('aria-pressed'), 'true');
});

test('kitchen dom: reject asks for the reason; a blank one sends nothing', async () => {
  const p = page({ orders: [ticket('o1', 'PENDING', 1, [line('Miso', 1)])] });
  await K.render(p.host);
  const stop = p.host.querySelector('[data-stop]');
  assert.equal(stop.getAttribute('data-act'), 'reject');
  await K.act(stop, p.host);
  const yes = () => p.doc.body.querySelector('[data-ui-close="yes"]');
  const click = el => { const scrim = p.doc.body.querySelector('.ui-scrim'); for (const f of scrim._l.click) f({ target: el }); };
  click(yes()); await tick();
  assert.deepEqual([p.f.calls.length, p.f.toast], [0, ['<kReasonNeeded>']]);
  await K.act(stop, p.host);
  const input = p.doc.body.querySelector('#kdsWhy');
  input.value = ' no salmon '; input.oninput();
  click(yes()); await tick();
  assert.deepEqual(p.f.calls, [['/owner/orders/o1/action', { location_id: 'V1', action: 'reject', reason: 'no salmon' }]]);
  // Cooking already: no stop on the ticket.
  const q = page({ orders: [ticket('o2', 'PREPARING', 1, [line('Miso', 1)])] });
  await K.render(q.host);
  assert.equal(q.host.querySelector('[data-stop]'), null);
});

test('kitchen dom: a dish is 86ed and put back from the stop list, and names are escaped', async () => {
  const p = page({ products: [{ id: 'p1', name: 'Dragon', available: true }, { id: 'p2', name: XSS, available: false, unavailableNote: 'no tuna' }] });
  await K.render(p.host);
  assert.deepEqual(injected(p.host.innerHTML), []);
  const rows = p.host.querySelectorAll('.kds-dish');
  assert.ok(rows[0].className.includes('off'), 'what is off sale is listed first');
  await K.act(p.host.querySelector('[data-off="p1"]'), p.host);
  await K.act(p.host.querySelector('[data-on="p2"]'), p.host);
  assert.deepEqual(p.f.calls, [['/owner/products/p1', { location_id: 'V1', available: false }], ['/owner/products/p2', { location_id: 'V1', available: true }]]);
  assert.equal(p.f.venue, 2);
});

test('kitchen dom: a refused request is said, and nothing else moves', async () => {
  const p = page({ orders: [ticket('o1', 'PENDING', 1, [line('Miso', 1)])], answers: [new Error('409 illegal edge')] });
  await K.render(p.host);
  await K.act(p.host.querySelector('[data-bump]'), p.host);
  assert.deepEqual([p.f.toast, p.f.loaded], [['409 illegal edge'], 0]);
  await K.act(null, p.host);
  await K.act(p.host.querySelector('h1'), p.host);
  assert.equal(p.f.calls.length, 1);
});

test('kitchen dom: the stop list search narrows the dishes in place', async () => {
  const p = page({ products: [{ id: 'a', name: 'Dragon roll', available: true }, { id: 'b', name: 'Miso', available: true }] });
  await K.render(p.host);
  const q = p.host.querySelector('#kdsQ');
  q.oninput({ target: { value: 'drag' } });
  assert.deepEqual(p.host.querySelectorAll('.kds-dish').length, 1);
});
