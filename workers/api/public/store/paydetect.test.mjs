// `node --test workers/api/public/store/paydetect.test.mjs`
// The device's wallet goes first; nothing is added, hidden, stored or sent (W-SENSE row 8).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { orderRails, support } from './paydetect.js';

const RAILS = [['cash'], ['card'], ['apple_pay'], ['google_pay'], ['crypto']];
test('the wallet this device offers goes first and every rail stays', () => {
  assert.deepEqual(orderRails(RAILS, { applePay: true }).map(r => r[0]), ['apple_pay', 'cash', 'card', 'google_pay', 'crypto']);
  assert.deepEqual(orderRails(RAILS, { googlePay: true }).map(r => r[0]), ['google_pay', 'cash', 'card', 'apple_pay', 'crypto']);
  assert.deepEqual(orderRails(RAILS, {}).map(r => r[0]), RAILS.map(r => r[0]), 'no wallet: the venue order');
  assert.deepEqual(orderRails([['cash']], { applePay: true }).map(r => r[0]), ['cash'], 'a rail the venue has not turned on is never added');
});
test('the checks answer yes or no and survive a hostile browser', () => {
  assert.deepEqual(support({ ApplePaySession: { canMakePayments: () => true } }), { applePay: true, googlePay: false });
  assert.deepEqual(support({ PaymentRequest: function(){}, navigator: { userAgent: 'Mozilla/5.0 (Linux; Android 14)' } }), { applePay: false, googlePay: true });
  assert.deepEqual(support({ ApplePaySession: { canMakePayments(){ throw new Error('x'); } } }), { applePay: false, googlePay: false });
  assert.deepEqual(support({}), { applePay: false, googlePay: false });
});
