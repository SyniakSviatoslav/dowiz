import React, { useEffect, useRef, useState } from 'react';
import { m } from 'framer-motion';
import { useI18n } from '../../lib/I18nProvider.js';
import type { CourierTask } from './types.js';

interface TaskCardProps {
  task: CourierTask;
  onAccept: (id: string) => void;
  onReject?: (id: string) => void;
  isLoading?: boolean;
  /** Offer accept/decline window in seconds (Bolt §F7). When it elapses the offer
   *  auto-declines (releases back to dispatch). Only runs for a decidable offer
   *  (onReject present). 0/undefined → no timer. */
  offerSeconds?: number;
}

export function TaskCard({ task, onAccept, onReject, isLoading, offerSeconds = 60 }: TaskCardProps) {
  const { t } = useI18n();
  // Countdown only for a live offer (it has a reject path) and a positive window.
  const timed = !!onReject && offerSeconds > 0;
  const [remaining, setRemaining] = useState(offerSeconds);
  const firedRef = useRef(false);
  useEffect(() => {
    if (!timed) return;
    setRemaining(offerSeconds);
    firedRef.current = false;
    const iv = setInterval(() => {
      setRemaining(prev => {
        if (prev <= 1) {
          clearInterval(iv);
          if (!firedRef.current && onReject) { firedRef.current = true; onReject(task.id); } // auto-decline → back to dispatch
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
    return () => clearInterval(iv);
  }, [timed, offerSeconds, task.id]);
  const pct = timed ? Math.max(0, Math.min(100, (remaining / offerSeconds) * 100)) : 0;
  const urgent = timed && remaining <= 10;

  return (
    <div data-testid={`task-card-${task.orderId || task.order_id || task.id}`} data-status={task.status} className={`bg-[var(--brand-surface)] rounded-[var(--brand-radius)] p-5 space-y-4 shadow-[var(--elevation-1)] transition-[transform,box-shadow] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none ${isLoading ? 'opacity-50 pointer-events-none' : '[@media(hover:hover)]:hover:shadow-[var(--elevation-3)] [@media(hover:hover)]:hover:-translate-y-0.5'}`}>

      {/* Offer countdown (Bolt §F7) — a thin shrinking bar + a seconds pill. */}
      {timed && (
        <div>
          <div className="h-1 w-full rounded-full overflow-hidden" style={{ background: 'var(--brand-border)' }}>
            <div
              data-testid="courier-offer-timer"
              data-remaining={remaining}
              className="h-full transition-[width] duration-1000 ease-linear"
              style={{ width: `${pct}%`, background: urgent ? 'var(--color-danger)' : 'var(--brand-primary)' }}
            />
          </div>
        </div>
      )}

      {/* Header */}
      <div className="flex justify-between items-start gap-3">
        <h3 className="font-bold text-lg text-[var(--brand-text)] min-w-0 truncate">
          {task.restaurant?.name || t('courier.new_delivery', 'New Delivery')}
        </h3>
        {timed
          ? <span className="shrink-0 tabular-nums font-bold px-2 py-1 rounded-[var(--brand-radius-sm)] text-sm" style={{ background: 'var(--status-pending-bg)', color: urgent ? 'var(--color-danger)' : 'var(--status-pending)' }}>{remaining}s</span>
          : task.eta && <span className="shrink-0 tabular-nums bg-[var(--status-pending-bg)] text-[var(--status-pending)] font-bold px-2 py-1 rounded-[var(--brand-radius-sm)] text-sm">{task.eta}</span>}
      </div>

      {/* Locations */}
      {(task.restaurant || task.customer) && (
      <div className="relative pl-6 space-y-4 before:content-[''] before:absolute before:left-2.5 before:top-2 before:bottom-2 before:w-0.5 before:bg-[var(--brand-border)]">
        
        {task.restaurant && (
        <div className="relative min-w-0">
          <div className="absolute -left-[26px] top-1 w-3 h-3 rounded-full bg-[var(--brand-primary)] border-2 border-[var(--brand-surface)]" />
          <div className="text-xs text-[var(--brand-text-muted)] uppercase font-bold tracking-wider">{t('courier.pickup', 'Pickup')}</div>
          <div className="font-medium text-[var(--brand-text)] truncate">{task.restaurant.name}</div>
          <div className="text-sm text-[var(--brand-text)] break-words">{task.restaurant.address}</div>
        </div>
        )}

        {task.customer && (
        <div className="relative min-w-0">
          <div className="absolute -left-[26px] top-1 w-3 h-3 rounded-full bg-[var(--color-success)] border-2 border-[var(--brand-surface)]" />
          <div className="text-xs text-[var(--brand-text-muted)] uppercase font-bold tracking-wider">{t('courier.dropoff', 'Drop-off')}</div>
          <div className="font-medium text-[var(--brand-text)] break-words">{task.customer.address}</div>
        </div>
        )}

      </div>
      )}

      <div className="border-t border-[var(--brand-border)] pt-4 flex gap-3">
        {onReject && (
          <m.button
            onClick={() => onReject(task.id)}
            data-testid="courier-offer-decline"
            className="flex-1 min-h-11 bg-[var(--brand-surface-raised)] [@media(hover:hover)]:hover:bg-[var(--brand-border)] text-[var(--brand-text)] py-3 rounded-[var(--brand-radius-btn)] font-semibold transition-[background-color,transform] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]"
            whileTap={{ scale: 0.97 }}
          >
            {t('common.reject', 'Reject')}
          </m.button>
        )}
        <m.button
          onClick={() => onAccept(task.id)}
          data-testid="task-accept"
          disabled={isLoading}
          className="flex-1 min-h-11 bg-[var(--brand-primary)] [@media(hover:hover)]:hover:bg-[var(--brand-primary-hover)] text-[var(--brand-bg)] py-3 rounded-[var(--brand-radius-btn)] font-semibold transition-[background-color,box-shadow,transform] duration-150 ease-[var(--ease-soft)] motion-reduce:transition-none active:scale-[0.98] shadow-[var(--elevation-1)] [@media(hover:hover)]:hover:shadow-[var(--elevation-2)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)] disabled:opacity-50 disabled:shadow-none disabled:cursor-not-allowed"
          whileTap={{ scale: 0.97 }}
        >
          {isLoading ? (
            <span className="inline-flex items-center gap-2">
              <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24" fill="none">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
              {t('common.loading', 'Loading...')}
            </span>
          ) : t('courier.accept_task', 'Accept Task')}
        </m.button>
      </div>

    </div>
  );
}

export type { TaskCardProps };
