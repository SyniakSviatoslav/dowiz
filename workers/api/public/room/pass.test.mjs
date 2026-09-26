// The door (W-WIRE row 10): `node --test workers/api/public/room/pass.test.mjs`
import test from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { Element, render } from '../lib/ui/dom-shim.mjs';

// The room's dictionary imports the console's, which imports `/store/...`
// by its site path: answered from the files beside this one.
const PUBLIC = new URL('../', import.meta.url).href;
register('data:text/javascript,' + encodeURIComponent(`const P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (/^\\/(admin|lib|store)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));
const { said, renderPass, bindPass, verifyPath } = await import('./pass.js');
const { T } = await import('./i18n.js');

Object.defineProperty(Element.prototype, 'value', {
  get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });
const t = k => T.en[k] ?? k;

test('a valid pass says so with its party and time; an invalid one says why', () => {
  assert.equal(said({ ok: true, party: 4, slotMin: 19 * 60 + 30 }, t), 'Valid · 4 people · 19:30');
  assert.equal(said({ ok: false, why: 'expired' }, t), 'Not valid: expired');
  assert.equal(said(null, t), '');
  assert.equal(said({ ok: true }, t), 'Valid · ? people · 00:00');
  assert.equal(verifyPath('dubin sushi'), '/public/locations/dubin%20sushi/pass/verify');
});

test('the door checks the typed code and draws the answer; an empty code asks for one', async () => {
  const calls = [];
  const c = { t, S: { slug: 'dubin' }, api: async (path, o) => { calls.push([path, o.body]); if (o.body.code === 'boom') throw new Error('offline'); return { ok: true, party: 2, slotMin: 600 }; } };
  const { root } = render(renderPass(c));
  let backed = 0;
  bindPass(c, root, () => backed++);
  const tap = a => root.onclick({ target: root.querySelector(`[data-act="${a}"]`) });
  await tap('verifyPass');
  assert.equal(root.querySelector('#passOut').textContent, 'Type the code.');
  assert.equal(calls.length, 0);
  root.querySelector('#passCode').value = ' ABC ';
  await tap('verifyPass');
  assert.deepEqual(calls[0], ['/public/locations/dubin/pass/verify', { code: 'ABC' }]);
  assert.equal(root.querySelector('#passOut').textContent, 'Valid · 2 people · 10:00');
  root.querySelector('#passCode').value = 'boom';
  await tap('verifyPass');
  assert.equal(root.querySelector('#passOut').textContent, 'offline');
  await tap('back');
  assert.equal(backed, 1);
  await root.onclick({ target: root });
  assert.equal(calls.length, 2, 'a tap on nothing does nothing');
});
