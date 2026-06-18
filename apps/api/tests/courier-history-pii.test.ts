import test from 'node:test';
import assert from 'node:assert/strict';
import { mapCourierHistoryRow } from '../src/routes/courier/me.js';

// H3 regression: GET /api/courier/me/history must NOT return the customer's
// plaintext name. Couriers are a lower-trust role; the rest of the system masks
// customer/courier names everywhere, this endpoint must too.

test('courier history row masks customer name', async (t) => {
  const row = {
    id: 'a1', order_id: 'o1', delivered_at: '2026-06-18T00:00:00Z', created_at: null,
    location_name: 'Demo Diner', customer_name: 'Johnathan Smith',
    cash_amount: '1500', total: '1500', status: 'delivered',
  };

  const out = mapCourierHistoryRow(row);

  await t.test('does not leak the plaintext name', () => {
    assert.notEqual(out.customerAddress, 'Johnathan Smith');
    assert.ok(!out.customerAddress.includes('ohnathan'), 'middle of name must be masked');
  });

  await t.test('returns a masked form', () => {
    // maskStr keeps first 2 + last 2 chars for long strings
    assert.equal(out.customerAddress, 'Jo***th');
  });

  await t.test('handles null name without throwing', () => {
    assert.equal(mapCourierHistoryRow({ ...row, customer_name: null }).customerAddress, '***');
  });
});
