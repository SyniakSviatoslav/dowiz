// The live partial-transcript pill (docs/design/voice-control/ui-spec.md §2/§3.4) — shown above
// the FAB during 'listening' (incremental interim text if the mic layer surfaces it, else just the
// "Listening…" label) and 'transcribing' (frozen final words + "…"). `aria-live="polite"`
// (incremental, non-interruptive — never steals focus). Text is ephemeral component state passed
// in by the caller; this module never logs or persists it (ui-spec §8).

import { useI18n } from '../lib/I18nProvider.js';
import { ANCHOR_ABOVE_FAB_STYLE } from './layout.js';

export interface PartialTranscriptPillProps {
  phase: 'listening' | 'transcribing';
  /** Partial transcript (listening) or the frozen final words (transcribing). May be empty. */
  text: string;
}

export function PartialTranscriptPill({ phase, text }: PartialTranscriptPillProps) {
  const { t } = useI18n();
  const label = t(phase === 'listening' ? 'voice.listening' : 'voice.transcribing');
  return (
    <div
      role="status"
      aria-live="polite"
      data-testid="voice-partial-transcript-pill"
      className="rounded-full px-4 py-2 flex items-center gap-2 text-sm"
      style={{
        ...ANCHOR_ABOVE_FAB_STYLE,
        background: 'var(--brand-surface-raised)',
        border: '1px solid var(--brand-border)',
        color: 'var(--brand-text)',
        boxShadow: 'var(--elev-2)',
      }}
    >
      <span className="font-medium">{label}</span>
      {text && <span className="text-[var(--brand-text-muted)] truncate">{text}</span>}
      {phase === 'transcribing' && <span aria-hidden="true">…</span>}
    </div>
  );
}
