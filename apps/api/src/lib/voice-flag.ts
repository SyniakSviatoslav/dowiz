// Single server-side source of truth for whether voice-control is live. Used by BOTH the
// GET /api/public/voice-config kill-switch endpoint AND the CSP connect-src R2 widening in
// spa-shell.ts (breaker finding R2-E) — so the two can never desync from separate flags.
//
//   enabled = VOICE_CONTROL_ENABLED === 'true'  AND  VOICE_KILL !== 'true'
//
// Default OFF (dark): an unset VOICE_CONTROL_ENABLED reads undefined → false, so the feature is
// fail-closed without relying on a schema default. VOICE_KILL is the runtime hot-kill — flip it to
// 'true' as a secret to disable voice across all clients WITHOUT a rebuild (the config endpoint is
// SW-exempt and fetched no-store, so the kill propagates on the next poll). (ADR-0015 §9.)
export function isVoiceEnabled(): boolean {
  return process.env.VOICE_CONTROL_ENABLED === 'true' && process.env.VOICE_KILL !== 'true';
}
