import { test } from 'node:test';
import assert from 'node:assert/strict';

// PULL-TO-REFRESH pure math (red→green proof for usePullToRefresh).
// The gesture math is exported as pure functions precisely so it can be asserted
// here without a DOM: resistance damping, threshold progress, and the
// scroll-position guard that stops a mid-scroll drag from ever firing a refresh.
import {
  applyPullResistance,
  computePullProgress,
  shouldActivatePull,
  PULL_THRESHOLD_PX,
  PULL_RESISTANCE,
  MAX_PULL_PX,
} from '../use-pull-to-refresh.js';

// ── applyPullResistance ────────────────────────────────────────────────────────

test('resistance: a pull is dampened — finger travel is never mapped 1:1', () => {
  // A naive un-resisted implementation (distance = deltaY) fails this.
  assert.ok(applyPullResistance(100) < 100, 'expected 100px of finger travel to move content < 100px');
  assert.equal(applyPullResistance(100), 100 * PULL_RESISTANCE);
});

test('resistance: non-positive / invalid deltas produce zero distance', () => {
  assert.equal(applyPullResistance(0), 0);
  assert.equal(applyPullResistance(-40), 0, 'an upward drag is not a pull');
  assert.equal(applyPullResistance(Number.NaN), 0, 'NaN never leaks into a transform');
});

test('resistance: distance is capped at MAX_PULL_PX no matter how far the finger goes', () => {
  assert.equal(applyPullResistance(10_000), MAX_PULL_PX);
  assert.ok(applyPullResistance(10_000) <= MAX_PULL_PX);
});

test('resistance: monotonic — pulling further never moves content back up', () => {
  let prev = -1;
  for (let d = 0; d <= 600; d += 25) {
    const cur = applyPullResistance(d);
    assert.ok(cur >= prev, `resisted(${d}) = ${cur} regressed below ${prev}`);
    prev = cur;
  }
});

// ── computePullProgress ────────────────────────────────────────────────────────
// progress = resisted(deltaY) / threshold, clamped to [0, 1]. With the default
// resistance of 0.5 the finger must travel 2× the threshold to arm the refresh.

test('progress: below threshold → strictly between 0 and 1 (indicator arming, not armed)', () => {
  const justBelow = (PULL_THRESHOLD_PX / PULL_RESISTANCE) - 2; // resisted distance = threshold - 1
  const p = computePullProgress(justBelow, PULL_THRESHOLD_PX);
  assert.ok(p > 0 && p < 1, `expected 0 < progress < 1, got ${p}`);
});

test('progress: exactly at threshold → 1 (armed)', () => {
  const atThreshold = PULL_THRESHOLD_PX / PULL_RESISTANCE; // resisted distance = threshold
  assert.equal(computePullProgress(atThreshold, PULL_THRESHOLD_PX), 1);
});

test('progress: far above threshold → clamped to 1, never overshoots', () => {
  assert.equal(computePullProgress(50_000, PULL_THRESHOLD_PX), 1);
});

test('progress: zero / upward drag → 0', () => {
  assert.equal(computePullProgress(0, PULL_THRESHOLD_PX), 0);
  assert.equal(computePullProgress(-120, PULL_THRESHOLD_PX), 0);
});

test('progress: degenerate threshold (0 / negative / NaN) never divides by zero', () => {
  assert.equal(computePullProgress(80, 0), 0);
  assert.equal(computePullProgress(80, -10), 0);
  assert.equal(computePullProgress(80, Number.NaN), 0);
});

test('progress: default threshold is the exported PULL_THRESHOLD_PX', () => {
  assert.equal(
    computePullProgress(PULL_THRESHOLD_PX / PULL_RESISTANCE),
    computePullProgress(PULL_THRESHOLD_PX / PULL_RESISTANCE, PULL_THRESHOLD_PX),
  );
});

// ── shouldActivatePull (scroll guard) ─────────────────────────────────────────
// The refresh gesture must ONLY arm when the scroll container sits at its very
// top. A downward drag mid-list is scrolling, never a refresh.

test('scroll guard: at top + downward drag → activates', () => {
  assert.equal(shouldActivatePull(0, 24), true);
});

test('scroll guard: mid-scroll downward drag NEVER activates (the core guard)', () => {
  assert.equal(shouldActivatePull(1, 24), false, 'scrollTop=1 is not the top');
  assert.equal(shouldActivatePull(480, 300), false);
});

test('scroll guard: at top but dragging up / not moving → does not activate', () => {
  assert.equal(shouldActivatePull(0, 0), false);
  assert.equal(shouldActivatePull(0, -12), false);
});

test('scroll guard: iOS rubber-band negative scrollTop still counts as top', () => {
  assert.equal(shouldActivatePull(-8, 24), true);
});
