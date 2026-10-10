// canvas-calib.mjs `calibrate`: the mark is first_ms scaled by the measured reference ratio in BOTH
// directions (a fast machine gets a tighter mark), and a machine past the cap is refused, never
// passed. Run by canvas-frame.prove.sh.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { calibrate } from './canvas-calib.mjs';

const B = { first_ms: 300, ref_nominal_ms: 150, cap: 3 };

test('a faster machine gets a TIGHTER mark: the gate is the board/reference ratio, not 300 ms', () => {
  assert.equal(calibrate([90, 150, 100], B).mark, 200);
  // CI, run 38068929237: reference 52.8 ms -> mark 106; the real board (61.6) passes, a first frame
  // twice as slow (~123) fails. Under the old floor at 300 both passed.
  const ci = calibrate([51.7, 52.8, 68.5], B);
  assert.equal(ci.mark, 106);
  assert.ok(61.6 <= ci.mark && 123 > ci.mark);
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
