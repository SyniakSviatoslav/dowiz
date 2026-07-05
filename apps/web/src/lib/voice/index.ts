// Voice PR-2 public surface (docs/design/voice-control/PHASE1-IMPLEMENTATION-PLAN.md §6). The
// future MicFab (PR-3) imports from here only — it should never need to reach into handlers.ts /
// menuContext.ts / gate.ts directly.

export type {
  VoiceStorefrontDeps,
  VoiceMenuProduct,
  VoiceMenuCategory,
  SortByValue,
  MacroLensValue,
  VoiceNoMatch,
} from './types.js';
export { buildMenuContext } from './menuContext.js';
export { createVoiceHandlers } from './handlers.js';
export { createVoiceGate } from './gate.js';
