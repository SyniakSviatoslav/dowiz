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
//
// THE DEVICE COPY (BN3, menu half; `lib/blocks.js`). The root and the objects it
// names are kept in IndexedDB, each object VERIFIED against its k64 name on
// every read. A return visit inside the root's 30 s asks nothing; after it, the
// root alone; offline, the device's last root and its objects; a new
// generation fetches only the names it has not seen. No order is kept (AX7).
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).

import { readAgain } from '/lib/retry.js';
import { API, SLUG } from '/store/state.js';
import { venueClock } from '/lib/booking-time.js';
import { openBlocks, k64Key, verified, FRESH_MS } from '/lib/blocks.js';

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

// ── the device copy (BN3, lib/blocks.js) ────────────────────────────────────
/// A root this shell can read for `slug`, or null.
const usable = (m, slug) => m && m.v === 1 && m.slug === slug && typeof m.fragment === 'string';
/// The keys a root names: what `prune` must keep.
function named(m) {
  const keys = [m.fragment, m.media, ...Object.values(m.words || {}), ...Object.values(m.blocks || {})];
  return new Set(keys.filter(k => typeof k === 'string'));
}

/// The root: from the device inside its 30 s (no request), else the network
/// (stored), else -- the network is GONE, not answering 404 -- the device's last.
/// A 404 or a root for another venue is null: an unpublished menu is never revived.
async function readRoot(base, slug, fetchFn, blocks, wall) {
  const saved = blocks ? await blocks.root(slug).catch(() => null) : null;
  let was = null;
  try { was = saved ? JSON.parse(saved.text) : null; } catch { /* a damaged root is no root */ }
  const age = saved ? wall() - saved.at : Infinity;
  if (usable(was, slug) && age >= 0 && age < FRESH_MS) return { m: was, seq: saved.seq, from: 'device' };
  let r;
  try {
    r = await fetchFn(base + 'manifest.json');
  } catch (e) {
    if (usable(was, slug)) return { m: was, seq: saved.seq, from: 'offline' };
    throw e;
  }
  if (!r.ok) return null;
  const text = await r.text();
  const m = JSON.parse(text);
  if (!usable(m, slug)) return null;
  const seq = blocks ? await blocks.putRoot(slug, text, wall()).catch(() => 0) : 0;
  return { m, seq, from: 'network' };
}

/// What the last `fromCdn` did, for the console and the gate (`storefront-replica.mjs`).
export const lastRead = { root: null, device: 0, network: 0, ms: 0 };

/// The menu in `locale` from the device or the CDN, or `null` when neither has
/// anything usable. `store` is the device copy (`lib/blocks.js`), null for none.
export async function fromCdn(locale, { origin = cdnOrigin(), slug = SLUG, now = Date.now(), wall = Date.now, fetchFn = fetch, store = openBlocks() } = {}) {
  if (!origin) return null;
  const base = `${origin}/v/${encodeURIComponent(slug)}/`;
  const t0 = globalThis.performance ? performance.now() : 0;
  try {
    const blocks = await store;
    const root = await readRoot(base, slug, fetchFn, blocks, wall);
    if (!root) return null;
    const { m, seq } = root;
    Object.assign(lastRead, { root: root.from, device: 0, network: 0 });
    // Each object: the device's verified copy, else the CDN's -- checked against its
    // own name before it is believed or kept (a key that is not a k64 is used, never kept).
    const get = async key => {
      let bytes = blocks ? await blocks.get(slug, key, seq).catch(() => null) : null;
      if (bytes) lastRead.device++;
      else {
        const r = await fetchFn(base + key);
        if (!r.ok) throw new Error(`cdn ${r.status} for ${key}`);
        bytes = new Uint8Array(await r.arrayBuffer());
        lastRead.network++;
        if (k64Key(key) && globalThis.crypto && globalThis.crypto.subtle) {
          if (!(await verified(key, bytes))) throw new Error(`cdn object ${key} is not what its name says`);
          if (blocks) await blocks.put(slug, key, bytes, seq).catch(() => false);
        }
      }
      return JSON.parse(new TextDecoder().decode(bytes));
    };
    const want = locale || m.default;
    const wordsKey = want !== m.default && m.words ? m.words[want] : null;
    const [fragment, words, media] = await Promise.all([
      get(m.fragment),
      wordsKey ? get(wordsKey) : null,
      typeof m.media === 'string' ? get(m.media).catch(() => []) : [],
    ]);
    if (blocks) blocks.prune(slug, named(m)).catch(() => {});
    const menu = assemble(fragment, words, media, { base, now });
    // The read's own cost (root + objects + verify + assemble), apart from the shell's load.
    lastRead.ms = globalThis.performance ? performance.now() - t0 : 0;
    try { performance.measure('dowiz:menu-read', { start: t0 }); } catch { /* an old browser, or node */ }
    return menu;
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
