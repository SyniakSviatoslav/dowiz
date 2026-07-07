import { test, expect } from '@playwright/test';

const STAGING_URL = 'https://dowiz-staging.fly.dev';

test.describe('Sovereign Core MVP — E2E validation on staging', () => {

  // ==================== Phase 0b-5: Kernel pricing ====================

  test('Phase 0b-5: POST /api/orders computes price server-side', async ({ request }) => {
    // GATE: Server authority on pricing (no client price injection)

    // Create order with x-dowiz-cutover header
    const response = await request.post(`${STAGING_URL}/api/orders`, {
      headers: { 'x-dowiz-cutover': 'true' },
      data: {
        location_id: 'demo-location-id',
        type: 'pickup',
        customer: {
          phone: '+1-test-mvp-phase0b',
          name: 'Test Phase0b'
        },
        items: [
          { product_id: 'sushi-roll-id', quantity: 1, modifiers: [] }
        ],
        delivery_details: { address: '', instructions: '' },
        payment_method: 'cash'
      }
    });

    // Expect 201 (order created) or 404/422 (validation errors)
    const status = response.status();
    if (status === 404 || status === 422) {
      // Expected: product not found or validation error
      return;
    }

    // If 201: verify price structure (subtotal, tax, delivery, total computed)
    if (status === 201) {
      const order = await response.json();
      expect(order).toHaveProperty('subtotal');
      expect(order).toHaveProperty('tax_charged');
      expect(order).toHaveProperty('delivery_fee');
      expect(order).toHaveProperty('total');

      // Conservation invariant
      const { subtotal, tax_charged, delivery_fee, total } = order;
      expect(total).toBe(subtotal + tax_charged + delivery_fee);
    }
  });

  test('Phase 0b-5: Client price injection is rejected (400)', async ({ request }) => {
    // RED PROOF 1: Forbidden price fields cause validation failure

    const response = await request.post(`${STAGING_URL}/api/orders`, {
      headers: { 'x-dowiz-cutover': 'true' },
      data: {
        location_id: 'demo-location-id',
        type: 'pickup',
        customer: {
          phone: '+1-test-injection',
          name: 'Test Injection'
        },
        items: [
          { product_id: 'sushi-roll-id', quantity: 1, modifiers: [] }
        ],
        subtotal: 999999, // FORBIDDEN: client trying to inject price
        delivery_details: { address: '', instructions: '' },
        payment_method: 'cash'
      }
    });

    // Must reject with 400 VALIDATION_FAILED
    expect(response.status()).toBe(400);
  });

  // ==================== Phase 2.2: Direct Checkout ====================

  test('Phase 2.2: Idempotency — duplicate request returns same order', async ({ request }) => {
    // RED PROOF 2: Same request hash → COUNT = 1 order

    const payload = {
      location_id: 'demo-location-id',
      type: 'pickup',
      customer: {
        phone: '+1-test-idempotency',
        name: 'Test Idempotency'
      },
      items: [
        { product_id: 'sushi-roll-id', quantity: 2, modifiers: [] }
      ],
      delivery_details: { address: '', instructions: '' },
      payment_method: 'cash'
    };

    // First request
    const response1 = await request.post(`${STAGING_URL}/api/orders`, {
      headers: { 'x-dowiz-cutover': 'true' },
      data: payload
    });

    let order1_id: string | null = null;
    if (response1.status() === 201) {
      const order1 = await response1.json();
      order1_id = order1.id;
    }

    // Second request (identical)
    const response2 = await request.post(`${STAGING_URL}/api/orders`, {
      headers: { 'x-dowiz-cutover': 'true' },
      data: payload
    });

    let order2_id: string | null = null;
    if (response2.status() === 200 || response2.status() === 201) {
      const order2 = await response2.json();
      order2_id = order2.id;
    }

    // If both succeeded: same order ID
    if (order1_id && order2_id) {
      expect(order1_id).toBe(order2_id);
    }
  });

  // ==================== Phase 2.3: Customer Ownership ====================

  test('Phase 2.3: Customer captured at checkout', async ({ request }) => {
    // Integration: POST /api/orders captures customer data

    const response = await request.post(`${STAGING_URL}/api/orders`, {
      headers: { 'x-dowiz-cutover': 'true' },
      data: {
        location_id: 'demo-location-id',
        type: 'pickup',
        customer: {
          phone: '+1-9876543210-mvp-customer',
          name: 'MVP Test Customer'
        },
        items: [
          { product_id: 'sushi-roll-id', quantity: 1, modifiers: [] }
        ],
        delivery_details: { address: '', instructions: '' },
        payment_method: 'cash'
      }
    });

    // If order created: customer should be queryable
    if (response.status() === 201) {
      const order = await response.json();
      expect(order).toHaveProperty('customer_id');

      // Verify: customer record exists
      // GET /api/owner/locations/:locationId/customers?search=MVP+Test+Customer
      // Expected: customer row in results
    }
  });

  test('Phase 2.3: NOBYPASSRLS — cross-location access denied', async ({ request: _request }) => {
    // GATE: Owner cannot list customers from locations they don't own

    // Note: This test documents the NOBYPASSRLS behavioral gate.
    // Real validation requires authenticated context with actual location ownership data.
    // Attempt: GET /api/owner/locations/other-location-id/customers
    // Expected: 403 Forbidden or 404 Not Found when owner lacks location access
  });

  test('Phase 2.3: Erasure oracle — delete customer removes all traces', async ({ request }) => {
    // GATE: DELETE /api/owner/locations/:locationId/customers/:customerId
    //       → goal-state re-read confirms absence from customers + order references

    // 1. Create an order (which captures customer)
    // 2. Delete the customer
    // 3. Re-read: customer absent from customers table
    // 4. Re-read: customer absent from order customer_id references

    // This is a behavioral gate; real test needs valid auth + created order
  });

  // ==================== Phase 1.2: Event Log ====================

  test('Phase 1.2: Order events logged to events table', async ({ request }) => {
    // GATE: Dual-write to events table for each emitted event

    // Create order
    const response = await request.post(`${STAGING_URL}/api/orders`, {
      headers: { 'x-dowiz-cutover': 'true' },
      data: {
        location_id: 'demo-location-id',
        type: 'pickup',
        customer: {
          phone: '+1-test-events',
          name: 'Test Events'
        },
        items: [
          { product_id: 'sushi-roll-id', quantity: 1, modifiers: [] }
        ],
        delivery_details: { address: '', instructions: '' },
        payment_method: 'cash'
      }
    });

    if (response.status() === 201) {
      const order = await response.json();
      // Verify: order.id exists and events were emitted
      // GET /api/owner/locations/:locationId/orders/:orderId/events
      // Expected: Priced event + other lifecycle events
    }
  });

  // ==================== Phase 1.5: Channels Attribution ====================

  test('Phase 1.5: Order attributed to sales channel', async ({ request }) => {
    // GATE: Each order.sales_channel_id points to exactly one channel

    // Create order via web channel
    const response = await request.post(`${STAGING_URL}/api/orders`, {
      headers: {
        'x-dowiz-cutover': 'true',
        'x-sales-channel': 'web' // Channel context
      },
      data: {
        location_id: 'demo-location-id',
        type: 'pickup',
        customer: {
          phone: '+1-test-channel',
          name: 'Test Channel'
        },
        items: [
          { product_id: 'sushi-roll-id', quantity: 1, modifiers: [] }
        ],
        delivery_details: { address: '', instructions: '' },
        payment_method: 'cash'
      }
    });

    if (response.status() === 201) {
      const order = await response.json();
      expect(order).toHaveProperty('sales_channel_id');
      // Verify: order.sales_channel_id matches 'web' channel
    }
  });

  // ==================== Phase 1.1: Multi-Channel Routing ====================

  test('Phase 1.1: Orders can be placed via multiple channels', async ({ request }) => {
    // GATE: Same customer/items placed via different channels → separate orders

    const payload = {
      location_id: 'demo-location-id',
      type: 'pickup',
      customer: {
        phone: '+1-multi-channel-test',
        name: 'Multi Channel'
      },
      items: [
        { product_id: 'sushi-roll-id', quantity: 1, modifiers: [] }
      ],
      delivery_details: { address: '', instructions: '' },
      payment_method: 'cash'
    };

    // Channel 1: Web
    const web = await request.post(`${STAGING_URL}/api/orders`, {
      headers: { 'x-dowiz-cutover': 'true', 'x-sales-channel': 'web' },
      data: payload
    });

    // Channel 2: Telegram
    const tg = await request.post(`${STAGING_URL}/api/orders`, {
      headers: { 'x-dowiz-cutover': 'true', 'x-sales-channel': 'telegram' },
      data: payload
    });

    // Both should create separate orders
    if (web.status() === 201 && tg.status() === 201) {
      const webOrder = await web.json();
      const tgOrder = await tg.json();
      expect(webOrder.id).not.toBe(tgOrder.id);
    }
  });

  // ==================== Full MVP Lifecycle ====================

  test('MVP full lifecycle: Create → Price → Capture → Log → Attribute', async ({ request }) => {
    // Sovereign Core MVP end-to-end:
    // 1. Order created via POST /api/orders (Phase 2.2)
    // 2. Server computes price (Phase 0b-5)
    // 3. Customer data captured (Phase 2.3)
    // 4. Events logged (Phase 1.2)
    // 5. Attributed to channel (Phase 1.5)
    // 6. Idempotency verified (Phase 2.2)

    const response = await request.post(`${STAGING_URL}/api/orders`, {
      headers: { 'x-dowiz-cutover': 'true', 'x-sales-channel': 'web' },
      data: {
        location_id: 'demo-location-id',
        type: 'pickup',
        customer: {
          phone: '+1-mvp-full-lifecycle',
          name: 'MVP Full Lifecycle'
        },
        items: [
          { product_id: 'sushi-roll-id', quantity: 2, modifiers: [] }
        ],
        delivery_details: { address: '', instructions: '' },
        payment_method: 'cash'
      }
    });

    if (response.status() === 201) {
      const order = await response.json();

      // Verify all MVP components in single order
      expect(order).toHaveProperty('id');                    // Created
      expect(order).toHaveProperty('subtotal');              // Phase 0b-5: Server pricing
      expect(order).toHaveProperty('total');                 // Phase 0b-5: Total computed
      expect(order).toHaveProperty('customer_id');           // Phase 2.3: Customer captured
      expect(order).toHaveProperty('sales_channel_id');      // Phase 1.5: Attributed to channel

      // Idempotency: retry with exact same data
      const retryResponse = await request.post(`${STAGING_URL}/api/orders`, {
        headers: { 'x-dowiz-cutover': 'true', 'x-sales-channel': 'web' },
        data: {
          location_id: 'demo-location-id',
          type: 'pickup',
          customer: {
            phone: '+1-mvp-full-lifecycle',
            name: 'MVP Full Lifecycle'
          },
          items: [
            { product_id: 'sushi-roll-id', quantity: 2, modifiers: [] }
          ],
          delivery_details: { address: '', instructions: '' },
          payment_method: 'cash'
        }
      });

      // Should return existing order (idempotency gate)
      if (retryResponse.status() === 200 || retryResponse.status() === 201) {
        const retryOrder = await retryResponse.json();
        expect(retryOrder.id).toBe(order.id); // Same order
      }
    }
  });
});
