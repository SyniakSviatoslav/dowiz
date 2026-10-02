// The published-menu reader (store/shell.js), in node: the clock derived the
// way the hub derives it, the words laid over the fragment, the photos pointed
// at the CDN, the CDN origin chosen per host, and the fallback to the hub.
// `node --test workers/api/public/store/shell.test.mjs`
//
// The instants are the ones `workers/api/src/hubdo/menu/tests.rs` and
// `fold/menu/tests.rs` pin: 2026-07-01 12:00 Tirane (open) and 03:00 (closed,
// `hours`). If the hub's law moves, both files move in the same commit.
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

// The screens import by absolute URL and touch `location`; give node both.
globalThis.location = { search: '', hostname: 'dubin.dowiz.org' };
globalThis.localStorage = { getItem: () => null, setItem() {}, removeItem() {} };
Object.defineProperty(globalThis, 'navigator', { value: { languages: ['sq'] }, configurable: true });
const here = new URL('.', import.meta.url);
const src = readFileSync(new URL('./shell.js', here), 'utf8')
  .replace("'/lib/retry.js'", JSON.stringify(new URL('../lib/retry.js', here).href))
  .replace("'/lib/booking-time.js'", JSON.stringify(new URL('../lib/booking-time.js', here).href))
  .replace("import { API, SLUG } from '/store/state.js';", "const API = '/api'; const SLUG = 'dubin';");
const shell = await import('data:text/javascript;base64,' + Buffer.from(src).toString('base64'));
const { cdnOrigin, schedule, isOpenAt, nextOpen, clockAt, assemble, fromCdn, fetchMenuIn } = shell;

const NOON = 1_782_900_000_000;
const NIGHT = 1_782_867_600_000;
const WEEK = Array.from({ length: 7 }, () => [{ open: 660, close: 1380 }]);
const loc = extra => ({ tz: 'Europe/Tirane', hours: WEEK, ownerStatus: 'open', deliveryPaused: false, ...extra });

test('cdnOrigin: cdn.<platform> for a venue host; ?cdn= wins; none for localhost, an IP or workers.dev', () => {
  assert.equal(cdnOrigin({ search: '', hostname: 'dubin-sushi.dowiz.org' }), 'https://cdn.dowiz.org');
  assert.equal(cdnOrigin({ search: '', hostname: 'qa-durres.dowiz.org' }), 'https://cdn.dowiz.org');
  assert.equal(cdnOrigin({ search: '?cdn=https://cdn.example.test/', hostname: '127.0.0.1' }), 'https://cdn.example.test');
  assert.equal(cdnOrigin({ search: '', hostname: 'localhost' }), null);
  assert.equal(cdnOrigin({ search: '', hostname: '127.0.0.1' }), null);
  assert.equal(cdnOrigin({ search: '', hostname: 'dowiz-api.x.workers.dev' }), null);
  assert.equal(cdnOrigin({ search: '', hostname: 'dowiz.org' }), null, 'the apex is the platform, not a venue');
  assert.equal(cdnOrigin({ search: '', hostname: 'www.dowiz.org' }), null);
});

test('schedule: hours::from_json\'s rule -- outside the day or zero length is skipped, eight days are seven', () => {
  const s = schedule([[{ open: -60, close: 600 }, { open: 660, close: 660 }, { open: 2000, close: 2100 }, { open: 600, close: 900 }]]);
  assert.deepEqual(s[0], [{ open: 600, close: 900 }]);
  assert.equal(schedule('nonsense').every(d => d.length === 0), true);
  assert.equal(schedule([[], [], [], [], [], [], [], [{ open: 0, close: 100 }]])[0].length, 0, 'an eighth day is not a day');
  assert.deepEqual(schedule([[{ open: 1080, close: 120 }]])[0], [{ open: 1080, close: 120 }], 'a window over midnight is a window');
});

test('isOpenAt / nextOpen: same-day windows, yesterday\'s spill-over, the next opening within eight days', () => {
  const late = schedule([[{ open: 1080, close: 120 }], [], [], [], [], [], []]);
  assert.equal(isOpenAt(late, 0, 1100), true, '18:20 Monday');
  assert.equal(isOpenAt(late, 1, 60), true, '01:00 Tuesday is still Monday\'s night');
  assert.equal(isOpenAt(late, 1, 130), false);
  assert.deepEqual(nextOpen(late, 1, 130), { weekday: 0, minute: 1080 }, 'next Monday');
  assert.deepEqual(nextOpen(schedule(WEEK), 2, 600), { weekday: 2, minute: 660 }, 'later today');
  assert.deepEqual(nextOpen(schedule(WEEK), 2, 1400), { weekday: 3, minute: 660 }, 'tomorrow');
  assert.equal(nextOpen(schedule([]), 2, 600), null, 'no schedule never opens -- and is never closed by one');
});

test('clockAt: the hub\'s instants -- noon open, 03:00 closed by hours; paused and manual outrank the schedule', () => {
  assert.deepEqual(clockAt(loc({}), NOON), { status: 'open', nextOpen: { weekday: 3, minute: 660 }, closedReason: null });
  assert.deepEqual(clockAt(loc({}), NIGHT), { status: 'closed', nextOpen: { weekday: 2, minute: 660 }, closedReason: 'hours' });
  assert.equal(clockAt(loc({ deliveryPaused: true }), NOON).closedReason, 'paused');
  assert.equal(clockAt(loc({ deliveryPaused: true }), NOON).status, 'closed');
  assert.equal(clockAt(loc({ ownerStatus: 'closed' }), NOON).closedReason, 'manual');
  assert.equal(clockAt(loc({ ownerStatus: 'busy' }), NOON).status, 'busy', 'an owner status that is not closed is passed through');
  assert.equal(clockAt(loc({ hours: null }), NIGHT).status, 'open', 'no schedule: open at any hour');
  assert.equal(clockAt(loc({ tz: undefined }), NOON).status, 'open', 'no zone: Tirane, as the hub');
});

const fragment = () => ({
  categories: [
    { id: 'c_soup', name: 'Supa', products: [{ id: 'p_s', name: 'Miso', price: 400, imageUrl: null }] },
    { id: 'c_rolls', name: 'Rolls', products: [
      { id: 'p_a', name: 'Nigiri', price: 700, imageUrl: '/media/missing.jpg' },
      { id: 'p_b', name: 'Maki', price: 900, ingredients: ['oriz'], imageUrl: '/media/aaa.jpg', imageUrlSmall: '/media/aaas.jpg' },
    ] },
  ],
  location: { ...loc({}), slug: 'dubin', logoUrl: '/media/logo.png' },
  stripePublishableKey: 'pk_test',
  warnings: [],
});
const words = { words: { p_b: { name: 'Maki roll', ingredients: ['rice'] }, c_rolls: { name: 'Rolls EN' } }, warnings: ['translations unavailable: x'] };
const BASE = 'https://cdn.dowiz.org/v/dubin/';

test('assemble: words over the fragment, photos the list has to the CDN, the others to the hub, the clock back', () => {
  const d = assemble(fragment(), words, ['aaa.jpg', 'aaas.jpg', 'logo.png'], { base: BASE, now: NOON });
  const maki = d.categories[1].products[1];
  assert.equal(maki.name, 'Maki roll');
  assert.deepEqual(maki.ingredients, ['rice']);
  assert.equal(maki.price, 900, 'a price is never a word');
  assert.equal(d.categories[1].name, 'Rolls EN');
  assert.equal(d.categories[0].name, 'Supa', 'untranslated stays the venue\'s own');
  assert.equal(maki.imageUrl, BASE + 'm/aaa.jpg');
  assert.equal(maki.imageUrlSmall, BASE + 'm/aaas.jpg');
  assert.equal(d.categories[1].products[0].imageUrl, '/media/missing.jpg', 'a photo the CDN does not have stays on the hub');
  assert.equal(d.categories[0].products[0].imageUrl, null, 'null is passed through as stored');
  assert.equal(d.location.logoUrl, BASE + 'm/logo.png');
  assert.equal(d.location.status, 'open');
  assert.equal(d.location.closedReason, null);
  assert.deepEqual(d.warnings, ['translations unavailable: x']);
  const own = assemble(fragment(), null, [], { base: BASE, now: NIGHT });
  assert.equal(own.categories[1].products[1].name, 'Maki');
  assert.equal(own.categories[1].products[1].imageUrl, '/media/aaa.jpg', 'no list: nothing is pointed at the CDN');
  assert.equal(own.location.status, 'closed');
});

/// A fetch over a map of URL -> body; records what was asked.
function cdn(objects, { manifest = true } = {}) {
  const asked = [];
  const fetchFn = async url => {
    asked.push(url);
    const key = url.startsWith(BASE) ? url.slice(BASE.length) : url;
    if (!(key in objects) || (key === 'manifest.json' && !manifest)) return { ok: false, status: 404, json: async () => ({}) };
    return { ok: true, status: 200, json: async () => JSON.parse(JSON.stringify(objects[key])) };
  };
  return { asked, fetchFn };
}
const published = {
  'manifest.json': { v: 1, slug: 'dubin', default: 'sq', locales: ['sq', 'en'], fragment: 'f1.json', words: { en: 'w1.json' }, blocks: {}, media: 'm1.json' },
  'f1.json': fragment(), 'w1.json': words, 'm1.json': ['aaa.jpg'],
};

test('fromCdn: the own language reads root + fragment + list; another language adds its words; the shape is the hub\'s', async () => {
  const sq = cdn(published);
  const d = await fromCdn('sq', { origin: 'https://cdn.dowiz.org', slug: 'dubin', now: NOON, fetchFn: sq.fetchFn });
  assert.deepEqual(sq.asked.map(u => u.slice(BASE.length)), ['manifest.json', 'f1.json', 'm1.json']);
  assert.equal(d.categories[1].products[1].name, 'Maki');
  assert.equal(d.categories[1].products[1].imageUrl, BASE + 'm/aaa.jpg');
  assert.equal(d.categories[1].products[1].imageUrlSmall, '/media/aaas.jpg', 'not in the list: the hub serves it');
  assert.equal(d.location.status, 'open');
  const en = cdn(published);
  const e = await fromCdn('en', { origin: 'https://cdn.dowiz.org', slug: 'dubin', now: NOON, fetchFn: en.fetchFn });
  assert.ok(en.asked.some(u => u.endsWith('/w1.json')));
  assert.equal(e.categories[1].products[1].name, 'Maki roll');
  const ru = cdn(published);
  const r = await fromCdn('ru', { origin: 'https://cdn.dowiz.org', slug: 'dubin', now: NOON, fetchFn: ru.fetchFn });
  assert.equal(r.categories[1].products[1].name, 'Maki', 'a language with no words published reads the venue\'s own');
  assert.equal(ru.asked.length, 3);
});

test('fromCdn: null -- and so the hub -- when there is no origin, no root, another venue\'s root, or a block that fails', async () => {
  assert.equal(await fromCdn('sq', { origin: null, fetchFn: async () => { throw new Error('must not be called'); } }), null);
  const gone = cdn(published, { manifest: false });
  assert.equal(await fromCdn('sq', { origin: 'https://cdn.dowiz.org', slug: 'dubin', fetchFn: gone.fetchFn }), null);
  assert.equal(gone.asked.length, 1, 'one read, the root');
  const other = cdn({ ...published, 'manifest.json': { ...published['manifest.json'], slug: 'someone-else' } });
  assert.equal(await fromCdn('sq', { origin: 'https://cdn.dowiz.org', slug: 'dubin', fetchFn: other.fetchFn }), null);
  const broken = cdn({ ...published, 'f1.json': undefined });
  const warned = [];
  const w = console.warn; console.warn = (...a) => warned.push(a.join(' '));
  try {
    assert.equal(await fromCdn('sq', { origin: 'https://cdn.dowiz.org', slug: 'dubin', fetchFn: broken.fetchFn }), null);
  } finally { console.warn = w; }
  assert.match(warned.join('\n'), /reading through the hub/);
  const dead = { fetchFn: async () => { throw new TypeError('Failed to fetch'); } };
  console.warn = () => {};
  try { assert.equal(await fromCdn('sq', { origin: 'https://cdn.dowiz.org', slug: 'dubin', ...dead }), null); } finally { console.warn = w; }
});

test('fetchMenuIn: the CDN answer carries the Stripe key onto the location; without a CDN the hub route is read', async () => {
  const c = cdn(published);
  const d = await fetchMenuIn('sq', { origin: 'https://cdn.dowiz.org', slug: 'dubin', now: NOON, fetchFn: c.fetchFn });
  assert.equal(d.location.stripePublishableKey, 'pk_test');
  const hub = [];
  const hubFetch = async url => { hub.push(url); return { ok: true, status: 200, json: async () => ({ categories: [], location: { slug: 'dubin' }, stripePublishableKey: null, warnings: [] }) }; };
  const h = await fetchMenuIn('en', { origin: null, slug: 'dubin', fetchFn: hubFetch });
  assert.deepEqual(hub, ['/api/public/locations/dubin/menu?locale=en']);
  assert.equal(h.location.slug, 'dubin');
});
