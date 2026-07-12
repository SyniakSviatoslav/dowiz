import React from 'react';
import { useI18n } from '../../lib/I18nProvider.js';

/**
 * MENU-AVAILABILITY · StateChip — a single brand-token-native availability pill.
 *
 * Covers both venue states (open|closed|busy) and item states (available|sold_out / "86").
 * Brand-token-native: every colour is a `var(--brand-*)` (or `color-mix` of one) so it
 * inherits the tenant theme. Two scopes only differ in copy + icon; the visual language
 * is shared so an eater reads the same semantics on the header and on a card.
 */
export type StateChipState =
  | 'open'
  | 'closed'
  | 'busy'
  | 'available'
  | 'sold_out';

const ICON: Record<StateChipState, string> = {
  open: 'ti-circle-check',
  closed: 'ti-clock-off',
  busy: 'ti-flame',
  available: 'ti-circle-check',
  sold_out: 'ti-circle-minus',
};

// tone => brand-token colour pair (text + translucent fill). 'live' = positive/open,
// 'warn' = busy (raised ETA), 'muted' = closed/sold-out. No raw hex — all derived.
const TONE: Record<StateChipState, { fg: string; bg: string; border: string }> = {
  open: {
    fg: 'var(--brand-primary)',
    bg: 'color-mix(in srgb, var(--brand-primary) 12%, transparent)',
    border: `border`,
  },
  available: {
    fg: 'var(--brand-primary)',
    bg: 'color-mix(in srgb, var(--brand-primary) 12%, transparent)',
    border: `border`,
  },
  busy: {
    fg: 'var(--color-warning, #D97706)',
    bg: 'color-mix(in srgb, var(--color-warning, #D97706) 14%, transparent)',
    border: `border`,
  },
  closed: {
    fg: 'var(--brand-text-muted)',
    bg: 'color-mix(in srgb, var(--brand-text-muted) 12%, transparent)',
    border: `border`,
  },
  sold_out: {
    fg: 'var(--brand-text-muted)',
    bg: 'color-mix(in srgb, var(--brand-text-muted) 12%, transparent)',
    border: `border`,
  },
};

export interface StateChipProps {
  state: StateChipState;
  /** 'venue' => header context (open/closed/busy); 'item' => card context (available/sold_out). */
  scope?: 'venue' | 'item';
  /** Optional extra detail, e.g. a raised ETA string for `busy`. */
  detail?: string | null;
  className?: string;
  'data-testid'?: string;
}

export function StateChip({ state, scope = 'venue', detail, className = '', ...rest }: StateChipProps) {
  const { t } = useI18n();
  const tone = TONE[state];

  const label =
    state === 'open' ? t('state.open', 'Open')
    : state === 'closed' ? t('state.closed', 'Closed')
    : state === 'busy' ? t('state.busy', 'Kitchen busy')
    : state === 'sold_out' ? t('state.sold_out', 'Sold out')
    : t('state.available', 'Available');

  const testId = rest['data-testid'] ?? (scope === 'venue' ? 'venue-state-chip' : 'item-state-chip');

  return (
    <span
      data-testid={testId}
      data-state={state}
      className={`inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-step-2xs font-semibold whitespace-nowrap border ${className}`}
      style={{ color: tone.fg, background: tone.bg, borderColor: tone.border }}
    >
      <i className={`ti ${ICON[state]}`} style={{ fontSize: '0.8em', lineHeight: 1 }} aria-hidden="true" />
      <span>{label}{detail ? ` · ${detail}` : ''}</span>
    </span>
  );
}
