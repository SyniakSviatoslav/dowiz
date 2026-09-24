// /wiki/ -- the lesson library: every lesson of every track, its video with three caption
// tracks, its chapters, a search, and a link back into the app at the lesson.
//
// Two static inputs, both built by tools/learn: /learn/lessons.json (every lesson's words, the
// one source the in-app tour reads too) and /learn/media/manifest.json (which cuts are
// published). CSP-clean: no inline script or style, text only through textContent.
import { T, LANGS, ROLES, parseRoute, href, appLink, pickLang, search, tracks, clock } from './wiki-core.js';

const $ = s => document.querySelector(s);
const main = $('#main');
const S = { lessons: [], media: {}, q: '' };

function h(tag, attrs = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v == null || v === false) continue;
    if (k === 'on') for (const [ev, fn] of Object.entries(v)) e.addEventListener(ev, fn);
    else if (k === 'text') e.textContent = v;
    else e.setAttribute(k, v === true ? '' : v);
  }
  for (const c of kids.flat()) if (c != null) e.append(c.nodeType ? c : document.createTextNode(String(c)));
  return e;
}
const safeGet = k => { try { return localStorage.getItem(k); } catch { return null; } };
const safeSet = (k, v) => { try { localStorage.setItem(k, v); } catch {} };

function langBar(r) {
  return h('nav', { class: 'w-langs', 'aria-label': 'language' }, LANGS.map(l =>
    h('a', { class: 'ui-chip' + (l === r.lang ? ' ui-chip--accent' : ''), href: href({ ...r, lang: l }), 'aria-current': l === r.lang ? 'true' : null, text: l.toUpperCase() })));
}

function roleTabs(r, t) {
  return h('nav', { class: 'w-roles', 'aria-label': t.title },
    h('a', { class: 'ui-chip' + (!r.role ? ' ui-chip--accent' : ''), href: href({ lang: r.lang }), text: t.all }),
    ROLES.map(role => h('a', { class: 'ui-chip' + (role === r.role ? ' ui-chip--accent' : ''), href: href({ lang: r.lang, role }), text: t[role] })));
}

function card(l, r, t) {
  const cut = S.media[l.id]?.cuts?.[r.lang];
  return h('li', {}, h('a', { class: 'w-card', href: href({ lang: r.lang, role: l.role, id: l.id }) },
    cut ? h('img', { class: 'w-thumb', src: cut.poster, alt: '', loading: 'lazy', width: 90, height: 160 }) : h('span', { class: 'w-thumb w-thumb--none', 'aria-hidden': 'true' }),
    h('span', { class: 'w-card-body' },
      h('strong', { text: `${l.id} · ${l.title[r.lang]}` }),
      h('span', { class: 'w-muted', text: l.goal[r.lang] }),
      h('span', { class: 'w-meta' }, h('span', { class: 'ui-chip', text: cut ? `${t.video} ${clock(cut.durationMs)}` : t.noVideo }),
        l.writes ? h('span', { class: 'ui-chip ui-chip--warning', text: t.writes }) : null))));
}

function list(r) {
  const t = T[r.lang];
  const pool = S.lessons.filter(l => !r.role || l.role === r.role);
  const found = search(pool, S.q, r.lang);
  const input = h('input', { class: 'w-search', type: 'search', placeholder: t.search, 'aria-label': t.search, value: S.q,
    on: { input: e => { S.q = e.target.value; draw(true); } } });
  main.replaceChildren(h('header', { class: 'w-head' }, h('h1', { text: t.title }), langBar(r)), roleTabs(r, t), input,
    found.length ? h('ul', { class: 'w-list' }, found.map(l => card(l, r, t))) : h('p', { class: 'w-muted', text: t.none }));
  return input;
}

function lessonPage(r) {
  const t = T[r.lang];
  const l = S.lessons.find(x => x.id === r.id);
  if (!l) { main.replaceChildren(h('p', { text: t.notFound }), h('a', { href: href({ lang: r.lang, role: r.role }), text: t.back })); return; }
  const cut = S.media[l.id]?.cuts?.[r.lang];
  let video = null, chapters = [];
  if (cut) {
    video = h('video', { class: 'w-video', controls: true, playsinline: true, preload: 'metadata', poster: cut.poster, src: cut.video },
      tracks(cut, r.lang).map(tr => h('track', { kind: 'subtitles', srclang: tr.lang, label: tr.label, src: tr.src, default: tr.default })));
    fetch(cut.chapters).then(x => x.json()).then(c => { chapters = c.chapters || []; paintSteps(); }).catch(() => {});
  }
  const steps = h('ol', { class: 'w-steps' });
  function paintSteps() {
    steps.replaceChildren(...l.steps.map((s, i) => {
      const ch = chapters[i];
      const title = ch && video ? h('button', { type: 'button', class: 'w-seek', on: { click: () => { video.currentTime = ch.startMs / 1000; video.play().catch(() => {}); } } },
        h('span', { class: 'w-time', text: clock(ch.startMs) }), s.title[r.lang]) : h('strong', { text: s.title[r.lang] });
      return h('li', {}, title, h('p', { class: 'w-muted', text: s.caption[r.lang] }));
    }));
  }
  paintSteps();
  main.replaceChildren(
    h('header', { class: 'w-head' }, h('a', { class: 'w-back', href: href({ lang: r.lang, role: l.role }), text: `← ${t.back}` }), langBar(r)),
    h('h1', { text: `${l.id} · ${l.title[r.lang]}` }),
    h('p', {}, h('strong', { text: `${t.goal}: ` }), l.goal[r.lang]),
    video || h('p', { class: 'ui-chip', text: t.noVideo }),
    h('p', {}, h('a', { class: 'ui-btn ui-btn--primary', href: appLink(l.role, l.id), text: t.open })),
    h('h2', { text: t.steps }), steps);
}

function draw(keepFocus = false) {
  const r = parseRoute(location.hash, pickLang([safeGet('dw_wiki_lang'), safeGet('dw_admin_lang'), safeGet('dw_room_lang'), safeGet('dw_c_lang'), safeGet('dw_lang')], navigator.languages || []));
  safeSet('dw_wiki_lang', r.lang);
  document.documentElement.lang = r.lang;
  document.title = `dowiz · ${T[r.lang].title}`;
  if (r.id) return lessonPage(r);
  const input = list(r);
  if (keepFocus) { input.focus(); input.setSelectionRange(input.value.length, input.value.length); }
}

async function boot() {
  main.replaceChildren(h('p', { class: 'w-muted', text: T.sq.loading }));
  try {
    const [les, med] = await Promise.all([fetch('/learn/lessons.json').then(r => r.json()),
      fetch('/learn/media/manifest.json').then(r => r.ok ? r.json() : { lessons: {} }).catch(() => ({ lessons: {} }))]);
    S.lessons = les.lessons || []; S.media = med.lessons || {};
  } catch { main.replaceChildren(h('p', { text: T.sq.failed })); return; }
  window.addEventListener('hashchange', () => { draw(); window.scrollTo(0, 0); });
  draw();
}
boot();
