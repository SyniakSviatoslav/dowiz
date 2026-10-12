// node --test workers/api/public/lib/draft-logic.test.mjs
// The draft rule (W-SESSION TZ 7): what is kept, under which key, for how long,
// and what is NEVER kept -- pure, no DOM, no clock.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PREFIX, KEEP_MS, MAX_PER_FORM, collectable, keyOf, shapeOf, hashOf, diff, remember, recall, forget, parseBook } from './draft-logic.js';

const T0 = 1_700_000_000_000;

test('the key is the registered prefix, then app / venue / scope / shape', () => {
  assert.equal(PREFIX, 'dw_draft_');
  assert.equal(keyOf('admin', 'v1', 'menu:dish', 'abc'), 'dw_draft_admin/v1/menu:dish/abc');
  // A venue slug with a slash cannot forge another venue's key.
  assert.equal(keyOf('admin', 'v/1', 's', 'x'), 'dw_draft_admin/v%2F1/s/x');
});

test('the shape is the sorted field names: order on screen does not matter, a new field does', () => {
  assert.equal(shapeOf(['b', 'a']), shapeOf(['a', 'b']));
  assert.notEqual(shapeOf(['a']), shapeOf(['a', 'b']));
  assert.match(shapeOf(['a']), /^[0-9a-f]{8}$/);
});

test('secrets, payment and one-time fields are never collected; typed text is', () => {
  assert.equal(collectable({ type: 'text' }), true);
  assert.equal(collectable({ type: 'number', autocomplete: 'off' }), true);
  assert.equal(collectable({ tag: 'textarea' }), true);
  assert.equal(collectable({ tag: 'select' }), true);
  for (const type of ['password', 'hidden', 'file', 'submit', 'button']) assert.equal(collectable({ type }), false, type);
  for (const ac of ['current-password', 'new-password', 'one-time-code', 'username', 'cc-number', 'cc-csc', 'cc-exp']) assert.equal(collectable({ type: 'text', autocomplete: ac }), false, ac);
  // The person's own identity fields are the privacy registry's, not a draft's.
  for (const ac of ['tel', 'email', 'name', 'given-name', 'family-name', 'street-address', 'address-line1', 'postal-code']) assert.equal(collectable({ type: 'text', autocomplete: ac }), false, ac);
  assert.equal(collectable({ type: 'tel' }), false);
  assert.equal(collectable({ type: 'email' }), false);
  // An explicit opt-out on the field wins over everything.
  assert.equal(collectable({ type: 'text', nodraft: true }), false);
});

test('diff keeps only what moved away from the base; nothing moved is null', () => {
  assert.equal(diff({ a: '1', b: '' }, { a: '1', b: '' }), null);
  assert.deepEqual(diff({ a: '1', b: '' }, { a: '1', b: 'x' }), { b: 'x' });
  // A field cleared back to blank is a change too (the base had text).
  assert.deepEqual(diff({ a: '1' }, { a: '' }), { a: '' });
});

test('a book remembers one draft per base, newest first, pruned at the cap and after seven days', () => {
  assert.equal(KEEP_MS, 7 * 24 * 60 * 60 * 1000);
  let book = remember({}, hashOf({ n: 'Nori' }), { n: 'Nori roll' }, T0);
  assert.deepEqual(recall(book, hashOf({ n: 'Nori' }), T0 + 1000), { n: 'Nori roll' });
  // Another record of the same shape has its own entry; a stale one is gone.
  book = remember(book, hashOf({ n: 'Tuna' }), { n: 'Tuna roll' }, T0 + KEEP_MS + 1);
  assert.equal(recall(book, hashOf({ n: 'Nori' }), T0 + KEEP_MS + 1), null, 'seven days: forgotten');
  assert.deepEqual(recall(book, hashOf({ n: 'Tuna' }), T0 + KEEP_MS + 1), { n: 'Tuna roll' });
  book = forget(book, hashOf({ n: 'Tuna' }));
  assert.equal(recall(book, hashOf({ n: 'Tuna' }), T0), null);
  // The cap: the oldest goes when a thirteenth arrives.
  let b = {};
  for (let i = 0; i <= MAX_PER_FORM; i++) b = remember(b, 'h' + i, { x: String(i) }, T0 + i);
  assert.equal(Object.keys(b).length, MAX_PER_FORM);
  assert.equal(recall(b, 'h0', T0 + 100), null, 'the oldest was evicted');
  assert.deepEqual(recall(b, 'h' + MAX_PER_FORM, T0 + 100), { x: String(MAX_PER_FORM) });
});

test('an unreadable book is an empty book, never a crash', () => {
  assert.deepEqual(parseBook(null), {});
  assert.deepEqual(parseBook('{not json'), {});
  assert.deepEqual(parseBook('[1,2]'), {});
  assert.deepEqual(parseBook('{"h":{"at":1,"fields":{"a":"b"}}}'), { h: { at: 1, fields: { a: 'b' } } });
});
