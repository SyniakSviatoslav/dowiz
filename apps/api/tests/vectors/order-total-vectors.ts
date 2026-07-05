// INDEPENDENT-CONSTANT tax/total vectors (ADR-audit-fix-money M4/M5).
//
// RULE: this file has ZERO imports and only literal initializers. Every `expectedTotal` /
// `expectedTax` / `expectedChargedTax` below is derived BY HAND (arithmetic shown in the comment),
// NOT by running the implementation. A composition test may import ONLY the module under test + this
// file. This is what makes the proof oracle-independent — it cannot certify the LC1 double-charge the
// way `fee-parity.test.ts`'s `sub + fee + tax` mirror-oracle did.
//
// applyTax semantics (half-up, integer minor units):
//   exclusive: tax = round(subtotal * rate)
//   inclusive: net = round(subtotal * 1e6 / (1e6 + rate*1e6)); tax = subtotal - net  (tax is EMBEDDED)
// composition (D1): total = subtotal + deliveryFee + (priceIncludesTax ? 0 : tax)

export const ORDER_TOTAL_VECTORS = [
  {
    name: 'exclusive round rate — tax added',
    subtotal: 1000, taxRate: 0.2, priceIncludesTax: false, deliveryFeeFlat: 200,
    // tax = round(1000 * 0.20) = 200 ; total = 1000 + 200(fee) + 200(tax) = 1400
    expectedTax: 200, expectedChargedTax: 200, expectedTotal: 1400,
  },
  {
    name: 'inclusive round rate — tax NOT re-added (the LC1 case)',
    subtotal: 1200, taxRate: 0.2, priceIncludesTax: true, deliveryFeeFlat: 250,
    // net = round(1200*1e6 / 1.2e6) = round(1000.5) → 1000 ; tax = 1200 - 1000 = 200 (embedded)
    // total = 1200 + 250(fee) + 0 = 1450   (the BUG would charge 1200+250+200 = 1650, +200 overcharge)
    expectedTax: 200, expectedChargedTax: 0, expectedTotal: 1450,
  },
  {
    name: 'inclusive 7.5% pickup — embedded tax, no fee',
    subtotal: 1075, taxRate: 0.075, priceIncludesTax: true, deliveryFeeFlat: 0,
    // tax embedded = 75 (net 1000) ; total = 1075 + 0(pickup) + 0 = 1075  (BUG: 1150)
    expectedTax: 75, expectedChargedTax: 0, expectedTotal: 1075,
  },
  {
    name: 'exclusive half-up boundary rate',
    subtotal: 1000, taxRate: 0.0745, priceIncludesTax: false, deliveryFeeFlat: 200,
    // tax = round(1000 * 0.0745) = round(74.5) = 75 (half-up) ; total = 1000 + 200 + 75 = 1275
    expectedTax: 75, expectedChargedTax: 75, expectedTotal: 1275,
  },
  {
    name: 'zero rate inclusive — nothing added',
    subtotal: 1000, taxRate: 0, priceIncludesTax: true, deliveryFeeFlat: 250,
    // rate 0 → tax 0 ; total = 1000 + 250 + 0 = 1250
    expectedTax: 0, expectedChargedTax: 0, expectedTotal: 1250,
  },
  {
    name: 'zero rate exclusive — nothing added',
    subtotal: 1000, taxRate: 0, priceIncludesTax: false, deliveryFeeFlat: 250,
    // rate 0 → tax 0 ; total = 1000 + 250 + 0 = 1250
    expectedTax: 0, expectedChargedTax: 0, expectedTotal: 1250,
  },
];
