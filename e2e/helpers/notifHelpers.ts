import { test, expect } from '@playwright/test';

// Base URL for the deployed service (staging)
const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

// Telegram bot credentials (should be set in environment variables for security)
const BOT_TOKEN = process.env.***REDACTED*** || '8996764379:AAHkuc5mgYQdkWG5rLZEjHc8a8k5MQsHDIk';
const BOT_SECRET = process.env.***REDACTED*** || 'Ihatenuclearwar';
const CHAT_ID = process.env.TELEGRAM_CHAT_ID ? parseInt(process.env.TELEGRAM_CHAT_ID, 10) : 999999;

// Helper to generate UUIDs
function uuid(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

// Authentication headers for owner/courier/customer
async function authHeaders(token: string): Promise<Record<string, string>> {
  return { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
}

// Send a GET request to Telegram's getUpdates method
async function sendTelegramGetUpdates(offset?: number): Promise<any> {
  const url = `https://api.telegram.org/bot${BOT_TOKEN}/getUpdates${offset ? `?offset=${offset}&limit=100&timeout=0` : ''}`;
  const resp = await fetch(url);
  if (!resp.ok) throw new Error(`Telegram getUpdates failed: ${resp.status}`);
  return await resp.json();
}

// Link Telegram for the given role (owner or courier)
export async function linkTelegram(role: 'owner' | 'courier'): Promise<{ connectToken: string; deepLink: string; locationId: string; userId: string }> {
  // Get auth token for the role
  let authRes;
  if (role === 'owner') {
    authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
  } else if (role === 'courier') {
    // For courier, we'll use the same mock-auth for simplicity in this helper
    authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
  }
  if (!authRes.ok) throw new Error(`Failed to get auth token for ${role}`);
  const authBody = await authRes.json();
  const token = authBody.access_token;
  const userId = authBody.userId;

  // For owner/courier, we can use the onboarding flow to create a location and get a Telegram connect token
  // Create a location
  const startOnboarding = await fetch(`${BASE_URL}/api/owner/onboarding/start`, {
    method: 'POST',
    headers: await authHeaders(token),
    body: JSON.stringify({
      name: `Test Loc for Notifs ${uuid()}`,
      phone: '+355600000000',
      slug: `test-notif-${uuid()}`,
      currency_code: 'ALL',
      default_locale: 'en',
      supported_locales: ['en'],
    }),
  });
  if (!startOnboarding.ok) throw new Error(`Failed to start onboarding: ${await startOnboarding.text()}`);
  const onboardingBody = await startOnboarding.json();
  const locationId = onboardingBody.locationId;

  // Complete onboarding steps 1-6
  for (let step = 1; step <= 6; step++) {
    const stepRes = await fetch(`${BASE_URL}/api/owner/onboarding/${locationId}/step/complete`, {
      method: 'POST',
      headers: await authHeaders(token),
      body: JSON.stringify({ step }),
    });
    if (!stepRes.ok) throw new Error(`Failed to complete onboarding step ${step}: ${await stepRes.text()}`);
  }

  // Skip step 7 (Telegram Alerts) – we will handle Telegram linking separately
  const skip7 = await fetch(`${BASE_URL}/api/owner/onboarding/${locationId}/step/7/skip`, {
    method: 'POST',
    headers: await authHeaders(token),
  });
  if (!skip7.ok) throw new Error(`Failed to skip onboarding step 7: ${await skip7.text()}`);

  // Complete step 8 (Publish & Go Live)
  const finish = await fetch(`${BASE_URL}/api/owner/onboarding/${locationId}/step/complete`, {
    method: 'POST',
    headers: await authHeaders(token),
    body: JSON.stringify({ step: 8 }),
  });
  if (!finish.ok) throw new Error(`Failed to complete onboarding step 8: ${await finish.text()}`);
  const finishBody = await finish.json();
  if (!finishBody.completed) throw new Error('Onboarding did not complete');

  // Now, get the Telegram connect-init token
  const connectInitRes = await fetch(`${BASE_URL}/api/owner/locations/${locationId}/notifications/telegram/connect-init`, {
    method: 'POST',
    headers: await authHeaders(token),
  });
  if (!connectInitRes.ok) throw new Error(`Failed to get Telegram connect-init: ${await connectInitRes.text()}`);
  const connectInitBody = await connectInitRes.json();
  const { token: connectToken, deepLink } = connectInitBody;

  // Return the connection info so the caller can use it to send the Telegram /start message
  return { connectToken, deepLink, locationId, userId };
}

// Opt-in to push notifications for the given role (owner or customer)
export async function optInPush(role: 'owner' | 'customer', locationId: string, userId: string): Promise<void> {
  // Get auth token - we'll need to get it again since we don't have it stored
  let token;
  if (role === 'owner') {
    const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
    if (!authRes.ok) throw new Error(`Failed to get owner token for push opt-in`);
    token = (await authRes.json()).access_token;
  } else if (role === 'customer') {
    // For customer, we would need customer credentials
    // We'll leave this as a placeholder for now
    throw new Error('optInPush for customer not implemented yet');
  }

  if (role === 'owner') {
    // For owner, we need to subscribe to push notifications
    // We'll create a dummy push subscription (in real test, we would use the Push API to get a subscription)
    // For simplicity, we'll use the owner/push/subscribe endpoint with a dummy subscription
    const subscription = {
      endpoint: 'https://example.com/push',
      keys: {
        p256dh: 'dummy_key',
        auth: 'dummy_auth'
      }
    };
    const res = await fetch(`${BASE_URL}/api/owner/locations/${locationId}/push/subscribe`, {
      method: 'POST',
      headers: await authHeaders(token),
      body: JSON.stringify({ subscription }),
    });
    if (!res.ok) throw new Error(`Failed to opt-in to push: ${await res.text()}`);
  }
}

// Place a test order and return order details
export async function placeOrder(locationId: string, customerPhone: string = '+355600000001'): Promise<any> {
  // Create a product first (we need a product to order)
  // We'll get an owner token to create a product
  const ownerToken = await getOwnerToken();
  const productRes = await fetch(`${BASE_URL}/api/owner/locations/${locationId}/products`, {
    method: 'POST',
    headers: await authHeaders(ownerToken),
    body: JSON.stringify({
      name: `Test Product ${uuid()}`,
      price: 1000,
      category_id: null,
      available: true,
    }),
  });
  if (!productRes.ok) throw new Error(`Failed to create product: ${await productRes.text()}`);
  const productBody = await productRes.json();
  const productId = productBody.id;

   // Create the order
const orderPayload = {
      locationId,
      type: 'delivery',
      items: [{ product_id: productId, quantity: 1 }],
      customer: { phone: customerPhone, name: 'Test Customer' },
      delivery: { address_text: 'Test Street', pin: { lat: 41.3275, lng: 19.8187 } },
      payment: { method: 'cash' },
      // No idempotency key to avoid conflict
    };
  const orderRes = await fetch(`${BASE_URL}/api/orders`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(orderPayload),
  });
  if (!orderRes.ok) throw new Error(`Failed to place order: ${await orderRes.text()}`);
  const orderBody = await orderRes.json();
  return { ...orderBody, productId };
}

// Advance order to a given state
export async function advanceOrder(orderId: string, state: string, ownerToken: string): Promise<void> {
  // We'll advance the order by updating its status via the owner PATCH /orders/:id/status endpoint
  // Note: not all states can be set directly; some require specific actions
  // For simplicity, we'll support a few states: CONFIRMED, PREPARING, READY, IN_DELIVERY, DELIVERED, REJECTED, CANCELLED
  const res = await fetch(`${BASE_URL}/api/orders/${orderId}/status`, {
    method: 'PATCH',
    headers: await authHeaders(ownerToken),
    body: JSON.stringify({ status: state }),
  });
  if (!res.ok) throw new Error(`Failed to advance order to ${state}: ${await res.text()}`);
}

// Open shift for courier
export async function openShift(courierToken: string, locationId: string): Promise<void> {
  // We'll use the courier shift start endpoint
  const res = await fetch(`${BASE_URL}/api/courier/me/shift/start`, {
    method: 'POST',
    headers: await authHeaders(courierToken),
    body: JSON.stringify({ locationId }),
  });
  if (!res.ok) throw new Error(`Failed to open shift: ${await res.text()}`);
}

// Close shift for courier with cash amount
export async function closeShift(courierToken: string, locationId: string, cashAmount: number): Promise<void> {
  // We'll use the courier shift end endpoint
  const res = await fetch(`${BASE_URL}/api/courier/me/shift/end`, {
    method: 'POST',
    headers: await authHeaders(courierToken),
    body: JSON.stringify({ locationId, cashPaid: cashAmount }),
  });
  if (!res.ok) throw new Error(`Failed to close shift: ${await res.text()}`);
}

let lastUpdateId: number = 0;

// Delete webhook before testing to avoid conflicts with getUpdates
export async function deleteTelegramWebhook(): Promise<void> {
  const deleteUrl = `https://api.telegram.org/bot${BOT_TOKEN}/deleteWebhook`;
  const resp = await fetch(deleteUrl, { method: 'POST' });
  if (!resp.ok) throw new Error(`Failed to delete Telegram webhook: ${resp.status}`);
}

// Clear pending updates and establish baseline lastUpdateId
export async function clearTelegramUpdates(): Promise<void> {
  const updates = await sendTelegramGetUpdates();
  if (updates.result && updates.result.length > 0) {
    lastUpdateId = updates.result[updates.result.length - 1].update_id + 1;
  }
}

// Wait for a Telegram message matching the matcher within budget time (in ms)
// Uses offset-based polling (not continuous) to avoid 409 conflicts with webhook
export async function waitTelegramMessage(
  matcher: (text: string) => boolean,
  budget: number = 10000,
  checkIntervalMs: number = 2000
): Promise<string> {
  const endTime = Date.now() + budget;
  
  while (Date.now() < endTime) {
    try {
      const updates = await sendTelegramGetUpdates(lastUpdateId + 1);
      if (updates.result && updates.result.length > 0) {
        for (const upd of updates.result) {
          if (upd.message && upd.message.chat?.id === CHAT_ID && upd.message.text && matcher(upd.message.text)) {
            lastUpdateId = upd.update_id + 1;
            return upd.message.text;
          }
        }
        lastUpdateId = updates.result[updates.result.length - 1].update_id + 1;
      }
    } catch (err) {
      console.error(`Error checking Telegram updates: ${err}`);
    }
    await new Promise(res => setTimeout(res, checkIntervalMs));
  }
  throw new Error(`Timeout waiting for Telegram message matching matcher`);
}

// Wait for a push notification matching the matcher within budget time
// This is more complex because we need to set up a push notification listener in the test.
// We'll leave a placeholder for now.
export async function waitPush(matcher: (payload: any) => boolean, budget: number = 10000): Promise<any> {
  throw new Error('waitPush not implemented yet');
}

// Wait for a WebSocket event in the given room of the given type
export async function waitWsEvent(page: any, room: string, type: string, budget: number = 10000): Promise<any> {
  // We'll use the page.evaluate to listen to WebSocket messages
  // This is complex; we'll leave a placeholder
  throw new Error('waitWsEvent not implemented yet');
}

// Tap an inline button in a Telegram message that matches the matcher
export async function tapTelegramInlineButton(matcher: (text: string) => boolean): Promise<void> {
  // We'll need to get the Telegram message, find the button, and tap it via the Telegram API
  // This is complex; we'll leave a placeholder
  throw new Error('tapTelegramInlineButton not implemented yet');
}

// Cleanup test data (orders, locations, etc.)
export async function cleanup(): Promise<void> {
  // We'll need to delete the test locations, orders, products, etc.
// This is complex; we'll leave a placeholder for now
  throw new Error('cleanup not implemented yet');
}

// Helper to get an owner token (mock-auth)
async function getOwnerToken(): Promise<string> {
  const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
  if (!authRes.ok) throw new Error(`Failed to get owner token: ${await authRes.text()}`);
  const authBody = await authRes.json();
  return authBody.access_token;
}