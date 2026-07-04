// Voice control — PR-3 (MicFab + UI state machine). INJECTED CONTRACT ONLY.
//
// This file defines the shape a voice engine + gate must satisfy so the UI in this folder can
// drive them. It deliberately does NOT import `@deliveryos/voice` or any `apps/web` adapter —
// PHASE1-IMPLEMENTATION-PLAN.md §3 requires the component to take the engine/gate as INJECTED
// props so a later lead-integration (PR-2's web adapter + PR-4's mic capture) can wire the real
// thing in. TypeScript structural typing means a real `ConfirmationGate` instance already
// satisfies `VoiceGate` below without either package depending on the other — the public shape
// (`pending` / `submit` / `confirm` / `cancel`) is mirrored 1:1 from
// `packages/voice/src/confirmation-gate.ts` and `packages/voice/src/types.ts`.

/** What the engine emits — mirrors `packages/voice`'s `IntentProposal` (pure, read-only data). */
export interface VoiceProposal {
  readonly kind: string;
  readonly args: Readonly<Record<string, unknown>>;
  readonly transcript: string;
  /** 0..1 matcher confidence. */
  readonly confidence: number;
}

export type VoiceCapability = 'READ_ONLY' | 'STATEFUL' | 'REJECT';
export type VoiceGateStatus = 'applied' | 'pending-confirm' | 'rejected';

export interface VoiceGateResult {
  readonly kind: string;
  readonly capability: VoiceCapability;
  readonly status: VoiceGateStatus;
  readonly reason?: string;
}

/**
 * The confirm-then-execute boundary (the ONLY write sink — ADR-0015 §6). A real
 * `ConfirmationGate` from `@deliveryos/voice` satisfies this structurally; the UI never
 * constructs one — it is injected by the mount site (PR-2 lead-integration).
 */
export interface VoiceGate {
  readonly pending: VoiceProposal | null;
  submit(proposal: VoiceProposal): VoiceGateResult;
  confirm(): VoiceGateResult;
  cancel(): void;
}

/** A candidate offered by disambiguation ("did you mean?") or ambiguous-tie recovery. */
export interface DisambiguationCandidate {
  readonly id: string;
  readonly label: string;
}

/** The recoverable, non-ambiguous error kinds (§3.6 error matrix). One i18n key per kind:
 *  `voice.err.<kind>`. */
export type VoiceErrorKind = 'mic_denied' | 'model_offline' | 'no_match' | 'try_again' | 'unavailable';

/**
 * Callbacks the injected `VoiceEngine` calls to report session lifecycle events into the FSM.
 * Owned by this hook (created once, stable identity) — the engine (PR-4: getUserMedia + VAD +
 * WhisperProvider) never receives a write-capable closure, only these narrow report callbacks.
 */
export interface VoiceEngineHandlers {
  onPermissionGranted(): void;
  onPermissionDenied(): void;
  /** Incremental partial transcript while listening — ephemeral, never logged (ui-spec §3.4/§8). */
  onPartialTranscript(text: string): void;
  /** Silence/cap reached — listening → transcribing. */
  onTranscribing(): void;
  /** A matched intent. The hook runs `gate.submit()` itself — the engine never touches the gate. */
  onProposal(proposal: VoiceProposal): void;
  /** Matcher returned no usable proposal (below MIN_CONFIDENCE or unresolved slot). */
  onNoMatch(candidates?: readonly DisambiguationCandidate[]): void;
  /** Two or more candidates tied — never guess-execute. */
  onAmbiguous(candidates: readonly DisambiguationCandidate[], transcript: string): void;
  /** Recoverable engine fault (mic denied surfaces via onPermissionDenied instead). */
  onError(kind: VoiceErrorKind): void;
}

/**
 * The injected engine port. PR-4 implements this with real `getUserMedia` + `AudioContext`@16kHz
 * + VAD + `WhisperProvider`; a Playwright/unit test double implements it with a scripted
 * `MockProvider`-backed stub. Push-to-talk: one `start()` = one utterance session.
 */
export interface VoiceEngine {
  /** Begin one push-to-talk session (native permission prompt if needed, then listen for a
   *  single utterance). Idempotent handlers object — always the same instance per hook. */
  start(handlers: VoiceEngineHandlers): void;
  /** Abort any in-flight permission/listen/transcribe session. Safe to call when idle (no-op). */
  abort(): void;
  /** Re-enter the proposal flow with a disambiguation pick. Optional — an engine that never
   *  emits `onAmbiguous` may omit it; selecting a candidate then falls back to cancel. */
  resolveCandidate?(candidate: DisambiguationCandidate, handlers: VoiceEngineHandlers): void;
}

/** The persisted first-run consent choice. `undefined` = not yet decided (show the disclosure
 *  sheet on first tap). Persistence itself is the mount site's job (its own prefs store). */
export type VoicePref = 'on' | 'off';

/** A live amplitude source (e.g. an AnalyserNode-backed RMS reader) for the halo's reactive ring.
 *  Optional — without one the halo still blooms (the calm "breath" loop) but ignores `--amp`. */
export interface VoiceAmplitudeSource {
  /** 0..1, sampled once per animation frame while listening. */
  getAmplitude(): number;
}
