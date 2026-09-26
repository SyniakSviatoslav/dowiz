// node --test workers/api/public/lib/body.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { bodyOf } from './body.js';

test('a JSON answer is parsed', async () => {
  assert.deepEqual(await bodyOf(new Response('{"jwt":"x","staff":{"locationId":"v"}}')), { jwt: 'x', staff: { locationId: 'v' } });
});

test('a plain-text refusal becomes { error } with the hub\'s words (the live defect)', async () => {
  assert.deepEqual(await bodyOf(new Response('invalid credentials\n', { status: 401 })), { error: 'invalid credentials' });
});

test('a bare JSON string and an empty body are words, never a throw', async () => {
  assert.deepEqual(await bodyOf(new Response('"oops"')), { error: 'oops' });
  assert.deepEqual(await bodyOf(new Response('')), { error: '' });
});
