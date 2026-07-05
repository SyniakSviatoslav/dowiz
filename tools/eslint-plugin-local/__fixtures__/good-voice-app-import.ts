// Test fixture for no-voice-app-import (valid — must NOT be flagged).
// CORRECT: packages/voice imports only in-package modules (relative) and pure data/node builtins.
// It never reaches into apps/web, a fetch/API client, or a Cart* mutator — the engine stays a pure
// source; the ConfirmationGate + VoiceHandlers port (owned by apps/web, injected in) is the sole
// write sink. (ADR-0015 §6 / proposal §6 guardrail #1 G1b.)

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

type IntentKind = 'ADD_TO_CART' | 'SET_SORT';
type Capability = 'READ_ONLY' | 'STATEFUL' | 'REJECT';

const CAPABILITY_TABLE: Record<IntentKind, Exclude<Capability, 'REJECT'>> = {
  ADD_TO_CART: 'STATEFUL',
  SET_SORT: 'READ_ONLY',
};

function classifyGood(kind: string): Capability {
  return (CAPABILITY_TABLE as Record<string, Capability | undefined>)[kind] ?? 'REJECT';
}

describe('good fixture', () => {
  it('classifies fail-closed', () => {
    assert.equal(classifyGood('ADD_TO_CART'), 'STATEFUL');
  });
});
