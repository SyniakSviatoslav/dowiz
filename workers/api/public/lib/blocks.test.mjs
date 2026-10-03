// The device copy of the published menu (lib/blocks.js), in node over the Map
// backend: verify-on-read, refuse-and-delete, never keep an unverifiable name,
// the root's sequence, and the per-venue budget.
// `node --test workers/api/public/lib/blocks.test.mjs`
// ASCII QUOTES ONLY (DOWIZ-COMMON-RULES rule 11).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { k64Key, k64Of, verified, memoryBackend, blockStore, BUDGET } from './blocks.js';

const bytes = s => new TextEncoder().encode(s);
const k64 = b => createHash('sha256').update(b).digest('hex').slice(0, 16);
const named = (s, ext = 'json') => { const b = bytes(s); return { key: `${k64(b)}.${ext}`, b }; };

test('k64Of is the first 16 hex of sha256, as hubdo/menu.rs pins it', async () => {
  assert.equal(await k64Of(bytes('')), 'e3b0c44298fc1c14');
  assert.equal(await k64Of(bytes('abc')), 'ba7816bf8f01cfea');
  assert.equal(k64Key('ba7816bf8f01cfea.json'), 'ba7816bf8f01cfea');
  assert.equal(k64Key('ba7816bf8f01cfea.dwb'), 'ba7816bf8f01cfea');
  assert.equal(k64Key('f1.json'), null, 'not content-addressed');
  assert.equal(k64Key('manifest.json'), null);
  assert.equal(k64Key('BA7816BF8F01CFEA.json'), null, 'the publisher writes lower case');
  assert.equal(await verified('ba7816bf8f01cfea.json', bytes('abc')), true);
  assert.equal(await verified('ba7816bf8f01cfea.json', bytes('abd')), false);
  assert.equal(await verified('f1.json', bytes('abc')), false, 'a name that names nothing is never verified');
});

test('put keeps only bytes that are their name; get answers them back', async () => {
  const be = memoryBackend();
  const st = blockStore(be);
  const o = named('{"price":900}');
  assert.equal(await st.put('dubin', o.key, o.b, 1), true);
  assert.deepEqual(await st.get('dubin', o.key), o.b);
  assert.equal(await st.put('dubin', o.key, bytes('{"price":950}'), 1), false, 'other bytes under that name are refused');
  assert.deepEqual(await st.get('dubin', o.key), o.b, 'and the kept copy is untouched');
  assert.equal(await st.put('dubin', 'f1.json', o.b, 1), false, 'a key that is not a k64 is never kept');
  assert.equal(await st.get('dubin', 'f1.json'), null);
  assert.equal(await st.get('other-venue', o.key), null, 'one venue does not read another venue\'s keys');
  assert.equal(st.stats.stored, 1);
});

test('a tampered blob is REFUSED on read, deleted and counted; its untampered twin is read', async () => {
  const be = memoryBackend();
  const st = blockStore(be);
  const o = named('{"categories":[]}');
  const p = named('{"categories":[1]}');
  await st.put('dubin', o.key, o.b, 1);
  await st.put('dubin', p.key, p.b, 1);
  const blob = be.raw.objects.get(`dubin/${o.key}`);
  new Uint8Array(blob.bytes)[2] ^= 1;
  const warned = [];
  const w = console.warn; console.warn = (...a) => warned.push(a.join(' '));
  try { assert.equal(await st.get('dubin', o.key), null); } finally { console.warn = w; }
  assert.equal(st.stats.refused, 1);
  assert.equal(be.raw.objects.has(`dubin/${o.key}`), false, 'deleted, so the next read goes to the network');
  assert.match(warned.join('\n'), /not what its name says/);
  assert.deepEqual(await st.get('dubin', p.key), p.b, 'the twin is read');
  assert.equal(st.stats.hits, 1);
});

test('putRoot: the sequence moves only when the root\'s text does; a damaged root is no root', async () => {
  const be = memoryBackend();
  const st = blockStore(be);
  assert.equal(await st.root('dubin'), null);
  assert.equal(await st.putRoot('dubin', '{"v":1}', 1000), 1);
  assert.equal(await st.putRoot('dubin', '{"v":1}', 2000), 1, 'same text, same generation');
  assert.equal((await st.root('dubin')).at, 2000, 'but the time it was read moves');
  assert.equal(await st.putRoot('dubin', '{"v":1,"x":2}', 3000), 2);
  be.raw.roots.set('dubin', { text: 7, at: 1, seq: 1 });
  assert.equal(await st.root('dubin'), null);
});

test('prune: least recently used first, never what the current root names, until it fits', async () => {
  const be = memoryBackend();
  const st = blockStore(be, { budget: 100 });
  const objs = [0, 1, 2, 3].map(i => named(`{"g":${i},"pad":"${'x'.repeat(30)}"}`));
  for (const [i, o] of objs.entries()) await st.put('dubin', o.key, o.b, i + 1);
  const size = objs[0].b.byteLength;
  assert.ok(size * 4 > 100 && size * 2 <= 100, `the fixture straddles the budget (${size} bytes each)`);
  const r = await st.prune('dubin', new Set([objs[0].key]));
  assert.equal(r.dropped, 2);
  assert.ok(be.raw.objects.has(`dubin/${objs[0].key}`), 'the current root\'s object stays, oldest or not');
  assert.ok(!be.raw.objects.has(`dubin/${objs[1].key}`) && !be.raw.objects.has(`dubin/${objs[2].key}`), 'the oldest others go');
  assert.ok(be.raw.objects.has(`dubin/${objs[3].key}`), 'the newest stays');
  assert.ok(r.total <= 100);
  const under = await st.prune('dubin', new Set());
  assert.equal(under.dropped, 0, 'under budget nothing is dropped');
  assert.equal(BUDGET, 1024 * 1024, 'the card\'s bound: under 1 MB per venue');
});

test('get moves an object\'s `seen` to the root that used it, so prune keeps what is in use', async () => {
  const be = memoryBackend();
  const st = blockStore(be);
  const o = named('{"a":1}');
  await st.put('dubin', o.key, o.b, 1);
  await st.get('dubin', o.key, 5);
  assert.equal(be.raw.objects.get(`dubin/${o.key}`).seen, 5);
  await st.get('dubin', o.key, 3);
  assert.equal(be.raw.objects.get(`dubin/${o.key}`).seen, 5, 'never backwards');
});
