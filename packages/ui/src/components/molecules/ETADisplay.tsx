import React from 'react';
import { useI18n } from '../../lib/I18nProvider.js';
import type { DeliveryEta } from '../../hooks/use-delivery-eta.js';

export interface ETADisplayProps {
  eta: DeliveryEta;
  /** Shown when no live ETA is computable yet (e.g. before pickup or WS down). */
  fallback?: string;
  className?: string;
}

/**
 * Renders the smoothed delivery ETA. Flips to "Arriving" at the proximity threshold
 * (not at ETA 0). Falls back gracefully — the customer never sees that routing
 * degraded; an advisory ETA always shows something sensible.
 */
export function ETADisplay({ eta, fallback, className }: ETADisplayProps) {
  const { t } = useI18n();

  let text: string;
  if (eta.arriving) {
    text = t('order.arriving', 'Arriving');
  } else if (eta.etaSeconds != null) {
    const minutes = Math.max(1, Math.round(eta.etaSeconds / 60));
    text = `${minutes} ${t('order.eta_min', 'min')}`;
  } else {
    text = fallback ?? '';
  }

  // data-dynamic: live ETA countdown varies run-to-run — masked from the visual net.
  return <span data-dynamic className={className} style={{ color: 'var(--brand-text)' }}>{text}</span>;
}
