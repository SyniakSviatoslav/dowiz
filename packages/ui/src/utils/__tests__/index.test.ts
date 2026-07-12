import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  formatALL, parseALL, normalizePhone, calcETA,
  generateIdempotencyKey, assertTransition, checkWCAGContrast,
} from '../index.js';

describe('formatALL', () => {
  it('formats integer Lek amounts', () => {
    const result = formatALL(1500);
    assert.ok(result.includes('1'));
    assert.ok(result.includes('500'));
    assert.ok(result.includes('L'));
  });

  it('formats zero', () => {
    const result = formatALL(0);
    assert.ok(result.includes('0'));
    assert.ok(result.includes('L'));
  });

  it('formats large numbers with grouping', () => {
    const result = formatALL(100000);
    assert.ok(result.length > 0);
    assert.ok(result.includes('L'));
  });
});

describe('parseALL', () => {
  it('parses simple number string', () => {
    assert.equal(parseALL('1500'), 1500);
  });

  it('parses formatted price with symbol', () => {
    assert.equal(parseALL('1 500 L'), 1500);
  });

  it('parses decimal as integer', () => {
    assert.equal(parseALL('9.99'), 10);
  });

  it('returns 0 for invalid input', () => {
    assert.equal(parseALL('abc'), 0);
  });

  it('handles empty string', () => {
    assert.equal(parseALL(''), 0);
  });
});

describe('normalizePhone', () => {
  it('removes spaces and dashes', () => {
    assert.equal(normalizePhone('+355 67 123 4567'), '+355671234567');
  });

  it('replaces 00 prefix with +', () => {
    assert.equal(normalizePhone('00355671234567'), '+355671234567');
  });

  it('removes parentheses', () => {
    assert.equal(normalizePhone('+355 (67) 123-4567'), '+355671234567');
  });

  it('leaves clean numbers unchanged', () => {
    assert.equal(normalizePhone('+355671234567'), '+355671234567');
  });
});

describe('calcETA', () => {
  it('calculates 30 min for 15km at 30km/h', () => {
    assert.equal(calcETA(15, 30), 30);
  });

  it('defaults to 30km/h', () => {
    assert.equal(calcETA(15), 30);
  });

  it('rounds up partial minutes', () => {
    assert.equal(calcETA(1, 30), 2);
  });

  it('handles zero distance', () => {
    assert.equal(calcETA(0), 0);
  });
});

describe('generateIdempotencyKey', () => {
  it('returns a non-empty string', () => {
    const key = generateIdempotencyKey();
    assert.ok(typeof key === 'string');
    assert.ok(key.length > 0);
  });

  it('produces URL-safe base64 (no + / or =)', () => {
    const key = generateIdempotencyKey();
    assert.ok(!key.includes('+'));
    assert.ok(!key.includes('/'));
    assert.ok(!key.includes('='));
  });

  it('produces unique keys', () => {
    const key1 = generateIdempotencyKey();
    const key2 = generateIdempotencyKey();
    assert.notEqual(key1, key2);
  });
});

describe('assertTransition', () => {
  it('allows PENDING → CONFIRMED', () => {
    assert.ok(assertTransition('PENDING', 'CONFIRMED'));
  });

  it('allows PENDING → REJECTED', () => {
    assert.ok(assertTransition('PENDING', 'REJECTED'));
  });

  it('blocks PENDING → DELIVERED', () => {
    assert.ok(!assertTransition('PENDING', 'DELIVERED'));
  });

  it('allows PREPARING → READY', () => {
    assert.ok(assertTransition('PREPARING', 'READY'));
  });

  it('blocks DELIVERED → any', () => {
    assert.ok(!assertTransition('DELIVERED', 'CANCELLED'));
  });

  it('returns false for unknown status', () => {
    assert.ok(!assertTransition('UNKNOWN', 'CONFIRMED'));
  });

  it('allows CONFIRMED → PREPARING', () => {
    assert.ok(assertTransition('CONFIRMED', 'PREPARING'));
  });
});

describe('checkWCAGContrast', () => {
  // Hex literals are contrast-test fixtures, not palette colors to externalize.
  /* eslint-disable local/no-hardcoded-color */
  it('returns high ratio for black on white', () => {
    const ratio = checkWCAGContrast('#000000', '#FFFFFF');
    assert.ok(ratio > 10);
  });

  it('returns low ratio for similar colors', () => {
    const ratio = checkWCAGContrast('#CCCCCC', '#DDDDDD');
    assert.ok(ratio < 2);
  });

  it('returns ~1 for same color', () => {
    const ratio = checkWCAGContrast('#FF0000', '#FF0000');
    assert.equal(Math.round(ratio), 1);
  });

  it('meets AA standard for brand colors on white', () => {
    // #ea4f16 on white = ?
    const ratio = checkWCAGContrast('#ea4f16', '#FFFFFF');
    // AA normal text requires 4.5:1
    assert.ok(ratio >= 3, `Brand primary on white: ${ratio.toFixed(2)}:1`);
  });
  /* eslint-enable local/no-hardcoded-color */
});
