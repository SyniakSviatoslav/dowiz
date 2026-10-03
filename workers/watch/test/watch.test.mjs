// dowiz-watch's pure halves, driven with plain objects and a fake fetch.
//   node --test workers/watch/test/        (through slot.sh on the box)
// The Worker-only modules (state.js, index.js) import cloudflare:* and are covered by the live
// proof instead: a real tick, /status read back, and a RED transition mail.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { targets, judge, dishCount, probe } from '../src/probes.js';
import { transitions, statusDoc, STALE_MS } from '../src/fold.js';
import { mailRaw } from '../src/mail.js';

const H = (o = {}) => new Headers(o);
const ENV = { WATCH_DOMAIN: 'dowiz.org', WATCH_VENUES: 'sushi-durres qa-durres', EXTRA_TARGETS: '', WATCH_VERSION: '0.1.0', WATCH_COMMIT: 'abc' };

test('targets: platform + 4 rows per venue, extras appended', () => {
  const t = targets(ENV);
  assert.equal(t.length, 1 + 2 * 4);
  assert.equal(t[0].url, 'https://dowiz.org/healthz');
  assert.ok(t.some((x) => x.url === 'https://qa-durres.dowiz.org/api/public/locations/qa-durres/eta' && x.kind === 'quote'));
  const x = targets({ ...ENV, EXTRA_TARGETS: 'qa-404=https://qa-durres.dowiz.org/healthz-404' });
  assert.deepEqual(x.at(-1), { name: 'qa-404', kind: 'healthz', url: 'https://qa-durres.dowiz.org/healthz-404' });
});

test('judge: a challenge is never up, even with 200', () => {
  assert.equal(judge('healthz', 200, H({ 'cf-mitigated': 'challenge' }), 'ok').status, 'challenged');
  assert.equal(judge('healthz', 403, H({ 'cf-mitigated': 'challenge' }), '<html>').status, 'challenged');
});

test('judge: healthz needs 200 AND body ok', () => {
  assert.equal(judge('healthz', 200, H(), 'ok\n').status, 'up');
  assert.equal(judge('healthz', 200, H(), 'nope').status, 'down');
  assert.equal(judge('healthz', 404, H(), 'ok').status, 'down');
  assert.equal(judge('healthz', 403, H(), '').status, 'down'); // the old heartbeat called this up
});

test('judge: storefront needs html AND a CSP header (positive twin)', () => {
  assert.equal(judge('store', 200, H({ 'content-type': 'text/html', 'content-security-policy': "default-src 'self'" }), '<html>').status, 'up');
  assert.equal(judge('store', 200, H({ 'content-type': 'text/html' }), '<html>').status, 'down');
});

test('judge: menu needs dishes; quote needs a range', () => {
  const menu = JSON.stringify({ categories: [{ products: [{}, {}] }, { products: [{}] }] });
  assert.equal(dishCount(menu), 3);
  assert.equal(judge('menu', 200, H(), menu).status, 'up');
  assert.equal(judge('menu', 200, H(), '{"categories":[]}').status, 'down');
  assert.equal(judge('menu', 200, H(), 'not json').status, 'down');
  assert.equal(judge('quote', 200, H(), '{"range":"20-30"}').status, 'up');
  assert.equal(judge('quote', 200, H(), '{}').status, 'down');
});

test('probe: asks twice before calling a target down; unreachable is down', async () => {
  let n = 0;
  const flaky = async () => (++n === 1 ? new Response('x', { status: 503 }) : new Response('ok', { status: 200 }));
  const r = await probe({ name: 'a', kind: 'healthz', url: 'https://x/healthz' }, flaky);
  assert.equal(r.status, 'up');
  assert.equal(n, 2);
  const dead = await probe({ name: 'b', kind: 'healthz', url: 'https://x/healthz' }, async () => { throw new Error('boom'); });
  assert.equal(dead.status, 'down');
  assert.match(dead.detail, /unreachable: boom/);
});

test('transitions: baseline up is silent, baseline down is news, flips are news, steady is silent', () => {
  const res = (s) => [{ name: 'a', url: 'u', status: s, detail: 'd' }];
  assert.deepEqual(transitions({}, res('up')), []);
  assert.equal(transitions({}, res('down'))[0].to, 'down');
  assert.deepEqual(transitions({ a: { status: 'up' } }, res('up')), []);
  const t = transitions({ a: { status: 'up' } }, res('challenged'));
  assert.equal(t.length, 1);
  assert.equal(t[0].from, 'up');
  assert.equal(transitions({ a: { status: 'down' } }, res('up'))[0].to, 'up');
});

test('statusDoc: ok only when fresh and every target up; stale after 15 min', () => {
  const now = 10_000_000;
  const state = { targets: { a: { status: 'up', url: 'u', since: now - 60_000 } }, tick: { atMs: now - 1000, latest: { a: { detail: '200 ok', ms: 12 } } }, mail: null };
  const d = statusDoc(state, ENV, now);
  assert.equal(d.ok, true);
  assert.equal(d.targets.a.forMinutes, 1);
  assert.equal(d.watcher.commit, 'abc');
  assert.equal(statusDoc({ ...state, tick: { atMs: now - STALE_MS - 1 } }, ENV, now).stale, true);
  assert.equal(statusDoc({ ...state, tick: { atMs: now - STALE_MS - 1 } }, ENV, now).ok, false);
  const down = statusDoc({ ...state, targets: { a: { ...state.targets.a, status: 'down' } } }, ENV, now);
  assert.equal(down.ok, false);
  assert.deepEqual(down.notUp, ['a']);
  assert.equal(statusDoc({ targets: {}, tick: null, mail: null }, ENV, now).ok, false);
});

test('mailRaw: one header block, CR/LF in a target name cannot add a header', () => {
  const raw = mailRaw({ from: 'watch@dowiz.org', to: 'op@x', atMs: 0, statusUrl: 'https://w/status', id: 'watch-0',
    changes: [{ name: 'evil\r\nBcc: spy@x', url: 'u', from: 'up', to: 'down', detail: '503' }] });
  const head = raw.split('\r\n\r\n')[0];
  assert.ok(!/^Bcc:/m.test(head), head);
  assert.match(head, /^Subject: dowiz-watch: 1 DOWN$/m);
  assert.match(raw, /DOWN {2}evil Bcc: spy@x {2}\(was up\)/);
});
