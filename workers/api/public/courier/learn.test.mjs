// The courier's lessons (C1..C6): the anchors, the first-run tour that IS C1,
// the Learn entry, its words and the offline shell.
// `node --test workers/api/public/courier/learn.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as screens from './screens.js';
import { useTranslator } from '../lib/ui/core.js';
import { render } from '../lib/ui/dom-shim.mjs';

const HERE = new URL('./', import.meta.url);
const REPO = new URL('../../../../', import.meta.url);
const read = f => readFileSync(new URL(f, HERE), 'utf8');
const LESSONS = JSON.parse(readFileSync(new URL('../learn/lessons.json', HERE), 'utf8')).lessons.filter(l => l.role === 'courier');
const SOURCES = ['app.js', 'screens.js', 'index.html'].map(f => [f, read(f)]);
// i18n.js imports '/store/storage.js' by its site path, which node cannot
// resolve; the table is loaded with that one import swapped for inert stubs.
const I18N = read('i18n.js').replace(/^import \{ safeGet, safeSet \} from '\/store\/storage\.js';$/m,
  'const safeGet = () => null, safeSet = () => {};');
const { T } = await import('data:text/javascript;base64,' + Buffer.from(I18N).toString('base64'));
useTranslator(k => `«${k}»`);

const listed = readFileSync(new URL('docs/learn/anchors-courier.txt', REPO), 'utf8')
  .split('\n').filter(l => l.trim() && !l.startsWith('#')).map(l => l.trim().split(/\s+/));

test('anchors: every data-tour the courier writes is listed with its file:line, and every listed one is there', () => {
  // A WRITER, not a reader: `[data-tour="…"]` in a selector is app.js pointing at one.
  const W = /(?<!\[)data-tour="([a-z][A-Za-z]*\.[A-Za-z]+)"|\btour: ?'([a-z][A-Za-z]*\.[A-Za-z]+)'/g;
  const inTree = new Map();
  for (const [f, s] of SOURCES) s.split('\n').forEach((line, i) => {
    for (const m of line.matchAll(W)) { const id = m[1] || m[2]; if (!inTree.has(id)) inTree.set(id, `workers/api/public/courier/${f}:${i + 1}`); }
  });
  const ids = listed.map(([id]) => id);
  assert.equal(new Set(ids).size, ids.length, 'an anchor is listed twice');
  for (const [id, at] of listed) {
    assert.equal(inTree.get(id), at, `${id}: listed at ${at}, written at ${inTree.get(id)}`);
  }
  for (const id of inTree.keys()) assert.ok(ids.includes(id), `${id} is in the tree and not listed`);
  assert.ok(ids.length >= 30, `only ${ids.length} anchors`);
});

test('lessons: C1..C6, and every step anchor is a listed courier anchor (none pending)', () => {
  assert.deepEqual(LESSONS.map(l => l.id), ['C1', 'C2', 'C3', 'C4', 'C5', 'C6']);
  const ids = new Set(listed.map(([id]) => id));
  for (const l of LESSONS) for (const s of l.steps) {
    if (!s.anchor) continue;
    assert.ok(ids.has(s.anchor), `${l.id} step ${s.n}: ${s.anchor} is not a courier anchor`);
    assert.equal(s.pending, false, `${l.id} step ${s.n}`);
  }
  const named = new Set(LESSONS.flatMap(l => l.steps.map(s => s.anchor)));
  for (const id of ids) assert.ok(named.has(id), `${id} is named by no courier lesson`);
});

test('C1 IS the first-run tour: same five keys in the same order, the same words in all three languages', () => {
  const c1 = LESSONS.find(l => l.id === 'C1');
  const tour = /const TOUR = \[([^\]]*)\]/.exec(read('app.js'))[1].match(/'(\w+)'/g).map(x => x.slice(1, -1));
  assert.deepEqual(c1.steps.map(s => s.key), tour);
  const KEY = { welcome: 'hWelcome', shift: 'hShift', sheet: 'hSheet', mic: 'hMic', help: 'hHelp' };
  for (const lang of ['sq', 'en', 'uk']) for (const s of c1.steps) {
    assert.equal(s.title[lang], T[lang][KEY[s.key] + 'T'], `${lang} ${s.key} title`);
    assert.equal(s.caption[lang], T[lang][KEY[s.key]], `${lang} ${s.key} caption`);
  }
});

test('the first-run tour points at the same anchors as C1, keeps dw_guide_courier, and ticks C1 when it ends', () => {
  const app = read('app.js');
  const c1 = LESSONS.find(l => l.id === 'C1');
  for (const s of c1.steps.filter(x => x.at)) {
    assert.ok(new RegExp(`${s.key}: \\{ at:'\\[data-tour="${s.anchor.replace('.', '\\.')}"\\]'`).test(app), `HELP.${s.key} is not on ${s.at}`);
  }
  assert.match(app, /key:'courier', help: HELP\(\), tour: TOUR/);
  assert.match(app, /onEnd: state => getLearn\(\)\.then\(l => l\?\.mark\('C1', state\)\)/);
  assert.match(app, /tour:'help\.open'/);
  assert.doesNotMatch(app, /at:'#(shiftTag|sheet|mic|askBox)'/, 'an id-anchored step is left');
});

test('the Learn entry: only where the courier stands still (offline and waiting), never beside a run', () => {
  const learnBtn = html => render(html).root.querySelector('#learn');
  for (const h of [screens.offShift(), screens.waiting()]) {
    const b = learnBtn(h);
    assert.ok(b, 'no #learn');
    assert.equal(b.getAttribute('data-tour'), 'panel.learn');
    assert.ok(!b.classList.contains('ui-btn--primary'), 'a second main action');
  }
  const ctx = { t: k => k, money: n => `${n}`, lang: 'en', langs: ['en'] };
  const o = { id: 'o1', total: 1, payment: 'cash', items: 1, status: 'IN_DELIVERY', address: { line: 'x' }, contact: { phone: '1' } };
  for (const h of [screens.active(o, {}, ctx), screens.cash(o, '', ctx), screens.offer({ ...o, status: 'READY' }, 30, ctx),
                   screens.pickList([o], 'o1', () => false, ctx)]) assert.equal(learnBtn(h), null);
  const app = read('app.js');
  assert.equal((app.match(/\$\('#learn'\)\.onclick = openLearn;/g) || []).length, 2, 'bound on both screens');
});

test('words: every Learn key in all three languages; the step count carries {n}', () => {
  const KEYS = ['learn', 'learnNew', 'learnDone', 'learnPaused', 'learnWatch', 'learnWrites', 'learnSteps', 'learnEmpty', 'learnOffline'];
  for (const lang of ['sq', 'en', 'uk']) for (const k of KEYS) {
    assert.equal(typeof T[lang][k], 'string', `${lang}.${k}`);
    assert.ok(T[lang][k].trim(), `${lang}.${k} empty`);
  }
  for (const lang of ['sq', 'en', 'uk']) assert.match(T[lang].learnSteps, /\{n\}/);
  const used = [...read('app.js').matchAll(/t\('(learn\w*)'/g)].map(m => m[1]);
  for (const k of used) assert.ok(KEYS.includes(k), `${k} used but not tested`);
});

test('offline shell: the lessons engine and both sheets are cached, under a new cache name', () => {
  const sw = read('sw.js');
  for (const f of ['/lib/learn.js', '/lib/learn.css', '/lib/guide.css']) assert.match(sw, new RegExp(`^  '${f.replace(/\./g, '\\.')}',$`, 'm'), f);
  assert.match(sw, /const SHELL_CACHE = 'dowiz-courier-shell-2026-09-24b';/);
  assert.match(read('app.js'), /^import \{ createLearn, loadLessons \} from '\/lib\/learn\.js';$/m);
});
