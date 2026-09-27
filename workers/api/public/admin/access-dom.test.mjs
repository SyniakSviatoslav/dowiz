// The screens the kitchen READS, in node (KITCHEN-ACCESS-2026-09-27): bookings,
// the floor plan and the printer, opened as staff, draw no write and send none;
// opened as the owner they are what they were.
// `node --test workers/api/public/admin/access-dom.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { Document, Element } from '../lib/ui/dom-shim.mjs';

// THE BROWSER SEAM (as wire-dom.test.mjs): the console's core is a fake
// reading globalThis.__af; every other /admin/ or /lib/ path is the real file.
const PUBLIC = new URL('../', import.meta.url).href;
const FAKES = {
  '/admin/core.js': `const F = () => globalThis.__af;
    import * as ui from '/lib/ui/index.js';
    export const esc = ui.esc, store = { loc: 'V1', t: 'tok' };
    export const $ = (s, r = document) => r.querySelector(s), $$ = (s, r = document) => [...r.querySelectorAll(s)];
    export const icon = n => '<i class="ti ti-' + n + '"></i>';
    export const t = k => '<' + k + '>';
    export const S = new Proxy({}, { get: (_, k) => F().S[k] });
    export const toast = m => F().toast.push(String(m));
    export const api = (path, opts = {}) => F().reply(opts.method || 'GET', path, opts.body);
    export const post = (path, body) => F().reply('POST', path, body);
    export const busy = (el, fn) => fn();
    export const confirm = async () => ({ ok: true, reason: '' });
    export const sheet = html => { document.querySelector('#sheetIn').innerHTML = html; };
    export const closeSheet = () => { F().closed = true; };
    export const withLoc = (b = {}) => ({ location_id: 'V1', ...b });`,
  '/admin/i18n.js': `export const T = { sq: {}, en: {}, uk: {} }; export const LANGS = Object.keys(T);`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));
Object.defineProperty(Element.prototype, 'value', {
  get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });

/// A page whose hub answers `routes[METHOD path-prefix]`; every call is kept.
function page(routes = {}){
  const doc = new Document();
  globalThis.document = doc;
  doc.head = doc.body; // the sheets' own stylesheet link lands somewhere
  globalThis.location = { origin: 'https://dubin.dowiz.org', hostname: 'dubin.dowiz.org' };
  const box = doc.createElement('div'); box.setAttribute('id', 'sheetIn'); doc.body.appendChild(box);
  const f = { S: { venue: { slug: 'dubin', tz: 'Europe/Tirane' } }, toast: [], calls: [],
    reply: async (method, path, body) => {
      f.calls.push([method, path, body]);
      const key = Object.keys(routes).filter(k => `${method} ${path}`.startsWith(k)).sort((a, b) => b.length - a.length)[0];
      return key ? routes[key] : {};
    } };
  globalThis.__af = f;
  return { box, f };
}
const writes = f => f.calls.filter(c => c[0] === 'POST');

// What the hub sends each side (access::bookings_for_kitchen strips the kitchen's).
const ownerRow = { id: 'b1', slotMin: 29000000, party: 4, name: 'Ana', phone: '+355690', status: 'CONFIRMED', next: ['SEATED', 'CANCELLED_BY_VENUE'] };
const kitchenRow = { id: 'b1', slotMin: 29000000, party: 4, occasion: 'birthday', status: 'CONFIRMED', next: [] };

test('bookings, read by the kitchen: time and covers, no guest, no button, no write', async () => {
  const p = page({ 'GET /owner/reservations': { reservations: [kitchenRow, { ...kitchenRow, id: 'b2', party: 2, status: 'NO_SHOW' }] } });
  const B = await import('./bookings.js');
  await B.open({ readOnly: true });
  const html = p.box.innerHTML;
  assert.ok(html.includes('acc_readOnly'), 'says the owner changes it');
  assert.ok(!p.box.querySelector('#rsNew'), 'no new booking');
  assert.equal(p.box.querySelectorAll('[data-to]').length, 0, 'no moves');
  assert.ok(html.includes('birthday'));
  assert.match(html, /2 &lt;rsCount&gt; · 4 &lt;rsGuests&gt;/, 'covers count the guests still coming, not the no-show');
  assert.equal(writes(p.f).length, 0);
});

test('bookings, the owner twin: the guest, the phone, the moves and "New booking"', async () => {
  const p = page({ 'GET /owner/reservations': { reservations: [ownerRow] } });
  const B = await import('./bookings.js');
  await B.open();
  assert.ok(p.box.querySelector('#rsNew'));
  assert.equal(p.box.querySelectorAll('[data-to]').length, 2);
  assert.ok(p.box.innerHTML.includes('+355690'));
  assert.match(p.box.innerHTML, /1 &lt;rsCount&gt; · 4 &lt;rsGuests&gt;/);
});

const plan = { zones: [{ id: 'hall', name: 'Hall', tables: [{ n: 4, seats: 2, x: 100, y: 100, w: 60, h: 60 }] }, { id: 'terrace', name: 'Terrace', tables: [] }], planW: 800, planH: 600, maxSeats: 20, maxTables: 60 };

test('floor plan, read by the kitchen: the rooms and their tables, nothing to change', async () => {
  const p = page({ 'GET /owner/floorplan': plan });
  const F = await import('./floorplan.js');
  await F.open({ readOnly: true });
  assert.ok(p.box.innerHTML.includes('acc_readOnly'));
  assert.ok(p.box.querySelector('svg') && p.box.innerHTML.includes('>4<'), 'table 4 is drawn');
  for (const id of ['fpSave', 'fpAddZone', 'fpAddT', 'fpDelZ', 'fp-zname']) assert.ok(!p.box.querySelector('#' + id), id);
  await p.box.querySelectorAll('[data-zi]')[1].onclick();
  assert.ok(p.box.innerHTML.includes('aria-label="Terrace"') && !p.box.innerHTML.includes('>4<'), 'the other room opens');
  assert.ok(!p.box.querySelector('#fpSave'), 'still read-only');
  assert.equal(writes(p.f).length, 0);
});

test('floor plan, the owner twin: the editor with its save', async () => {
  const p = page({ 'GET /owner/floorplan': plan });
  const F = await import('./floorplan.js');
  await F.open();
  for (const id of ['fpSave', 'fpAddZone', 'fpAddT']) assert.ok(p.box.querySelector('#' + id), id);
});

test('printer, for the kitchen: name it and read the queue; no key is minted', async () => {
  const p = page({ 'GET /owner/settings': { values: { 'print.kitchen': 'pass-1' } }, 'GET /owner/print/jobs': { jobs: [{ orderId: 'ord_1', state: 'queued' }] }, 'POST /owner/settings': { ok: true } });
  const P = await import('./printer.js');
  await P.open({ staff: true });
  assert.ok(!p.box.querySelector('#prKey'), 'no key button');
  assert.ok(p.box.innerHTML.includes('#ord_1'), 'the queue is read');
  p.box.querySelector('#pr-name').value = 'pass-2';
  await p.box.querySelector('#prSave').onclick();
  assert.deepEqual(writes(p.f)[0], ['POST', '/owner/settings', { key: 'print.kitchen', value: 'pass-2' }]);
  assert.ok(!writes(p.f).some(c => c[1] === '/owner/apikeys'));
});

test('printer, the owner twin: the key button is there', async () => {
  const p = page({ 'GET /owner/settings': { values: {} }, 'GET /owner/print/jobs': { jobs: [] } });
  const P = await import('./printer.js');
  await P.open();
  assert.ok(p.box.querySelector('#prKey'));
});

// ── passwords (operator 2026-09-27) ──
const tick = async (n = 4) => { for (let i = 0; i < n; i++) await new Promise(r => setTimeout(r, 0)); };

test('the owner sets a staff password: too short is not sent, a good one goes to that person\'s route', async () => {
  const p = page({ 'POST /owner/staff/': { ok: true, sessionsEnded: 2 } });
  const W = await import('./password.js');
  W.openReset('u 1', 'Cook');
  p.box.querySelector('#rpNew').value = 'short';
  await p.box.querySelector('#rpGo').onclick();
  assert.equal(writes(p.f).length, 0, 'a short password is not sent');
  assert.match(p.box.querySelector('#rpOut').textContent, /acc_short/);
  p.box.querySelector('#rpNew').value = 'long-enough-1';
  await p.box.querySelector('#rpGo').onclick();
  assert.deepEqual(writes(p.f), [['POST', '/owner/staff/u%201/password', { location_id: 'V1', new_password: 'long-enough-1' }]]);
  assert.equal(p.f.closed, true);
});

test('the owner reset, refused by the hub (another venue\'s person), shows the hub\'s words', async () => {
  const p = page();
  p.f.reply = async () => { throw new Error('not a member of staff at your venue'); };
  const W = await import('./password.js');
  W.openReset('u2');
  p.box.querySelector('#rpNew').value = 'long-enough-1';
  await p.box.querySelector('#rpGo').onclick();
  assert.match(p.box.querySelector('#rpOut').textContent, /not a member of staff at your venue/);
});

test('a member of staff changes their own password and carries on with the fresh token', async () => {
  const p = page();
  const asked = [];
  const answers = { '/api/staff/password': [200, { changed: true }], '/api/auth/login': [403, { error: 'no active owner membership' }], '/api/staff/login': [200, { jwt: 'fresh.jwt', staff: { locationId: 'V1' } }] };
  globalThis.fetch = async (url, init) => { asked.push([url, JSON.parse(init.body)]); const [st, b] = answers[url]; return new Response(JSON.stringify(b), { status: st }); };
  const W = await import('./password.js');
  const { store } = await import('/admin/core.js');
  W.openOwn();
  p.box.querySelector('#pwEmail').value = 'cook@x.al';
  p.box.querySelector('#pwOld').value = 'old-password';
  p.box.querySelector('#pwNew').value = 'new-password';
  await p.box.querySelector('#pwGo').onclick(); await tick();
  assert.deepEqual(asked[0], ['/api/staff/password', { email: 'cook@x.al', old_password: 'old-password', new_password: 'new-password' }]);
  assert.deepEqual(asked.at(-1), ['/api/staff/login', { email: 'cook@x.al', password: 'new-password' }], 'signed in again with the new one');
  assert.equal(store.t, 'fresh.jwt');
  assert.equal(p.f.closed, true);
});

test('a wrong old password: the sheet stays, with the hub\'s 401 words', async () => {
  const p = page();
  globalThis.fetch = async () => new Response('invalid credentials', { status: 401 });
  const W = await import('./password.js');
  W.openOwn();
  p.box.querySelector('#pwEmail').value = 'cook@x.al';
  p.box.querySelector('#pwOld').value = 'wrong';
  p.box.querySelector('#pwNew').value = 'new-password';
  await p.box.querySelector('#pwGo').onclick(); await tick();
  assert.equal(p.box.querySelector('#pwOut').textContent, 'invalid credentials');
  assert.notEqual(p.f.closed, true);
});
