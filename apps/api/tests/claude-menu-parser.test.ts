import test from 'node:test';
import assert from 'node:assert/strict';
import { mapToResult } from '../src/lib/claude-menu-parser.js';

// Proves the Claude vision parser's mapping logic (model JSON → ParseResult +
// restaurant metadata) without a network call or API key. The live extraction
// quality is exercised separately when ANTHROPIC_API_KEY is configured.

test('maps structured menu output to ParseResult + restaurant metadata', () => {
  const raw = JSON.stringify({
    restaurant: { name: 'Pizza Roma', address: 'Rruga Sami Frasheri 12, Tirana', phone: '+355 69 123 4567', hoursText: 'Mon–Sun 10:00–23:00' },
    categories: [{ externalKey: 'pizzas', name: 'Pizzas' }, { externalKey: 'drinks', name: 'Drinks' }],
    products: [
      { externalKey: 'margherita', categoryKey: 'pizzas', name: 'Margherita', description: 'Tomato, mozzarella, basil', price: 800, available: true, hasImage: true, allergens: ['dairy', 'gluten'] },
      { externalKey: 'espresso', categoryKey: 'drinks', name: 'Espresso', description: '', price: 150, available: true, hasImage: false, allergens: [] },
    ],
  });
  const r = mapToResult(raw, 'ALL', []);

  // NEW requirement — restaurant address + contact phone extracted from the PDF.
  assert.equal(r.restaurant?.name, 'Pizza Roma');
  assert.equal(r.restaurant?.address, 'Rruga Sami Frasheri 12, Tirana');
  assert.equal(r.restaurant?.phone, '+355 69 123 4567');
  assert.equal(r.restaurant?.hoursText, 'Mon–Sun 10:00–23:00');

  // Menu items: names, prices, categories, descriptions.
  assert.equal(r.draft.categories.length, 2);
  assert.equal(r.draft.products.length, 2);
  const m = r.draft.products[0];
  assert.equal(m.name, 'Margherita');
  assert.equal(m.categoryKey, 'pizzas');
  assert.equal(m.description, 'Tomato, mozzarella, basil');
  assert.equal(m.price, 800);
  assert.equal(m.currency, 'ALL');
  // allergens + per-item photo flag preserved.
  assert.deepEqual((m.attributesJson as any).allergens, ['dairy', 'gluten']);
  assert.equal((m.attributesJson as any).hasImage, true);
  // empty description normalises to undefined (not an empty string).
  assert.equal(r.draft.products[1].description, undefined);

  assert.equal(r.summary.valid, 2);
  assert.equal(r.summary.errors, 0);
});

test('fails soft (no throw) on invalid model output', () => {
  const r = mapToResult('not json at all', 'ALL', []);
  assert.equal(r.draft.products.length, 0);
  assert.equal(r.restaurant, undefined);
  assert.ok(r.issues.some((i) => i.code === 'PARSE_ERROR'), 'records a PARSE_ERROR issue');
  assert.equal(r.summary.errors, 1);
});
