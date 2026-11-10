// node --test public/admin/finance-logic.test.mjs -- the money sheet's small rules (W-FINANCE).
import test from 'node:test';
import assert from 'node:assert/strict';
import * as L from './finance-logic.js';
import { shiftMonth, monthName, shortDate, checkForm, srcKey, purchasesHint, startDate, undatedOf, formId } from './finance-logic.js';

const parseAmount = (amount, decimals = 0) => checkForm({ date: '2026-01-01', name: 'x', amount }, 'tax', decimals).body?.amount ?? null;

test('an amount is a whole number of minor units, typed the way people type it', () => {
  assert.equal(parseAmount('500'), 500);
  assert.equal(parseAmount('3 200'), 3200);
  assert.equal(parseAmount(' 3 200 '), 3200);
  assert.equal(parseAmount('12,50', 2), 1250);
  assert.equal(parseAmount('12.5', 2), 1250);
  assert.equal(parseAmount('1.200', 0), 1200, 'a thousands dot, in lek');
  assert.equal(parseAmount('7', 2), 700);
});

test('what is not an amount is refused, not guessed', () => {
  for (const bad of ['', 'abc', '-5', '0', '1,5', '12.345.678.9', '5 lek']) assert.equal(parseAmount(bad), null, JSON.stringify(bad));
  assert.equal(parseAmount('12,505', 2), null, 'finer than the currency');
  for (const [typed, decimals] of [['12,500', 2], ['1,500', 2], ['12.345', 2], ['0.125', 2]]) assert.equal(parseAmount(typed, decimals), null, `${typed}: a thousands mark or more precision than the currency has is not guessed`);
  assert.equal(parseAmount('1,500'), 1500, 'a thousands comma, in lek');
  assert.equal(parseAmount('99999999999999999999'), null, 'beyond exact integers');
});

test('the three shapes that were once mis-scaled, at no decimals and at two', () => {
  assert.equal(parseAmount('1,500', 2), null);
  assert.equal(parseAmount('12.345', 2), null);
  assert.equal(parseAmount('0.125', 2), null);
  assert.equal(parseAmount('1,500', 0), 1500, 'in lek a mark before three digits groups thousands');
  assert.equal(parseAmount('12.345', 0), 12345);
  assert.equal(parseAmount('0.125', 0), null, 'a group of thousands never follows a zero');
  assert.equal(parseAmount('1.000', 0), 1000);
  assert.equal(parseAmount('1,50', 2), 150);
  assert.equal(parseAmount('12.34', 2), 1234);
  assert.equal(parseAmount('0.12', 2), 12);
  assert.equal(L.parseAmount, undefined, 'one money parser in the console, not a second here');
});

test('a new entry starts in the month on screen, a date ahead of today is refused, the form id rides along', () => {
  assert.equal(startDate('2026-11', '2026-11-08'), '2026-11-08');
  assert.equal(startDate('2026-10', '2026-11-08'), '2026-10-01');
  assert.equal(startDate('2025-12', '2026-01-02'), '2025-12-01');
  assert.deepEqual(checkForm({ date: '2026-11-09', name: 'x', amount: '5' }, 'tax', 0, { today: '2026-11-08' }), { err: 'fn_futureDate' });
  assert.deepEqual(checkForm({ date: '2026-11-08', name: 'x', amount: '5' }, 'tax', 0, { today: '2026-11-08', id: 'abc12345' }), { body: { date: '2026-11-08', name: 'x', amount: 5, kind: 'tax', client_id: 'abc12345' } });
  assert.match(formId(1731000000000, '0.x9Z_k2'), /^[a-z0-9]{8,40}$/);
  assert.notEqual(formId(1731000000000, 'aaaa'), formId(1731000000000, 'bbbb'));
});

test('undated deliveries are counted only on the purchases row', () => {
  assert.equal(undatedOf({ key: 'purchases', undated: 2 }), 2);
  assert.equal(undatedOf({ key: 'purchases', undated: 0 }), 0);
  assert.equal(undatedOf({ key: 'purchases' }), 0);
  assert.equal(undatedOf({ key: 'taxes', undated: 2 }), 0);
});

test('the month moves across a year in both directions', () => {
  assert.equal(shiftMonth('2026-01', -1), '2025-12');
  assert.equal(shiftMonth('2025-12', 1), '2026-01');
  assert.equal(shiftMonth('2026-11', 0), '2026-11');
  assert.match(monthName('2026-11', 'en'), /November 2026/);
  assert.equal(shortDate('2026-11-03'), '03.11');
});

test('the form names what is missing and its twin sends exact minor units', () => {
  assert.deepEqual(checkForm({ date: '2026-11-03', name: ' Бензин ', amount: '500' }, 'other'), { body: { date: '2026-11-03', name: 'Бензин', amount: 500, kind: 'other' } });
  assert.deepEqual(checkForm({ date: '', name: 'x', amount: '500' }, 'tax'), { err: 'fn_noDate' });
  assert.deepEqual(checkForm({ date: '2026-11-03', name: '  ', amount: '500' }, 'tax'), { err: 'fn_noName' });
  assert.deepEqual(checkForm({ date: '2026-11-03', name: 'x', amount: 'x' }, 'tax'), { err: 'fn_noAmount' });
  assert.equal(srcKey('orders'), 'fn_src_orders');
});

test('the purchases row says why it is zero or incomplete', () => {
  assert.equal(purchasesHint({ key: 'purchases', receipts: 0, uncosted: 0 }), 'fn_noReceipts');
  assert.equal(purchasesHint({ key: 'purchases', receipts: 3, uncosted: 1 }), 'fn_uncosted');
  assert.equal(purchasesHint({ key: 'purchases', receipts: 3, uncosted: 0 }), null);
  assert.equal(purchasesHint({ key: 'taxes' }), null);
});
