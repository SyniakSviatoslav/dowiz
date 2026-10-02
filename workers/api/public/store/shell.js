// The storefront's READ PATH OFF THE WORKER (BN2; cost map A1/C3).
//
// The venue's object publishes its menu to R2 at every catalogue generation
// (`workers/api/src/hubdo/publish.rs`): one small root, `manifest.json`
// (`max-age=30`), naming immutable, content-addressed objects -- the FRAGMENT
// (the own-language menu without the clock), the WORDS of each other language
// (only what differs), the blocks, the photos. This module reads them and
// hands app.js the SAME object `/api/public/locations/<slug>/menu` answered,
// so nothing downstream knows where the menu came from.
//
// FAIL OPEN THROUGH THE HUB, NEVER BLANK. No CDN for this host, a manifest that
// is missing, malformed or for another venue, a block that will not load: each
// answers `null` here and `fetchMenuIn` reads the Worker's route exactly as
// before. The CDN is the fast, uncounted path; the Worker is the truth.
//
// THE CLOCK IS PUT BACK HERE. An immutable object cannot say whether the venue
// is open NOW, so the fragment carries `hours`, `tz`, `ownerStatus` and
// `deliveryPaused` and this file derives `status`, `nextOpen` and
// `closedReason` the way the hub does (`fold/menu_venue.rs::at`, over
// `dowiz_hub::hours`). The two are pinned to the same instants
// (`shell.test.mjs`, `hubdo/menu/tests.rs`). The price a customer pays is
// re-derived by the kernel at placement; what is read here is a VIEW.
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).

import { readAgain } from '/lib/retry.js';
import { API, SLUG } from '/store/state.js';
import { venueClock } from '/lib/booking-time.js';

/// A day is 1440 minutes; `dowiz_hub::hours::DAY`.
const DAY = 1440;
/// The fields a translation may overlay on a category or a product.
const WORD_FIELDS = ['name', 'description', 'ingredients'];

/// The CDN origin for the page: `https://cdn.<platform>` for a venue at
/// `<slug>.<platform>`. `?cdn=` wins (a stand, a test); a workers.dev host, a
/// bare IP or localhost has none, and so reads through the hub.
export function cdnOrigin(loc = location) {
  const explicit = new URLSearchParams(loc.search).get('cdn');
  if (explicit) return explicit.replace(/\/+$/, '');
  const host = loc.hostname.toLowerCase();
  if (host.endsWith('.workers.dev') || host === 'localhost' || /^[\d.]+$/.test(host)) return null;
  const labels = host.split('.');
  if (labels.length < 3 || labels[0] === 'www') return null;
  return `https://cdn.${labels.slice(1).join('.')}`;
}

// ── the clock, as the hub derives it ────────────────────────────────────────
/// `[[{open,close}], ...]` -> seven lists of windows, under `hours::from_json`'s
/// rule: a window outside the day or of zero length is skipped, not clamped.
export function schedule(hours) {
  const days = Array.from({ length: 7 }, () => []);
  if (!Array.isArray(hours)) return days;
  hours.slice(0, 7).forEach((day, i) => {
    for (const w of Array.isArray(day) ? day : []) {
      const o = Number(w?.open), c = Number(w?.close);
      if (Number.isInteger(o) && Number.isInteger(c) && o >= 0 && o < DAY && c >= 0 && c <= DAY && o !== c) days[i].push({ open: o, close: c });
    }
  });
  return days;
}
const wraps = w => w.close <= w.open;
const coversSameDay = (w, m) => wraps(w) ? m >= w.open : (m >= w.open && m < w.close);
const coversSpillover = (w, m) => wraps(w) && m < w.close;
export const isEmpty = days => days.every(d => d.length === 0);
export function isOpenAt(days, weekday, minute) {
  const today = weekday % 7, yesterday = (today + 6) % 7;
  return days[today].some(w => coversSameDay(w, minute)) || days[yesterday].some(w => coversSpillover(w, minute));
}
export function nextOpen(days, weekday, minute) {
  if (isEmpty(days)) return null;
  for (let ahead = 0; ahead < 8; ahead++) {
    const d = (weekday + ahead) % 7;
    for (const o of days[d].map(w => w.open).sort((a, b) => a - b)) {
      if (ahead > 0 || o > minute) return { weekday: d, minute: o };
    }
  }
  return null;
}
/// The three clock fields of a `location`, at `nowMs`: THE STATUS IS DERIVED.
/// A paused venue is closed however its flag reads, and a schedule can only close.
export function clockAt(loc, nowMs) {
  const days = schedule(loc.hours);
  const { weekday, minute } = venueClock(loc.tz || 'Europe/Tirane', nowMs);
  const scheduledOpen = isEmpty(days) || isOpenAt(days, weekday, minute);
  const owner = typeof loc.ownerStatus === 'string' ? loc.ownerStatus : 'open';
  const paused = loc.deliveryPaused === true;
  const closed = paused || owner === 'closed' || !scheduledOpen;
  return {
    status: closed ? 'closed' : owner,
    nextOpen: nextOpen(days, weekday, minute),
    closedReason: paused ? 'paused' : owner === 'closed' ? 'manual' : !scheduledOpen ? 'hours' : null,
  };
}

// ── the menu, assembled from the published objects ──────────────────────────
/// The fragment with one locale's words laid over it, the photos pointed at
/// the CDN (only those the list says are there) and the clock put back: the
/// object app.js has always consumed.
export function assemble(fragment, words, media, { base, now = Date.now() }) {
  const d = fragment;
  const w = (words && words.words) || {};
  const there = new Set(Array.isArray(media) ? media : []);
  const url = u => {
    if (typeof u !== 'string' || !u.startsWith('/media/')) return u;
    const name = u.slice('/media/'.length);
    return there.has(name) ? `${base}m/${name}` : u;
  };
  const say = (x, id) => { for (const f of WORD_FIELDS) if (w[id] && f in w[id]) x[f] = w[id][f]; };
  for (const c of d.categories || []) {
    say(c, c.id);
    for (const p of c.products || []) {
      say(p, p.id);
      if (typeof p.imageUrl === 'string') p.imageUrl = url(p.imageUrl);
      if (typeof p.imageUrlSmall === 'string') p.imageUrlSmall = url(p.imageUrlSmall);
    }
  }
  if (d.location) {
    if (typeof d.location.logoUrl === 'string') d.location.logoUrl = url(d.location.logoUrl);
    Object.assign(d.location, clockAt(d.location, now));
  }
  d.warnings = [...(Array.isArray(d.warnings) ? d.warnings : []), ...((words && words.warnings) || [])];
  return d;
}

/// The menu in `locale` from the CDN, or `null` when the CDN has nothing usable.
export async function fromCdn(locale, { origin = cdnOrigin(), slug = SLUG, now = Date.now(), fetchFn = fetch } = {}) {
  if (!origin) return null;
  const base = `${origin}/v/${encodeURIComponent(slug)}/`;
  try {
    const mr = await fetchFn(base + 'manifest.json');
    if (!mr.ok) return null;
    const m = await mr.json();
    if (m.v !== 1 || m.slug !== slug || typeof m.fragment !== 'string') return null;
    const get = async key => { const r = await fetchFn(base + key); if (!r.ok) throw new Error(`cdn ${r.status} for ${key}`); return r.json(); };
    const want = locale || m.default;
    const wordsKey = want !== m.default && m.words ? m.words[want] : null;
    const [fragment, words, media] = await Promise.all([
      get(m.fragment),
      wordsKey ? get(wordsKey) : null,
      typeof m.media === 'string' ? get(m.media).catch(() => []) : [],
    ]);
    return assemble(fragment, words, media, { base, now });
  } catch (e) {
    // Said on the console, where a developer looks; the customer gets the hub's answer.
    console.warn('shell: the published menu was not readable, reading through the hub:', e && e.message ? e.message : e);
    return null;
  }
}

/// The hub's own route: what every visit read before the CDN, unchanged.
export async function fromHub(locale, { slug = SLUG, fetchFn = fetch } = {}) {
  // A platform 503 is asked again before the customer is told the menu did not load (lib/retry.js).
  const r = await readAgain(fetchFn, `${API}/public/locations/${encodeURIComponent(slug)}/menu?locale=${locale}`);
  if (!r.ok) throw new Error('HTTP ' + r.status);
  return r.json();
}

/// The menu in `locale`: the CDN first, the hub when the CDN cannot answer.
export async function fetchMenuIn(locale, opts = {}) {
  const d = (await fromCdn(locale, opts)) || (await fromHub(locale, opts));
  // The key rides beside the location in the payload; the checkout reads it
  // off the location. Carried across once, here, so no module has to know.
  if (d.location && d.stripePublishableKey) d.location.stripePublishableKey = d.stripePublishableKey;
  // A degraded answer says so on the console, where a developer looks.
  if (Array.isArray(d.warnings) && d.warnings.length) console.warn('menu:', d.warnings.join('; '));
  return d;
}
