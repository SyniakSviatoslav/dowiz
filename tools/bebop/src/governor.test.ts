// Bebop governor tests — L5 signal-health monitor. Every property has a falsifiable RED+GREEN case.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  Governor, pidStep, icir, spearman, loopResonance, landauerFloor, bitsErased,
  detectAnomaly, simulatePlant, clamp, type TelemetrySample,
} from './governor.ts';

const baseCfg = {
  kp: 1.4, ki: 0.22, kd: 1.5, iMin: -1, iMax: 1, uMin: 0, uMax: 1,
  targetQuality: 0.9, deadIC: 0.02, icirVolatile: 0.3,
  plantM: 1, plantB: 0.6, samplePeriod: 0, anomalyK: 3, maxStep: 1,
};

function mk(): Governor { return new Governor(baseCfg); }

function sample(over: Partial<TelemetrySample>): TelemetrySample {
  return { t: 0, predictedQuality: 0.9, actualQuality: 0.4, cost: 1e-18, volume: 100, ...over };
}

// ── PID ───────────────────────────────────────────────────────────────────────

test('GREEN: PID drives a 1st-order agent to the quality setpoint', () => {
  const g = mk();
  const us: number[] = [];
  let actual = 0.4;
  for (let k = 0; k < 60; k++) {
    const st = g.step(sample({ actualQuality: actual, predictedQuality: actual + 0.0 }));
    us.push(st.authority);
    actual = simulatePlant([st.authority], 0.3, actual)[1];
  }
  assert.ok(Math.abs(actual - 0.9) < 0.05, `final quality ${actual} should be near 0.9`);
});

test('RED: a proportional-only loop (Ki=0) cannot hold the setpoint under steady bias', () => {
  const g = new Governor({ ...baseCfg, ki: 0 });
  const us: number[] = [];
  let actual = 0.4;
  for (let k = 0; k < 60; k++) {
    const st = g.step(sample({ actualQuality: actual }));
    us.push(st.authority);
    actual = simulatePlant([st.authority], 0.3, actual)[1];
  }
  assert.ok(Math.abs(actual - 0.9) > 0.1, `P-only should lag setpoint, got ${actual}`);
});

test('GREEN: integral anti-windup clamp keeps the accumulator bounded', () => {
  const st0 = { integral: 0, prevError: 0 };
  let st = st0;
  for (let k = 0; k < 1000; k++) st = pidStep({ ...baseCfg }, st, -1); // sustained full bias
  assert.ok(st.integral <= baseCfg.iMax + 1e-9 && st.integral >= baseCfg.iMin - 1e-9, 'integral stayed clamped');
});

// ── ICIR / factor health ───────────────────────────────────────────────────────

test('GREEN: a stable, predictive factor earns HIGH ICIR and stays authoritative', () => {
  const g = mk();
  let actual = 0.6;
  let state: any;
  for (let k = 0; k < 30; k++) {
    actual = clamp(actual + (Math.random() - 0.5) * 0.1, 0, 1);
    const pred = clamp(actual + (Math.random() - 0.5) * 0.05, 0, 1); // tight, correct self-knowledge
    state = g.step(sample({ actualQuality: actual, predictedQuality: pred }));
  }
  assert.ok(state.icir !== null && state.icir > 0.3, `ICIR ${state.icir} should be healthy`);
  assert.equal(state.factorStatus, 'healthy');
  assert.ok(state.authority > baseCfg.uMin, 'healthy factor keeps authority');
});

test('RED: a dead factor (no predictive power) is KILLED → authority floored', () => {
  const g = mk();
  let state: any;
  for (let k = 0; k < 30; k++) {
    // predicted is the EXACT INVERSE of actual ⇒ zero/negative predictive power ⇒ dead
    const actual = Math.random();
    state = g.step(sample({ actualQuality: actual, predictedQuality: 1 - actual }));
  }
  assert.ok(state.icir !== null && state.icir < baseCfg.deadIC, `dead ICIR ${state.icir}`);
  assert.equal(state.factorStatus, 'dead');
  assert.equal(state.authority, baseCfg.uMin, 'dead factor → authority floored to uMin');
});

test('GREEN: spearman of identical series = 1; of perfectly inverted = -1', () => {
  assert.ok(Math.abs(spearman([1, 2, 3, 4], [1, 2, 3, 4]) - 1) < 1e-9);
  assert.ok(Math.abs(spearman([1, 2, 3, 4], [4, 3, 2, 1]) + 1) < 1e-9);
});

test('GREEN: icir of a constant non-zero series is +Inf (perfectly stable)', () => {
  assert.equal(icir([0.1, 0.1, 0.1, 0.1]), Infinity);
});

test('RED: icir undefined for <2 samples (null)', () => {
  assert.equal(icir([0.3]), null);
});

// ── resonance prediction (predict change before applying) ──────────────────────

test('GREEN: a gain change that drops ζ<0.707 is flagged risky BEFORE applying', () => {
  const safe = loopResonance(1.4, 1.5, 1, 0.6); // ζ≈0.887 → well-damped
  const risky = loopResonance(8.0, 0.05, 1, 0.1); // high Kp, low Kd → under-damped
  assert.ok(!safe.risky, 'safe gains should not be risky');
  assert.ok(risky.risky && risky.zeta < 1 / Math.sqrt(2), 'under-damped step must be risky');
  assert.ok(risky.mr > 1.2, 'resonance peak magnification should be >1');
});

test('RED: a well-damped loop has magnification ≈ 1 (no harmonic blow-up)', () => {
  const r = loopResonance(1.0, 1.5, 1, 0.6);
  assert.ok(!r.risky && r.mr <= 1.01, `well-damped Mr ${r.mr} should be ~1`);
});

test('GREEN: discrete alias risk when ωn·T approaches Nyquist band', () => {
  const r = loopResonance(50, 1, 1, 0.6, 0.25); // wn≈7.07, wn*T≈1.77 > 0.3
  assert.ok(r.aliasRisk, 'high-frequency loop should flag alias risk');
});

// ── thermodynamics (Landauer) ──────────────────────────────────────────────────

test('GREEN: Landauer floor ≈ 2.87e-21 J/bit at 300K', () => {
  const f = landauerFloor(1, 300);
  assert.ok(Math.abs(f - 2.87e-21) < 1e-23, `floor ${f}`);
});

test('RED: negative bits throws (cannot erase negative information)', () => {
  assert.throws(() => landauerFloor(-1));
});

test('GREEN: bitsErased is monotonic log2 of volume', () => {
  assert.equal(bitsErased(2), 2);
  assert.ok(bitsErased(1000) > bitsErased(10));
});

// ── anomaly detection (operator priority) ─────────────────────────────────────

test('GREEN: a volume spike >3σ from history is flagged as anomaly', () => {
  const hist = Array.from({ length: 20 }, () => 100 + (Math.random() - 0.5) * 10);
  assert.ok(detectAnomaly(hist, 1000, 3), 'extreme spike must flag');
});

test('RED: an in-band volume is NOT an anomaly', () => {
  const hist = Array.from({ length: 20 }, () => 100 + (Math.random() - 0.5) * 10); // normal spread ~±5
  assert.equal(detectAnomaly(hist, 105, 3), false);
});

// ── governor integration: dead factor floors authority even when PID wants more ─

test('GREEN: governor overrides PID authority for a dead factor (kill-switch beats integral)', () => {
  const g = mk();
  // force a huge error so PID alone would push authority high
  let state: any;
  for (let k = 0; k < 30; k++) {
    state = g.step(sample({ actualQuality: 0.0, predictedQuality: Math.random() }));
  }
  // dead factor → authority must be floored regardless of PID
  assert.equal(state.authority, baseCfg.uMin);
});
