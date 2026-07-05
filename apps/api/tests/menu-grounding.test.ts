import { test } from 'node:test';
import assert from 'node:assert/strict';
import { extractTrailingPriceMinor, collectOcrPriceMinors, groundItems } from '../src/lib/menu-grounding.js';

// ADR-0011 B2/B7 — hallucination grounding via the SAME price normalizer, not a substring.
// The two load-bearing fixtures: a substring approach false-FLAGS "1.200 Lek" (reads 1.2) and
// false-PASSES price:1 (the char "1" is everywhere). Normalizer parity fixes both.

test('menu grounding (B7 normalizer-parity)', async (t) => {
  await t.test('normalizer: "1.200 Lek" → 1200 minor (ALL, minorUnit 0)', () => {
    assert.equal(extractTrailingPriceMinor('Pizza Margherita 1.200 Lek', 0)?.minor, 1200);
    assert.equal(extractTrailingPriceMinor('Sallatë .......... 800 Lek', 0)?.minor, 800);
    assert.equal(extractTrailingPriceMinor('Espresso 8.50 EUR', 2)?.minor, 850); // 2-dp currency
    assert.equal(extractTrailingPriceMinor('no price here', 0), null);
  });

  await t.test('FALSE-FLAG fixture: a parsed 1200 IS grounded by "1.200 Lek" (not flagged)', () => {
    const ocr = 'BUKË & VERË\nPizza Margherita 1.200 Lek\nSallatë Greke 800 Lek';
    const minors = collectOcrPriceMinors(ocr, 0);
    assert.ok(minors.has(1200), 'normalizer must read 1.200 Lek as 1200, not 1.2');
    const r = groundItems([{ name: 'Pizza Margherita', price: 1200 }], ocr, 0);
    assert.equal(r.groundedCount, 1);
    assert.equal(r.ungrounded.length, 0); // a substring approach would have FLAGGED this
  });

  await t.test('FALSE-PASS fixture: a hallucinated price:1 IS flagged (no OCR token normalizes to 1)', () => {
    const ocr = 'Pizza Margherita 800 Lek\nSallatë Greke 450 Lek';
    const r = groundItems([{ name: 'Ghost Item', price: 1 }], ocr, 0);
    assert.equal(r.groundedCount, 0);
    assert.equal(r.ungrounded.length, 1); // a substring "1" would have wrongly PASSED this
    assert.equal(r.ungrounded[0].price, 1);
  });

  await t.test('mixed: grounds real prices, flags the hallucinated one', () => {
    const ocr = 'Pizza 800 Lek\nPasta 950 Lek\nSalad 450 Lek';
    const items = [
      { externalKey: 'pizza', name: 'Pizza', price: 800 },
      { externalKey: 'pasta', name: 'Pasta', price: 950 },
      { externalKey: 'ghost', name: 'Ghost', price: 9999 }, // not in OCR
    ];
    const r = groundItems(items, ocr, 0);
    assert.equal(r.groundedCount, 2);
    assert.deepEqual(r.ungrounded.map((u) => u.externalKey), ['ghost']);
  });
});
