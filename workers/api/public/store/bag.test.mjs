// RED PROOF 6.2 (W-QR): THE BAG LANDING NEVER LEAVES sessionStorage BEFORE CHECKOUT.
// `node --test workers/api/public/store/bag.test.mjs`
//
// The core is driven with a recording session store and with the browser's other
// stores (localStorage, document.cookie) replaced by traps that fail the test on
// any touch; then the glue's source is read for every request it can make.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { KEY, parseLanding, capture, bodyFor, forget, outcome } from './bag-core.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));

function recorder(){
  const m = new Map(), log = [];
  return { log, getItem: k => (log.push(['get', k]), m.has(k) ? m.get(k) : null),
    setItem: (k, v) => { log.push(['set', k, v]); m.set(k, String(v)); }, removeItem: k => { log.push(['del', k]); m.delete(k); } };
}

test('the landing lives in the session store, under one key, and nowhere else', () => {
  const trap = what => new Proxy({}, { get(){ throw new Error(`${what} touched`); } });
  globalThis.localStorage = trap('localStorage');
  globalThis.document = { get cookie(){ throw new Error('cookie read'); }, set cookie(_){ throw new Error('cookie written'); } };
  const s = recorder();
  const l = capture('?src=bag&c=spring', s);
  assert.deepEqual(l, { src: 'bag', c: 'spring' });
  assert.deepEqual(s.log, [['set', KEY, '{"src":"bag","c":"spring"}']]);
  assert.deepEqual(capture('', s), { src: 'bag', c: 'spring' }, 'the next page of the same visit still has it');
  forget(s);
  assert.equal(capture('', s), null, 'forgotten after the order');
  delete globalThis.localStorage; delete globalThis.document;
});

test('it leaves the device only as the order body member', () => {
  assert.deepEqual(bodyFor({ src: 'bag', c: 'spring' }), { src: { src: 'bag', c: 'spring' } });
  assert.deepEqual(bodyFor({ src: 'bag', c: null }), { src: { src: 'bag' } });
  assert.deepEqual(bodyFor(null), {}, 'no landing, nothing sent');
  const glue = fs.readFileSync(path.join(HERE, 'bag.js'), 'utf8').replace(/\/\/[^\n]*/g, '');
  const fetches = [...glue.matchAll(/fetch\((`[^`]*`|'[^']*'|[^)`']*)/g)].map(m => m[1]);
  assert.deepEqual(fetches, ['`${API}/public/locations/${encodeURIComponent(SLUG)}/welcome`'], 'one request before checkout: the public offer');
  assert.ok(!fetches[0].includes('?') && !/LANDING|\bc\b/.test(fetches[0]), 'and it carries no landing and no campaign');
  assert.ok(!/localStorage|document\.cookie|navigator\.|sendBeacon|safeSet/.test(glue), 'no other store, no beacon, no fingerprint');
  const checkout = fs.readFileSync(path.join(HERE, 'checkout.js'), 'utf8');
  assert.ok(/\.\.\.bagBody\(\) \}\);/.test(checkout), 'checkout spreads it into the order body');
});

test('only the bag word is a landing; a campaign the hub would refuse is dropped', () => {
  assert.equal(parseLanding('?src=wolt'), null);
  assert.equal(parseLanding('?c=spring'), null);
  assert.deepEqual(parseLanding('?src=bag&c=Bad Word'), { src: 'bag', c: null });
  assert.deepEqual(parseLanding('?src=bag'), { src: 'bag', c: null });
});

test('the hub answer becomes a sentence key', () => {
  assert.equal(outcome({ welcome: { kind: 'fixed', discount: 300 } }), 'bg_ok');
  assert.equal(outcome({ welcome_refused: 'used' }), 'bg_r_used');
  assert.equal(outcome({}), null);
});
