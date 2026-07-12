/* eslint-disable @typescript-eslint/no-unused-vars -- test/spec/spike/helper code -- flagged strings are test data, selectors, logs, error codes and SQL, not user-facing UI copy; any/raw-any are deliberate test/integration seams */
import { test, expect } from '@playwright/test';
import { linkTelegram, placeOrder, advanceOrder, waitTelegramMessage, optInPush } from '../helpers/notifHelpers';

test.describe('Notification Flow Tests', () => {
  test('should receive telegram notification for order.created', async () => {
    // Step 1: Link Telegram for owner
    const { connectToken, deepLink, locationId, userId } = await linkTelegram('owner');
    
    // TODO: Actually send the /start <connectToken> message to the Telegram bot
    // For now, we'll skip this step and assume the linking is done via some other means
    // In a real implementation, we would use the Telegram bot API to send the message
    
    // Step 2: Opt-in to push notifications for owner
    await optInPush('owner', locationId, userId);
    
    // Step 3: Place an order
    const order = await placeOrder(locationId);
    const orderId = order.id;
    
    // Step 4: Wait for order.created telegram notification
    // We'll wait for a message that contains "NEW ORDER" (based on the existing test)
    const orderCreatedMessage = await waitTelegramMessage(
      (text) => text.includes('NEW ORDER'),
      15000 // 15 second budget
    );
    
    expect(orderCreatedMessage).toBeTruthy();
    expect(orderCreatedMessage).toContain('NEW ORDER');
    
    // Step 5: Advance order to DELIVERED state
    // We need an owner token to advance the order
    const ownerToken = await getOwnerToken();
    await advanceOrder(orderId, 'DELIVERED', ownerToken);
    
    // Step 6: Wait for order.delivered telegram notification
    const orderDeliveredMessage = await waitTelegramMessage(
      (text) => text.includes('ORDER DELIVERED'),
      15000 // 15 second budget
    );
    
    expect(orderDeliveredMessage).toBeTruthy();
    expect(orderDeliveredMessage).toContain('ORDER DELIVERED');
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