// The React binding for the voice FSM (packages/ui/src/voice/state-machine.ts). Wraps the pure
// reducer in `useReducer` and wires the injected `VoiceGate`/`VoiceEngine` (packages/ui/src/voice/
// types.ts) — this is the ONLY file in this module that touches React state or calls the gate.
//
// Session-boundary hardening (docs/design/voice-pr3-ui-statemachine/{breaker-findings,
// resolution}.md CRITICAL): every engine callback is wrapped so a dead session's late event
// (`onProposal`, `onPermissionGranted`, …) can never call `gate.submit()`/`gate.confirm()` or
// dispatch a transition for a session that has since been superseded by a barge-in re-tap. A fresh
// `VoiceEngineHandlers` object is created per session (via `makeHandlers`), closing over that
// session's id; `isStaleSession` (state-machine.ts) guards every method.

import { useCallback, useEffect, useReducer, useRef } from 'react';
import {
  decideTapAction,
  initialVoicePhase,
  isStaleSession,
  voiceReducer,
  type VoicePhase,
} from './state-machine.js';
import type {
  DisambiguationCandidate,
  VoiceEngine,
  VoiceEngineHandlers,
  VoiceGate,
  VoiceGateResult,
  VoicePref,
  VoiceProposal,
} from './types.js';

/** How long the "Done" check-pulse holds before auto-returning to idle (cosmetic — a bare re-tap
 *  from 'applied' also escapes immediately; see state-machine.ts CAN_START_FROM). */
const APPLIED_HOLD_MS = 900;

export interface UseVoiceControlOptions {
  /** The confirm-then-execute write sink (ADR-0015 §6). A real `ConfirmationGate` from
   *  `@deliveryos/voice` satisfies this structurally — inject it, don't construct it here. */
  gate: VoiceGate;
  /** The mic-capture + ASR session driver (PR-4). Inject a scripted test double for Playwright/
   *  MockProvider-fed E2E per plan §6 PR-3's proof requirement. */
  engine: VoiceEngine;
  /** The persisted first-run consent choice (`undefined` = not yet decided). Persistence itself is
   *  the mount site's job — this hook only reads it and reports changes via `onVoicePrefChange`. */
  voicePref: VoicePref | undefined;
  onVoicePrefChange?: (pref: VoicePref) => void;
  /** Fired after a READ_ONLY proposal auto-applies, so the mount site can show a toast + Undo (the
   *  "prior value to restore" lives only in the mount site's own setters — this hook has none). */
  onReadOnlyApplied?: (proposal: VoiceProposal, result: VoiceGateResult) => void;
}

export interface UseVoiceControlResult {
  phase: VoicePhase;
  tapFab: () => void;
  acceptDisclosure: () => void;
  declineDisclosure: () => void;
  confirm: () => void;
  cancel: () => void;
  retry: () => void;
  selectCandidate: (candidate: DisambiguationCandidate) => void;
}

export function useVoiceControl({
  gate,
  engine,
  voicePref,
  onVoicePrefChange,
  onReadOnlyApplied,
}: UseVoiceControlOptions): UseVoiceControlResult {
  const [phase, dispatch] = useReducer(voiceReducer, initialVoicePhase);

  // Latest-value refs so the (rarely-recreated) callbacks below never close over stale props
  // without needing to be regenerated — a fresh `gate`/`engine`/callback prop on every render must
  // not force a brand-new session id.
  const gateRef = useRef(gate);
  gateRef.current = gate;
  const engineRef = useRef(engine);
  engineRef.current = engine;
  const onReadOnlyAppliedRef = useRef(onReadOnlyApplied);
  onReadOnlyAppliedRef.current = onReadOnlyApplied;

  // The session-boundary guard (see file header). Bumped by every fresh `engine.start()` call —
  // barge-in, a plain re-tap from idle/applied/error/disambiguating, disclosure-accept, or retry.
  const sessionIdRef = useRef(0);

  const makeHandlers = useCallback((sessionId: number): VoiceEngineHandlers => {
    const stale = () => isStaleSession(sessionIdRef.current, sessionId);
    return {
      onPermissionGranted: () => {
        if (stale()) return;
        dispatch({ type: 'PERMISSION_GRANTED' });
      },
      onPermissionDenied: () => {
        if (stale()) return;
        dispatch({ type: 'PERMISSION_DENIED' });
      },
      onPartialTranscript: (text) => {
        if (stale()) return;
        dispatch({ type: 'PARTIAL_TRANSCRIPT', text });
      },
      onTranscribing: () => {
        if (stale()) return;
        dispatch({ type: 'TRANSCRIBING' });
      },
      onProposal: (proposal) => {
        // The CRITICAL fix: this guard sits BEFORE gate.submit() — the write-relevant effect the
        // reducer's phase guard cannot see — so a dead session's proposal can never overwrite a
        // live ConfirmationGate#pending or auto-apply a stale READ_ONLY mutation.
        if (stale()) return;
        const result = gateRef.current.submit(proposal);
        if (result.status === 'applied') {
          dispatch({ type: 'READ_ONLY_APPLIED', proposal });
          onReadOnlyAppliedRef.current?.(proposal, result);
        } else if (result.status === 'pending-confirm') {
          dispatch({ type: 'STATEFUL_PENDING', proposal });
        } else {
          // Rejected (unknown kind, excluded intent, or dietary-named category touch-only) — one
          // neutral no-match copy for all of it; never a message that implies a safety decision
          // was made (ui-spec §3.6).
          dispatch({ type: 'NO_MATCH' });
        }
      },
      onNoMatch: (candidates) => {
        if (stale()) return;
        dispatch({ type: 'NO_MATCH', candidates });
      },
      onAmbiguous: (candidates, transcript) => {
        if (stale()) return;
        dispatch({ type: 'AMBIGUOUS', candidates, transcript });
      },
      onError: (kind) => {
        if (stale()) return;
        dispatch({ type: 'ENGINE_ERROR', kind });
      },
    };
  }, []);

  /** Bump the session id, transition to permission-request, and start the engine on a FRESH
   *  handlers closure — this single call site is what invalidates every previously-issued
   *  session's callbacks (they compare against the now-advanced `sessionIdRef.current`). */
  const startSession = useCallback(() => {
    sessionIdRef.current += 1;
    const mySession = sessionIdRef.current;
    dispatch({ type: 'START_LISTENING' });
    engineRef.current.start(makeHandlers(mySession));
  }, [makeHandlers]);

  const tapFab = useCallback(() => {
    const action = decideTapAction(phase.type, voicePref);
    switch (action) {
      case 'show-disclosure':
        dispatch({ type: 'SHOW_DISCLOSURE' });
        break;
      case 'begin-listening':
        startSession();
        break;
      case 'barge-in':
        // A new tap always pre-empts an in-flight session / pending proposal, fail-safe to no
        // write (plan §3.3). gate.cancel()/engine.abort() are belt-and-suspenders — the session-id
        // guard above is what actually closes the CRITICAL race, independent of whether abort()
        // manages to cancel an already-queued callback.
        gateRef.current.cancel();
        engineRef.current.abort();
        dispatch({ type: 'RESET' });
        startSession();
        break;
      case 'noop':
        break;
    }
  }, [phase.type, voicePref, startSession]);

  const acceptDisclosure = useCallback(() => {
    onVoicePrefChange?.('on');
    dispatch({ type: 'DISCLOSURE_ACCEPT' });
    sessionIdRef.current += 1;
    engineRef.current.start(makeHandlers(sessionIdRef.current));
  }, [onVoicePrefChange, makeHandlers]);

  const declineDisclosure = useCallback(() => {
    // No engine/gate call of any kind — "Not now" must never import or touch the engine
    // (G11 / ui-spec §5). This function's entire body is the proof: two dispatches, no side effect.
    onVoicePrefChange?.('off');
    dispatch({ type: 'DISCLOSURE_DECLINE' });
  }, [onVoicePrefChange]);

  const confirm = useCallback(() => {
    if (phase.type !== 'confirming') return;
    gateRef.current.confirm();
    dispatch({ type: 'CONFIRM' });
  }, [phase.type]);

  const cancel = useCallback(() => {
    if (phase.type === 'confirming') gateRef.current.cancel();
    if (phase.type === 'listening' || phase.type === 'transcribing' || phase.type === 'permission-request') {
      engineRef.current.abort();
    }
    dispatch({ type: 'CANCEL' });
  }, [phase.type]);

  const retry = useCallback(() => {
    if (phase.type !== 'error') return;
    dispatch({ type: 'RETRY' });
    sessionIdRef.current += 1;
    engineRef.current.start(makeHandlers(sessionIdRef.current));
  }, [phase.type, makeHandlers]);

  const selectCandidate = useCallback(
    (candidate: DisambiguationCandidate) => {
      if (phase.type !== 'disambiguating') return;
      dispatch({ type: 'SELECT_CANDIDATE' });
      // Continuing the SAME session (not a fresh start) — reuse the current session id so a
      // resolveCandidate reply is not treated as stale by its own guard.
      const current = engineRef.current;
      if (current.resolveCandidate) {
        current.resolveCandidate(candidate, makeHandlers(sessionIdRef.current));
      } else {
        cancel();
      }
    },
    [phase.type, makeHandlers, cancel],
  );

  // Auto-return to idle after a brief "Done" hold (cosmetic — a bare re-tap already escapes
  // 'applied' immediately via decideTapAction, so a dropped timer is not a liveness bug).
  useEffect(() => {
    if (phase.type !== 'applied') return;
    const timer = setTimeout(() => dispatch({ type: 'APPLIED_DONE' }), APPLIED_HOLD_MS);
    return () => clearTimeout(timer);
  }, [phase.type]);

  // Unmount (route change, FAB predicate flips off mid-session) must not leave a hot mic — bump
  // the session id too so any in-flight callback becomes stale even if abort() itself no-ops.
  useEffect(
    () => () => {
      sessionIdRef.current += 1;
      engineRef.current.abort();
    },
    [],
  );

  return { phase, tapFab, acceptDisclosure, declineDisclosure, confirm, cancel, retry, selectCandidate };
}
