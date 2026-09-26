// The owner's mic, in node: the DOM half of admin/voice.js (the header button,
// the recogniser it drives, the read-back sheet and the request a Confirm makes).
// `node --test workers/api/public/admin/`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { Document, Element, fire } from '../lib/ui/dom-shim.mjs';

// THE BROWSER SEAM: admin/voice.js imports the console's /admin/core.js and the
// browser's /lib/voice.js by absolute path. The hooks answer those two with
// modules reading globalThis.__afake / __vfake, and every other absolute
// /admin/ or /lib/ path with the real file beside this one.
const PUBLIC = new URL('../', import.meta.url).href;
const FAKES = {
  '/admin/core.js': `const F = () => globalThis.__afake;
    export const lang = 'uk', store = { loc: 'V1' };
    export const t = k => '<' + k + '>';
    export const toast = m => F().toast.push(m);
    export const post = (path, body) => F().call('post', path, body);
    export const api = (path, o) => F().call('api', path, o.body);`,
  '/lib/voice.js': `const F = () => globalThis.__vfake;
    export const supported = () => F().supported;
    export const tagFor = l => 'tag-' + l;
    export const speak = (text, lang) => { F().spoken.push([text, lang]); };
    export function create(o){ F().created.push(o); return F().rec; }`,
};
register('data:text/javascript,' + encodeURIComponent(`const F = ${JSON.stringify(FAKES)}, P = ${JSON.stringify(PUBLIC)};
  export async function resolve(spec, ctx, next){
    if (F[spec]) return { url: 'data:text/javascript,' + encodeURIComponent(F[spec]), shortCircuit: true };
    if (/^\\/(admin|lib)\\//.test(spec)) return { url: P + spec.slice(1), shortCircuit: true };
    return next(spec, ctx);
  }`));
const { mountVoice } = await import('./voice.js');

// The shim has no firstElementChild, no insertBefore and no live `.value`.
Object.defineProperty(Element.prototype, 'firstElementChild', { get(){ return this.children[0] || null; }, configurable: true });
Object.defineProperty(Element.prototype, 'value', {
  get(){ return this.getAttribute('value') ?? ''; }, set(v){ this.setAttribute('value', v); }, configurable: true });
Element.prototype.insertBefore ||= function (n, ref) {
  if (!ref) return this.appendChild(n);
  if (n.parentNode) n.remove();
  n.parentNode = this;
  this.childNodes.splice(this.childNodes.indexOf(ref), 0, n);
  return n;
};

const tick = async (n = 3) => { for (let i = 0; i < n; i++) await new Promise(r => setTimeout(r, 0)); };

/// A fresh console header and a fake browser; `answers` are the hub's replies
/// to each post/api call, in order (an Error is thrown instead).
function page({ supported = true, answers = [] } = {}) {
  const doc = new Document();
  globalThis.document = doc;
  const rec = { started: 0, stopped: 0, start(){ this.started++; }, stop(){ this.stopped++; } };
  globalThis.__vfake = { supported, spoken: [], created: [], rec };
  const log = { calls: [], toast: [], refreshed: 0 };
  globalThis.__afake = { toast: log.toast,
    call: async (kind, path, body) => { log.calls.push([kind, path, body]); const a = answers.shift(); if (a instanceof Error) throw a; if (a && 'raw' in a) throw a.raw; return a; } };
  const header = doc.createElement('div');
  const prefs = doc.createElement('button'); prefs.id = 'prefs'; header.appendChild(prefs);
  doc.body.appendChild(header);
  mountVoice(header, prefs, async () => { log.refreshed++; });
  const btn = header.querySelector('#voiceBtn');
  const say = async transcript => {
    btn.onclick();
    globalThis.__vfake.created.at(-1).onResult({ transcript, confidence: 0.8, isFinal: true });
    await tick();
  };
  const sheetBtn = v => doc.body.querySelector(`[data-ui-close="${v}"]`);
  return { doc, log, header, btn, rec, say, sheetBtn, fake: globalThis.__vfake };
}

test('voice dom: the mic is drawn once, before the language button, only where speech is heard', () => {
  const p = page();
  assert.equal(p.header.children[0], p.btn);
  assert.equal(p.btn.getAttribute('data-tour'), 'hud.voice');
  mountVoice(p.header, null, () => {});
  assert.equal(p.header.querySelectorAll('#voiceBtn').length, 1);
  assert.equal(page({ supported: false }).btn, null);
  mountVoice(null, null, () => {});
  globalThis.__vfake.supported = true;
  const h = globalThis.document.createElement('div');
  mountVoice(h, undefined, () => {});
  assert.equal(h.children.length, 1);
});

test('voice dom: a tap listens in the console\'s language; a second stops; errors are said', () => {
  const p = page();
  p.btn.onclick();
  assert.equal(p.fake.created[0].lang, 'tag-uk');
  assert.equal(p.btn.getAttribute('aria-pressed'), 'true');
  p.btn.onclick();
  assert.equal(p.rec.stopped, 1);
  p.fake.created[0].onEnd();
  assert.equal(p.btn.getAttribute('aria-pressed'), 'false');
  for (const e of ['microphone-denied', 'network', 'audio-capture']) { p.btn.onclick(); p.fake.created.at(-1).onError(e); }
  assert.deepEqual(p.log.toast, ['<voiceDenied>', '<voiceOffline>', '<error>']);
  p.btn.onclick();
  p.fake.created.at(-1).onResult({ transcript: 'при', isFinal: false });
  assert.equal(p.log.calls.length, 0);
  p.fake.created.at(-1).onEnd();
  p.fake.rec = null; p.btn.onclick();
  p.fake.rec = { start(){ throw new Error('busy'); } }; p.btn.onclick();
  assert.equal(p.btn.getAttribute('aria-pressed'), 'false');
});

test('voice dom: a refusal says what was heard; a lost hub is said', async () => {
  const p = page({ answers: [{ understood: false, say: 'Not understood', heard: 'бла' }, { understood: false }, null, new Error('offline'), 'x'] });
  await p.say('бла');
  assert.deepEqual(p.log.calls[0], ['post', '/voice', { transcript: 'бла', confidence: 0.8, is_final: true, lang: 'uk' }]);
  await p.say('a'); await p.say('b'); await p.say('c');
  assert.deepEqual(p.log.toast, ['Not understood · «бла»', '<voiceFailed>', '<voiceFailed>', 'offline']);
  // An answer this console does not act on changes nothing and says nothing.
  const q = page({ answers: [{ understood: true, action: 'mystery' }] });
  await q.say('?');
  assert.deepEqual(q.log.toast, []);
});

test('voice dom: "how many are waiting" is said at once', async () => {
  const p = page({ answers: [{ understood: true, action: 'status', open: 3, waiting: 2 }] });
  await p.say('скільки чекає');
  assert.deepEqual(p.log.toast, ['<voiceStatus>']);
  assert.deepEqual(p.fake.spoken, [['<voiceStatus>', 'tag-uk']]);
});

test('voice dom: a question goes to the assistant, and its answer or its failure is said', async () => {
  const p = page({ answers: [{ understood: true, action: 'ask', question: 'what sold best' }, { answer: 'Margherita' }] });
  await p.say('what sold best');
  assert.deepEqual(p.log.calls[1], ['api', '/owner/assist', { question: 'what sold best' }]);
  assert.deepEqual(p.log.toast, ['<voiceAsking>', 'Margherita']);
  const q = page({ answers: [{ understood: true, action: 'ask', question: 'q' }, new Error('assistant off')] });
  await q.say('q');
  assert.deepEqual(q.log.toast, ['<voiceAsking>', 'assistant off']);
  const r = page({ answers: [{ understood: true, action: 'ask', question: 'q' }, { raw: 'off' }] });
  await r.say('q');
  assert.deepEqual(r.log.toast, ['<voiceAsking>', 'off']);
  const w = page({ answers: [{ raw: 'no hub' }] });
  await w.say('q');
  assert.deepEqual(w.log.toast, ['no hub']);
});

const READY = { understood: true, needsConfirmation: true, verb: 'ready', readback: 'Order 4821 ready', token: 'tok' };

test('voice dom: a change is read back in a sheet; Cancel sends nothing', async () => {
  const p = page({ answers: [READY] });
  await p.say('готово 4821');
  assert.equal(p.doc.body.querySelector('p.ui-sheet-text').textContent, 'Order 4821 ready');
  assert.equal(p.doc.body.querySelector('#vReason'), null);
  assert.deepEqual(p.fake.spoken, [['Order 4821 ready?', 'tag-uk']]);
  fire(p.sheetBtn('no'), 'click'); await tick();
  assert.equal(p.log.calls.length, 1);
});

test('voice dom: Confirm sends the token, and the hub\'s instruction goes to the button\'s route', async () => {
  // A phone that can buzz does, once, as the button does.
  const buzz = [];
  Object.defineProperty(globalThis, 'navigator', { value: { vibrate: ms => buzz.push(ms) }, configurable: true, writable: true });
  const p = page({ answers: [READY, { understood: true, verb: 'ready', orderId: 'o1' }, { ok: true }] });
  await p.say('готово 4821');
  fire(p.sheetBtn('yes'), 'click'); await tick();
  assert.deepEqual(buzz, [12]);
  assert.deepEqual(p.log.calls.slice(1), [['post', '/voice', { confirm: 'tok', lang: 'uk' }],
    ['post', '/owner/orders/o1/action', { location_id: 'V1', action: 'ready' }]]);
  assert.deepEqual([p.log.toast, p.log.refreshed], [['<saved>'], 1]);
});

test('voice dom: a refusal is read back with its reason, and the reason typed is the one sent', async () => {
  const p = page({ answers: [{ ...READY, verb: 'reject' }, { understood: true, verb: 'reject', orderId: 'o1' }, {}] });
  await p.say('відхили 4821');
  const input = p.doc.body.querySelector('#vReason');
  assert.equal(input.getAttribute('data-tour'), 'voice.reason');
  assert.ok(p.sheetBtn('yes').className.includes('danger'));
  input.value = 'Closed early'; input.oninput();
  fire(p.sheetBtn('yes'), 'click'); await tick();
  assert.deepEqual(p.log.calls[2], ['post', '/owner/orders/o1/action', { location_id: 'V1', action: 'reject', reason: 'Closed early' }]);
  // Untouched, the reason is the one the field was drawn with.
  const q = page({ answers: [{ ...READY, verb: 'cancel' }, { understood: true, verb: 'cancel', orderId: 'o1' }, {}] });
  await q.say('x');
  fire(q.sheetBtn('yes'), 'click'); await tick();
  assert.equal(q.log.calls[2][2].reason, '<outOfStock>');
});

test('voice dom: a confirmation refused, unplanned, unreachable or failing at its route is said', async () => {
  const cases = [
    [[{ understood: false, say: 'Expired' }], 'Expired'],
    [[null], '<voiceFailed>'],
    [[{ understood: true, verb: 'fly' }], '<voiceFailed>'],
    [[new Error('down')], 'down'],
    [[{ understood: true, verb: 'venue', args: { state: 'busy' } }, new Error('409 conflict')], '409 conflict'],
    // Something thrown that is not an Error is still said, as itself.
    [[{ raw: 'gone' }], 'gone'],
    [[{ understood: true, verb: 'venue', args: { state: 'busy' } }, { raw: 'refused' }], 'refused'],
  ];
  for (const [rest, said] of cases) {
    const p = page({ answers: [READY, ...rest] });
    await p.say('x');
    fire(p.sheetBtn('yes'), 'click'); await tick();
    assert.equal(p.log.toast.at(-1), said);
    assert.equal(p.log.refreshed, 0);
  }
});
