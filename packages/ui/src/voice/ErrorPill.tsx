// The error / no-match recovery surface (docs/design/voice-control/ui-spec.md §2/§3.6 error
// matrix). `aria-live="assertive"` — the user needs to know voice failed. NEUTRAL styling only:
// no red flash, no shake (a shake reads as blame; state is carried by glyph + label, never
// red/green colour-coding — VOICE-UI-REFERENCE.md §4 "never colour-code state with red/green").
// Every kind offers a recovery affordance; nothing dead-ends (touch stays fully usable regardless).

import { useI18n } from '../lib/I18nProvider.js';
import { ANCHOR_ABOVE_FAB_STYLE } from './layout.js';
import { DisambiguationChips } from './DisambiguationChips.js';
import type { DisambiguationCandidate, VoiceErrorKind } from './types.js';

const RETRYABLE: ReadonlySet<VoiceErrorKind> = new Set(['model_offline', 'try_again']);

export interface ErrorPillProps {
  kind: VoiceErrorKind;
  onRetry?: () => void;
  candidates?: readonly DisambiguationCandidate[];
  onSelectCandidate?: (candidate: DisambiguationCandidate) => void;
}

export function ErrorPill({ kind, onRetry, candidates, onSelectCandidate }: ErrorPillProps) {
  const { t } = useI18n();
  const showRetry = RETRYABLE.has(kind) && !!onRetry;
  const showDidYouMean = kind === 'no_match' && candidates && candidates.length > 0 && onSelectCandidate;

  return (
    <div
      role="alert"
      aria-live="assertive"
      data-testid="voice-error-pill"
      data-kind={kind}
      className="rounded-[var(--brand-radius)] px-4 py-3 flex flex-col gap-2 text-sm"
      style={{
        ...ANCHOR_ABOVE_FAB_STYLE,
        background: 'var(--brand-surface-raised)',
        border: '1px solid var(--brand-border)',
        color: 'var(--brand-text)',
        boxShadow: 'var(--elev-3)',
      }}
    >
      <div className="flex items-center justify-between gap-3">
        <span>{t(`voice.err.${kind}`)}</span>
        {showRetry && (
          <button
            type="button"
            data-testid="voice-error-retry"
            onClick={onRetry}
            className="shrink-0 rounded-full px-3 text-sm font-semibold transition-[background-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
            style={{ minHeight: 'var(--tap-min)', background: 'var(--brand-primary)', color: 'var(--color-on-primary)' }}
          >
            {t('voice.retry')}
          </button>
        )}
      </div>
      {showDidYouMean && (
        <DisambiguationChips
          label={t('voice.did_you_mean')}
          candidates={candidates!}
          onSelect={onSelectCandidate!}
          anchored={false}
        />
      )}
    </div>
  );
}
