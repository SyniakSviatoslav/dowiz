// `node --test workers/api/public/admin/qty-adapter.test.mjs` (W-TKUI, TZ 1; CONTRACT-QTY section 1).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseMilli, showMilli, wireQty, toMilli, fromMilli, stepMilli } from './qty-adapter.js';

test('parseMilli: dot or comma, up to 3 decimals, kg/l words on mass only, textual (no f64 drift)', () => {
  assert.equal(parseMilli('40'), 40000);
  assert.equal(parseMilli('40.5'), 40500);
  assert.equal(parseMilli(' 0,5 '), 500);
  assert.equal(parseMilli(',5'), 500);
  assert.equal(parseMilli('0.001'), 1);
  assert.equal(parseMilli('0.1'), 100, '0.1 is exactly 100 milli, not 100.00000000000001');
  assert.equal(parseMilli('1,5 kg'), 1500000);
  assert.equal(parseMilli('0.25 l', 'ml'), 250000);
  assert.equal(parseMilli('1 500'), 1500000);
  assert.equal(parseMilli('40.5000'), 40500, 'trailing zeros past the third place are not a fourth decimal');
  assert.equal(parseMilli('0.0005'), null, 'a fourth decimal is refused, as the hub does');
  assert.equal(parseMilli('2 kg', 'unit'), null, 'a piece has no kilograms');
  assert.equal(parseMilli('-1'), null); assert.equal(parseMilli('abc'), null); assert.equal(parseMilli(''), null); assert.equal(parseMilli(null), null);
});

test('showMilli / wireQty / toMilli: short decimals, integer when whole, exact round trip', () => {
  assert.equal(showMilli(40500), '40.5'); assert.equal(showMilli(40000), '40'); assert.equal(showMilli(500), '0.5'); assert.equal(showMilli(1), '0.001'); assert.equal(showMilli(0), '0');
  assert.equal(wireQty(40000), 40, 'a whole line goes as the integer it always was');
  assert.equal(wireQty(40500), '40.5', 'a fraction goes as text');
  assert.equal(toMilli(40.5), 40500); assert.equal(toMilli(40), 40000); assert.equal(toMilli(40.5, 40500), 40500, 'qtyMilli wins when answered');
  assert.equal(fromMilli(40500), 40.5);
  for (const s of ['0.1', '0.2', '0.3', '1.1', '99.999']) assert.equal(showMilli(parseMilli(s)), s);
  assert.equal(stepMilli(5000, -10000), 0); assert.equal(stepMilli(500, 10000), 10500);
});
