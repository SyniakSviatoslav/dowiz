import { test, describe } from 'node:test';
import assert from 'node:assert';
import { clampWindow } from '../src/lib/etaGather.js';

// SENSOR-BUS §1.1 (ADR-0009 v4) — the cap-last clamp is the value-level enforcement of
// range-never-point + eta_cap-absolute. These assertions fail if the order is ever wrong
// (R3-M1: v3 floored AFTER the cap and let hi exceed eta_cap).

const CAPS = { etaCapMin: 90, minWindowWidthMin: 10 };

describe('clampWindow — range-never-point + eta_cap (R3-M1 cap-last)', () => {
  test('normal band passes through, rounded', () => {
    const r = clampWindow(22, 34, CAPS);
    assert.deepStrictEqual(r, { loMin: 22, hiMin: 34 });
  });

  test('width floor: a band narrower than min_window_width_min is widened', () => {
    const r = clampWindow(20, 23, CAPS); // raw width 3 < 10
    assert.ok(r.hiMin - r.loMin >= CAPS.minWindowWidthMin, `band ${r.loMin}-${r.hiMin} < floor`);
  });

  test('never a point even when lo == hi', () => {
    const r = clampWindow(15, 15, CAPS);
    assert.ok(r.hiMin > r.loMin, `expected hi>lo, got ${r.loMin}-${r.hiMin}`);
  });

  test('eta_cap is ABSOLUTE — hi never exceeds the cap (R3-M1 regression)', () => {
    // v3 bug: lo=92 → floor pushed hi to 97 > cap 90. v4 clamps lo first, caps hi last.
    const r = clampWindow(92, 95, CAPS);
    assert.ok(r.hiMin <= CAPS.etaCapMin, `hi ${r.hiMin} must be ≤ cap ${CAPS.etaCapMin}`);
    assert.ok(r.loMin <= CAPS.etaCapMin - 1, `lo ${r.loMin} left no room under the cap`);
    assert.ok(r.hiMin > r.loMin, 'cap clamp must not collapse the band to a point');
  });

  test('huge inputs still clamp to the cap and keep a valid band', () => {
    const r = clampWindow(1000, 2000, CAPS);
    assert.strictEqual(r.hiMin, CAPS.etaCapMin);
    assert.ok(r.loMin >= 1 && r.loMin < r.hiMin);
  });

  test('never below 1, never NaN, on degenerate input', () => {
    const r = clampWindow(-5, Number.NaN, CAPS);
    assert.ok(Number.isFinite(r.loMin) && Number.isFinite(r.hiMin));
    assert.ok(r.loMin >= 1 && r.hiMin > r.loMin);
  });

  test('tiny cap below the width floor still yields hi >= lo + 1 (no inversion)', () => {
    const r = clampWindow(3, 4, { etaCapMin: 5, minWindowWidthMin: 10 });
    assert.ok(r.hiMin <= 5, `hi ${r.hiMin} must respect the absolute cap`);
    assert.ok(r.hiMin > r.loMin, `band inverted: ${r.loMin}-${r.hiMin}`);
  });
});
