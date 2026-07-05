import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  voiceReducer,
  initialVoicePhase,
  decideTapAction,
  isStaleSession,
  shouldAnimateHalo,
  smoothAmplitude,
  extractAddToCartLabel,
  type VoicePhase,
} from '../state-machine.js';
import type { VoiceProposal } from '../types.js';

const proposal: VoiceProposal = {
  kind: 'ADD_TO_CART',
  args: { productId: 'p1', productName: 'Sufllaqe', qty: 2 },
  transcript: 'shto 2 sufllaqe',
  confidence: 0.9,
};

// ── The full happy-path walk (ui-spec §2 diagram, 1:1) ──────────────────────────────────────────
describe('voiceReducer — the state diagram (ui-spec.md §2)', () => {
  it('IDLE → DISCLOSURE → PERMISSION-REQUEST → LISTENING → TRANSCRIBING → CONFIRMING → APPLIED → IDLE (stateful path)', () => {
    let s: VoicePhase = initialVoicePhase;
    assert.equal(s.type, 'idle');

    s = voiceReducer(s, { type: 'SHOW_DISCLOSURE' });
    assert.equal(s.type, 'disclosure');

    s = voiceReducer(s, { type: 'DISCLOSURE_ACCEPT' });
    assert.equal(s.type, 'permission-request');

    s = voiceReducer(s, { type: 'PERMISSION_GRANTED' });
    assert.equal(s.type, 'listening');

    s = voiceReducer(s, { type: 'PARTIAL_TRANSCRIPT', text: 'shto' });
    assert.equal(s.type, 'listening');
    assert.equal((s as { partialTranscript: string }).partialTranscript, 'shto');

    s = voiceReducer(s, { type: 'TRANSCRIBING' });
    assert.equal(s.type, 'transcribing');

    s = voiceReducer(s, { type: 'STATEFUL_PENDING', proposal });
    assert.equal(s.type, 'confirming');
    assert.deepEqual((s as { proposal: VoiceProposal }).proposal, proposal);

    s = voiceReducer(s, { type: 'CONFIRM' });
    assert.equal(s.type, 'applied');
    assert.equal((s as { capability: string }).capability, 'STATEFUL');

    s = voiceReducer(s, { type: 'APPLIED_DONE' });
    assert.equal(s.type, 'idle');
  });

  it('READ_ONLY path: TRANSCRIBING → READ_ONLY_APPLIED → applied(READ_ONLY)', () => {
    let s: VoicePhase = { type: 'transcribing', lastPartialTranscript: 'rendit' };
    s = voiceReducer(s, { type: 'READ_ONLY_APPLIED', proposal: { ...proposal, kind: 'SET_SORT' } });
    assert.equal(s.type, 'applied');
    assert.equal((s as { capability: string }).capability, 'READ_ONLY');
  });

  it('returning user (voicePref already "on") skips the disclosure sheet', () => {
    // decideTapAction, not the reducer, decides this — asserted below — but the reducer itself
    // must also accept START_LISTENING directly from idle (no disclosure detour required).
    const s = voiceReducer(initialVoicePhase, { type: 'START_LISTENING' });
    assert.equal(s.type, 'permission-request');
  });
});

// ── No dead-ends (ui-spec §2: "no node dead-ends") ──────────────────────────────────────────────
describe('voiceReducer — no dead-ends (breaker-findings.md HIGH, verified non-bug)', () => {
  it('error phase escapes via RETRY', () => {
    const s = voiceReducer({ type: 'error', kind: 'no_match' }, { type: 'RETRY' });
    assert.equal(s.type, 'permission-request');
  });

  it('error phase also escapes via a bare START_LISTENING (a plain FAB re-tap, not just the Retry button)', () => {
    const s = voiceReducer({ type: 'error', kind: 'model_offline' }, { type: 'START_LISTENING' });
    assert.equal(s.type, 'permission-request');
  });

  it('applied phase escapes via a bare START_LISTENING even if the auto-return timer never fires', () => {
    const s = voiceReducer({ type: 'applied', proposal, capability: 'STATEFUL' }, { type: 'START_LISTENING' });
    assert.equal(s.type, 'permission-request');
  });

  it('disambiguating phase escapes via a bare START_LISTENING (barge-in-equivalent restart)', () => {
    const s = voiceReducer(
      { type: 'disambiguating', candidates: [{ id: 'a', label: 'A' }], transcript: 'x' },
      { type: 'START_LISTENING' },
    );
    assert.equal(s.type, 'permission-request');
  });

  it('every cancelable phase fail-safes to idle on CANCEL (no write, no lingering phase)', () => {
    const phases: VoicePhase[] = [
      { type: 'disclosure' },
      { type: 'permission-request' },
      { type: 'listening', partialTranscript: '' },
      { type: 'transcribing', lastPartialTranscript: '' },
      { type: 'confirming', proposal },
      { type: 'disambiguating', candidates: [], transcript: '' },
      { type: 'error', kind: 'try_again' },
    ];
    for (const phase of phases) {
      const next = voiceReducer(phase, { type: 'CANCEL' });
      assert.equal(next.type, 'idle', `CANCEL from ${phase.type} must reach idle`);
    }
  });
});

// ── Invalid transitions are true no-ops (same reference, no accidental mutation) ────────────────
describe('voiceReducer — invalid events are no-ops', () => {
  it('CONFIRM outside "confirming" does nothing and returns the SAME reference', () => {
    const s: VoicePhase = { type: 'idle' };
    const next = voiceReducer(s, { type: 'CONFIRM' });
    assert.equal(next, s);
  });

  it('PERMISSION_GRANTED outside "permission-request" is a no-op', () => {
    const s: VoicePhase = { type: 'idle' };
    const next = voiceReducer(s, { type: 'PERMISSION_GRANTED' });
    assert.equal(next, s);
  });

  it('RESET from idle returns the same reference (no needless re-render)', () => {
    const s: VoicePhase = { type: 'idle' };
    assert.equal(voiceReducer(s, { type: 'RESET' }), s);
  });

  it('RESET from any non-idle phase always reaches idle', () => {
    const s = voiceReducer({ type: 'listening', partialTranscript: 'x' }, { type: 'RESET' });
    assert.equal(s.type, 'idle');
  });
});

// ── decideTapAction (barge-in / disclosure / restart routing) ───────────────────────────────────
describe('decideTapAction', () => {
  it('first-ever tap (no consent decided) shows the disclosure sheet', () => {
    assert.equal(decideTapAction('idle', undefined), 'show-disclosure');
  });

  it('returning user (pref already decided) begins listening directly', () => {
    assert.equal(decideTapAction('idle', 'on'), 'begin-listening');
    assert.equal(decideTapAction('idle', 'off'), 'begin-listening');
  });

  it('mid-flow phases are barge-in regardless of voicePref', () => {
    for (const phase of ['permission-request', 'listening', 'transcribing', 'confirming', 'disambiguating'] as const) {
      assert.equal(decideTapAction(phase, 'on'), 'barge-in');
      assert.equal(decideTapAction(phase, undefined), 'barge-in');
    }
  });

  it('the disclosure sheet itself ignores FAB taps (it owns its own two buttons)', () => {
    assert.equal(decideTapAction('disclosure', undefined), 'noop');
  });

  it('applied / error are fresh-start phases, not barge-in (breaker-findings.md HIGH/MED, closed)', () => {
    assert.equal(decideTapAction('applied', 'on'), 'begin-listening');
    assert.equal(decideTapAction('error', 'on'), 'begin-listening');
  });
});

// ── Session-boundary guard (breaker-findings.md CRITICAL fix) ───────────────────────────────────
describe('isStaleSession — the CRITICAL fix (resolution.md)', () => {
  it('a callback tagged with the CURRENT session id is not stale', () => {
    assert.equal(isStaleSession(3, 3), false);
  });

  it('a callback tagged with a PAST session id (superseded by barge-in) is stale', () => {
    assert.equal(isStaleSession(3, 2), true);
  });

  it('monotonic increment always invalidates the previous session', () => {
    let current = 1;
    const oldSession = current;
    current += 1; // barge-in / retry / disclosure-accept always bumps forward
    assert.equal(isStaleSession(current, oldSession), true);
    assert.equal(isStaleSession(current, current), false);
  });
});

// ── Reduced-motion path (VOICE-UI-REFERENCE.md — the loop is NOT covered by --motion-* zero-out) ─
describe('shouldAnimateHalo — reduced-motion path', () => {
  it('animates only while listening AND motion is not reduced', () => {
    assert.equal(shouldAnimateHalo('listening', false), true);
  });

  it('never animates under prefers-reduced-motion, even while listening', () => {
    assert.equal(shouldAnimateHalo('listening', true), false);
  });

  it('never animates outside the listening phase, motion preference aside', () => {
    for (const phase of ['idle', 'transcribing', 'confirming', 'applied', 'error'] as const) {
      assert.equal(shouldAnimateHalo(phase, false), false);
    }
  });
});

describe('smoothAmplitude', () => {
  it('clamps raw input to the 0..1 domain the CSS custom property expects', () => {
    assert.ok(smoothAmplitude(0, 5) <= 1);
    assert.ok(smoothAmplitude(0, -5) >= 0);
  });

  it('is a one-pole low-pass (does not jump instantly to the raw value)', () => {
    const next = smoothAmplitude(0, 1);
    assert.ok(next > 0 && next < 1, `expected a damped step, got ${next}`);
  });
});

describe('extractAddToCartLabel', () => {
  it('reads qty + productName from well-formed matcher args', () => {
    assert.deepEqual(extractAddToCartLabel(proposal), { qty: 2, item: 'Sufllaqe' });
  });

  it('defends against a malformed args bag (missing/wrong-typed qty and name)', () => {
    const malformed: VoiceProposal = { kind: 'ADD_TO_CART', args: {}, transcript: 'shto diçka', confidence: 0.7 };
    const label = extractAddToCartLabel(malformed);
    assert.equal(label.qty, 1); // safe default, never 0/negative/NaN
    assert.equal(label.item, 'shto diçka'); // falls back to the transcript, never blank
  });

  it('rejects a non-positive/NaN qty in favour of the safe default of 1', () => {
    const bad: VoiceProposal = { kind: 'ADD_TO_CART', args: { qty: -3, productName: 'X' }, transcript: 't', confidence: 0.7 };
    assert.equal(extractAddToCartLabel(bad).qty, 1);
  });
});
