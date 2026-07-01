// @deliveryos/voice — public surface of the voice-control engine (council voice-control / ADR-0015).
// Phase-0 safety core: types + fail-closed capability classification + confirm-then-execute gate.
// The transcription engine (transformers.js worker) and intent matcher land in later modules; they
// emit IntentProposal DATA only and never import the handlers — the ConfirmationGate is the sole sink.

export type { IntentKind, Capability, IntentProposal, GateStatus, GateResult } from './types.js';
export { classify } from './capability-table.js';
export { isDietaryCategory } from './dietary-denylist.js';
export { ConfirmationGate } from './confirmation-gate.js';
export type { VoiceHandlers } from './confirmation-gate.js';
export { matchIntent, MIN_CONFIDENCE } from './matcher.js';
export type { Locale, MenuContext, SortKey, MacroLens } from './matcher.js';
export { MockProvider } from './mock-provider.js';
export { WhisperProvider } from './whisper-provider.js';
export type { Transcriber, PcmAudio } from './transcriber.js';
export { TransformersTranscriber } from './transformers-transcriber.js';
export type { TransformersTranscriberOptions } from './transformers-transcriber.js';
export { normalize } from './normalize.js';
