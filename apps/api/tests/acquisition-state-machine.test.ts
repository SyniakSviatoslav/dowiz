import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  assertTransition,
  canTransition,
  everyNonTerminalHasExit,
  AcquisitionTransitionError,
  TERMINAL_STATES,
  REQUIRES_REASON,
  _LEGAL_FOR_TEST,
} from '../src/modules/acquisition/state-machine.js';
import { ACQUISITION_STATES } from '../src/modules/acquisition/types.js';

test('happy-path edges are legal', () => {
  const path = [
    'SOURCED',
    'PLACE_INGESTED',
    'MENU_EXTRACTED',
    'ENRICHED',
    'PROVISIONED',
    'VERIFIED',
    'CLAIM_OFFERED',
    'CLAIMED',
  ] as const;
  for (let i = 0; i < path.length - 1; i++) {
    assert.ok(canTransition(path[i], path[i + 1]), `${path[i]} → ${path[i + 1]} must be legal`);
  }
});

test('illegal transition throws a typed error', () => {
  assert.throws(() => assertTransition('SOURCED', 'VERIFIED'), AcquisitionTransitionError);
  assert.throws(() => assertTransition('CLAIMED', 'SOURCED'), AcquisitionTransitionError);
  assert.throws(() => assertTransition('ABANDONED', 'SOURCED'), AcquisitionTransitionError);
});

test('every non-terminal state has an exit (no silent stall)', () => {
  assert.ok(everyNonTerminalHasExit());
  // explicit: MANUAL_REVIEW (the classic sink) must resolve somewhere
  assert.ok(_LEGAL_FOR_TEST.MANUAL_REVIEW.length > 0);
});

test('terminal states have no outgoing edges', () => {
  for (const s of TERMINAL_STATES) {
    assert.equal(_LEGAL_FOR_TEST[s].length, 0, `${s} must be terminal`);
  }
  assert.deepEqual([...TERMINAL_STATES].sort(), ['ABANDONED', 'CLAIMED', 'DISQUALIFIED']);
});

test('the state list and the transition graph cover the same states', () => {
  assert.deepEqual(
    [...ACQUISITION_STATES].sort(),
    (Object.keys(_LEGAL_FOR_TEST) as string[]).sort(),
  );
});

test('CLAIMED (success terminal) does not require a failure_reason; exits do', () => {
  assert.ok(!REQUIRES_REASON.has('CLAIMED'));
  for (const s of ['MENU_NOT_FOUND', 'LOW_QUALITY', 'MANUAL_REVIEW', 'DISQUALIFIED', 'ABANDONED'] as const) {
    assert.ok(REQUIRES_REASON.has(s), `${s} must require a reason`);
  }
});
