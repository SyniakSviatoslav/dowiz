import { test, expect } from '@playwright/test';
import { linkTelegram, placeOrder, waitTelegramMessage, deleteTelegramWebhook, clearTelegramUpdates } from '../helpers/notifHelpers';
<<<<<<< Updated upstream
=======
import { expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Mutating spec (creates a location + order via mock-auth). Refuse to run against prod/unknown.
const BASE = process.env.VITE_BASE_URL;
>>>>>>> Stashed changes

test.describe('Order Created Notification Flow', () => {
  test.beforeAll(() => {
    requireStaging(BASE);
  });

  test('should receive telegram notification for order.created', async () => {
    // Step 1: Delete webhook to enable getUpdates (avoids 409 conflict)
    await deleteTelegramWebhook();

    // Step 2: Clear any pending updates to establish clean baseline
    await clearTelegramUpdates();

    // Step 3: Link Telegram for owner
    // TODO(needs_staging): linkTelegram only fetches connectToken/deepLink — the bot /start
    // handshake is never simulated, so the owner chat is not actually bound. A live staging run
    // must POST the /start payload (connectToken) to the bot webhook before the message can arrive.
    const { locationId } = await linkTelegram('owner');

    // Step 4: Place an order
    const order = await placeOrder(locationId);
    const orderId = order.id;
<<<<<<< Updated upstream
    expect(orderId).toBeTruthy();
    
    // Step 5: Wait for order.created telegram notification
=======
    expectUuid(orderId, 'orderId');
    // TODO(needs_staging): read the order back (GET /api/orders/:id with owner token) and assert
    // status === 'pending' to prove persistence, not just a UUID-shaped echo.

    // The order.created Telegram body embeds `#<shortOrderId>` where shortOrderId is the
    // first 4 hex chars of the order id, uppercased (apps/api/.../workers/index.ts:282).
    // Anchor the match to THIS order's short id so a stale / cross-tenant "NEW ORDER" can't pass.
    const shortId = orderId.slice(0, 4).toUpperCase();

    // Step 5: Wait for order.created telegram notification for this exact order
>>>>>>> Stashed changes
    const orderCreatedMessage = await waitTelegramMessage(
      (text) => text.includes('NEW ORDER') && text.includes(`#${shortId}`),
      20000, // 20 second budget
      2000   // check every 2 seconds
    );

    // Cross-tenant isolation: the received message must belong to this order.
    expect(orderCreatedMessage).toContain('NEW ORDER');
    expect(orderCreatedMessage).toContain(`#${shortId}`);

    console.log(`Received order.created telegram message: ${orderCreatedMessage}`);
  });
});