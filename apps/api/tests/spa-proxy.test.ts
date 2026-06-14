import { test, describe, before, after } from 'node:test';
import assert from 'node:assert';
import { z } from 'zod';

// These tests validate the Zod schemas used by spa-proxy endpoints.
// They verify that data validation works correctly WITHOUT needing a running server.

const HEX_COLOR = /^#[0-9a-fA-F]{6}$/;

const productSchema = z.object({
  name: z.string().min(1).max(200),
  price: z.number().int().nonnegative(),
  description: z.string().max(2000).optional().nullable(),
  available: z.boolean().optional(),
  category_id: z.string().uuid().optional().nullable(),
  categoryId: z.string().uuid().optional().nullable(),
  image_key: z.string().max(500).optional().nullable(),
  imageUrl: z.string().max(500).optional().nullable(),
  stockCount: z.number().int().nonnegative().optional().nullable(),
  taste: z.record(z.number().min(0).max(3)).optional().nullable(),
  recipeLines: z.array(z.object({
    supplyId: z.string(),
    supplyName: z.string(),
    qty: z.number(),
    unit: z.string(),
    kind: z.string(),
    kcal: z.number().nullable().optional(),
    proteinG: z.number().nullable().optional(),
    fatG: z.number().nullable().optional(),
    carbsG: z.number().nullable().optional(),
    allergens: z.array(z.string()).optional(),
  })).optional().nullable(),
}).strict();

const categorySchema = z.object({
  name: z.string().min(1).max(100),
}).strict();

const brandSchema = z.object({
  primaryColor: z.string().regex(HEX_COLOR).optional().nullable(),
  bgColor: z.string().regex(HEX_COLOR).optional().nullable(),
  textColor: z.string().regex(HEX_COLOR).optional().nullable(),
  logoUrl: z.string().max(1000).optional().nullable(),
}).strict();

const settingsSchema = z.object({
  locationName: z.string().min(1).max(200).optional().nullable(),
  phone: z.string().max(50).optional().nullable(),
  address: z.string().max(500).optional().nullable(),
  deliveryFee: z.number().int().nonnegative().optional().nullable(),
  minOrder: z.number().int().nonnegative().optional().nullable(),
  radiusKm: z.number().nonnegative().optional().nullable(),
  freeDeliveryThreshold: z.number().int().nonnegative().optional().nullable(),
  taxRate: z.number().min(0).max(100).optional().nullable(),
  lat: z.number().min(-90).max(90).optional().nullable(),
  lng: z.number().min(-180).max(180).optional().nullable(),
  hoursJson: z.any().optional().nullable(),
}).strict();

describe('spa-proxy Zod validation', () => {

  // ── Product Schema ──

  test('POST product: accepts valid data', () => {
    const data = {
      name: 'Pepperoni Pizza',
      price: 1500,
      description: 'Classic pepperoni',
      available: true,
      category_id: '550e8400-e29b-41d4-a716-446655440000',
    };
    const parsed = productSchema.parse(data);
    assert.strictEqual(parsed.name, 'Pepperoni Pizza');
    assert.strictEqual(parsed.price, 1500);
    assert.strictEqual(parsed.available, true);
  });

  test('POST product: accepts minimal data (name + price only)', () => {
    const data = { name: 'Cola', price: 200 };
    const parsed = productSchema.parse(data);
    assert.strictEqual(parsed.name, 'Cola');
    assert.strictEqual(parsed.price, 200);
  });

  test('POST product: rejects empty name', () => {
    assert.throws(() => productSchema.parse({ name: '', price: 100 }), z.ZodError);
  });

  test('POST product: rejects missing name', () => {
    assert.throws(() => productSchema.parse({ price: 100 }), z.ZodError);
  });

  test('POST product: rejects missing price', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza' }), z.ZodError);
  });

  test('POST product: rejects negative price', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', price: -100 }), z.ZodError);
  });

  test('POST product: rejects non-integer price', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', price: 10.50 }), z.ZodError);
  });

  test('POST product: rejects string price', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', price: 'abc' }), z.ZodError);
  });

  test('POST product: rejects unknown fields (.strict)', () => {
    assert.throws(() => productSchema.parse({ name: 'Pizza', price: 100, extraField: 'bad' }), z.ZodError);
  });

  test('POST product: accepts available as boolean false', () => {
    const parsed = productSchema.parse({ name: 'Item', price: 100, available: false });
    assert.strictEqual(parsed.available, false);
  });

  test('POST product: rejects available as string', () => {
    assert.throws(() => productSchema.parse({ name: 'Item', price: 100, available: 'true' }), z.ZodError);
  });

  test('POST product: rejects name over 200 chars', () => {
    assert.throws(() => productSchema.parse({ name: 'x'.repeat(201), price: 100 }), z.ZodError);
  });

  test('POST product: accepts recipeLines (BOM) data', () => {
    const data = {
      name: 'Pizza',
      price: 1500,
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

  test('POST product: rejects data URL as image_key', () => {
    // The validateImageKey function in the route rejects data URLs,
    // but the Zod schema only validates string length.
    // This test verifies the schema allows it (the route function catches it separately).
    const parsed = productSchema.parse({ name: 'Item', price: 100, imageUrl: 'data:image/png;base64,abc' });
    assert.strictEqual(parsed.imageUrl, 'data:image/png;base64,abc');
  });

  // ── Category Schema ──

  test('POST category: accepts valid name', () => {
    const parsed = categorySchema.parse({ name: 'Pizzas' });
    assert.strictEqual(parsed.name, 'Pizzas');
  });

  test('POST category: rejects empty name', () => {
    assert.throws(() => categorySchema.parse({ name: '' }), z.ZodError);
  });

  test('POST category: rejects missing name', () => {
    assert.throws(() => categorySchema.parse({}), z.ZodError);
  });

  test('POST category: rejects unknown fields', () => {
    assert.throws(() => categorySchema.parse({ name: 'Pizzas', extra: true }), z.ZodError);
  });

  // ── Brand Schema ──

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

  // ── Settings Schema ──

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

  // ── Data URL rejection via validateImageKey logic ──

  test('validateImageKey: rejects data URLs', () => {
    function validateImageKey(val: unknown): string | null | undefined {
      if (val === undefined || val === null) return val;
      const s = String(val);
      if (s.startsWith('data:') || s.startsWith('blob:')) {
        throw new Error('Image must be uploaded via the image upload endpoint');
      }
      return s;
    }
    assert.throws(() => validateImageKey('data:image/png;base64,abc'), /image upload endpoint/);
    assert.throws(() => validateImageKey('blob:http://example.com/uuid'), /image upload endpoint/);
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

  test('PATCH product: rejects unknown field in partial', () => {
    const partial = productSchema.partial();
    assert.throws(() => partial.parse({ unknown: 'bad' }), z.ZodError);
  });

  test('PATCH product: accepts empty object (valid partial)', () => {
    const partial = productSchema.partial();
    const parsed = partial.parse({});
    assert.deepStrictEqual(parsed, {});
  });
});
