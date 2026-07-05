// Shared layout + equal-affordance constants for the voice UI surfaces. Single source so every
// "above the FAB" surface (confirm chip, partial-transcript pill, error pill, disambiguation
// chips) anchors identically, and so the confirm/cancel + disclosure use/decline button PAIRS can
// never visually diverge (STOP-2 / C-2, hardened per docs/design/voice-pr3-ui-statemachine/
// resolution.md "[MED] Equal-affordance CI-assertion blind spot").

import type { CSSProperties } from 'react';

/** The MicFab's own fixed position (ui-spec §1 — bottom-right, safe-area aware, `z-sticky`). */
export const FAB_POSITION_STYLE: CSSProperties = {
  position: 'fixed',
  right: 'calc(env(safe-area-inset-right, 0px) + 1rem)',
  bottom: 'calc(env(safe-area-inset-bottom, 0px) + 1rem)',
  width: 'var(--tap-critical)',
  height: 'var(--tap-critical)',
};

/**
 * Anchor for every non-modal surface that floats above the FAB (confirm chip, partial-transcript
 * pill, error pill, disambiguation chips) — one shared offset so they never fight the FAB or each
 * other (at most one is visible per FSM phase). `z-toast` (500) per ui-spec §1's explicit z-scale:
 * "the confirm chip sits above the FAB that spawned it".
 */
export const ANCHOR_ABOVE_FAB_STYLE: CSSProperties = {
  position: 'fixed',
  right: 'calc(env(safe-area-inset-right, 0px) + 1rem)',
  bottom: 'calc(env(safe-area-inset-bottom, 0px) + 1rem + var(--tap-critical) + 0.75rem)',
  zIndex: 'var(--z-toast)',
  maxWidth: 'calc(100vw - 2rem)',
};

/**
 * EQUAL-AFFORDANCE button styling (STOP-2 / C-2 — ui-spec §3/§5). Confirm/Cancel on the safety
 * chip AND Use-voice/Not-now on the disclosure sheet MUST render from this ONE constant with no
 * per-button className/style override point — the breaker flagged that a narrower CI assertion
 * (background/border-width/min-height/font-weight only) leaves room for an unmeasured property
 * (padding, box-shadow, gap) or a parent-merged className to reintroduce asymmetry. Fixing every
 * box-model property here, and giving callers NO override prop, closes that by construction: there
 * is nothing for a caller to merge in, and a byte-identical object reference IS the CI assertion.
 * A glyph (`ti-check`/`ti-x`) may still differ — that is the one asymmetry ui-spec explicitly
 * permits and is accepted risk, not something a style constant can or should erase.
 */
export const EQUAL_AFFORDANCE_BUTTON_CLASSNAME =
  'flex-1 flex items-center justify-center gap-2 rounded-[var(--brand-radius)] text-sm ' +
  'transition-[background-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] ' +
  'active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 ' +
  'focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2';

export const EQUAL_AFFORDANCE_BUTTON_STYLE: CSSProperties = {
  minHeight: 'var(--tap-min)',
  padding: '0.625rem 1.25rem',
  gap: '0.5rem',
  background: 'var(--brand-surface-raised)',
  border: '1px solid var(--brand-border)',
  color: 'var(--brand-text)',
  fontWeight: 'var(--weight-semibold)',
  boxShadow: 'none',
};
