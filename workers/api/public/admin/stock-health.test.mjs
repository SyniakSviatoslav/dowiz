// node --test workers/api/public/admin/stock-health.test.mjs
// The stock check: the rebuild report of `GET /api/owner/health` joined with
// `GET /api/owner/stock`'s open reservations, in both shapes of the report
// (without `archived`, as the hub sends today, and with it, after DAG2).
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { findings, markup, drawHealth } from './stock-health.js';
import { WORDS, merge } from './stock-health-words.js';

const t = k => `[${k}]`;
// Shaped as `services/operations/stock.rs` answers: supplies + every open reservation.
const stock = {
  supplies: [{ id: 'salmon', name: 'Salmon <b>', unit: 'g' }, { id: 'soy', name: 'Soy', unit: 'ml' }],
  stranded: [
    { order: 'o-dead-000000001', item: 'salmon', qty: 120 },
    { order: 'o-dead-000000001', item: 'soy', qty: 15 },
    { order: 'o-arch-000000002', item: 'gone', qty: 7 },
    { order: 'o-live-000000003', item: 'salmon', qty: 90 },
  ],
};
// Shaped as `services/operations/mod.rs` `rebuild` (today: no `archived`).
const today = { rebuild: { intact: false, orders: 12, stale: [], stranded: ['o-dead-000000001', 'o-arch-000000002'], unheld: [], modelled: true } };
const dag2 = { rebuild: { ...today.rebuild, archived: [{ order: 'o-arch-000000002', archive: 'log@3', status: 'DELIVERED' }] } };

test('today\'s shape: stranded orders joined with their holds; live holds are counted, not listed', () => {
  const f = findings(today, stock);
  assert.equal(f.state, 'stranded');
  assert.equal(f.checked, 12);
  assert.equal(f.live, 1);
  assert.deepEqual(f.orders.map(o => o.order), ['o-dead-000000001', 'o-arch-000000002']);
  assert.deepEqual(f.orders[0].holds, [
    { item: 'salmon', name: 'Salmon <b>', qty: 120, unit: 'g' },
    { item: 'soy', name: 'Soy', qty: 15, unit: 'ml' },
  ]);
  // A retired supply is not in the list: its id and grams stand in.
  assert.deepEqual(f.orders[1].holds, [{ item: 'gone', name: 'gone', qty: 7, unit: 'g' }]);
  assert.equal(f.orders[1].archive, '');
  const html = markup(f, t);
  assert.match(html, /data-sh-state="stranded"/);
  assert.match(html, /Salmon &lt;b&gt; · 120 g/);
  assert.doesNotMatch(html, /Salmon <b>/);
  assert.match(html, /Soy · 15 ml/);
  assert.match(html, /#o-dead-0/);
  assert.match(html, /\[sh_notArchived\]/);
  assert.match(html, /\[sh_badHint\]/);
  assert.match(html, /12 \[sh_checked\] · 1 \[sh_live\]/);
  assert.doesNotMatch(html, /o-live/);
});

test('DAG2 shape: an archived order names its archive and its last status', () => {
  const f = findings(dag2, stock);
  assert.equal(f.orders[1].archive, 'log@3');
  assert.equal(f.orders[1].status, 'DELIVERED');
  const html = markup(f, t, s => (s === 'DELIVERED' ? 'Delivered' : s));
  assert.match(html, /\[sh_archive\] <span class="mono">log@3<\/span> · \[sh_final\]: Delivered/);
  // Without the console's status words the code itself is shown.
  assert.match(markup(f, t), /\[sh_final\]: DELIVERED/);
  const bare = findings({ rebuild: { ...dag2.rebuild, archived: [{ order: 'o-arch-000000002', archive: 'log@4' }] } }, stock);
  assert.doesNotMatch(markup(bare, t), /sh_final/);
  // An entry that lost its archive name reads as not archived, never as "undefined".
  const noName = findings({ rebuild: { ...dag2.rebuild, archived: [{ order: 'o-arch-000000002', status: 'DELIVERED' }] } }, stock);
  assert.equal(noName.orders[1].archive, '');
  assert.doesNotMatch(markup(noName, t), /undefined/);
});

test('consistent: one plain line, the facts, the button; no stranded rows', () => {
  const f = findings({ rebuild: { orders: 3, stale: [], stranded: [], modelled: true } }, stock);
  assert.equal(f.state, 'ok');
  assert.equal(f.live, 4);
  const html = markup(f, t);
  assert.match(html, /data-sh-state="ok"/);
  assert.match(html, /\[sh_ok\]/);
  assert.match(html, /3 \[sh_checked\] · 4 \[sh_live\]/);
  assert.doesNotMatch(html, /alert-triangle/);
  assert.match(html, /\[sh_again\]/);
  assert.doesNotMatch(html, /sh_stale/);
});

test('a stale memo is said out loud in both states', () => {
  const ok = findings({ rebuild: { orders: 3, stale: ['x', 'y'], stranded: [] } }, {});
  assert.equal(ok.stale, 2);
  assert.match(markup(ok, t), /2 \[sh_stale\]/);
  const bad = findings({ rebuild: { orders: 3, stale: ['x'], stranded: ['o'] } }, null);
  assert.match(markup(bad, t), /1 \[sh_stale\]/);
  // A stranded order with no hold left in the stock answer still gets a row.
  assert.deepEqual(bad.orders[0].holds, []);
  assert.match(markup(bad, t), /\[sh_noHolds\]/);
  assert.equal(findings({ rebuild: { stranded: [] } }, {}).checked, 0);
});

test('a rebuild that could not run, or no rebuild at all, is an error, never "consistent"', () => {
  assert.deepEqual(findings({ rebuild: { intact: false, error: 'bad image' } }, stock), { state: 'error', error: 'bad image' });
  assert.equal(findings({}, stock).state, 'error');
  assert.equal(findings(null, stock).state, 'error');
  const html = markup(findings({ rebuild: { error: 'bad <image>' } }, stock), t);
  assert.match(html, /\[sh_error\]/);
  assert.match(html, /bad &lt;image&gt;/);
  assert.doesNotMatch(html, /sh_ok/);
});

test('not checked yet: the hint and one button', () => {
  const html = markup(null, t);
  assert.match(html, /\[sh_hint\]/);
  assert.match(html, /\[sh_run\]/);
  assert.match(html, /data-sh-again="1"/);
});

/// The smallest element drawHealth needs: innerHTML and the one button in it.
function fakeEl(){
  const el = { innerHTML: '', btnEl: { disabled: false, onclick: null } };
  el.querySelector = sel => (sel === '[data-sh-again]' && el.innerHTML.includes('data-sh-again') ? el.btnEl : null);
  return el;
}

test('drawHealth: the button first; the check runs on tap, reads /owner/health, and can run again', async () => {
  const el = fakeEl();
  const calls = [];
  const api = async path => { calls.push(path); return calls.length === 1 ? today : { rebuild: { orders: 1, stranded: [] } }; };
  await drawHealth(el, { api, stock: () => stock, t, st: s => `<${s}>` });
  assert.deepEqual(calls, []);
  assert.match(el.innerHTML, /\[sh_run\]/);
  await el.btnEl.onclick();
  assert.equal(el.btnEl.disabled, true);
  assert.deepEqual(calls, ['/owner/health']);
  assert.match(el.innerHTML, /data-sh-state="stranded"/);
  await el.btnEl.onclick();
  assert.deepEqual(calls, ['/owner/health', '/owner/health']);
  assert.match(el.innerHTML, /data-sh-state="ok"/);
});

test('drawHealth: a refused health call is drawn as the error; a missing element is a no-op', async () => {
  const el = fakeEl();
  await drawHealth(el, { api: async () => { throw new Error('403 owner only'); }, stock: () => stock, t, run: true });
  assert.match(el.innerHTML, /403 owner only/);
  await drawHealth(el, { api: async () => { throw 'plain'; }, stock: () => stock, t, run: true });
  assert.match(el.innerHTML, /plain/);
  assert.equal(await drawHealth(null, { api: null, stock: () => stock, t }), undefined);
  // An element whose button is gone: nothing to wire, nothing thrown.
  const bare = { innerHTML: '', querySelector: () => null };
  await drawHealth(bare, { api: async () => today, stock: () => stock, t, run: true });
  assert.match(bare.innerHTML, /stranded/);
});

test('words: every language has every key, none empty; merge fills, never overwrites', () => {
  const keys = Object.keys(WORDS.en).sort();
  assert.ok(keys.length >= 15);
  for (const [lang, table] of Object.entries(WORDS)) {
    assert.deepEqual(Object.keys(table).sort(), keys, `${lang} keys`);
    for (const [k, v] of Object.entries(table)) assert.ok(typeof v === 'string' && v.trim(), `${lang}.${k}`);
  }
  const en = { sh_ok: 'kept' };
  const T = { en, xx: {} };
  assert.equal(merge(T), T);
  assert.equal(T.en, en, 'merged in place');
  assert.equal(T.en.sh_ok, 'kept');
  assert.equal(T.en.sh_run, WORDS.en.sh_run);
  assert.equal(T.xx.sh_run, WORDS.en.sh_run, 'a language without words reads English');
  assert.equal(T.uk.sh_run, WORDS.uk.sh_run);
  assert.deepEqual(merge({}, {}), {});
});

test('the new files use ASCII quotes as delimiters (rule 11)', () => {
  for (const f of ['./stock-health-words.js', './stock-health.js']) {
    const src = readFileSync(new URL(f, import.meta.url), 'utf8');
    assert.doesNotMatch(src, /[‘’“”]/, f);
  }
});
