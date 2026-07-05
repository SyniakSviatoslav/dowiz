import type { AcquisitionState } from './types.js';

// P6-1 — acquisition state-machine. Invariants (proven by tests):
//  1. Every NON-terminal state has ≥1 outgoing edge (no silent stall — breaker LOW).
//  2. Illegal transitions throw a typed error (orders state-machine pattern).
//  3. Every exit/terminal state requires a non-empty failure_reason (REQUIRES_REASON),
//     except the success terminal CLAIMED.

// Happy path: SOURCED → PLACE_INGESTED → MENU_EXTRACTED → ENRICHED → PROVISIONED →
// VERIFIED → CLAIM_OFFERED → CLAIMED. Every working state can also divert to an exit.
const LEGAL: Record<AcquisitionState, readonly AcquisitionState[]> = {
  SOURCED: ['PLACE_INGESTED', 'MANUAL_REVIEW', 'DISQUALIFIED', 'ABANDONED'],
  PLACE_INGESTED: ['MENU_EXTRACTED', 'MENU_NOT_FOUND', 'MANUAL_REVIEW', 'DISQUALIFIED', 'ABANDONED'],
  MENU_EXTRACTED: ['ENRICHED', 'LOW_QUALITY', 'MANUAL_REVIEW', 'ABANDONED'],
  ENRICHED: ['PROVISIONED', 'MANUAL_REVIEW', 'ABANDONED'],
  PROVISIONED: ['VERIFIED', 'LOW_QUALITY', 'MANUAL_REVIEW', 'ABANDONED'],
  VERIFIED: ['CLAIM_OFFERED', 'ABANDONED'],
  CLAIM_OFFERED: ['CLAIMED', 'ABANDONED'],
  CLAIMED: [], // success terminal
  // exit states: each still has a resolution edge (no stall) except the true terminals
  MENU_NOT_FOUND: ['MANUAL_REVIEW', 'ABANDONED'],
  LOW_QUALITY: ['MANUAL_REVIEW', 'ABANDONED'],
  MANUAL_REVIEW: ['DISQUALIFIED', 'ABANDONED'], // human resolves to a terminal
  DISQUALIFIED: [], // terminal
  ABANDONED: [], // terminal
};

export const TERMINAL_STATES: ReadonlySet<AcquisitionState> = new Set(
  (Object.keys(LEGAL) as AcquisitionState[]).filter((s) => LEGAL[s].length === 0),
);

// Exit states that demand a non-empty failure_reason when entered (CLAIMED is success).
export const REQUIRES_REASON: ReadonlySet<AcquisitionState> = new Set<AcquisitionState>([
  'MENU_NOT_FOUND',
  'LOW_QUALITY',
  'MANUAL_REVIEW',
  'DISQUALIFIED',
  'ABANDONED',
]);

export class AcquisitionTransitionError extends Error {
  constructor(
    readonly from: AcquisitionState,
    readonly to: AcquisitionState,
  ) {
    super(`illegal acquisition transition: ${from} → ${to}`);
    this.name = 'AcquisitionTransitionError';
  }
}

export function canTransition(from: AcquisitionState, to: AcquisitionState): boolean {
  return LEGAL[from]?.includes(to) ?? false;
}

/** Throws AcquisitionTransitionError if (from → to) is not a legal edge. */
export function assertTransition(from: AcquisitionState, to: AcquisitionState): void {
  if (!canTransition(from, to)) throw new AcquisitionTransitionError(from, to);
}

/** Invariant check used by tests: every non-terminal state has at least one exit. */
export function everyNonTerminalHasExit(): boolean {
  return (Object.keys(LEGAL) as AcquisitionState[]).every(
    (s) => TERMINAL_STATES.has(s) || LEGAL[s].length > 0,
  );
}

export const _LEGAL_FOR_TEST = LEGAL;
