// node --test workers/api/public/wiki/wiki-core.test.mjs -- the wiki's pure half: routes, words,
// search, links, and which token opens the gated videos.
import test from 'node:test';
import assert from 'node:assert/strict';
import { T, LANGS, ROLES, parseRoute, href, appLink, pickLang, words, search, tracks, clock, bearer, gatedUrl } from './wiki-core.js';

test('T: every language carries every word the pages use', () => {
  const keys = Object.keys(T.en).sort();
  for (const l of LANGS) assert.deepEqual(Object.keys(T[l]).sort(), keys, l);
  assert.ok(keys.includes('signIn'));
  assert.deepEqual(ROLES, ['owner', 'waiter', 'courier', 'guest']);
});

test('parseRoute + href: a lesson route round-trips; unknown parts fall back, never throw', () => {
  assert.deepEqual(parseRoute('#/uk/waiter/W3'), { lang: 'uk', role: 'waiter', id: 'W3' });
  assert.deepEqual(parseRoute('#/en/owner/menu/O1a'), { lang: 'en', role: 'owner', id: 'O1a' });
  assert.deepEqual(parseRoute('#/de/cook/x', 'en'), { lang: 'en', role: null, id: null });
  assert.deepEqual(parseRoute('', 'xx'), { lang: 'sq', role: null, id: null });
  assert.deepEqual(parseRoute(null), { lang: 'sq', role: null, id: null });
  assert.deepEqual(parseRoute('#/sq/owner/%E0%A4%A'), { lang: 'sq', role: 'owner', id: null });
  assert.deepEqual(parseRoute('#/sq/owner/lowercase'), { lang: 'sq', role: 'owner', id: null });
  assert.equal(href({ lang: 'uk', role: 'waiter', id: 'W3' }), '#/uk/waiter/W3');
  assert.equal(href({ lang: 'sq' }), '#/sq');
});

test('appLink + pickLang: the lesson opens in its app; the reader keeps the language already in use', () => {
  assert.equal(appLink('waiter', 'W3'), '/room/#learn=W3');
  assert.equal(appLink('nobody', 'X1'), '/#learn=X1');
  assert.equal(pickLang([null, 'uk'], ['en-US']), 'uk');
  assert.equal(pickLang([], ['EN-gb', null]), 'en');
  assert.equal(pickLang(), 'sq');
});

const L = [
  { id: 'W3', title: { en: 'Open a table' }, goal: { en: 'seat guests' }, steps: [{ title: { en: 'Tap' }, caption: { en: 'the plus button' } }] },
  { id: 'O1a', title: { en: 'Menu' }, goal: { en: 'a table of dishes' }, steps: [] },
  { id: 'C1' },
];
test('words + search: every term must match, a title hit ranks first; no terms is everything', () => {
  assert.match(words(L[0], 'en'), /w3 open a table seat guests tap the plus button/);
  assert.equal(words(L[2], 'en'), 'c1');
  assert.deepEqual(search(L, '', 'en'), L);
  assert.deepEqual(search(L, 'table', 'en').map(l => l.id), ['W3', 'O1a']);
  assert.deepEqual(search(L, 'plus table', 'en').map(l => l.id), ['W3']);
  assert.deepEqual(search(L, 'c1', 'en').map(l => l.id), ['C1']);
  assert.deepEqual(search(L, 'nothing', 'en'), []);
});

test('tracks + clock: the page language first and default; missing tracks are skipped', () => {
  const cut = { subs: { sq: 's', en: 'e', uk: 'u' } };
  assert.deepEqual(tracks(cut, 'uk').map(t => [t.lang, t.default]), [['uk', true], ['sq', false], ['en', false]]);
  assert.deepEqual(tracks({ subs: { en: 'e' } }, 'sq').map(t => t.src), ['e']);
  assert.deepEqual(tracks(null, 'sq'), []);
  assert.equal(clock(83456), '1:23');
  assert.equal(clock(-5), '0:00');
});

const mem = (session = {}, local = {}) => (area, k) => (area === 'session' ? session : local)[k] ?? null;
test('bearer: the console token first, then an unexpired room session, then the courier; the refresh token rides along', () => {
  assert.deepEqual(bearer(mem({ dw_at: 'A' }, { dw_rt: 'R', dw_c_jwt: 'K' })), { token: 'A', refresh: 'R' });
  const room = JSON.stringify({ jwt: 'J', staff: { expiresMs: 2000 } });
  assert.deepEqual(bearer(mem({}, { dw_room_session: room }), 1000), { token: 'J', refresh: null });
  assert.deepEqual(bearer(mem({}, { dw_room_session: room, dw_c_jwt: 'K' }), 3000), { token: 'K', refresh: null });   // expired room
  assert.deepEqual(bearer(mem({}, { dw_room_session: JSON.stringify({ jwt: 'J', staff: {} }) })), { token: 'J', refresh: null });
  assert.deepEqual(bearer(mem({}, { dw_room_session: '{broken', dw_c_jwt: 'K' })), { token: 'K', refresh: null });
  assert.deepEqual(bearer(mem({}, { dw_room_session: JSON.stringify({ staff: {} }) })), { token: null, refresh: null });
  assert.deepEqual(bearer(mem({}, { dw_rt: 'R' })), { token: null, refresh: 'R' });
});

test('gatedUrl: a static media path moves under /api/learn/media; a gated one is kept', () => {
  assert.equal(gatedUrl('/learn/media/W3/sq/video.mp4'), '/api/learn/media/W3/sq/video.mp4');
  assert.equal(gatedUrl('/api/learn/media/W3/sq/poster.jpg'), '/api/learn/media/W3/sq/poster.jpg');
  assert.equal(gatedUrl(undefined), '');
});
