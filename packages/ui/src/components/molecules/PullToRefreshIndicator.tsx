import React from 'react';
import { useReducedMotion } from 'framer-motion';
import { useI18n } from '../../lib/I18nProvider.js';

interface PullToRefreshIndicatorProps {
  /** A pull gesture is active (from usePullToRefresh). */
  pulling: boolean;
  /** 0..1 arming progress (1 = release will refresh). */
  progress: number;
  /** The refetch is in flight. */
  refreshing: boolean;
  className?: string;
}

// Visual companion to usePullToRefresh: a small brand-token pill that drops in
// from the top as the user pulls. The arrow rotates with progress (pointing up
// at 1 = "release"); while refreshing it swaps to a spinner. The aria-live
// region is mounted permanently (an aria-live node injected on demand does not
// announce) and only gains text while a refresh is in flight.
// Reduced motion: no translation and no spin — opacity + the live text carry
// the state instead.
export function PullToRefreshIndicator({ pulling, progress, refreshing, className = '' }: PullToRefreshIndicatorProps) {
  const { t } = useI18n();
  const reducedMotion = useReducedMotion();
  const visible = pulling || refreshing;
  const armed = progress >= 1;

  return (
    <div
      className={`pointer-events-none absolute inset-x-0 top-0 z-40 flex justify-center ${className}`}
      data-testid="ptr-indicator"
      data-ptr-state={refreshing ? 'refreshing' : pulling ? 'pulling' : 'idle'}
    >
      {/* Permanent polite live region — announces "Refreshing" to AT on gesture release. */}
      <span aria-live="polite" role="status" className="sr-only">
        {refreshing ? t('common.refreshing', 'Refreshing') : ''}
      </span>

      <div
        aria-hidden="true"
        className="mt-2 flex h-10 w-10 items-center justify-center rounded-full shadow-md"
        style={{
          backgroundColor: 'var(--brand-surface-raised)',
          border: '1px solid var(--brand-border)',
          color: armed || refreshing ? 'var(--brand-primary)' : 'var(--brand-text-muted)',
          opacity: visible ? Math.max(refreshing ? 1 : 0.35, progress) : 0,
          transform: reducedMotion
            ? 'none'
            : `translateY(${visible ? Math.round(progress * 40) : -48}px)`,
          transition: pulling && !reducedMotion ? 'opacity 80ms linear' : 'opacity 160ms ease, transform 160ms ease',
        }}
      >
        {refreshing ? (
          <svg
            className={reducedMotion ? 'h-5 w-5' : 'h-5 w-5 animate-spin'}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.5"
            strokeLinecap="round"
          >
            <path d="M21 12a9 9 0 1 1-3.2-6.9" />
          </svg>
        ) : (
          <i
            className="ti ti-arrow-down text-lg"
            style={{
              display: 'inline-block',
              transform: reducedMotion ? 'none' : `rotate(${Math.round(progress * 180)}deg)`,
            }}
          />
        )}
      </div>
    </div>
  );
}
