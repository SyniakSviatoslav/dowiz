import test from 'node:test';
import assert from 'node:assert/strict';
import { applyTax, computeLineTotal } from '../src/lib/money.js';

// Proof for the RED-LINE-2 hardening: tax is computed with integer/BigInt math
// (zero float arithmetic on the monetary value). These cases also pin equivalence
// with the previous float implementation's correct outputs.
test('applyTax — integer-safe tax computation', async (t) => {
  await t.test('exclusive: round-number rates', () => {
    assert.equal(applyTax(1000, 0.075, false, 0), 75);
    assert.equal(applyTax(1000, 0.1, false, 0), 100);
    assert.equal(applyTax(1200, 0.2, false, 0), 240);
  });

  await t.test('exclusive: half-up rounding at the boundary', () => {
    assert.equal(applyTax(1000, 0.0744, false, 0), 74); // 74.4 → 74
    assert.equal(applyTax(1000, 0.0745, false, 0), 75); // 74.5 → 75 (half-up)
    assert.equal(applyTax(999, 0.2, false, 0), 200);    // 199.8 → 200
  });

  await t.test('inclusive: extracts embedded tax', () => {
    assert.equal(applyTax(1075, 0.075, true, 0), 75);
    assert.equal(applyTax(1200, 0.2, true, 0), 200);
  });

  await t.test('zero rate / zero subtotal → 0', () => {
    assert.equal(applyTax(1000, 0, false, 0), 0);
    assert.equal(applyTax(0, 0.2, false, 0), 0);
  });

  await t.test('large values stay exact (no float drift)', () => {
    // 123456789 * 0.0825 = 10185185.0925 → half-up → 10185185
    assert.equal(applyTax(123456789, 0.0825, false, 0), 10185185);
  });

  await t.test('rejects non-integer (float) money input', () => {
    assert.throws(() => applyTax(100.5, 0.2, false, 0), /integer/);
  });
});

test('computeLineTotal — pure integer math', () => {
  assert.equal(computeLineTotal(500, [100, 50], 3), 1950);
  assert.equal(computeLineTotal(0, [], 5), 0);
});
