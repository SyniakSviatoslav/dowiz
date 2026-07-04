import { ConfirmationGate } from '@deliveryos/voice';
import { createVoiceHandlers } from './handlers.js';
import type { VoiceStorefrontDeps } from './types.js';

/**
 * Construct the ConfirmationGate wired to the real storefront setters (PHASE1-IMPLEMENTATION-PLAN.md
 * §6 PR-2 scope item 3). The gate is the sole write sink (confirmation-gate.ts) — this is the only
 * place apps/web builds one, so every voice-originated mutation in the storefront funnels through
 * this single instance. The future MicFab (PR-3) calls this once per mount with the real deps and
 * feeds the engine's IntentProposals into `gate.submit()`; a human tap on the confirm chip calls
 * `gate.confirm()`; re-tap/Esc/outside-tap calls `gate.cancel()` (ui-spec §3.3 barge-in).
 */
export function createVoiceGate(deps: VoiceStorefrontDeps): ConfirmationGate {
  return new ConfirmationGate(createVoiceHandlers(deps));
}
