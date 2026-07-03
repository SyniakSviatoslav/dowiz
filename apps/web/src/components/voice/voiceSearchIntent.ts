// Pure, React-free glue between the built @deliveryos/voice engine and the storefront search box.
// This module holds NO write capability and NO DOM/audio access — it is only (a) the render/enable
// predicate for the push-to-talk button and (b) the SET_SEARCH → query mapping. Keeping it pure is
// what makes both testable with `tsx --test` (no jsdom, no mic, no model). The React component
// (VoiceSearchButton.tsx) owns the mic capture + the dynamic engine import; the engine still yields
// pure IntentProposal data (source/sink closure, ADR-0015 §6) — this file just reads that data.
//
// Increment scope (Phase A, plan §8): the ONLY honored intent is SET_SEARCH (READ_ONLY). Every other
// IntentKind maps to null here → a no-op ("didn't catch that"). No cart, no money, no confirm chip.

import type { IntentProposal, Locale, MenuContext } from '@deliveryos/voice';

export type { Locale, MenuContext };

/**
 * Map a matcher IntentProposal to a search query string, or null.
 * Search-only: anything that is not a SET_SEARCH with a non-empty string `query` is ignored. A
 * mis-heard command therefore does nothing (fail-quiet) — it can never trigger a non-search action.
 */
export function intentToSearchQuery(proposal: IntentProposal | null | undefined): string | null {
  if (!proposal) return null;
  if (proposal.kind !== 'SET_SEARCH') return null;
  const q = proposal.args.query;
  if (typeof q !== 'string') return null;
  const trimmed = q.trim();
  return trimmed.length > 0 ? trimmed : null;
}

/** Observable browser capabilities that decide whether the push-to-talk button may exist at all. */
export interface VoiceCapabilities {
  /** VITE_VOICE_CONTROL === 'true' — the build-time kill switch (default off = true-dark). */
  readonly flagEnabled: boolean;
  /** window.isSecureContext — getUserMedia requires HTTPS/localhost. */
  readonly secureContext: boolean;
  /** navigator.mediaDevices?.getUserMedia is present (mic capture path exists). */
  readonly hasMediaDevices: boolean;
}

export type VoiceUnavailableReason = 'flag-off' | 'insecure-context' | 'no-media-devices';

export type VoiceAvailability =
  | { readonly render: false; readonly reason: VoiceUnavailableReason }
  | { readonly render: true };

/**
 * Render predicate for the MicFab (plan §6): voice is strictly additive, so when it cannot work the
 * control is ABSENT (never a greyed disabled button that invites support load). Fail-closed: any
 * missing capability → do not render. This is the true-dark guarantee at the UI layer — with the flag
 * off the button never mounts and the engine module is never dynamically imported.
 */
export function voiceSearchAvailability(caps: VoiceCapabilities): VoiceAvailability {
  if (!caps.flagEnabled) return { render: false, reason: 'flag-off' };
  if (!caps.secureContext) return { render: false, reason: 'insecure-context' };
  if (!caps.hasMediaDevices) return { render: false, reason: 'no-media-devices' };
  return { render: true };
}
