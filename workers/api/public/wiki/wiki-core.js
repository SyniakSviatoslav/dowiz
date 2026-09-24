// The wiki's pure half: routes, words, search, links. No DOM, so wiki.test.mjs proves it.
//
// Routes (hash, so the static shell needs no server rule):
//   #/<lang>                              every track
//   #/<lang>/<role>                       one track (owner | waiter | courier | guest)
//   #/<lang>/<role>/<id>                  a lesson (what lib/learn.js links to)
//   #/<lang>/<role>/<module>/<id>         the same, with the module the blueprint names
export const LANGS = ['sq', 'en', 'uk'];
export const ROLES = ['owner', 'waiter', 'courier', 'guest'];
const ID = /^[A-Z][0-9]+[a-z]?$/;

export const T = {
  sq: { title: 'Mësime', owner: 'Pronari', waiter: 'Kamarieri', courier: 'Korrieri', guest: 'Mysafiri', all: 'Të gjitha',
    search: 'Kërko një mësim', video: 'Video', noVideo: 'Video së shpejti', steps: 'Hapat', goal: 'Qëllimi',
    open: 'Hapeni në aplikacion', back: 'Mbrapa', none: 'Asnjë mësim nuk përputhet.', writes: 'Ndryshon të dhëna reale',
    loading: 'Po ngarkohet…', failed: 'Mësimet nuk u ngarkuan.', notFound: 'Ky mësim nuk ekziston.', subs: 'Titrat' },
  en: { title: 'Lessons', owner: 'Owner', waiter: 'Waiter', courier: 'Courier', guest: 'Guest', all: 'All',
    search: 'Search the lessons', video: 'Video', noVideo: 'Video coming soon', steps: 'Steps', goal: 'Goal',
    open: 'Open it in the app', back: 'Back', none: 'No lesson matches.', writes: 'Changes real data',
    loading: 'Loading…', failed: 'The lessons did not load.', notFound: 'There is no such lesson.', subs: 'Subtitles' },
  uk: { title: 'Уроки', owner: 'Власник', waiter: 'Офіціант', courier: 'Кур’єр', guest: 'Гість', all: 'Усі',
    search: 'Пошук уроку', video: 'Відео', noVideo: 'Відео незабаром', steps: 'Кроки', goal: 'Мета',
    open: 'Відкрити в застосунку', back: 'Назад', none: 'Жоден урок не підходить.', writes: 'Змінює реальні дані',
    loading: 'Завантаження…', failed: 'Уроки не завантажилися.', notFound: 'Такого уроку немає.', subs: 'Субтитри' },
};

/// '#/uk/waiter/W3' -> { lang, role, id }. Unknown parts fall back, never throw.
export function parseRoute(hash, fallbackLang = 'sq') {
  const parts = String(hash || '').replace(/^#\/?/, '').split('/').map(p => { try { return decodeURIComponent(p); } catch { return ''; } }).filter(Boolean);
  const lang = LANGS.includes(parts[0]) ? parts[0] : (LANGS.includes(fallbackLang) ? fallbackLang : 'sq');
  const role = ROLES.includes(parts[1]) ? parts[1] : null;
  const last = parts[parts.length - 1];
  const id = role && parts.length >= 3 && ID.test(last) ? last : null;
  return { lang, role, id };
}

export function href({ lang, role = null, id = null }) {
  return '#/' + [lang, role, id].filter(Boolean).map(encodeURIComponent).join('/');
}

/// The app a lesson opens in, at the lesson (lib/learn.js reads #learn=<id>).
const APP = { owner: '/admin/', waiter: '/room/', courier: '/courier/', guest: '/' };
export const appLink = (role, id) => `${APP[role] || '/'}#learn=${encodeURIComponent(id)}`;

/// The first language the reader already uses in one of the apps, else Albanian.
export function pickLang(stored = [], nav = []) {
  for (const v of [...stored, ...nav.map(n => String(n || '').slice(0, 2).toLowerCase())]) if (LANGS.includes(v)) return v;
  return 'sq';
}

/// Words of a lesson in one language, lowercased, for the search.
export function words(lesson, lang) {
  const t = [lesson.id, lesson.title?.[lang], lesson.goal?.[lang],
    ...(lesson.steps || []).flatMap(s => [s.title?.[lang], s.caption?.[lang]])].filter(Boolean).join(' ');
  return t.toLocaleLowerCase();
}

/// Lessons matching every term of `q`, best first (a title hit counts three).
export function search(lessons, q, lang) {
  const terms = String(q || '').toLocaleLowerCase().split(/\s+/).filter(Boolean);
  if (!terms.length) return lessons;
  return lessons.map(l => {
    const all = words(l, lang), head = `${l.id} ${l.title?.[lang] || ''}`.toLocaleLowerCase();
    if (!terms.every(t => all.includes(t))) return null;
    return { l, score: terms.reduce((n, t) => n + (head.includes(t) ? 3 : 1), 0) };
  }).filter(Boolean).sort((a, b) => b.score - a.score).map(x => x.l);
}

/// The three caption tracks, the page's language first and default.
export function tracks(cut, lang) {
  const order = [lang, ...LANGS.filter(l => l !== lang)];
  return order.filter(l => cut?.subs?.[l]).map(l => ({ lang: l, src: cut.subs[l], label: l.toUpperCase(), default: l === lang }));
}

/// "83.4 s" -> "1:23"
export function clock(ms) {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}
