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

  await t.test('short names (<=4 chars) are fully masked, never leaked verbatim', () => {
    // maskStr returns '***' for length<=4; a regression that returns the
    // plaintext for short names (e.g. 'Ali', 'Jo') must turn this red.
    for (const shortName of ['Ali', 'Jo', 'A', 'John']) {
      const masked = mapCourierHistoryRow({ ...row, customer_name: shortName }).customerAddress;
      assert.equal(masked, '***', `short name "${shortName}" must be fully masked`);
      assert.notEqual(masked, shortName, `short name "${shortName}" must not be returned verbatim`);
    }
    // 5-char boundary: first 2 + *** + last 2, never the whole plaintext.
    const five = mapCourierHistoryRow({ ...row, customer_name: 'Maria' }).customerAddress;
    assert.equal(five, 'Ma***ia');
    assert.ok(!five.includes('ari'), 'middle of a 5-char name must be masked');
  });
});

// TODO(needs_staging): unit coverage exercises mapCourierHistoryRow only. The
// live HTTP route GET /api/courier/me/history (apps/api/src/routes/courier/me.ts:249)
// decrypts customer_name via the SQL join and must also return masked output —
// assert against deployed staging with a real courier JWT (status 200 + every
// row's customerAddress matches /^\*\*\*$|^.{2}\*\*\*.{2}$/, no plaintext name).
// TODO(needs_staging): cross-tenant IDOR — a courier whose JWT activeLocationId
// is tenant A must receive 0 rows belonging to a REAL second tenant B (never an
// all-zero nil-UUID). Requires two seeded tenants on staging to prove enforcement.
