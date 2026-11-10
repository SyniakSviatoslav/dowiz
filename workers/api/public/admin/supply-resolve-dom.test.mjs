// node --test workers/api/public/admin/supply-resolve-dom.test.mjs
// A position TYPED on a delivery, driven through the real sheet
// (ingredients-count.js openDelivery): the match, the question that must be
// answered, the new position with its unit, type and category, one list call
// for the whole paper, several prices on one position, a paper that half fails.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { page, tick } from './dom_test.harness.mjs';

// The harness's console core has no `clock`, which the stock sheets import:
// the same fake, plus that one export.
register('data:text/javascript,' + encodeURIComponent(`export async function resolve(spec, ctx, next){
  if (spec === 'harness:core') return next('/admin/core.js', ctx);
  if (spec === '/admin/core.js') return { url: 'data:text/javascript,' + encodeURIComponent("export * from 'harness:core'; export const clock = ms => String(ms);"), shortCircuit: true };
  return next(spec, ctx);
}`));

page({});
const D = await import('./ingredients-count.js');
const I = await import('./ingredients.js');

const eggs = { id: 'eggs', name: 'Яйця', unit: 'unit', kind: 'food_ingredient', category: 'Продукти', onHand: 0, available: 0 };
const rice = { id: 'rice', name: 'Рис', unit: 'g', kind: 'food_ingredient', category: 'Продукти', onHand: 0, available: 0 };
const box = { id: 'box', name: 'Коробка', unit: 'unit', kind: 'packaging', category: 'Упаковка', onHand: 0, available: 0 };

function open(routes = {}, supplies = [eggs, rice, box]){
  const p = page({ 'POST /owner/stock/received': b => ({ lines: [{ value: 1 }], item: b.item }), ...routes });
  const ctx = { data: { supplies: supplies.map(s => ({ ...s })), suppliers: ['Agro'] }, reloads: 0, reload(){ this.reloads++; } };
  D.openDelivery(ctx);
  const $ = (s, r = p.box) => r.querySelector(s), $$ = (s, r = p.box) => r.querySelectorAll(s);
  const line = i => $$('[data-dl]')[i];
  const type = (i, text, settle = true) => { const inp = $('[data-srq]', line(i)); inp.value = text; inp.oninput(); if (settle) inp.onchange(); };
  const pair = (i, j, qty, price) => { const pr = $$('[data-dp]', line(i))[j]; $('[data-dlq]', pr).value = qty; $('[data-dlp]', pr).value = price; };
  const save = async () => { await $('#dGo').onclick(); await tick(); };
  const posts = path => p.f.calls.filter(c => c[0] === 'POST' && c[1] === path).map(c => c[2]);
  return { p, ctx, $, $$, line, type, pair, save, posts };
}

test('match: a typed name that is a supply uses its id, shows its name and unit, asks nothing', async () => {
  const s = open();
  s.type(0, '  ЯЙЦЯ ');
  const line = s.line(0);
  assert.ok(line.textContent.includes('Яйця') && line.textContent.includes('unit'), 'the name and unit are shown');
  assert.equal(s.$$('[data-sryes], [data-srnew], [data-srunit]', line).length, 0, 'no prompt');
  s.pair(0, 0, '30', '20');
  s.$('#d-sup').value = 'Agro'; s.$('#d-doc').value = 'F-1';
  await s.save();
  assert.deepEqual(s.posts('/owner/supplies/bulk'), [], 'nothing is created');
  assert.deepEqual(s.posts('/owner/stock/received'), [{ item: 'eggs', qty: '30', unitCost: 20, per: 1, supplier: 'Agro', doc: 'F-1' }]);
  assert.equal(s.$$('[data-dl]').length, 0, 'a saved line leaves the form');
});

test('typing shows suggestions and a tap takes one', async () => {
  const s = open();
  s.type(0, 'яйц', false);
  const hits = s.$$('[data-srpick]', s.line(0));
  assert.deepEqual(hits.map(b => b.dataset.srpick), ['eggs']);
  hits[0].onclick();
  assert.equal(s.$('[data-srq]', s.line(0)).value, 'Яйця');
  s.pair(0, 0, '10', '');
  await s.save();
  assert.deepEqual(s.posts('/owner/stock/received'), [{ item: 'eggs', qty: '10' }]);
});

test('near: the line must be answered before anything is saved; yes takes that id', async () => {
  const s = open();
  s.type(0, 'яйцо');
  s.pair(0, 0, '30', '20');
  await s.save();
  assert.equal(s.p.f.calls.length, 0, 'not merged, not created: nothing is sent');
  assert.deepEqual(s.p.f.toast, ['<sr_answer>']);
  assert.ok(s.line(0).textContent.includes('<sr_isIt>'), 'the question is on the line');
  s.$('[data-sryes]', s.line(0)).onclick();
  await s.save();
  assert.deepEqual(s.posts('/owner/supplies/bulk'), []);
  assert.deepEqual(s.posts('/owner/stock/received').map(b => b.item), ['eggs']);
});

test('near answered "no, a new position": unit and type are mandatory, then it is created first and received by the id answered', async () => {
  const s = open({ 'POST /owner/supplies/bulk': b => ({ added: b.items.map(i => ({ id: 'id-' + i.name, name: i.name, unit: i.unit, kind: i.kind, category: i.category || '' })), existing: [], matched: [] }) });
  s.type(0, 'яйцо');
  s.$('[data-srnew]', s.line(0)).onclick();
  s.pair(0, 0, '12', '');
  await s.save();
  assert.equal(s.p.f.calls.length, 0, 'no unit, no type: nothing is sent');
  assert.deepEqual(s.p.f.toast, ['<sr_fillNew>']);
  assert.equal(s.$$('[data-srunit]', s.line(0)).length, 3);
  assert.equal(s.$$('[data-srkind]', s.line(0)).length, 5, 'every kind the hub knows, and no free one');
  assert.equal(s.$$('[aria-pressed="true"]', s.line(0)).length, 0, 'nothing is chosen for the person');
  s.$('[data-srunit="unit"]', s.line(0)).onclick();
  s.$('[data-srkind="food_ingredient"]', s.line(0)).onclick();
  await s.save();
  assert.deepEqual(s.p.f.calls.map(c => c[1]), ['/owner/supplies/bulk', '/owner/stock/received'], 'created first');
  assert.deepEqual(s.posts('/owner/supplies/bulk')[0].items, [{ name: 'яйцо', unit: 'unit', kind: 'food_ingredient', notNear: ['eggs'] }]);
  assert.deepEqual(s.posts('/owner/stock/received'), [{ item: 'id-яйцо', qty: '12' }]);
});

test('none: a container is a new position with a unit, a non-food type and its own category; no nutrition is asked', async () => {
  const s = open({ 'POST /owner/supplies/bulk': b => ({ added: b.items.map(i => ({ id: 'n-' + i.name, name: i.name, unit: i.unit, kind: i.kind, category: i.category || '' })), existing: [], matched: [] }) });
  s.type(0, 'Судочок 500 мл');
  const line = s.line(0);
  assert.equal(s.$$('[data-sryes]', line).length, 0, 'nothing like it: no question');
  assert.ok(!/kcal|protein/i.test(line.innerHTML), 'no nutrition field');
  assert.deepEqual(s.$$('datalist option', line).map(o => o.getAttribute('value')), ['Продукти', 'Упаковка'], 'the venue\'s own groups are suggested');
  s.$('[data-srunit="unit"]', line).onclick();
  s.$('[data-srkind="packaging"]', line).onclick();
  const cat = s.$('[data-srcat]', line); cat.value = ' упаковка '; cat.onchange();
  assert.equal(cat.value, 'Упаковка', 'the same group under the spelling the venue has');
  s.pair(0, 0, '200', '15');
  s.$('#dAdd').onclick();
  s.type(1, 'Палички');
  s.$('[data-srunit="unit"]', s.line(1)).onclick();
  s.$('[data-srkind="utensil"]', s.line(1)).onclick();
  const cat2 = s.$('[data-srcat]', s.line(1)); cat2.value = 'Одноразове'; cat2.onchange();
  s.pair(1, 0, '100', '');
  s.$('#dAdd').onclick();
  s.type(2, 'судочок 500мл');
  assert.equal(s.$$('[data-srunit]', s.line(2)).length, 3, 'not created yet, so the second line of the same name is new too');
  s.$('[data-srunit="unit"]', s.line(2)).onclick();
  s.$('[data-srkind="packaging"]', s.line(2)).onclick();
  s.pair(2, 0, '50', '16');
  await s.save();
  const bulk = s.posts('/owner/supplies/bulk');
  assert.equal(bulk.length, 1, 'one call for the whole paper');
  assert.deepEqual(bulk[0].items, [{ name: 'Судочок 500 мл', unit: 'unit', kind: 'packaging', category: 'Упаковка', notNear: [] }, { name: 'Палички', unit: 'unit', kind: 'utensil', category: 'Одноразове', notNear: [] }],
    'the same name typed on two lines is one position');
  assert.deepEqual(s.posts('/owner/stock/received'), [
    { item: 'n-Судочок 500 мл', qty: '200', unitCost: 15, per: 1 }, { item: 'n-Палички', qty: '100' }, { item: 'n-Судочок 500 мл', qty: '50', unitCost: 16, per: 1 }]);
  // The new positions and the new group are on the screen's own list at once.
  const made = s.ctx.data.supplies.find(x => x.id === 'n-Палички');
  assert.deepEqual([made.kind, made.unit, made.category], ['utensil', 'unit', 'Одноразове']);
  I.openSupply(null, s.ctx);
  assert.ok(s.$$('#catList option').map(o => o.getAttribute('value')).includes('Одноразове'), 'the full form suggests the new group with no reload');
  assert.equal(s.ctx.reloads, 1);
});

test('a name the hub says it already has is received onto that record; a retired one is refused on its line', async () => {
  const s = open({ 'POST /owner/supplies/bulk': () => ({ added: [], existing: ['Кришка', 'Вугор'],
    matched: [{ typed: 'Кришка', id: 'lid', name: 'Кришка', unit: 'unit', retired: false }, { typed: 'Вугор', id: 'eel', name: 'Вугор', unit: 'g', retired: true }] }) });
  for (const [i, name] of [[0, 'Кришка'], [1, 'Вугор']]) {
    if (i) s.$('#dAdd').onclick();
    s.type(i, name);
    s.$('[data-srunit="g"]', s.line(i)).onclick();
    s.$('[data-srkind="packaging"]', s.line(i)).onclick();
    s.pair(i, 0, '7', '');
  }
  await s.save();
  assert.deepEqual(s.posts('/owner/stock/received'), [{ item: 'lid', qty: '7' }], 'the record\'s id, and its unit reads the quantity');
  assert.equal(s.$$('[data-dl]').length, 1, 'the retired one stays');
  assert.ok(s.line(0).textContent.includes('<sr_retired>'));
  // Positions created while every receipt of the paper fails are still on the list behind the sheet.
  const f = open({ 'POST /owner/supplies/bulk': b => ({ added: [{ id: 'cup', name: b.items[0].name, unit: 'unit', kind: 'packaging', category: '' }], existing: [], matched: [] }),
    'POST /owner/stock/received': new Error('hub said no') });
  f.type(0, 'Стакан');
  f.$('[data-srunit="unit"]', f.line(0)).onclick();
  f.$('[data-srkind="packaging"]', f.line(0)).onclick();
  f.pair(0, 0, '40', '');
  await f.save();
  assert.equal(f.ctx.reloads, 1, 'the list is refreshed for the new position');
  assert.ok(f.line(0).textContent.includes('hub said no'), 'and the receipt stays on the form with its error');
  f.p.f.calls.length = 0;
  await f.save();
  assert.deepEqual(f.p.f.calls.map(c => c[1]), ['/owner/stock/received'], 'the retry does not create the position again');
});

test('several prices on one position: each pair is its own record, same supplier and paper, its own lot code', async () => {
  const s = open();
  s.type(0, 'рис');
  s.$('[data-dmore]', s.line(0)).onclick();
  assert.equal(s.$$('[data-dp]', s.line(0)).length, 2);
  s.pair(0, 0, '10 kg', '500');
  s.pair(0, 1, '5 kg', '600');
  s.$('[data-dll]', s.line(0)).value = 'L1';
  s.$('#d-sup').value = 'Agro'; s.$('#d-doc').value = 'F-9';
  await s.save();
  assert.deepEqual(s.posts('/owner/stock/received'), [
    { item: 'rice', qty: '10000', unitCost: 500, per: 1000, supplier: 'Agro', doc: 'F-9', lot: 'L1' },
    { item: 'rice', qty: '5000', unitCost: 600, per: 1000, supplier: 'Agro', doc: 'F-9', lot: 'L1/2' }],
    'one label, two prices: the second gets its own code so the lot keeps its price');
  assert.equal(s.$$('[data-dl]').length, 0);
});

test('a paper that half fails: saved pairs leave, the failed one stays with its error, and a second save writes only it', async () => {
  let n = 0;
  const s = open({ 'POST /owner/stock/received': b => { n++; if (b.qty === '5000' && n < 4) throw new Error('hub said no'); return { lines: [{ value: 3000 }] }; } });
  s.type(0, 'рис');
  s.$('[data-dmore]', s.line(0)).onclick();
  s.pair(0, 0, '10 kg', '500');
  s.pair(0, 1, '5 kg', '600');
  s.$('#dAdd').onclick();
  s.type(1, 'яйця');
  s.pair(1, 0, '30', '20');
  await s.save();
  assert.deepEqual(s.posts('/owner/stock/received').map(b => `${b.item} ${b.qty}`), ['rice 10000', 'rice 5000', 'eggs 30']);
  assert.equal(s.$$('[data-dl]').length, 1, 'the eggs line is gone, the rice line stays');
  const left = s.$$('[data-dp]', s.line(0));
  assert.equal(left.length, 1, 'only the failed pair');
  assert.equal(s.$('[data-dlq]', left[0]).value, '5 kg');
  assert.ok(left[0].textContent.includes('hub said no'), 'with its error');
  // Two taps at once, then the retry: the failed pair is written exactly once more.
  s.p.f.calls.length = 0; n = 10;
  const a = s.$('#dGo').onclick(), b = s.$('#dGo').onclick();
  await a; await b; await tick();
  assert.deepEqual(s.posts('/owner/stock/received'), [{ item: 'rice', qty: '5000', unitCost: 600, per: 1000 }], 'no double write');
  assert.equal(s.$$('[data-dl]').length, 0);
});

test('a price that does not read is refused on its pair, never received as "no price"; a blank extra pair is not a line', async () => {
  const s = open();
  s.type(0, 'рис');
  s.$('[data-dmore]', s.line(0)).onclick();
  s.$('[data-dmore]', s.line(0)).onclick();
  s.pair(0, 0, '10 kg', '12,5');
  s.pair(0, 1, '5 kg', '600');
  await s.save();
  assert.deepEqual(s.posts('/owner/stock/received').map(b => [b.qty, b.unitCost]), [['5000', 600]], 'the unreadable price is not sent as an unpriced delivery');
  const left = s.$$('[data-dp]', s.line(0));
  assert.equal(left.length, 1, 'the blank third pair is gone with the saved one');
  assert.ok(left[0].textContent.includes('<inv_badPrice>'));
  const m = move('received');
  m.$('#m-qty').value = '2 kg'; m.$('#m-price').value = 'дорого';
  await m.save();
  assert.deepEqual(m.posts(), []);
  assert.deepEqual(m.p.f.toast, ['<inv_badPrice>']);
  // Twin: no price at all is an unpriced delivery, as it always was.
  const u = move('received');
  u.$('#m-qty').value = '2 kg';
  await u.save();
  assert.deepEqual(u.posts(), [['/owner/stock/received', { item: 'rice', qty: '2000' }]]);
});

const onePrice = async (price, currency) => {
  const s = open();
  if (currency) s.p.f.S.venue.currencyCode = currency;
  s.type(0, 'рис');
  s.pair(0, 0, '10 kg', price);
  await s.save();
  return s;
};

test('a price is read in the currency of the venue, on both sheets', async () => {
  for (const [typed, minor] of [['12,50', 1250], ['12', 1200], ['0,05', 5]]) {
    const s = await onePrice(typed, 'EUR');
    assert.deepEqual(s.posts('/owner/stock/received').map(b => b.unitCost), [minor], typed);
  }
  const fine = await onePrice('12,505', 'EUR');
  assert.deepEqual(fine.posts('/owner/stock/received'), [], 'finer than the coin is refused, not rounded');
  assert.ok(fine.$('[data-dp]', fine.line(0)).textContent.includes('<inv_badPrice>'));
  const lek = await onePrice('1 500');
  assert.deepEqual(lek.posts('/owner/stock/received').map(b => b.unitCost), [1500], 'lek has no minor digits');
  const m = move('received');
  m.p.f.S.venue.currencyCode = 'EUR';
  m.$('#m-qty').value = '2 kg'; m.$('#m-price').value = '12,50';
  await m.save();
  assert.deepEqual(m.posts().map(([, b]) => b.unitCost), [1250]);
});

test('the shelf search finds a name however it is spelled, and still finds a code or a barcode', async () => {
  const R = await import('./supply-resolve.js');
  assert.ok(R.stockSearch('яйцо')(eggs), 'the other language');
  assert.ok(!R.stockSearch('яйцо')(rice), 'and not everything');
  assert.ok(R.stockSearch('4820')({ id: 'salt', name: 'Сіль', code: '482011' }));
  assert.ok(R.stockSearch('990')({ id: 'salt', name: 'Сіль', barcode: '1990' }));
  assert.ok(R.stockSearch('')(rice), 'no words, no filter');
});

// The hub knows a position this phone has not loaded yet: another phone made it a moment ago.
const staleHub = () => ({ 'POST /owner/supplies/bulk': b => b.items[0].notNear.length
  ? { added: [{ id: 'made', name: b.items[0].name, unit: 'unit', kind: 'packaging', category: '' }], existing: [], matched: [], near: [] }
  : { added: [], existing: [], matched: [], near: [{ typed: b.items[0].name, id: 'lid', name: 'Кришка', unit: 'unit' }] } });
const typedNew = async s => {
  s.type(0, 'Кришки');
  s.$('[data-srunit="unit"]', s.line(0)).onclick();
  s.$('[data-srkind="packaging"]', s.line(0)).onclick();
  s.pair(0, 0, '7', '');
  await s.save();
};

test('a name another phone just made is a question on this one; yes receives onto that position', async () => {
  const s = open(staleHub(), [eggs, rice]);
  await typedNew(s);
  assert.deepEqual(s.posts('/owner/stock/received'), [], 'nothing is received while the question is open');
  assert.equal(s.$$('[data-sryes]', s.line(0)).length, 1, 'the question is asked on the line');
  s.$('[data-sryes]', s.line(0)).onclick();
  s.pair(0, 0, '7', '');
  await s.save();
  assert.deepEqual(s.posts('/owner/stock/received'), [{ item: 'lid', qty: '7' }]);
  assert.equal(s.posts('/owner/supplies/bulk').length, 1, 'no position was made');
});

test('no, a new position: the answer goes to the hub, and only then is it made', async () => {
  const s = open(staleHub(), [eggs, rice]);
  await typedNew(s);
  s.$('[data-srnew]', s.line(0)).onclick();
  s.$('[data-srunit="unit"]', s.line(0)).onclick();
  s.$('[data-srkind="packaging"]', s.line(0)).onclick();
  s.pair(0, 0, '7', '');
  await s.save();
  assert.deepEqual(s.posts('/owner/supplies/bulk').map(b => b.items[0].notNear), [[], ['lid']]);
  assert.deepEqual(s.posts('/owner/stock/received'), [{ item: 'made', qty: '7' }]);
});

test('a refused list of new positions writes nothing and keeps every line', async () => {
  const s = open({ 'POST /owner/supplies/bulk': new Error('line 1: a name of 1 to 80 characters') });
  s.type(0, 'Нова річ');
  s.$('[data-srunit="g"]', s.line(0)).onclick();
  s.$('[data-srkind="condiment"]', s.line(0)).onclick();
  s.pair(0, 0, '5', '');
  await s.save();
  assert.deepEqual(s.posts('/owner/stock/received'), []);
  assert.equal(s.$$('[data-dl]').length, 1);
  assert.ok(s.p.f.toast.some(m => m.includes('a name of 1 to 80')));
});

test('the write-off and prep picker: another spelling finds the position, and an unknown name becomes one on the spot', async () => {
  const p = page({ 'POST /owner/supplies/bulk': b => ({ added: b.items.map(i => ({ id: 'n1', name: i.name, unit: i.unit, kind: i.kind, category: '' })), existing: [], matched: [] }) });
  const ctx = { data: { supplies: [eggs, rice, box].map(s => ({ ...s })) }, reloads: 0, reload(){ this.reloads++; } };
  const got = [];
  I.pickSupply(ctx, 'inv_writeOff', s => got.push(s.id));
  const q = p.box.querySelector('#pkQ');
  const find = text => { q.value = text; q.oninput(); return p.box.querySelectorAll('[data-pk]').map(b => b.dataset.pk); };
  assert.deepEqual(find('яйцо'), ['eggs'], 'the Russian word finds the Ukrainian name');
  assert.deepEqual(find('упаковка'), ['box'], 'by its group');
  assert.deepEqual(find('коробки'), ['box']);
  assert.ok(p.box.querySelector('#pkNewGo'), 'a near name may still be a new position: the person decides');
  assert.deepEqual(find('КОРОБКА'), ['box']);
  assert.equal(p.box.querySelector('#pkNewGo'), null, 'a name the venue has is not offered as new');
  assert.equal(find('').length, 3);
  p.box.querySelector('#pkList').onclick({ target: p.box.querySelector('[data-pk="rice"]') });
  assert.deepEqual(got, ['rice']);
  assert.deepEqual(find('Палички бамбукові'), []);
  p.box.querySelector('#pkNewGo').onclick();
  await p.box.querySelector('#srGo').onclick();
  assert.deepEqual(p.f.toast, ['<sr_fillNew>'], 'unit and type first');
  p.box.querySelector('[data-srunit="unit"]').onclick();
  p.box.querySelector('[data-srkind="utensil"]').onclick();
  await p.box.querySelector('#srGo').onclick(); await tick();
  assert.deepEqual(p.f.calls.map(c => [c[1], c[2].items]), [['/owner/supplies/bulk', [{ name: 'Палички бамбукові', unit: 'unit', kind: 'utensil', notNear: [] }]]]);
  assert.deepEqual(got, ['rice', 'n1'], 'the new position goes straight to what the picker was opened for');
  assert.ok(ctx.data.supplies.some(s => s.id === 'n1' && s.kind === 'utensil'));
});

test('the storages Move sheet finds its position by typing, in any spelling, and never creates one', async () => {
  const ST = await import('./stock-storages.js');
  const p = page({ 'POST /owner/stock/moved': {}, 'GET /owner/stock': { supplies: [], storages: [] } });
  const stock = { supplies: [eggs, rice].map(s => ({ ...s })), storages: [{ id: 'kitchen', name: 'Kitchen' }, { id: 'bar', name: 'Bar' }] };
  ST.openMove(stock, 'kitchen');
  const $ = s => p.box.querySelector(s);
  const inp = $('[data-srq]');
  assert.equal(inp.getAttribute('data-tour'), 'store.moveItem', 'the lesson anchor is kept');
  inp.value = 'нічого такого'; inp.oninput(); inp.onchange();
  assert.equal(p.box.querySelectorAll('[data-srnew], [data-srunit]').length, 0, 'a transfer cannot invent a position');
  $('#mvQty').value = '3';
  $('[data-mvt="bar"]').onclick();
  await $('#mvGo').onclick();
  assert.deepEqual(p.f.calls, [], 'no position, nothing sent');
  assert.deepEqual(p.f.toast, ['<sto_item>']);
  inp.value = 'яйцо'; inp.oninput(); inp.onchange();
  $('[data-sryes="eggs"]').onclick();
  await $('#mvGo').onclick(); await tick();
  const sent = p.f.calls.filter(c => c[1] === '/owner/stock/moved').map(c => c[2]);
  assert.equal(sent.length, 1);
  assert.deepEqual([sent[0].item, sent[0].qty, sent[0].from, sent[0].to], ['eggs', 3, 'kitchen', 'bar']);
});

test('an e-invoice with one item on two lines at two prices posts both lines, each with its own total', async () => {
  const RP = await import('./receipt-photo.js');
  const p = page({ 'GET /owner/stock': { supplies: [{ ...rice }, { ...eggs }], supplierCards: [], supplierAliases: {} }, 'POST /owner/stock/received': {} });
  await RP.open();
  const one = (q, amt, name) => `<cac:InvoiceLine><cbc:InvoicedQuantity unitCode="KGM">${q}</cbc:InvoicedQuantity><cbc:LineExtensionAmount currencyID="ALL">${amt}</cbc:LineExtensionAmount><cac:Item><cbc:Name>${name}</cbc:Name></cac:Item></cac:InvoiceLine>`;
  const xml = `<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="c" xmlns:cac="a">${one('10', '5000', 'Рис')}${one('5', '3000', 'рис басматі')}</Invoice>`;
  await p.box.querySelector('#rpFileIn').onchange({ target: { files: [{ arrayBuffer: async () => new TextEncoder().encode(xml).buffer }], value: '' } });
  assert.deepEqual(p.box.querySelectorAll('[data-rpi]').map(s => s.querySelector('option[selected]')?.getAttribute('value')), ['rice', 'rice'], 'both lines are matched to the one position');
  await p.box.querySelector('#rpGo').onclick(); await tick();
  const sent = p.f.calls.filter(c => c[1] === '/owner/stock/received').map(c => c[2]);
  assert.deepEqual(sent.map(b => [b.item, b.qty, b.total]), [['rice', 10000, 5000], ['rice', 5000, 3000]], 'two records, so two lots and both prices in the average');
  assert.ok(sent.every(b => !b.lot), 'no lot code is shared: each record opens its own lot');
});

// ── the single-position sheet (ingredients.js openMove) ──────────────────────
function move(kind, routes = {}){
  const p = page({ 'POST /owner/stock/': () => ({ lines: [{ value: 5 }] }), ...routes });
  const ctx = { data: { supplies: [{ ...rice }], suppliers: [], storages: [] }, reloads: 0, reload(){ this.reloads++; } };
  I.openMove(ctx.data.supplies[0], kind, ctx);
  const $ = s => p.box.querySelector(s), $$ = s => p.box.querySelectorAll(s);
  const posts = () => p.f.calls.filter(c => c[0] === 'POST').map(c => [c[1], c[2]]);
  return { p, ctx, $, $$, posts, save: async () => { await $('#mGo').onclick(); await tick(); } };
}

test('one position, "another price": every quantity and price is its own record with the same paper', async () => {
  const s = move('received');
  s.$('#m-qty').value = '10 kg'; s.$('#m-price').value = '500';
  s.$('#mMoreGo').onclick();
  const extra = s.$$('[data-mp]');
  assert.equal(extra.length, 1);
  extra[0].querySelector('[data-mpq]').value = '5 kg'; extra[0].querySelector('[data-mpp]').value = '600';
  s.$('#m-sup').value = 'Agro'; s.$('#m-doc').value = 'F-9'; s.$('#m-lot').value = 'L7';
  await s.save();
  assert.deepEqual(s.posts(), [
    ['/owner/stock/received', { item: 'rice', qty: '10000', unitCost: 500, per: 1000, supplier: 'Agro', doc: 'F-9', lot: 'L7' }],
    ['/owner/stock/received', { item: 'rice', qty: '5000', unitCost: 600, per: 1000, supplier: 'Agro', doc: 'F-9', lot: 'L7/2' }]]);
  assert.equal(s.p.f.closed, 1);
  assert.equal(s.ctx.reloads, 1);
});

test('one position, the second price refused: the first is not written again by the retry', async () => {
  let refuse = true;
  const s = move('received', { 'POST /owner/stock/received': b => { if (b.qty === '5000' && refuse) throw new Error('hub said no'); return { lines: [{ value: 1 }] }; } });
  s.$('#m-qty').value = '10 kg'; s.$('#m-price').value = '500';
  s.$('#mMoreGo').onclick();
  s.$$('[data-mp]')[0].querySelector('[data-mpq]').value = '5 kg';
  await s.save();
  assert.deepEqual(s.posts().map(c => c[1].qty), ['10000', '5000']);
  assert.equal(s.p.f.closed, 0, 'the sheet stays open with what is left');
  assert.ok(s.p.f.toast.some(m => m.includes('hub said no')));
  assert.ok(s.$('#m-qty').disabled, 'the written quantity cannot be sent again');
  s.p.f.calls.length = 0; refuse = false;
  const a = s.$('#mGo').onclick(), b = s.$('#mGo').onclick();
  await a; await b; await tick();
  assert.deepEqual(s.posts().map(c => c[1].qty), ['5000'], 'only what was not written, once');
  assert.equal(s.p.f.closed, 1);
});

test('one position: a write-off and a count are one record, as before; an empty quantity sends nothing', async () => {
  const w = move('wasted');
  w.$('#m-qty').value = '100';
  w.$$('[data-r]')[0].onclick();
  await w.save();
  assert.deepEqual(w.posts(), [['/owner/stock/wasted', { item: 'rice', qty: '100', reason: 'spoiled' }]]);
  const c = move('stocktake');
  c.$('#m-qty').value = '0';
  await c.save();
  assert.deepEqual(c.posts(), [['/owner/stock/stocktake', { item: 'rice', observed: '0' }]]);
  const e = move('received');
  await e.save();
  assert.deepEqual(e.posts(), []);
  assert.deepEqual(e.p.f.toast, ['<required>']);
});
