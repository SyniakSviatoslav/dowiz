import test from 'node:test';
import assert from 'node:assert/strict';
import { applyTax as serverApplyTax } from '../src/lib/money.js';
import { estimateOrderTotal } from '../../../packages/ui/src/lib/money.js';
import { ORDER_TOTAL_VECTORS } from './vectors/order-total-vectors.js';

// LC1 (ADR-audit-fix-money D1) — inclusive VAT is EXTRACTED from the subtotal (already paid inside
// the price); adding it to the order total double-charges the customer by r/(1+r) of the cart.
// These proofs are ORACLE-INDEPENDENT: expected totals are hand-derived literals in the vector file
// (zero imports), and the property test asserts the definitional invariant — neither is computed from
// the implementation, unlike the fee-parity mirror that certified the bug.

test('LC1: order total matches hand-derived independent constants', async (t) => {
  for (const v of ORDER_TOTAL_VECTORS) {
    await t.test(v.name, () => {
      // server applyTax = the tax figure stored/displayed (informational on inclusive venues)
      assert.equal(
        serverApplyTax(v.subtotal, v.taxRate, v.priceIncludesTax, 0),
        v.expectedTax,
        'taxTotal (extracted/added figure) drifted from the hand-derived constant',
      );
      const est = estimateOrderTotal(v.subtotal, {
        isPickup: v.deliveryFeeFlat === 0,
        freeDeliveryThreshold: null,
        deliveryFeeFlat: v.deliveryFeeFlat,
        hasDistanceTiers: false,
        taxRate: v.taxRate,
        priceIncludesTax: v.priceIncludesTax,
        minOrderValue: null,
      });
      assert.equal(est.taxTotal, v.expectedTax, `${v.name}: taxTotal`);
      assert.equal(est.chargedTax, v.expectedChargedTax, `${v.name}: chargedTax`);
      assert.equal(est.total, v.expectedTotal, `${v.name}: total`);
    });
  }
});

test('LC1 property: inclusive venue never adds tax to the charge (total === subtotal + fee)', () => {
  // References no implementation output — the definitional invariant D1 asserts.
  const rates = [0, 0.075, 0.1, 0.2];
  const subtotals = [500, 1000, 1075, 1999, 5000, 123_456];
  for (const rate of rates) {
    for (const sub of subtotals) {
      const est = estimateOrderTotal(sub, {
        isPickup: false, freeDeliveryThreshold: null, deliveryFeeFlat: 300,
        hasDistanceTiers: false, taxRate: rate, priceIncludesTax: true, minOrderValue: null,
      });
      assert.equal(est.chargedTax, 0, `inclusive chargedTax must be 0 @ rate=${rate} sub=${sub}`);
      assert.equal(est.total, sub + 300, `inclusive total must equal subtotal+fee @ rate=${rate} sub=${sub}`);
    }
  }
});

test('LC1 property: exclusive venue adds exactly the extracted tax', () => {
  const rate = 0.2;
  const subtotals = [500, 1000, 2000, 5000];
  for (const sub of subtotals) {
    const est = estimateOrderTotal(sub, {
      isPickup: false, freeDeliveryThreshold: null, deliveryFeeFlat: 300,
      hasDistanceTiers: false, taxRate: rate, priceIncludesTax: false, minOrderValue: null,
    });
    assert.equal(est.chargedTax, est.taxTotal, `exclusive chargedTax === taxTotal @ sub=${sub}`);
    assert.equal(est.total, sub + 300 + est.taxTotal, `exclusive total @ sub=${sub}`);
  }
});
