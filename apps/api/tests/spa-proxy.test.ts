import './_env-stub.js';
import { test, describe } from 'node:test';
import assert from 'node:assert';
import { z } from 'zod';
import { validateImageKey } from '../src/lib/image-key.js';
import { menuProductCreateSchema } from '../src/routes/owner/products.js';
import { categoryCreateSchema } from '../src/routes/owner/categories.js';

// These tests validate the Zod schemas used by the owner menu + spa-proxy endpoints.
// They verify validation WITHOUT a running server by importing the REAL schemas
// (not drifting local copies) — Test Integrity: a green schema test is worthless if
// it asserts a shape production never uses.
//
// productSchema / categorySchema are imported from their owning routes.
// validateImageKey is imported from its lib.
const productSchema = menuProductCreateSchema;
const categorySchema = categoryCreateSchema;

// brandSchema / settingsSchema live in apps/api/src/routes/spa-proxy.ts (lines 13-47).
// That module trips the PII red-line edit-gate, so its schemas cannot be exported for
// import here. The objects below are FAITHFUL MIRRORS — keep in exact sync with
// spa-proxy.ts. TODO(import): export brandSchema/settingsSchema from spa-proxy once
// the red-line gate allows a touch (or move them to a PII-free schema module).
const HEX_COLOR = /^#[0-9a-fA-F]{6}$/;

const brandSchema = z.object({
  primaryColor: z.string().regex(HEX_COLOR).optional().nullable(),
  bgColor: z.string().regex(HEX_COLOR).optional().nullable(),
  textColor: z.string().regex(HEX_COLOR).optional().nullable(),
  logoUrl: z.string().max(1000).optional().nullable(),
  googleRating: z.number().min(0).max(5).optional().nullable(),
  googleReviewCount: z.number().int().nonnegative().optional().nullable(),
  googleMapsUrl: z.string().max(500).optional().nullable(),
  googlePlaceId: z.string().max(128).regex(/^[A-Za-z0-9_-]+$/, 'Invalid Place ID').optional().nullable().or(z.literal('')),
  socialInstagram: z.string().max(500).refine((v) => !v || /^https:\/\/(www\.)?(instagram\.com|instagr\.am)\//i.test(v), 'Must be an instagram.com link').optional().nullable(),
  socialFacebook: z.string().max(500).refine((v) => !v || /^https:\/\/(www\.)?(facebook\.com|fb\.com|m\.facebook\.com)\//i.test(v), 'Must be a facebook.com link').optional().nullable(),
}).strict();

const coerceNum = (inner: z.ZodType) =>
  z.preprocess((v) => (v != null && v !== '' ? Number(v) : v), inner);

const settingsSchema = z.object({
  locationName: z.string().min(1).max(200).optional().nullable(),
  phone: z.string().max(50).optional().nullable(),
  address: z.string().max(500).optional().nullable(),
  deliveryFee: coerceNum(z.number().int().nonnegative()).optional().nullable(),
  minOrder: coerceNum(z.number().int().nonnegative()).optional().nullable(),
  radiusKm: coerceNum(z.number().nonnegative()).optional().nullable(),
  freeDeliveryThreshold: coerceNum(z.number().int().nonnegative()).optional().nullable(),
  taxRate: coerceNum(z.number().min(0).max(100)).optional().nullable(),
  currencyCode: z.enum(['ALL', 'EUR']).optional().nullable(),
  lat: coerceNum(z.number().min(-90).max(90)).optional().nullable(),
  lng: coerceNum(z.number().min(-180).max(180)).optional().nullable(),
  hoursJson: z.any().optional().nullable(),
  deliveryPaused: z.boolean().optional(),
}).strip();

describe('spa-proxy Zod validation', () => {

  // ── Product Schema (real: owner/products.ts menuProductCreateSchema, .strip) ──
  // prep_time_minutes is REQUIRED in production — every valid case includes it.

  test('POST product: accepts valid data', () => {
    const data = {
      name: 'Pepperoni Pizza',
      price: 1500,
      prep_time_minutes: 15,
      description: 'Classic pepperoni',
      available: true,
      category_id: '550e8400-e29b-41d4-a716-446655440000',
    };
    const parsed = productSchema.parse(data);
    assert.strictEqual(parsed.name, 'Pepperoni Pizza');
    assert.strictEqual(parsed.price, 1500);
    assert.strictEqual(parsed.prep_time_minutes, 15);
    assert.strictEqual(parsed.available, true);
  });

  test('POST product: accepts name + price + prep_time only', () => {
    const data = { name: 'Cola', price: 200, prep_time_minutes: 5 };
    const parsed = productSchema.parse(data);
    assert.strictEqual(parsed.name, 'Cola');
    assert.strictEqual(parsed.price, 200);
    assert.strictEqual(parsed.prep_time_minutes, 5);
  });

  test('POST product: rejects empty name', () => {
    assert.throws(() => productSchema.parse({ name: '', price: 100, prep_time_minutes: 5 }), z.ZodError);
  });

  test('POST product: rejects missing name', () => {
    assert.throws(() => productSchema.parse({ price: 100, prep_time_minutes: 5 }), z.ZodError);
  });

  test('POST product: rejects missing price', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', prep_time_minutes: 5 }), z.ZodError);
  });

  test('POST product: rejects missing prep_time_minutes (required in prod)', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', price: 100 }), z.ZodError);
  });

  test('POST product: rejects prep_time_minutes below 1', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', price: 100, prep_time_minutes: 0 }), z.ZodError);
  });

  test('POST product: rejects prep_time_minutes above 1440', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', price: 100, prep_time_minutes: 1441 }), z.ZodError);
  });

  test('POST product: rejects negative price', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', price: -100, prep_time_minutes: 5 }), z.ZodError);
  });

  test('POST product: rejects non-integer price', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', price: 10.50, prep_time_minutes: 5 }), z.ZodError);
  });

  test('POST product: rejects string price', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', price: 'abc', prep_time_minutes: 5 }), z.ZodError);
  });

  test('POST product: strips unknown fields (.strip — keeps known, drops unknown)', () => {
    const parsed = productSchema.parse({ name: 'Pizza', price: 100, prep_time_minutes: 5, extraField: 'bad' });
    assert.strictEqual(parsed.name, 'Pizza');
    assert.strictEqual((parsed as Record<string, unknown>).extraField, undefined);
    assert.ok(!('extraField' in parsed));
  });

  test('POST product: accepts available as boolean false', () => {
    const parsed = productSchema.parse({ name: 'Item', price: 100, prep_time_minutes: 5, available: false });
    assert.strictEqual(parsed.available, false);
  });

  test('POST product: rejects available as string', () => {
    assert.throws(() => productSchema.parse({ name: 'Item', price: 100, prep_time_minutes: 5, available: 'true' }), z.ZodError);
  });

  test('POST product: rejects name over 200 chars', () => {
    assert.throws(() => productSchema.parse({ name: 'x'.repeat(201), price: 100, prep_time_minutes: 5 }), z.ZodError);
  });

  test('POST product: accepts recipeLines (BOM) data', () => {
    const data = {
      name: 'Pizza',
      price: 1500,
      prep_time_minutes: 20,
      recipeLines: [
        { supplyId: 's1', supplyName: 'Dough', qty: 1, unit: 'pc', kind: 'ingredient', kcal: 200 },
        { supplyId: 's2', supplyName: 'Cheese', qty: 100, unit: 'g', kind: 'ingredient', fatG: 25 },
      ],
    };
    const parsed = productSchema.parse(data);
    assert.strictEqual(parsed.recipeLines!.length, 2);
    assert.strictEqual(parsed.recipeLines![0].supplyName, 'Dough');
    assert.strictEqual(parsed.recipeLines![1].fatG, 25);
  });

  test('POST product: accepts a plain object-storage image_key', () => {
    const parsed = productSchema.parse({ name: 'Item', price: 100, prep_time_minutes: 5, image_key: 'products/abc/def.webp' });
    assert.strictEqual(parsed.image_key, 'products/abc/def.webp');
  });

  // ── Category Schema (real: owner/categories.ts categoryCreateSchema, .strict) ──

  test('POST category: accepts valid name', () => {
    const parsed = categorySchema.parse({ name: 'Pizzas' });
    assert.strictEqual(parsed.name, 'Pizzas');
  });

  test('POST category: accepts sort_order and image_key', () => {
    const parsed = categorySchema.parse({ name: 'Pizzas', sort_order: 3, image_key: 'cat/x.webp' });
    assert.strictEqual(parsed.sort_order, 3);
    assert.strictEqual(parsed.image_key, 'cat/x.webp');
  });

  test('POST category: rejects empty name', () => {
    assert.throws(() => categorySchema.parse({ name: '' }), z.ZodError);
  });

  test('POST category: rejects missing name', () => {
    assert.throws(() => categorySchema.parse({}), z.ZodError);
  });

  test('POST category: rejects non-integer sort_order', () => {
    assert.throws(() => categorySchema.parse({ name: 'Pizzas', sort_order: 1.5 }), z.ZodError);
  });

  test('POST category: rejects unknown fields (.strict)', () => {
    assert.throws(() => categorySchema.parse({ name: 'Pizzas', extra: true }), z.ZodError);
  });

  // ── Brand Schema (mirror of spa-proxy.ts:13-26) ──

  test('PUT brand: accepts valid hex colors', () => {
    const parsed = brandSchema.parse({ primaryColor: '#ea4f16', bgColor: '#121212' });
    assert.strictEqual(parsed.primaryColor, '#ea4f16');
  });

  test('PUT brand: rejects invalid hex (no hash)', () => {
    assert.throws(() => brandSchema.parse({ primaryColor: 'ea4f16' }), z.ZodError);
  });

  test('PUT brand: rejects invalid hex (short)', () => {
    assert.throws(() => brandSchema.parse({ primaryColor: '#fff' }), z.ZodError);
  });

  test('PUT brand: rejects invalid hex (wrong chars)', () => {
    assert.throws(() => brandSchema.parse({ primaryColor: '#zzzzzz' }), z.ZodError);
  });

  test('PUT brand: accepts empty object (all optional)', () => {
    const parsed = brandSchema.parse({});
    assert.deepStrictEqual(parsed, {});
  });

  test('PUT brand: rejects unknown fields', () => {
    assert.throws(() => brandSchema.parse({ unknownField: 'bad' }), z.ZodError);
  });

  test('PUT brand: accepts googleRating within 0..5', () => {
    const parsed = brandSchema.parse({ googleRating: 4.5, googleReviewCount: 120 });
    assert.strictEqual(parsed.googleRating, 4.5);
    assert.strictEqual(parsed.googleReviewCount, 120);
  });

  test('PUT brand: rejects googleRating above 5', () => {
    assert.throws(() => brandSchema.parse({ googleRating: 6 }), z.ZodError);
  });

  test('PUT brand: rejects negative googleReviewCount', () => {
    assert.throws(() => brandSchema.parse({ googleReviewCount: -1 }), z.ZodError);
  });

  test('PUT brand: accepts valid googlePlaceId and empty-string literal', () => {
    assert.strictEqual(brandSchema.parse({ googlePlaceId: 'ChIJ_abc-123' }).googlePlaceId, 'ChIJ_abc-123');
    assert.strictEqual(brandSchema.parse({ googlePlaceId: '' }).googlePlaceId, '');
  });

  test('PUT brand: rejects googlePlaceId with illegal chars', () => {
    assert.throws(() => brandSchema.parse({ googlePlaceId: 'bad id!' }), z.ZodError);
  });

  test('PUT brand: accepts an instagram.com socialInstagram, rejects other host', () => {
    assert.strictEqual(brandSchema.parse({ socialInstagram: 'https://instagram.com/dowiz' }).socialInstagram, 'https://instagram.com/dowiz');
    assert.throws(() => brandSchema.parse({ socialInstagram: 'https://evil.com/dowiz' }), z.ZodError);
  });

  test('PUT brand: accepts a facebook.com socialFacebook, rejects other host', () => {
    assert.strictEqual(brandSchema.parse({ socialFacebook: 'https://facebook.com/dowiz' }).socialFacebook, 'https://facebook.com/dowiz');
    assert.throws(() => brandSchema.parse({ socialFacebook: 'https://evil.com/dowiz' }), z.ZodError);
  });

  // ── Settings Schema (mirror of spa-proxy.ts:33-47, .strip + coerceNum) ──

  test('PUT settings: accepts valid data', () => {
    const parsed = settingsSchema.parse({
      locationName: 'Pizza Roma',
      phone: '+355691234567',
      deliveryFee: 200,
      minOrder: 500,
      radiusKm: 5.5,
      taxRate: 10,
      lat: 41.3275,
      lng: 19.8187,
    });
    assert.strictEqual(parsed.locationName, 'Pizza Roma');
    assert.strictEqual(parsed.deliveryFee, 200);
  });

  test('PUT settings: coerces numeric strings (pg NUMERIC echo) to numbers', () => {
    const parsed = settingsSchema.parse({ deliveryFee: '200', taxRate: '10', lat: '41.3' });
    assert.strictEqual(parsed.deliveryFee, 200);
    assert.strictEqual(parsed.taxRate, 10);
    assert.strictEqual(parsed.lat, 41.3);
  });

  test('PUT settings: rejects a non-numeric string for a numeric field', () => {
    assert.throws(() => settingsSchema.parse({ deliveryFee: 'abc' }), z.ZodError);
  });

  test('PUT settings: accepts currencyCode enum, rejects unknown currency', () => {
    assert.strictEqual(settingsSchema.parse({ currencyCode: 'EUR' }).currencyCode, 'EUR');
    assert.throws(() => settingsSchema.parse({ currencyCode: 'USD' }), z.ZodError);
  });

  test('PUT settings: accepts deliveryPaused boolean', () => {
    assert.strictEqual(settingsSchema.parse({ deliveryPaused: true }).deliveryPaused, true);
  });

  test('PUT settings: strips unknown fields (.strip)', () => {
    const parsed = settingsSchema.parse({ locationName: 'X', unknownKey: 'drop me' });
    assert.strictEqual(parsed.locationName, 'X');
    assert.ok(!('unknownKey' in parsed));
  });

  test('PUT settings: rejects negative taxRate', () => {
    assert.throws(() => settingsSchema.parse({ taxRate: -1 }), z.ZodError);
  });

  test('PUT settings: rejects taxRate over 100', () => {
    assert.throws(() => settingsSchema.parse({ taxRate: 101 }), z.ZodError);
  });

  test('PUT settings: rejects invalid lat', () => {
    assert.throws(() => settingsSchema.parse({ lat: 100 }), z.ZodError);
  });

  test('PUT settings: accepts address and hoursJson (were silently dropped before)', () => {
    const parsed = settingsSchema.parse({
      locationName: 'Test',
      address: 'Rruga Kavajes, Tirana',
      hoursJson: { mon: '09:00-22:00' },
    });
    assert.strictEqual(parsed.address, 'Rruga Kavajes, Tirana');
    assert.deepStrictEqual(parsed.hoursJson, { mon: '09:00-22:00' });
  });

  // ── Data URL rejection via the REAL validateImageKey (lib/image-key.ts) ──

  test('validateImageKey: rejects data: and blob: URLs with the real message', () => {
    assert.throws(() => validateImageKey('data:image/png;base64,abc'), /not sent as a data URL/);
    assert.throws(() => validateImageKey('blob:http://example.com/uuid'), /not sent as a data URL/);
    assert.strictEqual(validateImageKey('products/abc/def.webp'), 'products/abc/def.webp');
    assert.strictEqual(validateImageKey(undefined), undefined);
    assert.strictEqual(validateImageKey(null), null);
  });

  // ── PATCH partial update validation ──

  test('PATCH product: accepts partial update with single field', () => {
    const partial = productSchema.partial();
    const parsed = partial.parse({ name: 'Updated Name' });
    assert.strictEqual(parsed.name, 'Updated Name');
    assert.strictEqual(parsed.price, undefined);
  });

  test('PATCH product: strips unknown field in partial (.strip)', () => {
    const partial = productSchema.partial();
    const parsed = partial.parse({ unknown: 'bad' });
    assert.ok(!('unknown' in parsed));
  });

  test('PATCH product: accepts empty object (valid partial)', () => {
    const partial = productSchema.partial();
    const parsed = partial.parse({});
    assert.deepStrictEqual(parsed, {});
  });
});

// ── HTTP-layer authz + cross-tenant (IDOR) — NOT covered here ──
// These require a booted Fastify instance (or live staging) to exercise getLocationId()
// JWT derivation on PUT /api/owner/brand and PUT /api/owner/settings. This file is a
// pure schema unit suite (no server), so faking them would be a false-green.
// TODO(needs_staging): add an integration/E2E suite asserting:
//   1. no Authorization header → 401 (negative control)
//   2. courier/customer-signed JWT → 401/403 on the owner-only PUTs (wrong-role)
//   3. valid owner JWT → 200 + value read back (positive control)
//   4. owner-A JWT cannot mutate owner-B's brand/settings — use a REAL second tenant id
//      (expect 401/403/404 by tenant scope), never an all-zero nil-UUID.
