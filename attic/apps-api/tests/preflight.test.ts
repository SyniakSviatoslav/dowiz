import { test } from 'node:test';
import assert from 'node:assert';
import { evaluatePreflight, type PreflightInput } from '../src/lib/preflight.js';

function makeInput(overrides: Partial<PreflightInput> = {}): PreflightInput {
  return {
    lines: [],
    signals: {
      velocityPhoneCount: 0,
      velocityIpCount: 0,
      noShowCount: 0,
      noShowAgeDays: null,
      completedCount: 0,
      otpRequired: false,
      otpVerified: false,
    },
    acknowledgedCodes: [],
    ...overrides,
  };
}

// ─── HARD_BLOCK tests ───────────────────────────────────────────────
test('evaluatePreflight: clean order → outcome clean, no reasons', () => {
  const result = evaluatePreflight(makeInput());
  assert.strictEqual(result.outcome, 'clean');
  assert.strictEqual(result.reasons.length, 0);
});

test('evaluatePreflight: product not in menu → hard_block', () => {
  const result = evaluatePreflight(makeInput({
    lines: [{
      productId: 'abc', quantity: 1, modifierIds: [],
      productAvailable: null, modifierAvailability: {},
    }],
  }));
  assert.strictEqual(result.outcome, 'hard_block');
  assert.strictEqual(result.reasons.length, 1);
  assert.strictEqual(result.reasons[0].code, 'item_unavailable');
  assert.strictEqual(result.reasons[0].severity, 'objective');
  assert.strictEqual(result.reasons[0].itemId, 'abc');
});

test('evaluatePreflight: product in stop-list → hard_block', () => {
  const result = evaluatePreflight(makeInput({
    lines: [{
      productId: 'abc', quantity: 1, modifierIds: [],
      productAvailable: false, modifierAvailability: {},
    }],
  }));
  assert.strictEqual(result.outcome, 'hard_block');
  assert.strictEqual(result.reasons[0].code, 'item_unavailable');
});

test('evaluatePreflight: modifier not in menu → hard_block', () => {
  const result = evaluatePreflight(makeInput({
    lines: [{
      productId: 'abc', quantity: 1, modifierIds: ['mod1'],
      productAvailable: true,
      modifierAvailability: { mod1: null },
    }],
  }));
  assert.strictEqual(result.outcome, 'hard_block');
  assert.strictEqual(result.reasons[0].itemId, 'mod1');
});

test('evaluatePreflight: modifier in stop-list → hard_block', () => {
  const result = evaluatePreflight(makeInput({
    lines: [{
      productId: 'abc', quantity: 1, modifierIds: ['mod1'],
      productAvailable: true,
      modifierAvailability: { mod1: false },
    }],
  }));
  assert.strictEqual(result.outcome, 'hard_block');
});

// ─── SOFT_CONFIRM tests ─────────────────────────────────────────────
test('evaluatePreflight: velocity exceeded → soft_confirm', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 5, velocityIpCount: 0,
      noShowCount: 0, noShowAgeDays: null, completedCount: 0,
      otpRequired: false, otpVerified: false,
    },
  }));
  assert.strictEqual(result.outcome, 'soft_confirm');
  assert.strictEqual(result.requiresConfirmation, true);
  assert.ok(result.reasons.some(r => r.code === 'velocity'));
});

test('evaluatePreflight: IP velocity exceeded → soft_confirm', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 0, velocityIpCount: 5,
      noShowCount: 0, noShowAgeDays: null, completedCount: 0,
      otpRequired: false, otpVerified: false,
    },
  }));
  assert.strictEqual(result.outcome, 'soft_confirm');
  assert.ok(result.reasons.some(r => r.code === 'velocity'));
});

test('evaluatePreflight: active no-show → soft_confirm', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 0, velocityIpCount: 0,
      noShowCount: 3, noShowAgeDays: 1, completedCount: 5,
      otpRequired: false, otpVerified: false,
    },
  }));
  assert.strictEqual(result.outcome, 'soft_confirm');
  assert.ok(result.reasons.some(r => r.code === 'no_show_history'));
});

test('evaluatePreflight: OTP required → soft_confirm with requiresOtp', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 0, velocityIpCount: 0,
      noShowCount: 0, noShowAgeDays: null, completedCount: 0,
      otpRequired: true, otpVerified: false,
    },
  }));
  assert.strictEqual(result.outcome, 'soft_confirm');
  assert.strictEqual(result.requiresOtp, true);
  assert.ok(result.reasons.some(r => r.code === 'otp_required'));
});

// ─── SOFT_CONFIRM → CLEAN after acknowledge ─────────────────────────
test('evaluatePreflight: velocity acknowledged + no OTP → clean', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 5, velocityIpCount: 0,
      noShowCount: 0, noShowAgeDays: null, completedCount: 0,
      otpRequired: false, otpVerified: false,
    },
    acknowledgedCodes: ['velocity'],
  }));
  assert.strictEqual(result.outcome, 'clean');
  assert.ok(result.confirmedReasons?.includes('velocity'));
});

test('evaluatePreflight: no-show acknowledged → clean', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 0, velocityIpCount: 0,
      noShowCount: 3, noShowAgeDays: 1, completedCount: 5,
      otpRequired: false, otpVerified: false,
    },
    acknowledgedCodes: ['no_show_history'],
  }));
  assert.strictEqual(result.outcome, 'clean');
});

test('evaluatePreflight: OTP required + verified + ack velocity → clean', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 5, velocityIpCount: 0,
      noShowCount: 0, noShowAgeDays: null, completedCount: 0,
      otpRequired: true, otpVerified: true,
    },
    acknowledgedCodes: ['velocity'],
  }));
  assert.strictEqual(result.outcome, 'clean');
});

test('evaluatePreflight: OTP required + not verified + ack → still soft_confirm', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 5, velocityIpCount: 0,
      noShowCount: 0, noShowAgeDays: null, completedCount: 0,
      otpRequired: true, otpVerified: false,
    },
    acknowledgedCodes: ['velocity'],
  }));
  assert.strictEqual(result.outcome, 'soft_confirm');
  assert.strictEqual(result.requiresOtp, true);
});

// ─── DECAY tests ────────────────────────────────────────────────────
test('evaluatePreflight: old no-show (outside 90d window) → no signal', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 0, velocityIpCount: 0,
      noShowCount: 3, noShowAgeDays: 95, completedCount: 5,
      otpRequired: false, otpVerified: false,
    },
  }));
  // ageDays > 90 → outside window, but decay factor is very low anyway
  // The function checks both strength > 0.5 AND ageDays <= 90
  assert.strictEqual(result.outcome, 'clean');
});

test('evaluatePreflight: noShowAgeDays=null → no signal', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 0, velocityIpCount: 0,
      noShowCount: 3, noShowAgeDays: null, completedCount: 5,
      otpRequired: false, otpVerified: false,
    },
  }));
  assert.strictEqual(result.outcome, 'clean');
});

test('evaluatePreflight: complete no-show cleanup (counter alone) → no signal', () => {
  const result = evaluatePreflight(makeInput({
    signals: {
      velocityPhoneCount: 0, velocityIpCount: 0,
      noShowCount: 3, noShowAgeDays: 60, completedCount: 5,
      otpRequired: false, otpVerified: false,
    },
  }));
  // 60 days + 3 no-shows with 5 completes → strength = 3 * exp(-60/30) / 5 = 3 * 0.135 / 5 = 0.081
  // Below 0.5 → clean
  assert.strictEqual(result.outcome, 'clean');
});

// ─── Edge cases ─────────────────────────────────────────────────────
test('evaluatePreflight: mixed hard_block + soft → hard_block wins', () => {
  const result = evaluatePreflight(makeInput({
    lines: [{
      productId: 'abc', quantity: 1, modifierIds: [],
      productAvailable: null, modifierAvailability: {},
    }],
    signals: {
      velocityPhoneCount: 5, velocityIpCount: 0,
      noShowCount: 0, noShowAgeDays: null, completedCount: 0,
      otpRequired: false, otpVerified: false,
    },
  }));
  assert.strictEqual(result.outcome, 'hard_block');
  assert.strictEqual(result.reasons.length, 1);
  assert.strictEqual(result.reasons[0].code, 'item_unavailable');
});
