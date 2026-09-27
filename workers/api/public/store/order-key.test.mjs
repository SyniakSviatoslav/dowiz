// `node --test workers/api/public/store/order-key.test.mjs`
// The storefront's Idempotency-Key: same basket, same key; new basket, new key;
// answered, forgotten. Every branch of order-key.js is reached here.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { keyFor, answered, settle, defaultMint } from './order-key.js';

const minter = () => { let n = 0; return () => `k${++n}`; };

test('the same body sent again carries the same key, and the mint is called once', () => {
  const keys = {}, mint = minter();
  const body = '{"items":[{"product_id":"sake","quantity":1}]}';
  assert.equal(keyFor(keys, body, mint), 'k1');
  assert.equal(keyFor(keys, body, mint), 'k1', 'a retry after a lost response replays, it does not re-order');
  assert.equal(keyFor(keys, body, mint), 'k1');
});

test('a different body gets a new key: the Worker would 409 a reused key with a new body', () => {
  const keys = {}, mint = minter();
  assert.equal(keyFor(keys, '{"a":1}', mint), 'k1');
  assert.equal(keyFor(keys, '{"a":2}', mint), 'k2');
  assert.equal(keyFor(keys, '{"a":2}', mint), 'k2');
  assert.equal(keyFor(keys, '{"a":1}', mint), 'k3', 'going back to the old basket is a new attempt too');
});

test('once answered, the same basket is a new order with a new key', () => {
  const keys = {}, mint = minter();
  assert.equal(keyFor(keys, '{"a":1}', mint), 'k1');
  answered(keys);
  assert.equal(keys.key, null);
  assert.equal(keys.body, null);
  assert.equal(keyFor(keys, '{"a":1}', mint), 'k2');
});

test('a 409 keeps the key (the first call is still running); any answer spends it', () => {
  const keys = {}, mint = minter();
  assert.equal(keyFor(keys, '{"a":1}', mint), 'k1');
  assert.equal(settle(keys, 409), false);
  assert.equal(keyFor(keys, '{"a":1}', mint), 'k1', 'the next tap asks for the first call\'s answer');
  for (const status of [200, 201, 400, 402, 422, 500]) {
    keyFor(keys, '{"a":1}', mint);
    assert.equal(settle(keys, status), true, `status ${status} is an answer`);
    assert.equal(keys.key, null);
  }
});

test('no CSPRNG: no key is sent, and nothing is remembered', () => {
  const keys = { key: 'stale', body: 'stale' };
  assert.equal(keyFor(keys, '{"a":1}', () => null), null);
  assert.equal(keys.key, null);
  assert.equal(keys.body, null);
});

test('the default mint is the platform CSPRNG, and refuses where there is none', () => {
  const a = defaultMint(), b = defaultMint();
  assert.match(a, /^[0-9a-f-]{36}$/);
  assert.notEqual(a, b);
  const saved = globalThis.crypto;
  Object.defineProperty(globalThis, 'crypto', { value: undefined, configurable: true });
  try { assert.equal(defaultMint(), null); }
  finally { Object.defineProperty(globalThis, 'crypto', { value: saved, configurable: true }); }
});
