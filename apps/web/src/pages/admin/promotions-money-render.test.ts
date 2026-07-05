import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { formatALL } from '@deliveryos/shared-types';

// GUARDRAIL — Albania go-to-market gap: the promotions cards rendered fixed-discount and
// min-order amounts as `(value / 100).toFixed(0) ALL`. But ALL/Lekë is a minor_unit=0
// currency (DB: locations.currency_minor_unit DEFAULT 0; products seeded in whole Lekë),
// and the promotions form STORES the value the owner types (no ×100) — matching the server,
// which reads discount_value/min_order_amount as whole Lekë (promotions.ts: Math.min(
// discount_value, order_subtotal) / order_subtotal < min_order_amount). So dividing by 100
// in the display made every fixed discount / min-order render 100× TOO SMALL (owner types
// 200, card shows "2"). The fix routes both through the shared formatALL() (no division) —
// DISPLAY-ONLY: storage, the form, and the server's discount computation are untouched;
// discountTotal is 0 in orders.ts so no charged total is involved.
//
// This repo's web test runner (tsx --test) has no jsdom (see AGENTS.md), so — mirroring
// promotions-audit-fixes.test.ts — the wiring is proven by source-content assertions with a
// red arm, and the VALUE semantics by a direct formatALL() unit assertion.

const DIR = dirname(fileURLToPath(import.meta.url));
const SRC = readFileSync(join(DIR, 'PromotionsPage.tsx'), 'utf8');

/** Red-arm detector: the buggy `(p.<field> / 100)` shape. */
const DIVIDES_BY_100 = /\(p\.\w+\s*\/\s*100\)/;

test('PromotionsPage renders promo money via formatALL (no erroneous /100)', () => {
  // GREEN: both money renders route through the shared formatter.
  assert.ok(SRC.includes('formatALL(p.discount_value)'), 'fixed-discount not rendered via formatALL');
  assert.ok(SRC.includes('formatALL(p.min_order_amount)'), 'min-order not rendered via formatALL');
  // formatALL is actually imported.
  assert.ok(/import\s*\{[^}]*\bformatALL\b[^}]*\}\s*from\s*'@deliveryos\/shared-types'/.test(SRC),
    'formatALL not imported from @deliveryos/shared-types');
  // REGRESSION: the buggy /100 shape is gone from the file entirely.
  assert.ok(!DIVIDES_BY_100.test(SRC), 'a `(p.<field> / 100)` money-render is still present');
});

test('red arm — the /100 detector actually detects the pre-fix shape', () => {
  // Proves the regression assertion above is NOT vacuously true.
  const OLD = "p.type === 'fixed' ? `${(p.discount_value / 100).toFixed(0)} ALL` : x";
  const NEW = "p.type === 'fixed' ? formatALL(p.discount_value) : x";
  assert.ok(DIVIDES_BY_100.test(OLD), 'detector failed to flag the old buggy shape');
  assert.ok(!DIVIDES_BY_100.test(NEW), 'detector false-positives on the fixed shape');
});

test('formatALL renders whole-Lekë value without dividing by 100', () => {
  // Independent, hand-derived expectations: ALL is minor_unit 0 → identity on the integer.
  // Owner types 200 → card must show "200 …", NEVER "2 …" (the pre-fix 100×-too-small bug).
  assert.ok(formatALL(200).startsWith('200'), `formatALL(200) should start with 200, got "${formatALL(200)}"`);
  assert.ok(!formatALL(200).startsWith('2 '), `formatALL(200) divided by 100 → "${formatALL(200)}"`);
  assert.ok(formatALL(80000).startsWith('80000'), `formatALL(80000) should start with 80000, got "${formatALL(80000)}"`);
});
