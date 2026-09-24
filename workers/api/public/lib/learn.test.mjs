// /lib/learn.js in node: the pure parts against the real compiled lessons, the
// engine against a recording stand-in for createGuide (the guide draws; what
// is checked here is WHICH tour it is handed, at WHICH step, and what progress
// is kept), and the list's markup and wiring through the ui tests' DOM shim.
// `node --test workers/api/public/lib/learn.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parseHash, watchUrl, pick, toTour, listHtml, loadLessons, createLearn, ROLE_APP, WORDS, LESSONS_URL } from './learn.js';
import { render, fire, injected, XSS, Document } from './ui/dom-shim.mjs';

const ALL = JSON.parse(readFileSync(new URL('../learn/lessons.json', import.meta.url), 'utf8')).lessons;

/// A storage that behaves like localStorage, and one that throws like a
/// private window with storage blocked.
function mem(){ const m = new Map(); return { m, getItem: k => (m.has(k) ? m.get(k) : null), setItem: (k, v) => { m.set(k, String(v)); } }; }
const blocked = { getItem(){ throw new Error('blocked'); }, setItem(){ throw new Error('blocked'); } };

/// Records every guide it makes: its options, whether init ran, where start went.
function fakeGuides(){
  const made = [];
  const createGuide = opts => { const g = { opts, inits: 0, starts: [], init(){ g.inits++; }, start(i){ g.starts.push(i); } }; made.push(g); return g; };
  return { made, createGuide };
}

test('parseHash: a lesson link, with and without a 1-based step; anything else is not one', () => {
  assert.deepEqual(parseHash('#learn=C3'), { id: 'C3', step: null });
  assert.deepEqual(parseHash('#learn=O16a/2'), { id: 'O16a', step: 1 });
  assert.deepEqual(parseHash('#tab=orders&learn=W7/1'), { id: 'W7', step: 0 });
  assert.deepEqual(parseHash('#learn=W7/0'), { id: 'W7', step: 0 });
  for (const h of ['', '#', '#orders', '#learn=', 'learn=C3', null, undefined, '#relearn=C3']) assert.equal(parseHash(h), null, String(h));
});

test('watchUrl: the gated wiki route, every part escaped', () => {
  assert.equal(watchUrl('sq', 'courier', 'C3'), '/wiki/#/sq/courier/C3');
  assert.equal(watchUrl('uk', 'a/b', 'x y'), '/wiki/#/uk/a%2Fb/x%20y');
});

test('pick: the reader language, then English, then whatever there is; never undefined', () => {
  assert.equal(pick({ sq: 'a', en: 'b', uk: 'c' }, 'uk'), 'c');
  assert.equal(pick({ sq: 'a', en: 'b' }, 'uk'), 'b');
  assert.equal(pick({ sq: 'a' }, 'uk'), 'a');
  assert.equal(pick({}, 'uk'), '');
  assert.equal(pick(null, 'uk'), '');
});

test('toTour: one soft, hint-less row per step, anchored on the data-tour selector, in the language asked', () => {
  const c2 = ALL.find(l => l.id === 'C2');
  const rows = toTour(c2, 'uk');
  assert.equal(rows.length, c2.steps.length);
  assert.deepEqual(rows[0], { key: '1', at: '[data-tour="shift.open"]', soft: true, hint: false,
    title: c2.steps[0].title.uk, body: c2.steps[0].caption.uk });
  const c1 = ALL.find(l => l.id === 'C1');
  assert.equal(toTour(c1, 'en')[0].at, undefined, 'the welcome step points at nothing and is centred');
});

test('the compiled bundle: every role the engine knows has lessons, every lesson a known app', () => {
  for (const role of Object.keys(ROLE_APP)) assert.ok(ALL.some(l => l.role === role), role);
  for (const l of ALL) assert.equal(l.app, ROLE_APP[l.role], l.id);
});

test('listHtml: title, goal, step count, state badge, "writes" mark and the watch link, all escaped', () => {
  const lessons = ALL.filter(l => l.role === 'courier');
  const html = listHtml(lessons, { C1: { state: 'done' }, C2: { state: 'paused' }, C3: { state: 'bogus' } }, 'en',
    { steps: n => `${n} st`, writes: 'REAL' });
  const { root } = render(html);
  const items = root.querySelectorAll('.lr-item');
  assert.equal(items.length, lessons.length);
  assert.deepEqual(items.map(i => i.getAttribute('data-state')).slice(0, 4), ['done', 'paused', 'new', 'new']);
  assert.deepEqual(root.querySelectorAll('[data-learn]').map(b => b.getAttribute('data-learn')), lessons.map(l => l.id));
  assert.equal(root.querySelector('.lr-watch').getAttribute('href'), '/wiki/#/en/courier/C1');
  assert.ok(root.querySelector('[data-learn="C2"]').textContent.includes('REAL'), 'C2 writes');
  assert.ok(!root.querySelector('[data-learn="C1"]').textContent.includes('REAL'), 'C1 does not write');
  assert.ok(root.querySelector('[data-learn="C1"]').textContent.includes('5 st'));
  assert.ok(root.querySelector('[data-learn="C1"] .ui-badge').textContent.includes(WORDS.done));
});

test('listHtml: an empty list says so; hostile lesson text stays text', () => {
  assert.match(listHtml([], {}, 'en', { empty: 'nothing' }), /lr-empty">nothing</);
  const evil = [{ id: 'X1', role: 'courier', writes: false, steps: [{}], title: { en: XSS }, goal: { en: XSS } }];
  assert.deepEqual(injected(listHtml(evil, {}, 'en')), []);
});

test('loadLessons: the lessons array; [] on a 404, a network error, bad JSON or the wrong shape', async () => {
  const seen = [];
  const ok = async (url, o) => { seen.push([url, o]); return { ok: true, json: async () => ({ lessons: [{ id: 'A1' }] }) }; };
  assert.deepEqual(await loadLessons(ok), [{ id: 'A1' }]);
  assert.deepEqual(seen[0], [LESSONS_URL, { credentials: 'same-origin' }]);
  assert.deepEqual(await loadLessons(async () => ({ ok: false })), []);
  assert.deepEqual(await loadLessons(async () => { throw new Error('offline'); }), []);
  assert.deepEqual(await loadLessons(async () => ({ ok: true, json: async () => { throw new SyntaxError('x'); } })), []);
  assert.deepEqual(await loadLessons(async () => ({ ok: true, json: async () => ({ lessons: 'no' }) })), []);
});

test('createLearn: an unknown role is refused; the role filter and the order hold', () => {
  assert.throws(() => createLearn({ role: 'chef', lessons: ALL }), /unknown role chef/);
  const L = createLearn({ role: 'waiter', lessons: ALL, storage: mem() });
  assert.equal(L.app, 'room');
  assert.deepEqual(L.lessons.map(l => l.id), ['W1', 'W2', 'W3', 'W4', 'W5', 'W6', 'W7', 'W8', 'W9', 'W10']);
});

test('open: builds the lesson as a tour with its own key, starts at the step asked, clamped', () => {
  const G = fakeGuides(), st = mem(), toasts = [];
  const L = createLearn({ role: 'courier', lessons: ALL, lang: () => 'sq', createGuide: G.createGuide, storage: st,
    toast: m => toasts.push(m), guideWords: { next: 'N' } });
  assert.equal(L.open('C3', 2), true);
  const g = G.made[0];
  assert.equal(g.opts.key, 'courier_C3');
  assert.deepEqual(g.opts.tour, ['1', '2', '3', '4', '5']);
  assert.equal(g.opts.help['1'].at, '[data-tour="run.eta"]');
  assert.equal(g.opts.help['1'].title, ALL.find(l => l.id === 'C3').steps[0].title.sq);
  assert.deepEqual(g.opts.words, { next: 'N' });
  g.opts.toast('hi'); assert.deepEqual(toasts, ['hi']);
  assert.equal(g.inits, 1);
  assert.deepEqual(g.starts, [2]);
  L.open('C3', 99); L.open('C3', -4);
  assert.deepEqual(g.starts, [2, 4, 0], 'past the end -> last step; negative -> first');
  assert.equal(G.made.length, 1, 'the same lesson in the same language reuses its guide');
  assert.equal(L.open('Z9'), false, 'an unknown lesson opens nothing');
  assert.equal(L.open('W1'), false, "another role's lesson is not this app's");
});

test('open: a new language builds a new guide, so the card reads the new words', () => {
  const G = fakeGuides();
  let lang = 'sq';
  const L = createLearn({ role: 'courier', lessons: ALL, lang: () => lang, createGuide: G.createGuide, storage: mem() });
  L.open('C2', 0); lang = 'uk'; L.open('C2', 0);
  assert.equal(G.made.length, 2);
  assert.equal(G.made[1].opts.help['1'].body, ALL.find(l => l.id === 'C2').steps[0].caption.uk);
});

test('progress: the guide ending records done or paused; a paused lesson resumes at its saved step', () => {
  const G = fakeGuides(), st = mem();
  const L = createLearn({ role: 'courier', lessons: ALL, createGuide: G.createGuide, storage: st });
  assert.deepEqual(L.progress(), {});
  L.open('C4');
  G.made[0].opts.onEnd('paused');
  // What guide.js itself writes when a tour is paused at step 3 (0-based).
  st.setItem('dw_guide_courier_C4', JSON.stringify({ state: 'paused', step: 3 }));
  assert.equal(L.progress().C4.state, 'paused');
  assert.ok(Number.isFinite(L.progress().C4.at));
  L.open('C4');
  assert.deepEqual(G.made[0].starts, [0, 3], 'no step asked: resumes where it paused');
  G.made[0].opts.onEnd('done');
  L.open('C4');
  assert.deepEqual(G.made[0].starts, [0, 3, 0], 'done: starts over');
  L.mark('C1', 'done');
  assert.deepEqual(Object.keys(JSON.parse(st.m.get('dw_learn_courier'))).sort(), ['C1', 'C4']);
});

test('progress: blocked or corrupt storage is an empty list, never an exception', () => {
  const G = fakeGuides();
  const L = createLearn({ role: 'courier', lessons: ALL, createGuide: G.createGuide, storage: blocked });
  assert.deepEqual(L.progress(), {});
  L.mark('C2', 'done');                       // swallowed
  assert.equal(L.open('C2'), true);
  const st = mem(); st.setItem('dw_learn_courier', '[1,2]'); st.setItem('dw_guide_courier_C2', '{not json');
  const L2 = createLearn({ role: 'courier', lessons: ALL, createGuide: G.createGuide, storage: st });
  assert.deepEqual(L2.progress(), {}, 'an array is not a progress map');
  st.setItem('dw_learn_courier', JSON.stringify({ C2: { state: 'paused' } }));
  L2.open('C2');
  assert.deepEqual(G.made.at(-1).starts, [0], 'paused but the guide key is unreadable: step 0');
  const L3 = createLearn({ role: 'courier', lessons: ALL, createGuide: G.createGuide, storage: null });
  assert.deepEqual(L3.progress(), {});
  L3.mark('C3', 'done');
});

test('deepLink: #learn=<id>[/<step>] opens that lesson; anything else opens nothing', () => {
  const G = fakeGuides();
  const L = createLearn({ role: 'waiter', lessons: ALL, createGuide: G.createGuide, storage: mem() });
  assert.equal(L.deepLink('#learn=W7/3'), true);
  assert.equal(G.made[0].opts.key, 'room_W7');
  assert.deepEqual(G.made[0].starts, [2]);
  assert.equal(L.deepLink('#learn=C2'), false);
  assert.equal(L.deepLink('#orders'), false);
  assert.equal(L.deepLink(), false, 'no location in node: nothing to open');
  globalThis.location = { hash: '#learn=W2' };
  try { assert.equal(L.deepLink(), true, 'with no argument it reads location.hash'); } finally { delete globalThis.location; }
  assert.equal(G.made.at(-1).opts.key, 'room_W2');
  G.made.at(-1).opts.toast('no toast given: a quiet no-op');
});

test('renderList + bindList: the reader language, one <link> to learn.css, a click opens that lesson after onPick', () => {
  const G = fakeGuides(), st = mem();
  st.setItem('dw_learn_owner', JSON.stringify({ O1a: { state: 'done' } }));
  const doc = new Document();
  doc.head = doc.createElement('head');
  const L = createLearn({ role: 'owner', lessons: ALL, lang: () => 'uk', createGuide: G.createGuide, storage: st, doc,
    words: { done: 'ГОТОВО' } });
  const { root } = render(L.renderList());
  assert.ok(root.querySelector('[data-learn="O1a"]').textContent.includes(ALL.find(l => l.id === 'O1a').title.uk));
  assert.ok(root.querySelector('[data-learn="O1a"]').textContent.includes('ГОТОВО'));
  assert.equal(root.querySelector('.lr-watch').getAttribute('href'), '/wiki/#/uk/owner/O1a');
  const picked = [];
  L.bindList(root, id => picked.push([id, G.made.length]));
  L.bindList(root);                                   // a second bind adds no second stylesheet
  // (the shim reads '.css' in a selector as a class, so the links are counted by hand)
  assert.deepEqual(doc.head.querySelectorAll('link').map(l => [l.getAttribute('rel'), l.getAttribute('href')]), [['stylesheet', '/lib/learn.css']]);
  fire(root.querySelector('[data-learn="O5a"]'), 'click');
  assert.deepEqual(picked[0], ['O5a', 0], 'onPick runs BEFORE the lesson opens (a panel closes first)');
  assert.equal(G.made.at(-1).opts.key, 'owner_O5a');
  const bare = createLearn({ role: 'owner', lessons: ALL, createGuide: G.createGuide, storage: st, doc: null });
  bare.bindList(render('<p></p>').root);              // no document: no stylesheet, no throw
});
