// Voice control — the finite state machine (docs/design/voice-control/ui-spec.md §2 / §3,
// docs/design/voice-control/PHASE1-IMPLEMENTATION-PLAN.md §3). Pure, framework-free, and
// unit-testable without a DOM: `voiceReducer` is a plain `(state, event) => state` function;
// `useVoiceControl.ts` is the only file that wraps it in React (`useReducer`) and wires side
// effects (gate.submit/confirm/cancel, engine.start/abort).
//
//   IDLE → (tap) DISCLOSURE → (accept) PERMISSION-REQUEST → LISTENING → TRANSCRIBING →
//   { APPLIED (read-only or confirmed-stateful) | CONFIRMING (stateful, awaiting human tap) |
//     DISAMBIGUATING (ambiguous tie) | ERROR (mic/model/no-match/unavailable) } → IDLE
//
// A new tap during LISTENING/TRANSCRIBING/CONFIRMING/PERMISSION-REQUEST/DISAMBIGUATING is
// barge-in (plan §3.3) — handled by the hook (cancel + reset + restart), not by this reducer.
//
// Council-hardened (docs/design/voice-pr3-ui-statemachine/{breaker-findings,resolution}.md):
// the breaker found that a stray/late `onProposal`/`onPermissionGranted` callback from an ABORTED
// session could still reach `gate.submit()` / dispatch a valid transition on a phase that looks
// identical to the new session's phase (a pure reducer with no session id cannot tell "listening
// of session N" from "listening of session N+1" apart). The fix lives in the HOOK (a session-id
// guard wraps every engine callback BEFORE it can call gate.submit() or dispatch — see
// `useVoiceControl.ts`'s `isStaleSession`), not in this reducer — this file stays a pure
// state-shape transform; `isStaleSession` is exported from here so it has the same
// framework-free, DOM-free unit-test treatment as the rest of this module.

import type { DisambiguationCandidate, VoiceErrorKind, VoicePref, VoiceProposal } from './types.js';

export type VoicePhase =
  | { type: 'idle' }
  | { type: 'disclosure' }
  | { type: 'permission-request' }
  // partialTranscript is EPHEMERAL — never log, never serialize to telemetry/crash-reporting
  // (ui-spec §8, C-1/R2-C zero-egress). It is opaque display data only.
  | { type: 'listening'; partialTranscript: string }
  | { type: 'transcribing'; lastPartialTranscript: string }
  | { type: 'confirming'; proposal: VoiceProposal }
  | { type: 'applied'; proposal: VoiceProposal; capability: 'READ_ONLY' | 'STATEFUL' }
  | { type: 'disambiguating'; candidates: readonly DisambiguationCandidate[]; transcript: string }
  | { type: 'error'; kind: VoiceErrorKind; candidates?: readonly DisambiguationCandidate[] };

export type VoiceEvent =
  | { type: 'SHOW_DISCLOSURE' }
  | { type: 'DISCLOSURE_DECLINE' }
  | { type: 'DISCLOSURE_ACCEPT' }
  | { type: 'START_LISTENING' }
  | { type: 'PERMISSION_GRANTED' }
  | { type: 'PERMISSION_DENIED' }
  | { type: 'PARTIAL_TRANSCRIPT'; text: string }
  | { type: 'TRANSCRIBING' }
  | { type: 'READ_ONLY_APPLIED'; proposal: VoiceProposal }
  | { type: 'STATEFUL_PENDING'; proposal: VoiceProposal }
  | { type: 'NO_MATCH'; candidates?: readonly DisambiguationCandidate[] }
  | { type: 'AMBIGUOUS'; candidates: readonly DisambiguationCandidate[]; transcript: string }
  | { type: 'ENGINE_ERROR'; kind: VoiceErrorKind }
  | { type: 'SELECT_CANDIDATE' }
  | { type: 'CONFIRM' }
  | { type: 'CANCEL' }
  | { type: 'APPLIED_DONE' }
  | { type: 'RETRY' }
  | { type: 'RESET' };

export const initialVoicePhase: VoicePhase = { type: 'idle' };

/**
 * Phases from which a fresh push-to-talk session may begin — a bare re-tap of the FAB (not just
 * the dedicated Retry button) restarts from any of these. `'error'` and `'applied'` are DELIBERATE
 * members: this is what closes the breaker's "error/applied could be a dead-end" findings — a tap
 * always has somewhere to go from a terminal-ish phase, independent of any timer. Never mid-flow
 * (that is barge-in, a distinct path — see `MID_FLOW` below).
 */
const CAN_START_FROM = new Set<VoicePhase['type']>(['idle', 'disclosure', 'applied', 'error', 'disambiguating']);

/** Phases a CANCEL (Esc / outside-tap / timeout / explicit cancel) can fail-safe out of. */
const CANCELABLE = new Set<VoicePhase['type']>([
  'disclosure',
  'permission-request',
  'listening',
  'transcribing',
  'confirming',
  'disambiguating',
  'error',
]);

/** Pure state transition. Any event that doesn't apply to the current phase is a no-op (returns
 *  the SAME reference) — this guards DISPLAY only. It does NOT, by itself, guard the write-relevant
 *  `gate.submit()` call the hook makes on `onProposal` — that guard is `isStaleSession` below,
 *  applied at the hook boundary BEFORE this reducer ever sees the event (resolution.md CRITICAL). */
export function voiceReducer(state: VoicePhase, event: VoiceEvent): VoicePhase {
  switch (event.type) {
    case 'SHOW_DISCLOSURE':
      return state.type === 'idle' ? { type: 'disclosure' } : state;

    case 'DISCLOSURE_DECLINE':
      return state.type === 'disclosure' ? { type: 'idle' } : state;

    case 'DISCLOSURE_ACCEPT':
    case 'START_LISTENING':
      return CAN_START_FROM.has(state.type) ? { type: 'permission-request' } : state;

    case 'PERMISSION_GRANTED':
      return state.type === 'permission-request' ? { type: 'listening', partialTranscript: '' } : state;

    case 'PERMISSION_DENIED':
      return state.type === 'permission-request' ? { type: 'error', kind: 'mic_denied' } : state;

    case 'PARTIAL_TRANSCRIPT':
      return state.type === 'listening' ? { type: 'listening', partialTranscript: event.text } : state;

    case 'TRANSCRIBING':
      return state.type === 'listening'
        ? { type: 'transcribing', lastPartialTranscript: state.partialTranscript }
        : state;

    case 'READ_ONLY_APPLIED':
      return state.type === 'transcribing'
        ? { type: 'applied', proposal: event.proposal, capability: 'READ_ONLY' }
        : state;

    case 'STATEFUL_PENDING':
      return state.type === 'transcribing' ? { type: 'confirming', proposal: event.proposal } : state;

    case 'NO_MATCH':
      return state.type === 'transcribing'
        ? { type: 'error', kind: 'no_match', candidates: event.candidates }
        : state;

    case 'AMBIGUOUS':
      return state.type === 'transcribing'
        ? { type: 'disambiguating', candidates: event.candidates, transcript: event.transcript }
        : state;

    case 'ENGINE_ERROR':
      // An engine fault (model fetch fail, worker crash/timeout) can interrupt any in-flight
      // phase, not just 'transcribing' — the microphone can die mid-listen too.
      return { type: 'error', kind: event.kind };

    case 'SELECT_CANDIDATE':
      return state.type === 'disambiguating'
        ? { type: 'transcribing', lastPartialTranscript: state.transcript }
        : state;

    case 'CONFIRM':
      return state.type === 'confirming'
        ? { type: 'applied', proposal: state.proposal, capability: 'STATEFUL' }
        : state;

    case 'CANCEL':
      return CANCELABLE.has(state.type) ? { type: 'idle' } : state;

    case 'APPLIED_DONE':
      return state.type === 'applied' ? { type: 'idle' } : state;

    case 'RETRY':
      return state.type === 'error' ? { type: 'permission-request' } : state;

    case 'RESET':
      return state.type === 'idle' ? state : { type: 'idle' };

    default:
      return state;
  }
}

/** What a FAB tap should do, given the current phase and whether the user has already decided
 *  the voice preference. Pure — the hook (`useVoiceControl`) turns the result into imperative
 *  calls (`gate.cancel()`, `engine.abort()`, `engine.start()`) plus a `dispatch`. */
export type TapAction = 'show-disclosure' | 'begin-listening' | 'barge-in' | 'noop';

const MID_FLOW = new Set<VoicePhase['type']>([
  'permission-request',
  'listening',
  'transcribing',
  'confirming',
  'disambiguating',
]);

export function decideTapAction(phaseType: VoicePhase['type'], voicePref: VoicePref | undefined): TapAction {
  if (phaseType === 'disclosure') return 'noop'; // the sheet owns its own two buttons
  if (MID_FLOW.has(phaseType)) return 'barge-in'; // plan §3.3: a new tap always pre-empts
  if (voicePref === undefined) return 'show-disclosure'; // first-ever tap, not yet decided
  return 'begin-listening'; // returning user (idle / applied / error / disambiguating)
}

/**
 * Session-boundary guard (resolution.md CRITICAL fix). Every `VoiceEngineHandlers` callback the
 * hook creates for a session closes over that session's id; before doing ANYTHING — including
 * calling `gate.submit()`, which is the write-relevant effect the reducer's phase guard cannot see
 * — it must check `isStaleSession(currentSessionId, callbackSessionId)` and no-op if true. This is
 * what makes a dead session's late `onProposal`/`onPermissionGranted`/etc. a TRUE no-op (nothing
 * happens, not just "the reducer ignored it after the fact") rather than a race that can overwrite
 * a live `ConfirmationGate#pending` or auto-apply a stale READ_ONLY mutation.
 */
export function isStaleSession(currentSessionId: number, callbackSessionId: number): boolean {
  return callbackSessionId !== currentSessionId;
}

/** Whether the halo's looping bloom + amp-reactive ring should run. The looping keyframe is NOT
 *  covered by the `--motion-*` token zero-out (VOICE-UI-REFERENCE.md "Reduced-motion note") so it
 *  must be gated explicitly, both in CSS (`@media (prefers-reduced-motion: reduce)`) and here. */
export function shouldAnimateHalo(phaseType: VoicePhase['type'], prefersReducedMotion: boolean): boolean {
  return phaseType === 'listening' && !prefersReducedMotion;
}

/** One-pole low-pass smoothing for the amplitude reactive ring (VOICE-UI-REFERENCE.md §3
 *  "amp = amp*0.8 + rms*0.2"), clamped to the CSS custom property's valid 0..1 domain. */
export function smoothAmplitude(previous: number, raw: number): number {
  const clamped = Math.min(1, Math.max(0, raw));
  return previous * 0.8 + clamped * 0.2;
}

/** Extracts the confirm-chip label from an ADD_TO_CART proposal's semantic args
 *  (`{ productId, productName, qty }` per PHASE1-IMPLEMENTATION-PLAN.md §2.1). Defensive against
 *  a malformed/partial args bag — args crosses an engine boundary as `Record<string, unknown>`. */
export function extractAddToCartLabel(proposal: VoiceProposal): { qty: number; item: string } {
  const rawQty = proposal.args['qty'];
  const qty = typeof rawQty === 'number' && Number.isFinite(rawQty) && rawQty > 0 ? Math.round(rawQty) : 1;
  const rawName = proposal.args['productName'];
  const item = typeof rawName === 'string' && rawName.trim().length > 0 ? rawName.trim() : proposal.transcript;
  return { qty, item };
}
