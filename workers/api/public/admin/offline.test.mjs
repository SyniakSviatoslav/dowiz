// The Offline sales pane (W-OFFSALE), drawn in node against the real /lib/ui.
// `node --test workers/api/public/admin/offline.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { Document } from '../lib/ui/dom-shim.mjs';

// THE BROWSER SEAM (access-dom.test.mjs): the console's core is a fake.
const PUBLIC = new URL('../', import.meta.url).href;
const FAKES = {
  '/admin/core.js': `import * as ui from '/lib/ui/index.js';
    export const esc = ui.esc, store = { loc: 'V1', t: 'tok' };
    export const icon = n => '<i class="ti ti-' + n + '"></i>';
    export const t = k => '<' + k + '>';
    export const toast = m => globalThis.__of.toast.push(String(m));
    export const api = path => globalThis.__of.reply(path);
    export const sheet = html => { globalThis.__of.html = html; };
    export const day = ms => 'D' + ms, clock = ms => 'C' + ms;
    export const moneyEl = n => '<span class="money" data-money="' + (Number(n) | 0) + '">' + n + '</span>';`,
  '/admin/i18n.js': `export const T = { sq: {}, en: {}, uk: {}, ru: {} }; export const LANGS = Object.keys(T); export const retranslate = () => {};`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));
globalThis.document = new Document();
const { paneHtml, saleRow, openOfflineSales } = await import('./offline.js');
const { WORDS } = await import('./offline-i18n.js');
const { LANGS } = await import('../lib/langs.js');

const sale = (o = {}) => ({ order_id: 'offline:abcdef12-99', sold_at_ms: 10, synced_at_ms: 20, total: 300, currency: 'ALL', by: 's1',
  deadline_ms: 172_800_010, fiscal: 'queued', overdue: false, alerted: false, conflicts: [], ...o });

test('an overdue sale is RED: its class, its danger icon and its pill', () => {
  const late = saleRow(sale({ overdue: true, alerted: true }));
  assert.ok(late.includes('of-overdue') && late.includes('ti-alert-triangle') && late.includes('ui-badge--danger'));
  assert.ok(late.includes('data-t="of_alerted"'));
  const fine = saleRow(sale());
  assert.ok(!fine.includes('of-overdue') && fine.includes('ti-receipt'));
  assert.ok(fine.includes('>abcdef12<') && !fine.includes('>offline:'), 'the short ref, not the prefix');
  assert.ok(fine.includes('data-money="300"'), 'money through the console helper, integer');
});

test('the conflicts are named, and a product id is escaped', () => {
  const r = saleRow(sale({ conflicts: [{ kind: 'price_changed', product_id: '<b>p</b>', paid: 350, now: 400 }, { kind: 'off_sale', product_id: 'p2' }] }));
  assert.ok(r.includes('data-t="of_c_price_changed"') && r.includes('data-t="of_c_off_sale"'));
  assert.ok(!r.includes('<b>p</b>') && r.includes('&lt;b&gt;p&lt;/b&gt;'));
});

test('the pane: count, oldest, overdue, conflicts, and sending OFF said from the answer', () => {
  const d = { count: 2, oldest_sold_at_ms: 10, overdue: 1, conflicted: 1, send_enabled: false, items: [sale({ overdue: true }), sale({ order_id: 'offline:zz' })] };
  const h = paneHtml(d);
  assert.ok(h.includes('data-t="of_sendOff"'), 'sending is off: said');
  assert.ok(!paneHtml({ ...d, send_enabled: true }).includes('of_sendOff'));
  assert.equal((h.match(/class="ui-row[^"]*of-overdue/g) || []).length, 2, 'the summary row and the late sale');
  assert.ok(h.includes('data-tour="offline.list"') && h.includes('data-tour="offline.summary"'));
  assert.ok(paneHtml({ count: 0, items: [], send_enabled: false }).includes('data-t="of_none"'));
});

test('open: reads GET /api/owner/offline_sales for the venue, and a failure is a toast', async () => {
  const asked = [];
  globalThis.__of = { toast: [], reply: async p => { asked.push(p); return { count: 0, items: [], send_enabled: false }; } };
  await openOfflineSales();
  assert.deepEqual(asked, ['/owner/offline_sales?location_id=V1']);
  assert.ok(globalThis.__of.html.includes('of_none'));
  globalThis.__of = { toast: [], reply: async () => { throw new Error('HTTP 403'); } };
  await openOfflineSales();
  assert.deepEqual(globalThis.__of.toast, ['HTTP 403']);
});

test('the words: every language, the same keys, every conflict kind the server writes', () => {
  assert.deepEqual(Object.keys(WORDS).sort(), [...LANGS].sort());
  const keys = Object.keys(WORDS.en).sort();
  for (const l of LANGS) assert.deepEqual(Object.keys(WORDS[l]).sort(), keys, l);
  for (const k of ['price_changed', 'off_sale', 'unknown', 'no_price', 'options', 'unreadable', 'clock', 'currency', 'tax', 'stock', 'quantity'])
    assert.ok(keys.includes('of_c_' + k), k);
  for (const f of ['queued', 'registered', 'not_queued']) assert.ok(keys.includes('of_fiscal_' + f), f);
});
