import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';

test.describe('Quick Order Test', () => {
  test('should attempt to create an order and report error', async () => {
    test.setTimeout(15000); // 15 second timeout
    
    console.log('Starting quick order test');
    const BASE_URL = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
    console.log(`Using BASE_URL: ${BASE_URL}`);
    
    // Get owner token
    console.log('Getting owner token...');
    const authRes = await fetch(`${BASE_URL}/api/dev/mock-auth`, { method: 'POST' });
    console.log(`Auth response status: ${authRes.status}`);
    if (!authRes.ok) {
      console.error(`Failed to get owner token: ${await authRes.text()}`);
      throw new Error(`Failed to get owner token`);
    }
    const authBody = await authRes.json();
    const ownerToken = authBody.access_token;
    console.log('Got owner token successfully');
    
    // Create a minimal location
    console.log('Creating location...');
    const startOnboarding = await fetch(`${BASE_URL}/api/owner/onboarding/start`, {
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      method: 'POST',
      body: JSON.stringify({
        name: 'Quick Test Loc',
        phone: '+355600000000',
        slug: `quick-test-${Date.now()}`,
        currency_code: 'ALL',
        default_locale: 'en',
        supported_locales: ['en'],
      }),
    });
    console.log(`Location creation response status: ${startOnboarding.status}`);
    if (!startOnboarding.ok) {
      console.error(`Failed to create location: ${await startOnboarding.text()}`);
      throw new Error(`Failed to create location`);
    }
    const onboardingBody = await startOnboarding.json();
    const locationId = onboardingBody.locationId;
    console.log(`Created location: ${locationId}`);
    
    // Create a product
    console.log('Creating product...');
    const productRes = await fetch(`${BASE_URL}/api/owner/locations/${locationId}/products`, {
      headers: { Authorization: `Bearer ${ownerToken}`, 'Content-Type': 'application/json' },
      method: 'POST',
      body: JSON.stringify({
        name: `Quick Test Product`,
        price: 1000,
        category_id: null,
        available: true,
      }),
    });
    console.log(`Product creation response status: ${productRes.status}`);
    if (!productRes.ok) {
      console.error(`Failed to create product: ${await productRes.text()}`);
      throw new Error(`Failed to create product`);
    }
    const productBody = await productRes.json();
    const productId = productBody.id;
    console.log(`Created product: ${productId}`);
    
    // Try to place an order
    console.log('Placing order...');
    const orderPayload = {
      locationId,
      type: 'delivery',
      items: [{ productId, quantity: 1 }], // Using productId to match working test
      customer: { phone: '+355600000001', name: 'Test Customer' },
      delivery: { address_text: 'Test Street', pin: { lat: 41.3275, lng: 19.8187 } },
      payment: { method: 'cash' },
    };
    
    console.log(`Order payload: ${JSON.stringify(orderPayload)}`);
    
    try {
      const orderRes = await fetch(`${BASE_URL}/api/orders`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(orderPayload),
      });
      console.log(`Order response status: ${orderRes.status}`);
      console.log(`Order response text: ${await orderRes.text()}`);
      
      if (!orderRes.ok) {
        throw new Error(`Failed to place order: ${await orderRes.text()}`);
      }
      
      const orderBody = await orderRes.json();
      console.log(`Order created successfully:`, orderBody);
      expectUuid(orderBody.id);
    } catch (error) {
      console.error(`Error placing order:`, error);
      throw error;
    }
  });
});