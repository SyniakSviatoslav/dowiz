// `node --test workers/api/public/admin/fresh-logic.test.mjs` (W-TZFRESH, TZ 6: no F5 after a save).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createFresh, fresh } from './fresh-logic.js';

/// A hub answer that changes with every read, and counts the reads.
const shelf = () => { let n = 0; const read = async () => ({ n: ++n }); read.count = () => n; return read; };

test('a cache reads once and answers from memory until something is written', async () => {
  const f = createFresh(), read = shelf(), get = f.cache(read);
  assert.deepEqual(await get(), { n: 1 });
  assert.deepEqual(await get(), { n: 1 });
  assert.equal(read.count(), 1);
  assert.deepEqual(get.peek(), { n: 1 });
});

test('TZ 6: a write that landed makes the NEXT ask re-read -- the old name is never shown after a save', async () => {
  const f = createFresh(), read = shelf(), get = f.cache(read);
  await get();
  f.bump('/owner/supplies');
  assert.deepEqual(await get(), { n: 2 }, 'the ask after the save reads again');
  assert.deepEqual(await get(), { n: 2 }, 'and then keeps it');
  assert.equal(f.version(), 2);
});

test('a read in flight when the save lands is not fresh: the ask after it reads again', async () => {
  const f = createFresh();
  let release = null, n = 0;
  // Only the FIRST read is slow (held until `release`); the second answers at once.
  const get = f.cache(async () => { n += 1; if (n === 1) await new Promise(r => { release = r; }); return n; });
  const first = get();
  f.bump('/owner/supplies');   // lands while the first read is out
  release();
  assert.equal(await first, 1);
  assert.equal(await get(), 2, 'the save was not in the answer that was in flight');
});

test('concurrent asks share one read', async () => {
  const f = createFresh(), read = shelf(), get = f.cache(read);
  const [a, b] = await Promise.all([get(), get()]);
  assert.deepEqual([a, b], [{ n: 1 }, { n: 1 }]);
  assert.equal(read.count(), 1);
});

test('`stale` narrows which writes a cache hears; the default hears every one', async () => {
  const f = createFresh(), read = shelf();
  const settings = f.cache(read, { stale: p => p.startsWith('/owner/settings') });
  await settings();
  f.bump('/owner/orders/1/status');
  assert.deepEqual(await settings(), { n: 1 }, 'an order action does not re-read the settings');
  f.bump('/owner/settings');
  assert.deepEqual(await settings(), { n: 2 }, 'a settings save does');
  const any = f.cache(shelf());
  await any(); f.bump('/anything');
  assert.deepEqual(await any(), { n: 2 });
});

test('`ttlMs` still bounds the age: a save on ANOTHER device shows up after it', async () => {
  const f = createFresh(), read = shelf();
  let clock = 1000;
  const get = f.cache(read, { ttlMs: 60_000, now: () => clock });
  await get();
  clock += 59_000; assert.deepEqual(await get(), { n: 1 });
  clock += 2_000; assert.deepEqual(await get(), { n: 2 });
});

test('a read that fails keeps the cache stale and hands the previous value to the next read', async () => {
  const f = createFresh();
  let fail = false, prevSeen;
  const get = f.cache(async prev => { prevSeen = prev; if (fail) throw new Error('offline'); return 'book'; });
  assert.equal(await get(), 'book');
  f.bump('/x'); fail = true;
  await assert.rejects(get(), /offline/);
  fail = false;
  assert.equal(await get(), 'book', 'the ask after the failure reads again (still stale)');
  assert.equal(prevSeen, 'book', 'the reader saw what the cache held, to fall back on');
});

test('forget() is an explicit bump for one cache; on() hears bumps and can stop', async () => {
  const f = createFresh(), read = shelf(), get = f.cache(read);
  await get(); get.forget();
  assert.deepEqual(await get(), { n: 2 });
  const heard = []; const off = f.on((p, v) => heard.push([p, v]));
  f.bump('/a'); off(); f.bump('/b');
  assert.deepEqual(heard, [['/a', 2]]);
});

test('the console has exactly one shared version', () => {
  assert.equal(typeof fresh.bump, 'function');
  const v = fresh.version(); fresh.bump('/owner/products/x'); assert.equal(fresh.version(), v + 1);
});
