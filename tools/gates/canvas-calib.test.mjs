// canvas-calib.mjs `calibrate`: the mark never drops below first_ms, scales by the measured
// reference ratio, and refuses (never passes) a machine past the cap. Run by canvas-frame.prove.sh.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { calibrate } from './canvas-calib.mjs';

const B = { first_ms: 300, ref_nominal_ms: 150, cap: 3 };

test('a machine at or faster than the nominal is held to the absolute mark (CI is not loosened)', () => {
  assert.equal(calibrate([90, 150, 100], B).mark, 300);
  assert.equal(calibrate([40, 50, 60], B).mark, 300);
});

test('a slower machine scales the mark by the MEDIAN reference, not the worst', () => {
  const c = calibrate([203, 193, 900], B);
  assert.equal(c.ref, 203);
  assert.equal(c.mark, Math.round(300 * 203 / 150));
  assert.equal(c.refused, undefined);
});

test('past the cap the run is refused, never judged', () => {
  const c = calibrate([451, 460, 470], B);
  assert.match(c.refused, /too slow to judge/);
});
