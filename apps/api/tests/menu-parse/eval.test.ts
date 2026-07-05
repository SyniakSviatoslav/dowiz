import { test } from 'node:test';
import assert from 'node:assert/strict';
import { scoreParse, THRESHOLDS } from './scorer.js';
import { runEval } from './run.js';
import type { CanonicalMenuDraft } from '@deliveryos/shared-types';

// ADR-0011 B1 — the deterministic parse-eval gate. (1) the committed fixture baseline is within
// thresholds; (2) the scorer BLOCKS a measured cascade-swap regression (wrong price / dropped item /
// broken modifier structure). Price is zero-tolerance.

const GOLDEN: CanonicalMenuDraft = {
  categories: [{ externalKey: 'c', name: 'Pizza' }],
  products: [
    { externalKey: 'p1', categoryKey: 'c', name: 'Margherita', price: 800, currency: 'ALL', available: true },
    { externalKey: 'p2', categoryKey: 'c', name: 'Pepperoni', price: 1200, currency: 'ALL', available: true },
  ],
  modifierGroups: [{ externalKey: 's', name: 'Size', minSelect: 1, maxSelect: 1, required: true }],
  modifiers: [],
  links: [],
  translations: [],
};
const clone = (): CanonicalMenuDraft => JSON.parse(JSON.stringify(GOLDEN));

test('menu-parse eval (B1)', async (t) => {
  await t.test('committed fixture baseline is within thresholds', () => {
    const results = runEval();
    assert.ok(results.length >= 2, 'expected committed fixtures');
    for (const { name, report } of results) {
      assert.ok(report.pass, `fixture ${name} below threshold: ${report.failures.join('; ')}`);
    }
  });

  await t.test('identical parse scores 100% / 100% / 100% and passes', () => {
    const r = scoreParse(GOLDEN, clone());
    assert.equal(r.priceExact.rate, 1);
    assert.equal(r.itemRecall.rate, 1);
    assert.equal(r.modifierStructure.rate, 1);
    assert.ok(r.pass);
  });

  await t.test('BLOCKS a wrong price (zero tolerance) — 50% price-exact fails', () => {
    const bad = clone();
    bad.products[1].price = 1199; // a 1-minor hallucination on one of two items
    const r = scoreParse(GOLDEN, bad);
    assert.equal(r.itemRecall.rate, 1); // item still present
    assert.equal(r.priceExact.rate, 0.5); // 1 of 2 prices exact
    assert.ok(!r.pass);
    assert.ok(r.failures.some((f) => f.includes('price-exact')));
  });

  await t.test('BLOCKS a dropped item — item-recall below 0.95 fails', () => {
    const bad = clone();
    bad.products.pop(); // dropped Pepperoni → recall 0.5
    const r = scoreParse(GOLDEN, bad);
    assert.equal(r.itemRecall.rate, 0.5);
    assert.ok(!r.pass);
    assert.ok(r.failures.some((f) => f.includes('item-recall')));
  });

  await t.test('BLOCKS a broken modifier structure — below 0.90 fails', () => {
    const bad = clone();
    bad.modifierGroups[0].maxSelect = 3; // was 1 → structure mismatch
    const r = scoreParse(GOLDEN, bad);
    assert.equal(r.modifierStructure.rate, 0);
    assert.ok(!r.pass);
    assert.ok(r.failures.some((f) => f.includes('modifier-structure')));
  });

  await t.test('thresholds are the version-controlled ADR-0011 values', () => {
    assert.equal(THRESHOLDS.priceExact, 1.0);
    assert.equal(THRESHOLDS.itemRecall, 0.95);
    assert.equal(THRESHOLDS.modifierStructure, 0.9);
  });
});
