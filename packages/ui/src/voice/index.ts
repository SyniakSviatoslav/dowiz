// Voice control PR-3 public surface (MicFab + UI state machine). See docs/design/voice-control/
// ui-spec.md + PHASE1-IMPLEMENTATION-PLAN.md §3. Nothing here imports @deliveryos/voice or
// apps/web — the engine/gate are injected via the structural contract in types.ts.

export type {
  VoiceProposal,
  VoiceCapability,
  VoiceGateStatus,
  VoiceGateResult,
  VoiceGate,
  DisambiguationCandidate,
  VoiceErrorKind,
  VoiceEngineHandlers,
  VoiceEngine,
  VoicePref,
  VoiceAmplitudeSource,
} from './types.js';

export type { VoicePhase, VoiceEvent, TapAction } from './state-machine.js';
export {
  initialVoicePhase,
  voiceReducer,
  decideTapAction,
  isStaleSession,
  shouldAnimateHalo,
  smoothAmplitude,
  extractAddToCartLabel,
} from './state-machine.js';

export { useVoiceControl } from './useVoiceControl.js';
export type { UseVoiceControlOptions, UseVoiceControlResult } from './useVoiceControl.js';

export { MicFab } from './MicFab.js';
export type { MicFabProps } from './MicFab.js';

export { ConfirmChip } from './ConfirmChip.js';
export type { ConfirmChipProps } from './ConfirmChip.js';

export { DisclosureSheet } from './DisclosureSheet.js';
export type { DisclosureSheetProps } from './DisclosureSheet.js';

export { ReadBackPanel } from './ReadBackPanel.js';
export type { ReadBackPanelProps, ReadBackLine } from './ReadBackPanel.js';

export { PartialTranscriptPill } from './PartialTranscriptPill.js';
export type { PartialTranscriptPillProps } from './PartialTranscriptPill.js';

export { ErrorPill } from './ErrorPill.js';
export type { ErrorPillProps } from './ErrorPill.js';

export { DisambiguationChips } from './DisambiguationChips.js';
export type { DisambiguationChipsProps } from './DisambiguationChips.js';

export { VoiceSettingToggle } from './VoiceSettingToggle.js';
export type { VoiceSettingToggleProps } from './VoiceSettingToggle.js';

export {
  FAB_POSITION_STYLE,
  ANCHOR_ABOVE_FAB_STYLE,
  EQUAL_AFFORDANCE_BUTTON_CLASSNAME,
  EQUAL_AFFORDANCE_BUTTON_STYLE,
} from './layout.js';
