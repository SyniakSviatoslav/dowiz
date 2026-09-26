// node --test workers/api/public/lib/retry.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readAgain, TRIES } from './retry.js';

const answers = list => { let i = 0; const seen = []; const f = async url => { seen.push(url); const a = list[Math.min(i++, list.length - 1)]; if (a instanceof Error) throw a; return new Response('x', { status: a }); }; return { f, seen }; };
const waits = [];
const sleep = async ms => { waits.push(ms); };

test('a 503 then a 200: the customer gets the menu, not "did not load" (the live defect)', async () => {
  const h = answers([503, 200]);
  const r = await readAgain(h.f, '/api/public/locations/x/menu', {}, { sleep });
  assert.equal(r.status, 200);
  assert.equal(h.seen.length, 2);
});

test('a 200 is asked once', async () => {
  const h = answers([200]);
  assert.equal((await readAgain(h.f, '/m', {}, { sleep })).status, 200);
  assert.equal(h.seen.length, 1);
});

test('a 404 is an answer, not a reason to ask again', async () => {
  const h = answers([404, 200]);
  assert.equal((await readAgain(h.f, '/m', {}, { sleep })).status, 404);
  assert.equal(h.seen.length, 1);
});

test('bounded: three 503s answer the last 503, with growing waits', async () => {
  waits.length = 0;
  const h = answers([503, 503, 503, 200]);
  assert.equal((await readAgain(h.f, '/m', {}, { sleep, waitMs: 10 })).status, 503);
  assert.equal(h.seen.length, TRIES);
  assert.deepEqual(waits, [10, 20]);
});

test('a dropped connection is asked again; the last one throws', async () => {
  const ok = answers([new Error('net'), 200]);
  assert.equal((await readAgain(ok.f, '/m', {}, { sleep })).status, 200);
  const bad = answers([new Error('net')]);
  await assert.rejects(readAgain(bad.f, '/m', {}, { sleep }), /net/);
});
