// orderRef() and codeWord() (core.js): what a person reads instead of a code.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { orderRef, codeWord } from './index.js';

test('orderRef: the storage prefix is dropped and the ref is short', () => {
  assert.equal(orderRef('ord_0001'), '#0001');
  assert.equal(orderRef('ord_0001abcdef1'), '#0001abcd');
  assert.equal(orderRef('ebills:3f2a9c10-77aa'), '#3f2a9c10');
  assert.equal(orderRef('glovo-7781'), '#7781');
  assert.equal(orderRef('ord_0001abcdef1', 4), '#0001');
});

test('orderRef: an id with no prefix is kept as it is (positive twin), nothing -> nothing', () => {
  assert.equal(orderRef('A1B2C3'), '#A1B2C3');
  assert.equal(orderRef('0001'), '#0001');
  // an upper-case head is not a storage prefix
  assert.equal(orderRef('ORD_1'), '#ORD_1');
  assert.equal(orderRef(''), '');
  assert.equal(orderRef(null), '');
});

const WORDS = { rsSt_CONFIRMED: 'Confirmed', rsSt_NO_SHOW: 'No-show' };
const t = k => WORDS[k] ?? k;

test('codeWord: the surface word when the surface has the key', () => {
  assert.equal(codeWord(t, 'rsSt_', 'CONFIRMED'), 'Confirmed');
  assert.equal(codeWord(t, 'rsSt_', 'NO_SHOW'), 'No-show');
});

test('codeWord: a missing key never leaks the key -- the code is made readable', () => {
  assert.equal(codeWord(t, 'rsSt_', 'PENDING'), 'Pending');
  assert.equal(codeWord(t, 'rsSt_', 'DONE'), 'Done');
  assert.equal(codeWord(t, 'rsSt_', 'CANCELLED_BY_GUEST'), 'Cancelled by guest');
  assert.notEqual(codeWord(t, 'rsSt_', 'PENDING'), 'rsSt_PENDING');
  assert.equal(codeWord(t, 'rsSt_', ''), '');
  assert.equal(codeWord(null, 'x_', 'A_B'), 'A b');
});
