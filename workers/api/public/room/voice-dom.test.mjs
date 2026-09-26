// The waiter's mic, in node: the DOM half of room/voice.js (the header button,
// the recogniser it drives, the read-back sheet and the request a Confirm makes).
// `node --test workers/api/public/room/`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { register } from 'node:module';
import { Document, Element, fire } from '../lib/ui/dom-shim.mjs';

// The shim has no firstElementChild and appendChild only; the header mic is
// lifted out of a template and put BEFORE the language button.
Object.defineProperty(Element.prototype, 'firstElementChild', { get(){ return this.children[0] || null; }, configurable: true });
Element.prototype.insertBefore ||= function (n, ref) {
  if (!ref) return this.appendChild(n);
  if (n.parentNode) n.remove();
  n.parentNode = this;
  this.childNodes.splice(this.childNodes.indexOf(ref), 0, n);
  return n;
};

// THE BROWSER SEAM: /lib/voice.js reads SpeechRecognition once, at import. The
// hook answers it with a module reading globalThis.__vfake, so a test decides
// whether the browser "can hear" and what the recogniser does.
const FAKE = `const F = () => globalThis.__vfake;
  export const supported = () => F().supported;
  export const tagFor = l => 'tag-' + l;
  export const speak = (text, lang) => { F().spoken.push([text, lang]); };
  export function create(o){ F().created.push(o); return F().rec; }`;
register('data:text/javascript,' + encodeURIComponent(`const SRC = ${JSON.stringify(FAKE)};
  export async function resolve(spec, ctx, next){
    return spec.endsWith('/lib/voice.js') ? { url: 'data:text/javascript,' + encodeURIComponent(SRC), shortCircuit: true } : next(spec, ctx);
  }`));
const { mountVoice } = await import('./voice.js');

const tick = () => new Promise(r => setTimeout(r, 0));

/// A fresh page, a fake browser, and the room's context; `answers` are the
/// hub's replies to POST /voice then the place route, in order.
function page({ supported = true, answers = [], write = { landed: true } } = {}) {
  const doc = new Document();
  globalThis.document = doc;
  const rec = { started: 0, stopped: 0, start(){ this.started++; }, stop(){ this.stopped++; } };
  globalThis.__vfake = { supported, spoken: [], created: [], rec };
  const log = { api: [], write: [], toast: [], render: 0, reload: 0 };
  const S = { view: 'room', loc: 'L1', slug: 'dubin', currency: 'ALL' };
  const c = {
    S, t: k => `<${k}>`, lang: () => 'sq', locale: () => 'sq-AL',
    toast: m => log.toast.push(m), render: () => { log.render++; }, reload: async () => { log.reload++; },
    api: async (path, o) => { log.api.push([path, o.body]); const a = answers.shift(); if (a instanceof Error) throw a; return a; },
    write: async (path, body, tag) => { log.write.push([path, body, tag]); if (write instanceof Error) throw write; return write; },
  };
  const header = doc.createElement('div');
  const lang = doc.createElement('button'); lang.id = 'langBtn'; header.appendChild(lang);
  doc.body.appendChild(header);
  mountVoice(c, header, lang);
  const btn = header.querySelector('#voiceBtn');
  /// Tap the mic and hear `transcript` as a final result.
  const say = async transcript => {
    btn.onclick();
    const o = globalThis.__vfake.created.at(-1);
    o.onResult({ transcript, confidence: 0.9, isFinal: true });
    await tick(); await tick();
  };
  const sheetBtn = v => doc.body.querySelector(`[data-ui-close="${v}"]`);
  return { doc, c, S, log, header, btn, rec, say, sheetBtn, fake: globalThis.__vfake };
}

test('voice dom: the mic is drawn once, before the language button, only where speech is heard', () => {
  const p = page();
  assert.equal(p.header.children[0], p.btn);
  assert.equal(p.btn.getAttribute('data-tour'), 'hud.voice');
  mountVoice(p.c, p.header, null);
  assert.equal(p.header.querySelectorAll('#voiceBtn').length, 1);
  const q = page({ supported: false });
  assert.equal(q.btn, null);
  // No "before": the mic goes last.
  globalThis.__vfake.supported = true;
  const h = q.doc.createElement('div');
  mountVoice(q.c, h);
  assert.equal(h.children.length, 1);
});

test('voice dom: a tap listens in the app\'s language, a second tap stops, the end is idle again', () => {
  const p = page();
  p.btn.onclick();
  assert.equal(p.fake.created[0].lang, 'tag-sq');
  assert.equal(p.btn.getAttribute('aria-pressed'), 'true');
  assert.equal(p.rec.started, 1);
  p.btn.onclick();
  assert.equal(p.rec.stopped, 1);
  p.fake.created[0].onEnd();
  assert.equal(p.btn.getAttribute('aria-pressed'), 'false');
  // An interim result is never posted.
  p.btn.onclick();
  p.fake.created[1].onResult({ transcript: 'tav', isFinal: false });
  assert.equal(p.log.api.length, 0);
});

test('voice dom: no recogniser, or one that will not start, leaves the mic idle', () => {
  const p = page();
  p.fake.rec = null;
  p.btn.onclick();
  assert.equal(p.btn.getAttribute('aria-pressed'), 'false');
  p.fake.rec = { start(){ throw new Error('busy'); } };
  p.btn.onclick();
  assert.equal(p.btn.getAttribute('aria-pressed'), 'false');
});

test('voice dom: a denied mic, a lost network and any other error are each said', () => {
  const p = page();
  for (const e of ['microphone-denied', 'network', 'audio-capture']) { p.btn.onclick(); p.fake.created.at(-1).onError(e); }
  assert.deepEqual(p.log.toast, ['<voiceDenied>', '<voiceOffline>', '<error>']);
  assert.equal(p.btn.getAttribute('aria-pressed'), 'false');
});

test('voice dom: words go to the hub with the round being built; a refusal says what was heard', async () => {
  const p = page({ answers: [{ understood: false, say: 'No such table', heard: 'tavolina 99' }, { understood: false }, null, new Error('offline')] });
  Object.assign(p.S, { view: 'open', openTable: '5', basket: { marg: 1 } });
  await p.say('tavolina 99');
  assert.deepEqual(p.log.api[0], ['/voice', { transcript: 'tavolina 99', confidence: 0.9, is_final: true, lang: 'sq', draft: { table: '5', items: [{ product_id: 'marg', quantity: 1 }] } }]);
  await p.say('x'); await p.say('y'); await p.say('z');
  assert.deepEqual(p.log.toast, ['No such table · «tavolina 99»', '<error>', '<error>', 'offline']);
  // A failure with no message is still said.
  const q = page({ answers: [Object.assign(new Error(''), { message: '' })] });
  await q.say('x');
  assert.deepEqual(q.log.toast, ['<error>']);
});

test('voice dom: an answer that runs now changes this phone, is said, and redraws', async () => {
  const p = page({ answers: [{ understood: true, action: 'open', table: '5', guests: 2 }, { understood: true, action: 'nothing' }] });
  await p.say('hap tavolinën 5 për 2');
  assert.equal(p.S.openTable, '5');
  assert.deepEqual(p.log.toast, ['<voiceOpened> · <voiceGuests>']);
  assert.deepEqual(p.fake.spoken, [['<voiceOpened> · <voiceGuests>', 'tag-sq']]);
  await p.say('?');
  assert.equal(p.log.toast.length, 1);
  assert.equal(p.log.render, 2);
});

const PAY = { understood: true, needsConfirmation: true, verb: 'pay', amount: 1500, readback: 'Table 5 paid cash', token: 'tok' };

test('voice dom: a write is read back in a sheet; Cancel sends nothing', async () => {
  const p = page({ answers: [PAY] });
  await p.say('tavolina 5 paguar cash');
  const text = p.doc.body.querySelector('p.ui-sheet-text').textContent;
  // (The shim reads a "." inside [attr="v"] as a class, so the anchor is checked as an attribute.)
  assert.equal(p.doc.body.querySelector("p.ui-sheet-text").getAttribute("data-tour"), "voice.readback");
  assert.match(text, /^Table 5 paid cash · /);
  assert.equal(p.fake.spoken[0][0], text + '?');
  fire(p.sheetBtn('no'), 'click'); await tick();
  assert.equal(p.log.api.length, 1);
  assert.equal(p.log.write.length, 0);
});

test('voice dom: Confirm sends the token, and the hub\'s own instruction goes through the outbox', async () => {
  const done = { understood: true, verb: 'pay', args: { orderId: 'o/1', amount: 1500, method: 'cash', baseSeq: 7 } };
  for (const [write, said] of [[{ landed: true }, '<saved>'], [{ queued: true }, '<queuedSaved>'], [{}, '<queueNoStore>'], [new Error('409'), '409'], [new Error(''), '<error>']]) {
    const p = page({ answers: [PAY, done], write });
    await p.say('paguar');
    fire(p.sheetBtn('yes'), 'click'); await tick(); await tick();
    assert.deepEqual(p.log.api[1], ['/voice', { confirm: 'tok', lang: 'sq' }]);
    assert.deepEqual(p.log.write[0], ['/staff/orders/o%2F1/pay', { location_id: 'L1', amount: 1500, method: 'cash', base_seq: 7 }, 'pay:o/1']);
    assert.equal(p.log.toast.at(-1), said);
    assert.equal(p.log.reload, write instanceof Error ? 0 : 1);
  }
});

test('voice dom: a confirmed round is placed like the Open view places it, then shown', async () => {
  const done = { understood: true, verb: 'place', args: { table: '5', items: [{ product_id: 'marg', quantity: 2 }] } };
  const p = page({ answers: [{ ...PAY, verb: 'place', readback: 'Send the round' }, done, { id: 'r1', sitting_id: 's1' }] });
  Object.assign(p.S, { view: 'open', openTable: '5', basket: { marg: 2 } });
  await p.say('dërgo');
  fire(p.sheetBtn('yes'), 'click'); await tick(); await tick(); await tick();
  assert.equal(p.log.api[2][0], '/public/locations/dubin/orders');
  assert.deepEqual([p.S.view, p.S.sittingId, p.S.roundId, p.S.openTable], ['round', 's1', 'r1', '']);
  assert.equal(p.log.toast.at(-1), '<saved>');
  // A placement answered with nothing still clears the draft, and stays put.
  const q = page({ answers: [{ ...PAY, verb: 'place' }, done, null] });
  await q.say('dërgo');
  fire(q.sheetBtn('yes'), 'click'); await tick(); await tick(); await tick();
  assert.equal(q.S.view, 'room');
});

test('voice dom: a confirmation the hub refuses, cannot plan, or cannot reach is said', async () => {
  const cases = [
    [{ understood: false, say: 'Expired' }, 'Expired'],
    [{ understood: false }, '<error>'],
    [{ understood: true, verb: 'fly', args: {} }, '<error>'],
    [new Error('down'), 'down'],
    [new Error(''), '<error>'],
  ];
  for (const [answer, said] of cases) {
    const p = page({ answers: [PAY, answer] });
    await p.say('paguar');
    fire(p.sheetBtn('yes'), 'click'); await tick(); await tick();
    assert.equal(p.log.toast.at(-1), said);
    assert.equal(p.log.write.length, 0);
  }
});
