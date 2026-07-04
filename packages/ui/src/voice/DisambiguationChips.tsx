// Disambiguation ("did you mean?" / ambiguous-tie) chips (docs/design/voice-control/ui-spec.md
// §2/§3). Tapping a chip re-enters the proposal flow — never a guess-execute. Also reused by
// ErrorPill for the `no_match` "did you mean?" recovery affordance.

import { ANCHOR_ABOVE_FAB_STYLE } from './layout.js';
import type { DisambiguationCandidate } from './types.js';

export interface DisambiguationChipsProps {
  candidates: readonly DisambiguationCandidate[];
  onSelect: (candidate: DisambiguationCandidate) => void;
  /** Optional heading (e.g. "Did you mean?"). Omit when nested inside another surface (ErrorPill)
   *  that already renders its own label. */
  label?: string;
  /** Omit the fixed anchor positioning when rendered inside a parent that already positions
   *  itself (ErrorPill embeds this inline). */
  anchored?: boolean;
}

export function DisambiguationChips({ candidates, onSelect, label, anchored = true }: DisambiguationChipsProps) {
  return (
    <div
      data-testid="voice-disambiguation-chips"
      className="flex flex-col gap-2"
      style={anchored ? { ...ANCHOR_ABOVE_FAB_STYLE } : undefined}
    >
      {label && <p className="text-sm font-medium text-[var(--brand-text)]">{label}</p>}
      <div className="flex flex-wrap gap-2">
        {candidates.map((candidate) => (
          <button
            key={candidate.id}
            type="button"
            onClick={() => onSelect(candidate)}
            className="rounded-full px-4 text-sm transition-[background-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
            style={{
              minHeight: 'var(--tap-min)',
              background: 'var(--brand-surface-raised)',
              border: '1px solid var(--brand-border)',
              color: 'var(--brand-text)',
            }}
          >
            {candidate.label}
          </button>
        ))}
      </div>
    </div>
  );
}
