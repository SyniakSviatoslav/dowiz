import { test } from '@playwright/test';
import { placeOrder } from '../helpers/notifHelpers';
import { expectUuid } from '../helpers/assert-shape';

test.describe('Debug Order Creation', () => {
  test('should create an order successfully', async () => {
    console.log('Starting debug order test');
    // First get a locationId by creating a minimal location
    const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
    console.log(`Using BASE_URL: ${BASE_URL}`);
    
    // Get owner token
    console.log('Getting owner token...');
    const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
    console.log(`Auth response status: ${authRes.status}`);
    if (!authRes.ok) throw new Error(`Failed to get owner token: ${await authRes.text()}`);
    const authBody = await authRes.json();
    const ownerToken = authBody.access_token;
    console.log('Got owner token successfully');
    
    // Create a minimal location
    console.log('Creating location...');
    const startOnboarding = await fetch(`${BASE_URL}/api/owner/onboarding/start`, {
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      method: 'POST',
      body: JSON.stringify({
        name: 'Debug Loc',
        phone: '+355600000000',
        slug: `debug-loc-${Date.now()}`,
        currency_code: 'ALL',
        default_locale: 'en',
        supported_locales: ['en'],
      }),
    });
    console.log(`Location creation response status: ${startOnboarding.status}`);
    if (!startOnboarding.ok) throw new Error(`Failed to create location: ${await startOnboarding.text()}`);
    const onboardingBody = await startOnboarding.json();
    const locationId = onboardingBody.locationId;
    
    console.log(`Created location: ${locationId}`);
    
    // Try to place an order
    console.log('Placing order...');
    try {
      const order = await placeOrder(locationId);
      console.log(`Order created successfully:`, order);
      expectUuid(order.id);
    } catch (error) {
      console.error(`Failed to create order:`, error);
      throw error;
    }
  });
});