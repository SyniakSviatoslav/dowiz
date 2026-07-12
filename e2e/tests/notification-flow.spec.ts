import { test, expect } from '@playwright/test';
import { linkTelegram, placeOrder, advanceOrder, waitTelegramMessage, optInPush } from '../helpers/notifHelpers';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Mutating spec (creates locations/orders, advances state). Fail fast if VITE_BASE_URL is
// unset or points at prod — both the spec and notifHelpers read the same env var, so this
// guards the whole run (no order/location writes against prod).
test.beforeAll(() => {
  requireStaging(process.env.VITE_BASE_URL);
});

test.describe('Notification Flow Tests', () => {
  test('should receive telegram notification for order.created', async () => {
    // Step 1: Link Telegram for owner
    const { connectToken, locationId, userId } = await linkTelegram('owner');
    expectUuid(locationId, 'locationId');
    expectUuid(connectToken, 'connectToken');

    // TODO(needs_staging): drive the real `/start <connectToken>` bot handshake (or a dev
    // seeding endpoint) so the owner chat is actually linked. Until then the waits below time
    // out (a real RED — never weakened/skipped). connectToken is asserted above so this step
    // is wired the moment the handshake helper lands.

    // Step 2: Opt-in to push notifications for owner
    await optInPush('owner', locationId, userId);

    // Step 3: Place an order
    const order = await placeOrder(locationId);
    const orderId = order.id;
    expectUuid(orderId, 'orderId');
    // Server renders the short id as the first 4 chars of the order id, uppercased
    // (apps/api/src/notifications/workers/index.ts:282). Anchor every wait on THIS order's id
    // so a stale buffered message or a concurrent order on shared staging can't false-green.
    const shortId = `#${String(orderId).substring(0, 4).toUpperCase()}`;

    // Step 4: Wait for order.created telegram notification — anchored on this order's short id
    const orderCreatedMessage = await waitTelegramMessage(
      (text) => text.includes('NEW ORDER') && text.includes(shortId),
      15000 // 15 second budget
    );
    expect(orderCreatedMessage).toContain('NEW ORDER');
    expect(orderCreatedMessage).toContain(shortId);

    // Step 5: Advance order to DELIVERED state
    // We need an owner token to advance the order
    const ownerToken = await getOwnerToken();
    expectJwt(ownerToken, 'ownerToken');
    // advanceOrder throws unless the PATCH /orders/:id/status returns ok (200), so a refused
    // or no-op transition fails the test before the wait below.
    await advanceOrder(orderId, 'DELIVERED', ownerToken);

    // Step 6: Wait for order.delivered telegram notification — anchored on this order's short id
    const orderDeliveredMessage = await waitTelegramMessage(
      (text) => text.includes('ORDER DELIVERED') && text.includes(shortId),
      15000 // 15 second budget
    );
    expect(orderDeliveredMessage).toContain('ORDER DELIVERED');
    expect(orderDeliveredMessage).toContain(shortId);

    // TODO(needs_staging): cross-tenant isolation — link a SECOND owner for a different tenant
    // and assert that owner's chat received NO message for THIS order's shortId (zero-message
    // claim requires the live bot handshake above + a real second tenant; do not fake).
    // TODO(needs_staging): error-matrix — assert advanceOrder(orderId, 'NOPE') → 400,
    // a wrong-role token → 403, and a random UUID orderId → 404 (real staging run required).
  });
});

// Helper to get an owner token (mock-auth). Defaults to staging, never prod.
async function getOwnerToken(): Promise<string> {
  const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
  const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
  if (!authRes.ok) throw new Error(`Failed to get owner token: ${await authRes.text()}`);
  const authBody = await authRes.json();
  return authBody.access_token;
}
