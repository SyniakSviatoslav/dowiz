import { test, expect } from '@playwright/test';
import { linkTelegram, placeOrder, waitTelegramMessage, deleteTelegramWebhook, clearTelegramUpdates } from '../helpers/notifHelpers';
import { expectUuid } from '../helpers/assert-shape';

test.describe('Order Created Notification Flow', () => {
  test('should receive telegram notification for order.created', async () => {
    // Step 1: Delete webhook to enable getUpdates (avoids 409 conflict)
    await deleteTelegramWebhook();
    
    // Step 2: Clear any pending updates to establish clean baseline
    await clearTelegramUpdates();
    
    // Step 3: Link Telegram for owner
    const { connectToken, deepLink, locationId, userId } = await linkTelegram('owner');
    
    // Step 4: Place an order
    const order = await placeOrder(locationId);
    const orderId = order.id;
    expectUuid(orderId, 'orderId');
    
    // Step 5: Wait for order.created telegram notification
    const orderCreatedMessage = await waitTelegramMessage(
      (text) => text.includes('NEW ORDER') || text.includes('YANGI BUYURTMA'),
      20000, // 20 second budget
      2000   // check every 2 seconds
    );
    
    expect(orderCreatedMessage).toBeTruthy(`Expected order.created telegram message not found`);
    expect(orderCreatedMessage).toMatch(/NEW ORDER|YANGI BUYURTMA/);
    
    console.log(`Received order.created telegram message: ${orderCreatedMessage}`);
  });
});

// Helper to get an owner token (mock-auth)
async function getOwnerToken(): Promise<string> {
  const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
  const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
  if (!authRes.ok) throw new Error(`Failed to get owner token: ${await authRes.text()}`);
  const authBody = await authRes.json();
  return authBody.access_token;
}