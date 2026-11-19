// node --test workers/api/public/admin/supply-form-dom.test.mjs
// The ingredient form (ingredients.js openSupply), driven through the real
// sheet: what a save sends for a new and an edited ingredient, and that
// nothing typed is dropped behind a "Saved".
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { page, tick } from './dom_test.harness.mjs';

// The harness's console core has no `clock` or `day`, which the stock sheets import.
register('data:text/javascript,' + encodeURIComponent(`export async function resolve(spec, ctx, next){
  if (spec === 'harness:core') return next('/admin/core.js', ctx);
  if (spec === '/admin/core.js') return { url: 'data:text/javascript,' + encodeURIComponent("export * from 'harness:core'; export const clock = ms => String(ms);"), shortCircuit: true };
  return next(spec, ctx);
}`));

page({});
const I = await import('./ingredients.js');

const salmon = { id: 'salmon', name: 'Лосось', unit: 'g', kind: 'food_ingredient', category: 'Риба', costPerBasis: 240, kcalPer100: 208, shelfDays: 3, lowAt: 500 };
const bytes = s => new TextEncoder().encode(s).length;

function open(sup, supplies = [salmon]){
  const p = page({ 'POST /owner/supplies': b => ({ ...b }) });
  const ctx = { data: { supplies: supplies.map(s => ({ ...s })) }, reloads: 0, reload(){ this.reloads++; } };
  I.openSupply(sup ? { ...sup } : null, ctx);
  const $ = s => p.box.querySelector(s);
  const save = async () => { await $('#sSave').onclick(); await tick(); };
  const posts = () => p.f.calls.filter(c => c[0] === 'POST' && c[1] === '/owner/supplies').map(c => c[2]);
  return { p, ctx, $, save, posts };
}

test('an edit that empties the price, calories and shelf days clears them; the minimum emptied is 0', async () => {
  const s = open(salmon);
  for (const f of ['#s-cost', '#s-kcal', '#s-shelf', '#s-low']) s.$(f).value = '';
  await s.save();
  const [b] = s.posts();
  assert.deepEqual(b.clear?.slice().sort(), ['costPerBasis', 'kcalPer100', 'shelfDays']);
  assert.equal(b.lowAt, 0);
  assert.ok(!('costPerBasis' in b), 'an emptied box is cleared, not sent');
  // TWIN: an untouched edit clears nothing.
  const t = open(salmon); await t.save();
  assert.equal(t.posts()[0].clear, undefined);
  assert.equal(t.posts()[0].costPerBasis, 240);
});

test('a price typed with a space or a no-break space is read whole', async () => {
  const s = open(salmon);
  s.$('#s-cost').value = '1 200'; s.$('#s-kcal').value = '20,5'; s.$('#s-low').value = '1 000';
  await s.save();
  const [b] = s.posts();
  assert.deepEqual([b.costPerBasis, b.kcalPer100, b.lowAt], [1200, 20.5, 1000]);
});

test('a box that does not read is refused where it was typed and nothing is sent', async () => {
  const s = open(salmon);
  s.$('#s-cost').value = '12 kg';
  await s.save();
  assert.equal(s.posts().length, 0);
  assert.match(s.p.f.toast.at(-1), /<costPer>: <inv_badNumber>/);
});

test('a long Ukrainian name makes an id the hub takes', async () => {
  const s = open(null);
  s.$('#s-name').value = 'Лосось охолоджений філе преміум якість від постачальника';
  s.$('#s-name').oninput({ target: s.$('#s-name') });
  await s.save();
  const [b] = s.posts();
  assert.ok(b && bytes(b.id) <= 64 && bytes(b.id) > 40, b && `${b.id} is ${bytes(b.id)} bytes`);
  assert.equal(b.name, 'Лосось охолоджений філе преміум якість від постачальника');
});

test('a new ingredient whose id the venue holds is refused, never written over the old one', async () => {
  const s = open(null, [{ ...salmon, id: 'лосось' }]);
  s.$('#s-name').value = 'ЛОСОСЬ';
  s.$('#s-name').oninput({ target: s.$('#s-name') });
  await s.save();
  assert.equal(s.posts().length, 0);
  assert.match(s.p.f.toast.at(-1), /ЛОСОСЬ.*<nom_exists>/);
  // TWIN: another name is written.
  s.$('#s-name').value = 'Тунець'; s.$('#s-name').oninput({ target: s.$('#s-name') });
  await s.save();
  assert.equal(s.posts()[0]?.id, 'тунець');
});
