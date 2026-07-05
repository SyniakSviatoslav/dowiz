import { test } from 'node:test';
import assert from 'node:assert';
import crypto from 'node:crypto';
import { loadEnv } from '@deliveryos/config';
import { createSessionPool } from '@deliveryos/db';
import { signAuthToken } from '@deliveryos/platform';
import { evaluatePreflight, type PreflightInput } from '../src/lib/preflight.js';

const env = loadEnv();
const BASE = `http://127.0.0.1:${env.PORT || 3003}`;

function makeInput(overrides: Partial<PreflightInput> = {}): PreflightInput {
  return {
    lines: [],
    signals: {
      velocityPhoneCount: 0,
      velocityIpCount: 0,
      noShowCount: 0,
      noShowAgeDays: null,
      completedCount: 0,
      otpRequired: false,
      otpVerified: false,
    },
    acknowledgedCodes: [],
    ...overrides,
  };
}

function delay(ms: number) { return new Promise(r => setTimeout(r, ms)); }

test('Stage 27: Preflight Soft-Confirm Engine', async (t) => {
  const pool = createSessionPool();
  const orgId = crypto.randomUUID();
  const locId = crypto.randomUUID();
  const userId = crypto.randomUUID();
  const custId = crypto.randomUUID();
  const prodId = crypto.randomUUID();
  const prodIdUnavail = crypto.randomUUID();

  let ownerToken: string;

  // ─── Setup ────────────────────────────────────────────────────────
  await t.test('setup test data', async () => {
    await pool.query(`INSERT INTO users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING`,
      [userId, `owner-p27-${Date.now()}@test.com`]);
    await pool.query(`INSERT INTO organizations (id, name, owner_id) VALUES ($1, 'P27 Org', $2) ON CONFLICT DO NOTHING`,
      [orgId, userId]);
    await pool.query(`INSERT INTO locations (id, org_id, slug, name, phone, status, confirm_timeout_min, require_phone_otp, currency_code, currency_minor_unit, tax_rate, price_includes_tax, min_order_value, free_delivery_threshold, delivery_fee_flat)
      VALUES ($1, $2, $3, 'P27 Loc', '123', 'open', 10, false, 'ALL', 0, 0, true, null, null, 200) ON CONFLICT DO NOTHING`,
      [locId, orgId, `p27-loc-${Date.now()}`]);
    await pool.query(`INSERT INTO customers (id, location_id, phone, name, no_show_count, completed_count, last_no_show_at)
      VALUES ($1, $2, '+355691234567', 'Test Customer', 0, 0, null) ON CONFLICT DO NOTHING`,
      [custId, locId]);
    await pool.query(`INSERT INTO products (id, location_id, name, price, available)
      VALUES ($1, $2, 'Available Product', 500, true) ON CONFLICT DO NOTHING`,
      [prodId, locId]);
    await pool.query(`INSERT INTO products (id, location_id, name, price, available)
      VALUES ($1, $2, 'Unavailable Product', 500, false) ON CONFLICT DO NOTHING`,
      [prodIdUnavail, locId]);

    ownerToken = await signAuthToken({ role: 'owner', userId, activeLocationId: locId }, '15m');
  });

  // ═══════════════════════════════════════════════════════════════════
  // PURE FUNCTION: evaluatePreflight — ALL BRANCHES
  // ═══════════════════════════════════════════════════════════════════

  await t.test('evaluatePreflight: clean order → outcome clean, no reasons', () => {
    const result = evaluatePreflight(makeInput());
    assert.strictEqual(result.outcome, 'clean');
    assert.strictEqual(result.reasons.length, 0);
  });

  await t.test('evaluatePreflight: product not in menu → hard_block', () => {
    const result = evaluatePreflight(makeInput({
      lines: [{
        productId: 'abc', quantity: 1, modifierIds: [],
        productAvailable: null, modifierAvailability: {},
      }],
    }));
    assert.strictEqual(result.outcome, 'hard_block');
    assert.strictEqual(result.reasons.length, 1);
    assert.strictEqual(result.reasons[0].code, 'item_unavailable');
    assert.strictEqual(result.reasons[0].severity, 'objective');
    assert.strictEqual(result.reasons[0].itemId, 'abc');
  });

  await t.test('evaluatePreflight: product in stop-list → hard_block', () => {
    const result = evaluatePreflight(makeInput({
      lines: [{
        productId: 'abc', quantity: 1, modifierIds: [],
        productAvailable: false, modifierAvailability: {},
      }],
    }));
    assert.strictEqual(result.outcome, 'hard_block');
    assert.strictEqual(result.reasons[0].code, 'item_unavailable');
    assert.strictEqual(result.reasons[0].message, 'Item is currently unavailable (stop-list).');
  });

  await t.test('evaluatePreflight: modifier not in menu → hard_block', () => {
    const result = evaluatePreflight(makeInput({
      lines: [{
        productId: 'abc', quantity: 1, modifierIds: ['mod1'],
        productAvailable: true,
        modifierAvailability: { mod1: null },
      }],
    }));
    assert.strictEqual(result.outcome, 'hard_block');
    assert.strictEqual(result.reasons[0].itemId, 'mod1');
    assert.strictEqual(result.reasons[0].message, 'Modifier is not in the menu.');
  });

  await t.test('evaluatePreflight: modifier in stop-list → hard_block', () => {
    const result = evaluatePreflight(makeInput({
      lines: [{
        productId: 'abc', quantity: 1, modifierIds: ['mod1'],
        productAvailable: true,
        modifierAvailability: { mod1: false },
      }],
    }));
    assert.strictEqual(result.outcome, 'hard_block');
    assert.strictEqual(result.reasons[0].message, 'Modifier is currently unavailable (stop-list).');
  });

  await t.test('evaluatePreflight: phone velocity exceeded → soft_confirm', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 5, velocityIpCount: 0,
        noShowCount: 0, noShowAgeDays: null, completedCount: 0,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(result.outcome, 'soft_confirm');
    assert.strictEqual(result.requiresConfirmation, true);
    assert.strictEqual(result.requiresOtp, false);
    assert.ok(result.reasons.some(r => r.code === 'velocity'));
  });

  await t.test('evaluatePreflight: IP velocity exceeded → soft_confirm', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 5,
        noShowCount: 0, noShowAgeDays: null, completedCount: 0,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(result.outcome, 'soft_confirm');
    assert.ok(result.reasons.some(r => r.code === 'velocity'));
  });

  await t.test('evaluatePreflight: active no-show history → soft_confirm', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 0,
        noShowCount: 3, noShowAgeDays: 1, completedCount: 5,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(result.outcome, 'soft_confirm');
    assert.ok(result.reasons.some(r => r.code === 'no_show_history'));
    assert.ok(result.reasons.some(r => r.message.includes('3 no-show')));
  });

  await t.test('evaluatePreflight: OTP required → soft_confirm with requiresOtp', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 0,
        noShowCount: 0, noShowAgeDays: null, completedCount: 0,
        otpRequired: true, otpVerified: false,
      },
    }));
    assert.strictEqual(result.outcome, 'soft_confirm');
    assert.strictEqual(result.requiresOtp, true);
    assert.ok(result.reasons.some(r => r.code === 'otp_required'));
  });

  await t.test('evaluatePreflight: ack all + no OTP → clean with confirmedReasons', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 5, velocityIpCount: 0,
        noShowCount: 0, noShowAgeDays: null, completedCount: 0,
        otpRequired: false, otpVerified: false,
      },
      acknowledgedCodes: ['velocity'],
    }));
    assert.strictEqual(result.outcome, 'clean');
    assert.ok(result.confirmedReasons?.includes('velocity'));
  });

  await t.test('evaluatePreflight: OTP verified + ack → clean', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 5, velocityIpCount: 0,
        noShowCount: 0, noShowAgeDays: null, completedCount: 0,
        otpRequired: true, otpVerified: true,
      },
      acknowledgedCodes: ['velocity'],
    }));
    assert.strictEqual(result.outcome, 'clean');
    assert.ok(result.confirmedReasons?.includes('velocity'));
  });

  await t.test('evaluatePreflight: OTP required + not verified + ack → still soft_confirm', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 5, velocityIpCount: 0,
        noShowCount: 0, noShowAgeDays: null, completedCount: 0,
        otpRequired: true, otpVerified: false,
      },
      acknowledgedCodes: ['velocity'],
    }));
    assert.strictEqual(result.outcome, 'soft_confirm');
    assert.strictEqual(result.requiresOtp, true);
  });

  await t.test('evaluatePreflight: no-show ack + no OTP → clean', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 0,
        noShowCount: 3, noShowAgeDays: 1, completedCount: 5,
        otpRequired: false, otpVerified: false,
      },
      acknowledgedCodes: ['no_show_history'],
    }));
    assert.strictEqual(result.outcome, 'clean');
    assert.ok(result.confirmedReasons?.includes('no_show_history'));
  });

  await t.test('evaluatePreflight: partial ack (missing velocity) → still soft_confirm', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 5, velocityIpCount: 0,
        noShowCount: 3, noShowAgeDays: 1, completedCount: 5,
        otpRequired: false, otpVerified: false,
      },
      acknowledgedCodes: ['no_show_history'],
    }));
    assert.strictEqual(result.outcome, 'soft_confirm');
    assert.strictEqual(result.requiresConfirmation, true);
  });

  await t.test('evaluatePreflight: mixed hard_block + soft → hard_block wins', () => {
    const result = evaluatePreflight(makeInput({
      lines: [{
        productId: 'abc', quantity: 1, modifierIds: [],
        productAvailable: null, modifierAvailability: {},
      }],
      signals: {
        velocityPhoneCount: 5, velocityIpCount: 0,
        noShowCount: 0, noShowAgeDays: null, completedCount: 0,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(result.outcome, 'hard_block');
    assert.strictEqual(result.reasons.length, 1);
    assert.strictEqual(result.reasons[0].code, 'item_unavailable');
  });

  // ═══════════════════════════════════════════════════════════════════
  // NO-SHOW DECAY: calcNoShowStrength (via evaluatePreflight)
  // ═══════════════════════════════════════════════════════════════════

  await t.test('calcNoShowStrength: recent no-show (1 day) → triggers soft_confirm', () => {
    // 3 no-shows, 5 completed, 1 day ago → strength = 3 * exp(-1/30) / 5 ≈ 0.58 > 0.5
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 0,
        noShowCount: 3, noShowAgeDays: 1, completedCount: 5,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(result.outcome, 'soft_confirm');
    assert.ok(result.reasons.some(r => r.code === 'no_show_history'));
  });

  await t.test('calcNoShowStrength: moderate age (30 days) → may or may not trigger depending on count', () => {
    // 5 no-shows, 5 completed, 30 days ago → strength = 5 * exp(-30/30) / 5 = 0.368 < 0.5
    const weakResult = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 0,
        noShowCount: 5, noShowAgeDays: 30, completedCount: 5,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(weakResult.outcome, 'clean');

    // 10 no-shows, 5 completed, 30 days ago → strength = 10 * exp(-30/30) / 5 = 0.736 > 0.5
    const strongResult = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 0,
        noShowCount: 10, noShowAgeDays: 30, completedCount: 5,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(strongResult.outcome, 'soft_confirm');
  });

  await t.test('calcNoShowStrength: old no-show (60 days) → strength below threshold', () => {
    // 3 no-shows, 5 completed, 60 days ago → strength = 3 * exp(-60/30) / 5 = 3 * 0.135 / 5 = 0.081 < 0.5
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 0,
        noShowCount: 3, noShowAgeDays: 60, completedCount: 5,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(result.outcome, 'clean');
  });

  await t.test('calcNoShowStrength: null age → no signal', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 0,
        noShowCount: 3, noShowAgeDays: null, completedCount: 5,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(result.outcome, 'clean');
  });

  await t.test('calcNoShowStrength: outside 90-day window → no signal (age > 90)', () => {
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 0,
        noShowCount: 3, noShowAgeDays: 95, completedCount: 5,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(result.outcome, 'clean');
  });

  await t.test('calcNoShowStrength: high count but many completed → ratio dampens signal', () => {
    // 5 no-shows, 50 completed, 1 day ago → strength = 5 * exp(-1/30) / 50 ≈ 0.097 < 0.5
    const result = evaluatePreflight(makeInput({
      signals: {
        velocityPhoneCount: 0, velocityIpCount: 0,
        noShowCount: 5, noShowAgeDays: 1, completedCount: 50,
        otpRequired: false, otpVerified: false,
      },
    }));
    assert.strictEqual(result.outcome, 'clean');
  });

  // ═══════════════════════════════════════════════════════════════════
  // INTEGRATION: POST /orders preflight flow
  // ═══════════════════════════════════════════════════════════════════

  await t.test('POST /orders: hard_block returns 422, idempotency key NOT consumed', async () => {
    const idempKey = crypto.randomUUID();

    // Use the actual POST /orders endpoint (anonymous, no auth required per route definition)
    const hardRes = await fetch(`${BASE}/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        locationId: locId,
        type: 'delivery',
        items: [{ product_id: prodIdUnavail, quantity: 1 }],
        customer: { phone: '+355691234567', name: 'Test' },
        delivery: { pin: { lat: 41.3275, lng: 19.8187 }, address_text: 'Test address' },
        payment: { method: 'cash' },
        idempotency_key: idempKey,
      }),
    });
    assert.strictEqual(hardRes.status, 422);
    const hardBody = await hardRes.json();
    assert.strictEqual(hardBody.outcome, 'hard_block');
    assert.ok(hardBody.reasons.length > 0);
    assert.strictEqual(hardBody.reasons[0].code, 'item_unavailable');

    // Now send a valid order with the SAME idempotency key — should succeed (preflight did not consume it)
    const cleanRes = await fetch(`${BASE}/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        locationId: locId,
        type: 'delivery',
        items: [{ product_id: prodId, quantity: 1 }],
        customer: { phone: '+355691234567', name: 'Test' },
        delivery: { pin: { lat: 41.3275, lng: 19.8187 }, address_text: 'Test address' },
        payment: { method: 'cash' },
        idempotency_key: idempKey,
      }),
    });
    assert.strictEqual(cleanRes.status, 201, `Expected 201, got ${cleanRes.status}: ${await cleanRes.text()}`);
    const cleanBody = await cleanRes.json();
    assert.ok(cleanBody.id);
    assert.strictEqual(cleanBody.preflight.outcome, 'clean');
  });

  await t.test('POST /orders: soft_confirm returns 200, does NOT consume idempotency key', async () => {
    // Insert velocity events to trigger soft_confirm
    const phone = '+355699990001';
    const phoneHash = crypto.createHash('sha256').update(phone.replace(/\D/g, '')).digest('hex');

    // Insert 4 velocity events (threshold is 3) within the last hour
    for (let i = 0; i < 4; i++) {
      await pool.query(
        `INSERT INTO velocity_events (location_id, phone_hash, kind, window_started_at)
         VALUES ($1, $2, 'order_placed', now() - interval '10 minutes')`,
        [locId, phoneHash],
      );
    }

    await delay(200);

    const idempKey = crypto.randomUUID();

    // Request with high velocity phone → soft_confirm (no acknowledged_codes)
    const softRes = await fetch(`${BASE}/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        locationId: locId,
        type: 'delivery',
        items: [{ product_id: prodId, quantity: 1 }],
        customer: { phone, name: 'Velocity Test' },
        delivery: { pin: { lat: 41.3275, lng: 19.8187 }, address_text: 'Test address' },
        payment: { method: 'cash' },
        idempotency_key: idempKey,
      }),
    });
    assert.strictEqual(softRes.status, 200);
    const softBody = await softRes.json();
    assert.strictEqual(softBody.outcome, 'soft_confirm');
    assert.strictEqual(softBody.requiresConfirmation, true);
    assert.ok(softBody.reasons.some((r: any) => r.code === 'velocity'));

    // Same idempotency key with acknowledged_codes should now succeed as clean
    const ackRes = await fetch(`${BASE}/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        locationId: locId,
        type: 'delivery',
        items: [{ product_id: prodId, quantity: 1 }],
        customer: { phone, name: 'Velocity Test' },
        delivery: { pin: { lat: 41.3275, lng: 19.8187 }, address_text: 'Test address' },
        payment: { method: 'cash' },
        idempotency_key: idempKey,
        acknowledged_codes: ['velocity'],
      }),
    });
    assert.strictEqual(ackRes.status, 201, `Expected 201, got ${ackRes.status}: ${await ackRes.text()}`);
    const ackBody = await ackRes.json();
    assert.ok(ackBody.id);
    assert.strictEqual(ackBody.preflight.outcome, 'clean');

    // Cleanup velocity events
    await pool.query(`DELETE FROM velocity_events WHERE location_id = $1 AND phone_hash = $2`, [locId, phoneHash]);
  });

  await t.test('POST /orders: clean returns 201 with preflight info', async () => {
    const idempKey = crypto.randomUUID();

    const res = await fetch(`${BASE}/orders`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        locationId: locId,
        type: 'delivery',
        items: [{ product_id: prodId, quantity: 2 }],
        customer: { phone: '+355691234567', name: 'Clean Test' },
        delivery: { pin: { lat: 41.3275, lng: 19.8187 }, address_text: 'Test address' },
        payment: { method: 'cash' },
        idempotency_key: idempKey,
      }),
    });
    assert.strictEqual(res.status, 201, `Expected 201, got ${res.status}: ${await res.text()}`);
    const body = await res.json();
    assert.ok(body.id);
    assert.strictEqual(body.status, 'PENDING');
    assert.strictEqual(body.preflight.outcome, 'clean');
    assert.ok(Array.isArray(body.preflight.reasons));
    assert.ok(Array.isArray(body.preflight.confirmedReasons));
  });

  // ═══════════════════════════════════════════════════════════════════
  // CLEANUP
  // ═══════════════════════════════════════════════════════════════════
  await t.test('cleanup test data', async () => {
    await pool.query(`DELETE FROM velocity_events WHERE location_id = $1`, [locId]);
    await pool.query(`DELETE FROM order_items WHERE order_id IN (SELECT id FROM orders WHERE location_id = $1)`, [locId]);
    await pool.query(`DELETE FROM idempotency_keys WHERE location_id = $1`, [locId]);
    await pool.query(`DELETE FROM orders WHERE location_id = $1`, [locId]);
    await pool.query(`DELETE FROM products WHERE id IN ($1, $2)`, [prodId, prodIdUnavail]);
    await pool.query(`DELETE FROM customers WHERE id = $1`, [custId]);
    await pool.query(`DELETE FROM locations WHERE id = $1`, [locId]);
    await pool.query(`DELETE FROM organizations WHERE id = $1`, [orgId]);
    await pool.query(`DELETE FROM users WHERE id = $1`, [userId]);
  });

  await pool.end();
});
