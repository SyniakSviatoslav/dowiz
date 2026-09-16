// The money module's tests. `node public/lib/money.test.mjs`, or via
// `bash scripts/money-gate.sh`, which is what CI and the pre-push gate run.
//
// These are here rather than in a Rust crate because this is where the bug was:
// the formatting rule lived three times in three surfaces, and nothing compared
// them. A test that a Rust crate passes says nothing about what a browser shows.

import assert from 'node:assert/strict';
import { convert, format, formatter, decimalsOf, CURRENCIES } from './money.js';

let run = 0;
const test = (name, fn) => { fn(); run++; console.log(`  ok  ${name}`); };

const PPM = { ALL: 1_000_000, EUR: 10_900, USD: 12_577 };

test('conversion is integer and matches the rate', () => {
  // 2400 lek at 0.0109 EUR = 26.16 EUR = 2616 cents.
  assert.equal(convert(2400, PPM.EUR, 'ALL', 'EUR'), 2616);
  assert.equal(convert(900, PPM.EUR, 'ALL', 'EUR'), 981);
  assert.equal(convert(2400, PPM.USD, 'ALL', 'USD'), 3018);
});

test('a currency with no minor unit does not gain one', () => {
  // Lek has no cents. This rendered "900.00 ALL" when Intl was left to guess.
  assert.equal(decimalsOf('ALL'), 0);
  const out = format(900, 'ALL', 'sq');
  assert.ok(!out.includes('.00'), `lek must not show cents: ${out}`);
});

test('converting to the same currency changes nothing', () => {
  assert.equal(convert(900, PPM.ALL, 'ALL', 'ALL'), 900);
  assert.equal(formatter({ base: 'ALL', display: 'ALL', rates: { ppm: PPM } })(900),
               format(900, 'ALL', 'sq'));
});

test('a missing rate shows the REAL currency, never a made-up number', () => {
  // THE BUG THIS EXISTS FOR. Falling through to the identity conversion
  // rendered 900 lek as "€9.00": wrong by two orders of magnitude, entirely
  // plausible, on the one screen where being believed is the problem.
  const out = formatter({ base: 'ALL', display: 'EUR', rates: null, locale: 'en' })(900);
  assert.ok(!out.includes('€'), `must not claim euros with no rate: ${out}`);
  assert.ok(out.includes('900'), `must show the real amount: ${out}`);

  // Same when the rates object exists but lacks the requested currency.
  const partial = formatter({ base: 'ALL', display: 'USD', rates: { ppm: { EUR: 10_900 } }, locale: 'en' })(900);
  assert.ok(!partial.includes('$'), `must not claim dollars with no USD rate: ${partial}`);
});

test('every offered currency has decimals declared', () => {
  // A currency may not be half-supported: the switcher offers it, so the
  // formatter has to know how many digits it has.
  for (const c of CURRENCIES) {
    assert.ok(Number.isInteger(decimalsOf(c)), `${c} has no declared decimals`);
  }
});

test('junk amounts do not render NaN at a customer', () => {
  assert.equal(convert(undefined, PPM.EUR, 'ALL', 'EUR'), 0);
  assert.equal(convert(NaN, PPM.EUR, 'ALL', 'EUR'), 0);
  const out = formatter({ base: 'ALL', display: 'ALL', rates: { ppm: PPM } })(undefined);
  assert.ok(!/NaN/.test(out), `NaN reached the page: ${out}`);
});

console.log(`\n${run} money tests passed`);
