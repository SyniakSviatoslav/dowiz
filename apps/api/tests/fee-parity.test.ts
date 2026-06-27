import test from 'node:test';
import assert from 'node:assert/strict';
import { applyTax as serverApplyTax } from '../src/lib/money.js';
import { applyTax as mirrorApplyTax, estimateOrderTotal, computeDeliveryFee } from '../../../packages/ui/src/lib/money.js';
import { resolveDeliveryFee } from '../src/lib/order-pricing.js';

// PARITY GUARDRAIL (ADR-0005, Approach M) — the client-side total MIRROR must agree with the
// server's authoritative order-total math TO THE CENT, or the customer is shown a number the
// courier won't collect at the door. This is a ship-blocker (Mandatory Proof Rule + money red-line):
// it fails red the moment the mirror drifts from the server. The server math is NOT changed by this
// work — this test pins the mirror to it.

// The exact server fee ladder (apps/api/src/routes/orders.ts:528-560), replicated here as the ORACLE
// the mirror is checked against. Returns null where the server would compute a distance-tiered fee
// (RLS-hidden from the client) or 422 (delivery not configured) — i.e. the client must NOT pre-quote.
function serverFeeOracle(subtotal: number, cfg: {
  isPickup: boolean; freeDeliveryThreshold: number | null; deliveryFeeFlat: number | null; hasDistanceTiers: boolean;
}): number | null {
  // The pickup / free-threshold short-circuits live in the POST /orders caller (orders.ts section 8),
  // NOT in resolveDeliveryFee — replicate only those two here.
  if (cfg.isPickup) return 0;
  if (cfg.freeDeliveryThreshold !== null && subtotal >= cfg.freeDeliveryThreshold) return 0;
  if (cfg.hasDistanceTiers) return null; // server resolves via delivery_tiers + distance — RLS-hidden from the client
  // Delegate the flat-fee / DELIVERY_NOT_CONFIGURED decision to the ACTUAL server function so the oracle
  // drifts red the moment order-pricing.ts changes (no tiers/pin → pure flat-vs-unconfigured branch).
  const res = resolveDeliveryFee({
    location: { lat: null, lng: null, delivery_fee_flat: cfg.deliveryFeeFlat },
    pin: null,
    tiers: [],
  });
  return res.ok ? res.deliveryFee : null; // false → DELIVERY_NOT_CONFIGURED → client must NOT pre-quote
}

const SUBTOTALS = [0, 1, 499, 500, 799, 800, 1000, 1999, 2000, 2001, 5000, 123_456];
const TAX_RATES = [0, 0.075, 0.0744, 0.0745, 0.1, 0.2];
const TAX_MODES = [false, true];

test('parity: mirror applyTax === server applyTax across the matrix', () => {
  for (const sub of SUBTOTALS) {
    for (const rate of TAX_RATES) {
      for (const inc of TAX_MODES) {
        assert.equal(
          mirrorApplyTax(sub, rate, inc),
          serverApplyTax(sub, rate, inc, 0),
          `applyTax drift @ subtotal=${sub} rate=${rate} inclusive=${inc}`,
        );
      }
    }
  }
});

test('parity: estimateOrderTotal.total === subtotal + serverFee + serverTax (computable venues)', () => {
  const venues = [
    { name: 'flat-fee + free-over-2000', isPickup: false, freeDeliveryThreshold: 2000, deliveryFeeFlat: 250, hasDistanceTiers: false },
    { name: 'flat-fee no threshold',     isPickup: false, freeDeliveryThreshold: null, deliveryFeeFlat: 200, hasDistanceTiers: false },
    { name: 'pickup',                    isPickup: true,  freeDeliveryThreshold: 2000, deliveryFeeFlat: 250, hasDistanceTiers: false },
  ];
  for (const v of venues) {
    for (const sub of SUBTOTALS) {
      for (const rate of TAX_RATES) {
        for (const inc of TAX_MODES) {
          const cfg = { ...v, taxRate: rate, priceIncludesTax: inc, minOrderValue: 800 };
          const est = estimateOrderTotal(sub, cfg);
          const fee = serverFeeOracle(sub, v);
          assert.equal(est.feeKnown, fee !== null, `${v.name}: feeKnown @ ${sub}`);
          if (fee !== null) {
            const expected = sub + fee + serverApplyTax(sub, rate, inc, 0);
            assert.equal(est.total, expected, `${v.name}: total drift @ subtotal=${sub} rate=${rate} inc=${inc}`);
          } else {
            assert.equal(est.total, null, `${v.name}: must not pre-quote @ ${sub}`);
          }
        }
      }
    }
  }
});

test('parity: distance-tiered + unconfigured venues are NOT pre-quoted (feeKnown=false)', () => {
  const tiered = { isPickup: false, freeDeliveryThreshold: 2000, deliveryFeeFlat: 250, hasDistanceTiers: true };
  const unconfigured = { isPickup: false, freeDeliveryThreshold: null, deliveryFeeFlat: null, hasDistanceTiers: false };
  // Below the free threshold the tiered venue's fee is server-only → no client quote.
  assert.equal(computeDeliveryFee(1500, tiered), null);
  assert.equal(computeDeliveryFee(1500, unconfigured), null);
  // At/above the free threshold even a tiered venue is free → computable (fee 0).
  assert.equal(computeDeliveryFee(2000, tiered), 0);
});

test('parity: free-over-threshold boundary — the sharpest divergence the hardcode caused', () => {
  const v = { isPickup: false, freeDeliveryThreshold: 2000, deliveryFeeFlat: 250, hasDistanceTiers: false, taxRate: 0, priceIncludesTax: false, minOrderValue: 800 };
  assert.equal(estimateOrderTotal(1999, v).deliveryFee, 250); // just under → pay the fee
  assert.equal(estimateOrderTotal(2000, v).deliveryFee, 0);   // at threshold → free (hardcode wrongly charged 200/250 here)
});

test('parity: min-order gate mirrors the server (pickup AND delivery)', () => {
  const base = { isPickup: false, freeDeliveryThreshold: 2000, deliveryFeeFlat: 250, hasDistanceTiers: false, taxRate: 0, priceIncludesTax: false, minOrderValue: 800 };
  assert.equal(estimateOrderTotal(799, base).minNotMet, true);
  assert.equal(estimateOrderTotal(800, base).minNotMet, false);
  assert.equal(estimateOrderTotal(799, { ...base, isPickup: true }).minNotMet, true); // server gates pickup too
});
