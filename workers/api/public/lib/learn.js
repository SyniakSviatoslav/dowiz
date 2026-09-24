// dowiz learn -- the LESSONS on top of the guide. One module for the owner
// console, the waiter's room, the courier app and the storefront.
//
// WHY ON guide.js AND NOT BESIDE IT. The guide already draws a tour: the ring,
// the card, skipping or centring a step, keyboard and focus, the remount after
// every re-render, reduced motion. A lesson is a tour with a name, a list it
// sits in and a place in the learner's progress -- so this module owns exactly
// that and hands every drawing to `createGuide` (docs/design/BLUEPRINT-
// LEARNING-VIDEOS-WIKI-2026-09-24.md §5.2-5.3).
//
// WHERE THE WORDS COME FROM. docs/learn/lessons/<role>/<id>.yaml, compiled by
// tools/learn/build-lessons.mjs into /learn/lessons.json. The same captions are
// the video's subtitles and burned-in step titles, so the tour card, the wiki
// and the video read one sentence and none of them can drift.
//
// PROGRESS lives on the device: the guide keeps `dw_guide_<app>_<id>` (state and
// step, as for every tour) and this module one index `dw_learn_<app>` =
// { [id]: { state: 'done'|'paused', at } }. Both are wrapped: a private window
// or blocked storage gives a list with nothing ticked, never an exception.
//
// CSP: no inline style, no eval, no inline handler. The list is markup with
// `data-learn` hooks bound by `bindList`; the sheet is /lib/learn.css, added as
// a same-origin <link> exactly as guide.js adds its own.
//
// A LESSON NEVER ACTS. The tour highlights and explains; the learner presses
// the control. A step marked `writes` changes real data when the learner does
// it, and the list says so beside the lesson.
import { esc, badge, icon, row } from './ui/index.js';

export const LANGS = ['sq', 'en', 'uk'];
/// The app a role's lessons run in, which is also the storage suffix.
export const ROLE_APP = { owner: 'owner', waiter: 'room', courier: 'courier', guest: 'store' };
const CSS_HREF = '/lib/learn.css';
export const LESSONS_URL = '/learn/lessons.json';

/// The list's own words. A caller passes its language's; these are the defaults.
export const WORDS = { title: 'Lessons', new: 'New', done: 'Done', paused: 'Paused', watch: 'Watch the video',
  writes: 'Changes real data', steps: n => `${n} steps`, empty: 'No lessons here yet', offline: 'Lessons need a connection' };

/// `#learn=C3` opens lesson C3; `#learn=C3/2` opens it at step 2 (1-based, as
/// the card counts). Anything else is not a lesson link.
export function parseHash(hash){
  const m = /(?:^#|[#&])learn=([A-Za-z0-9]+)(?:\/(\d+))?/.exec(String(hash || ''));
  if (!m) return null;
  return { id: m[1], step: m[2] ? Math.max(0, Number(m[2]) - 1) : null };
}

/// The wiki page of a lesson (gated behind the same sign-in; the wiki builds it).
export const watchUrl = (lang, role, id) =>
  `/wiki/#/${encodeURIComponent(lang)}/${encodeURIComponent(role)}/${encodeURIComponent(id)}`;

/// A lesson's text in `lang`, falling back to English, then to the first there is.
export function pick(map, lang){
  if (!map || typeof map !== 'object') return '';
  return map[lang] ?? map.en ?? Object.values(map)[0] ?? '';
}

function safe(storage){
  return {
    get(k){ try { return JSON.parse(storage.getItem(k) || 'null'); } catch { return null; } },
    set(k, v){ try { storage.setItem(k, JSON.stringify(v)); return true; } catch { return false; } },
  };
}

/// The rows the guide draws: every step is `soft` (explained in the middle when
/// its control is on another screen) and none is a "?" hint.
export function toTour(lesson, lang){
  return lesson.steps.map(s => ({ key: s.key, at: s.at || undefined, soft: true, hint: false,
    title: pick(s.title, lang), body: pick(s.caption, lang) }));
}

/// The lesson list as markup. `progress` is `{ [id]: { state } }`.
export function listHtml(lessons, progress, lang, words = {}){
  const W = { ...WORDS, ...words };
  if (!lessons.length) return `<p class="lr-empty">${esc(W.empty)}</p>`;
  const tone = { done: 'success', paused: 'warning', new: 'neutral' };
  return `<ul class="lr-list" aria-label="${esc(W.title)}">${lessons.map(l => {
    const st = progress[l.id]?.state === 'done' || progress[l.id]?.state === 'paused' ? progress[l.id].state : 'new';
    return `<li class="lr-item" data-state="${st}">
      ${row({ act: true, cls: 'lr-open', data: { learn: l.id },
              leading: `<span class="lr-id">${esc(l.id)}</span>`,
              title: pick(l.title, lang),
              sub: [pick(l.goal, lang), W.steps(l.steps.length), l.writes ? W.writes : ''].filter(Boolean).join(' · '),
              trailing: badge({ label: W[st], tone: tone[st] }) })}
      <a class="lr-watch" href="${esc(watchUrl(lang, l.role, l.id))}">${icon('player-play')}<span>${esc(W.watch)}</span></a>
    </li>`;
  }).join('')}</ul>`;
}

/// Fetch the compiled lessons. Returns [] on any failure (offline, 404, bad
/// JSON); the caller shows `words.offline` then.
export async function loadLessons(fetchFn = globalThis.fetch, url = LESSONS_URL){
  try {
    const r = await fetchFn(url, { credentials: 'same-origin' });
    if (!r.ok) return [];
    const j = await r.json();
    return Array.isArray(j?.lessons) ? j.lessons : [];
  } catch { return []; }
}

function ensureCss(doc){
  if (!doc?.head || [...doc.head.querySelectorAll('link')].some(l => l.getAttribute('href') === CSS_HREF)) return;
  const l = doc.createElement('link');
  l.setAttribute('rel', 'stylesheet'); l.setAttribute('href', CSS_HREF);
  doc.head.appendChild(l);
}

/// Lessons for one role in one app.
///   role        owner | waiter | courier | guest
///   lessons     the compiled list (all roles; filtered here)
///   lang        () => the reader's language, read at every call
///   createGuide the guide factory (/lib/guide.js), injected so node can test
///   words       the list's words; guideWords: the tour card's
///   storage     localStorage by default
export function createLearn({ role, lessons = [], lang = () => 'en', createGuide, toast = () => {},
                              words = {}, guideWords = {}, storage = globalThis.localStorage, doc = globalThis.document } = {}){
  const app = ROLE_APP[role];
  if (!app) throw new Error(`learn: unknown role ${role}`);
  const mine = lessons.filter(l => l.role === role).sort((a, b) => a.order - b.order);   // order: the build refuses a lesson without one
  const store = safe(storage || { getItem: () => null, setItem: () => { throw new Error('no storage'); } });
  const IDX = `dw_learn_${app}`;
  const guides = new Map();

  const progress = () => { const v = store.get(IDX); return v && typeof v === 'object' && !Array.isArray(v) ? v : {}; };
  function mark(id, state){
    const p = progress();
    p[id] = { state, at: Date.now() };
    store.set(IDX, p);
  }
  const find = id => mine.find(l => l.id === id) || null;

  function guideFor(l){
    const g = guides.get(l.id);
    if (g && g.lang === lang()) return g.guide;
    const rows = toTour(l, lang());
    const guide = createGuide({ key: `${app}_${l.id}`, help: Object.fromEntries(rows.map(r => [r.key, r])),
      tour: rows.map(r => r.key), toast, words: guideWords, onEnd: state => mark(l.id, state) });
    guide.init();
    guides.set(l.id, { lang: lang(), guide });
    return guide;
  }

  /// Open a lesson; `step` is 0-based. With no step, a paused lesson resumes.
  function open(id, step = null){
    const l = find(id);
    if (!l) return false;
    let at = step;
    if (at == null) {
      const g = store.get(`dw_guide_${app}_${l.id}`);
      at = progress()[l.id]?.state === 'paused' && Number.isInteger(g?.step) ? g.step : 0;
    }
    guideFor(l).start(Math.min(Math.max(0, at), l.steps.length - 1));
    return true;
  }

  /// Open the lesson a `#learn=<id>` link names. False when there is none.
  function deepLink(hash = globalThis.location?.hash){
    const h = parseHash(hash);
    return h ? open(h.id, h.step) : false;
  }

  /// The list in the reader's language, and its click wiring. `onPick` runs
  /// before the lesson opens (a panel closes itself so the controls are there).
  const renderList = () => listHtml(mine, progress(), lang(), words);
  function bindList(root, onPick = () => {}){
    ensureCss(doc);
    for (const b of root.querySelectorAll('[data-learn]')) {
      b.addEventListener('click', () => { onPick(b.dataset.learn); open(b.dataset.learn); });
    }
  }

  return { lessons: mine, app, progress, mark, open, deepLink, renderList, bindList };
}
