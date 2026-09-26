// /wiki/ -- the lesson library: every lesson of every track, its video with three caption
// tracks, its chapters, a search, and a link back into the app at the lesson.
//
// Two inputs: /learn/lessons.json (static; every lesson's words, the one source the in-app tour
// reads too) and GET /api/learn/manifest (GATED: the cuts this reader's role may watch). Every
// media file comes through GET /api/learn/media/... with the app's own Bearer token and is
// played from a blob: URL, because a <video> or <track> cannot send an Authorization header.
// CSP-clean: no inline script or style, text only through textContent.
import { T, LANGS, ROLES, parseRoute, href, appLink, pickLang, search, tracks, clock, bearer, gatedUrl } from './wiki-core.js';

const $ = s => document.querySelector(s);
const main = $('#main');
const S = { lessons: [], media: {}, q: '', signedOut: false };

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
const store = area => area === 'session' ? sessionStorage : localStorage;
const read = (area, k) => { try { return store(area).getItem(k); } catch { return null; } };
const write = (area, k, v) => { try { store(area).setItem(k, v); } catch {} };

/// A gated GET with the app's token. The console's access token is renewed from dw_rt when
/// absent (a new tab) or refused (401). The refresh token ROTATES, so a second concurrent use of
/// the same one would read as a reuse: one renewal per refresh token, shared. null = no token.
const renewals = new Map();
function renew(rt) {
  if (!renewals.has(rt)) renewals.set(rt, fetch('/api/auth/refresh', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ refresh_token: rt }) })
    .then(x => x.ok ? x.json() : null).then(d => {
      if (!d || !d.access_token) return null;
      write('session', 'dw_at', d.access_token); if (d.refresh_token) write('local', 'dw_rt', d.refresh_token);
      return d.access_token;
    }).catch(() => null));
  return renewals.get(rt);
}
async function authed(url) {
  const b = bearer(read);
  const t = b.token || (b.refresh ? await renew(b.refresh) : null);
  if (!t) return null;
  const go = x => fetch(url, { headers: { authorization: 'Bearer ' + x } });
  const r = await go(t);
  if (r.status !== 401 || !b.refresh || t !== read('session', 'dw_at')) return r;
  const n = await renew(b.refresh);
  return n && n !== t ? go(n) : r;
}
const blobs = new Map();
/// One media file as a blob: URL, fetched once per page.
function blobUrl(url) {
  const u = gatedUrl(url);
  if (!blobs.has(u)) blobs.set(u, authed(u).then(r => { if (!r || !r.ok) throw new Error(`${u}: ${r ? r.status : 'signed out'}`); return r.blob(); }).then(b => URL.createObjectURL(b)));
  return blobs.get(u);
}

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
  const img = cut ? h('img', { class: 'w-thumb', alt: '', width: 90, height: 160 }) : null;
  if (img) blobUrl(cut.poster).then(u => { img.src = u; }).catch(() => {});
  return h('li', {}, h('a', { class: 'w-card', href: href({ lang: r.lang, role: l.role, id: l.id }) },
    img || h('span', { class: 'w-thumb w-thumb--none', 'aria-hidden': 'true' }),
    h('span', { class: 'w-card-body' },
      h('strong', { text: `${l.id} · ${l.title[r.lang]}` }),
      h('span', { class: 'w-muted', text: l.goal[r.lang] }),
      h('span', { class: 'w-meta' }, h('span', { class: 'ui-chip', text: cut ? `${t.video} ${clock(cut.durationMs)}` : S.signedOut ? t.signIn : t.noVideo }),
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
  const slot = cut ? h('p', { class: 'w-muted', text: t.loading }) : null;
  if (cut) {
    const tr = tracks(cut, r.lang);
    Promise.all([blobUrl(cut.video), blobUrl(cut.poster), ...tr.map(x => blobUrl(x.src))]).then(([v, p, ...subs]) => {
      video = h('video', { class: 'w-video', controls: true, playsinline: true, preload: 'metadata', poster: p, src: v },
        tr.map((x, i) => h('track', { kind: 'subtitles', srclang: x.lang, label: x.label, src: subs[i], default: x.default })));
      slot.replaceWith(video); paintSteps();
    }).catch(() => { slot.textContent = t.failed; });
    authed(gatedUrl(cut.chapters)).then(x => x.json()).then(c => { chapters = c.chapters || []; paintSteps(); }).catch(() => {});
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
    slot || h('p', { class: 'ui-chip', text: S.signedOut ? t.signIn : t.noVideo }),
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
      authed('/api/learn/manifest').then(r => { if (r && r.ok) return r.json(); S.signedOut = true; return { lessons: {} }; }).catch(() => ({ lessons: {} }))]);
    S.lessons = les.lessons || []; S.media = med.lessons || {};
  } catch { main.replaceChildren(h('p', { text: T.sq.failed })); return; }
  window.addEventListener('hashchange', () => { draw(); window.scrollTo(0, 0); });
  draw();
}
boot();
