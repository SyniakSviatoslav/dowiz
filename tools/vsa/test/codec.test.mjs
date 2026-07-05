// Lossless is the load-bearing invariant: decode(encode(x)) must deep-equal x for every
// JSON value — including the shapes that broke parity elsewhere in this project
// (null-vs-absent keys, imageUrl:null) and strings that collide with the codec's own
// sigils. A codec that loses shape is corruption, not compression.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { encode, decode } from '../src/codec.mjs';
import { hvFor, bind, bundle, cosine, textHv, predictionError } from '../src/hv.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));

function roundTrip(value) {
  const frame = encode(value);
  assert.deepEqual(decode(frame), value);
  return frame;
}

test('primitives and empties', () => {
  for (const v of [null, true, false, 0, -1.5, 1e21, '', 'plain', [], {}, [[]], { a: {} }]) {
    roundTrip(v);
  }
});

test('sigil collisions: literal § strings and keys survive', () => {
  roundTrip({ '§t': 'not a table', '§r': ['x'], '§0': '§1', '§~weird': '§~' });
  roundTrip(['§0', '§', '§§', '§~', '§12', { '§d': ['§b'] }]);
});

test('null-vs-absent is preserved (the imageUrl parity class)', () => {
  const a = [{ x: null, y: 1 }, { x: null, y: 2 }, { x: null, y: 3 }];
  const b = [{ y: 1 }, { y: 2 }, { y: 3 }];
  assert.deepEqual(decode(encode(a)), a);
  assert.deepEqual(decode(encode(b)), b);
  assert.notDeepEqual(decode(encode(a)), decode(encode(b)));
});

test('heterogeneous arrays never columnarize away their differences', () => {
  roundTrip([{ a: 1 }, { a: 1, b: 2 }, { b: 2 }, 'str', 7, null]);
  // same keys, different ORDER — must stay lossless (order carries meaning in JSON wire parity)
  const mixedOrder = [{ a: 1, b: 2 }, { b: 2, a: 1 }, { a: 3, b: 4 }];
  assert.deepEqual(decode(encode(mixedOrder)), mixedOrder);
});

test('unicode, quotes, newlines', () => {
  roundTrip({ 'ключ з пробілами': 'значення\n"з лапками" — і тире', emoji: '🍕§🍣' });
});

test('deterministic: same input → byte-identical frame', () => {
  const v = { rows: [{ id: 'x', n: 1 }, { id: 'y', n: 2 }, { id: 'z', n: 3 }] };
  assert.equal(encode(v), encode(v));
});

test('real payloads round-trip byte-exactly (menu/products/info)', () => {
  const dir = path.join(HERE, '..', 'bench', 'payloads');
  for (const f of fs.readdirSync(dir).filter((f) => f.endsWith('.json'))) {
    const value = JSON.parse(fs.readFileSync(path.join(dir, f), 'utf8'));
    assert.equal(JSON.stringify(decode(encode(value))), JSON.stringify(value), f);
  }
});

test('hv: bind is self-inverse, bundle preserves members, symbols are stable', () => {
  const a = hvFor('SEARCH_LOGISTICS');
  const b = hvFor('ORDER_CREATE');
  assert.equal(cosine(a, hvFor('SEARCH_LOGISTICS')), 1, 'pure function of the string');
  assert.ok(Math.abs(cosine(a, b)) < 0.1, 'distinct symbols ≈ orthogonal');
  const bound = bind(a, b);
  assert.ok(cosine(bind(bound, b), a) === 1, 'unbind recovers exactly (bipolar)');
  const bag = bundle([a, b, hvFor('GDPR_ERASE')]);
  assert.ok(cosine(bag, a) > 0.3 && cosine(bag, b) > 0.3, 'members visible in the bundle');
});

test('prediction error: identical ≈ 0, unrelated ≈ 1, related in between', () => {
  const s = 'S1 flipped rust theme byte-identical menu zero diffs';
  assert.ok(predictionError(s, s) < 0.01);
  assert.ok(predictionError(s, 'courier cash settlement payout dispute') > 0.8);
  const near = predictionError(s, 'S1 flipped rust theme byte-identical menu three diffs');
  assert.ok(near > 0.01 && near < 0.5, `near-miss should be mid-range, got ${near}`);
});

test('textHv word order matters (bigram binding)', () => {
  const ab = cosine(textHv('rollback then flip'), textHv('flip then rollback'));
  assert.ok(ab < 0.995, 'order-swapped text must not be identical');
});
