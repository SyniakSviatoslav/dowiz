import React from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import { useI18n } from '../../lib/I18nProvider.js';
import { ease, duration } from '../../lib/motion.js';
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
  const reduceMotion = useReducedMotion();

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
  // tabular-nums keeps the digits from jittering as the value ticks; a gentle
  // crossfade softens each update (instant under reduced-motion).
  return (
    <span data-dynamic className={`tabular-nums ${className ?? ''}`} style={{ color: 'var(--brand-text)' }}>
      <AnimatePresence mode="popLayout" initial={false}>
        <motion.span
          key={text}
          className="inline-block"
          initial={reduceMotion ? false : { opacity: 0, y: 4 }}
          animate={{ opacity: 1, y: 0 }}
          exit={reduceMotion ? { opacity: 0 } : { opacity: 0, y: -4 }}
          transition={{ duration: duration.base, ease: ease.out }}
        >
          {text}
        </motion.span>
      </AnimatePresence>
    </span>
  );
}
