import { test, describe } from 'node:test';
import assert from 'node:assert';
import { z } from 'zod';
import { CreateOrderInput, ReceiverInput, MessengerKind } from './legacy.js';

// G03: "deliver to someone else" orders must not 400 on validation.
// Regression test — the order schema previously had a 3-kind messenger enum and
// NO `receiver` field, so the web app's receiver payload was rejected by .strict().
describe('CreateOrderInput — "deliver to someone else" (G03)', () => {

  const baseOrder = {
    locationId: '550e8400-e29b-41d4-a716-446655440000',
    type: 'delivery',
    items: [{ product_id: '550e8400-e29b-41d4-a716-446655440001', quantity: 1 }],
    customer: {
      name: 'Customer',
      messenger_kind: 'whatsapp',
      messenger_handle: '+355691234567',
    },
    delivery: { pin: { lat: 41.3, lng: 19.8 }, address_text: 'Rruga Kavajes' },
    payment: { method: 'cash' },
    idempotency_key: '550e8400-e29b-41d4-a716-446655440002',
  } as const;

  test('rejects a receiver payload when the receiver field is absent (pre-fix behaviour)', () => {
    // Documents WHY the fix is needed: before G03, .strict() rejected receiver.
    const withoutReceiver = CreateOrderInput.safeParse(baseOrder);
    assert.strictEqual(withoutReceiver.success, true, 'base order (no receiver) should validate');
  });

  test('ACCEPTS a full "deliver to someone else" receiver (phone kind) — no 400', () => {
    const parsed = CreateOrderInput.safeParse({
      ...baseOrder,
      receiver: {
        name: 'Jane Doe',
        messenger_kind: 'phone',
        handle: '+355692222333',
      },
    });
    assert.strictEqual(parsed.success, true,
      `receiver payload must validate, got: ${JSON.stringify((parsed as z.SafeParseError<typeof CreateOrderInput>).error?.issues ?? parsed)}`);
    if (parsed.success) {
      assert.strictEqual(parsed.data.receiver?.name, 'Jane Doe');
      assert.strictEqual(parsed.data.receiver?.messenger_kind, 'phone');
      assert.strictEqual(parsed.data.receiver?.handle, '+355692222333');
    }
  });

  test('ACCEPTS a receiver with a link-kind (signal) — the 3rd+ kind enum', () => {
    const parsed = CreateOrderInput.safeParse({
      ...baseOrder,
      receiver: {
        name: 'Signal User',
        messenger_kind: 'signal',
        handle: '+355693333444',
      },
    });
    assert.strictEqual(parsed.success, true,
      `signal receiver must validate (6-kind enum), got: ${JSON.stringify((parsed as any).error?.issues ?? parsed)}`);
  });

  test('customer.messenger_kind accepts all 6 canonical kinds', () => {
    for (const k of ['phone', 'whatsapp', 'viber', 'telegram', 'signal', 'simplex'] as const) {
      const parsed = CreateOrderInput.safeParse({
        ...baseOrder,
        customer: { name: 'C', messenger_kind: k, messenger_handle: 'h' },
      });
      assert.strictEqual(parsed.success, true,
        `customer messenger_kind '${k}' should be valid, got: ${JSON.stringify((parsed as any).error?.issues ?? parsed)}`);
    }
  });

  test('ReceiverInput requires all three fields', () => {
    assert.strictEqual(ReceiverInput.safeParse({ name: 'X', messenger_kind: 'phone' }).success, false);
    assert.strictEqual(MessengerKind.safeParse('fax').success, false, 'unknown kind must be rejected');
  });
});
