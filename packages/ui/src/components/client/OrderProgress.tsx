import React from 'react';
import { useI18n } from '../../lib/I18nProvider.js';

// ORDER-TRACKING: honest stepper for the real 10-state order machine.
//
// The machine (packages/domain/src/order-machine.ts) branches at READY:
//   delivery: PENDING → CONFIRMED → PREPARING → READY → IN_DELIVERY → DELIVERED
//   pickup:   PENDING → CONFIRMED → PREPARING → READY → PICKED_UP
// REJECTED/CANCELLED are terminal and short-circuit the whole flow.
//
// This component is correct with ONLY `status` (the cheap honest version) and
// additionally lights up each step from per-transition timestamps when present
// (passed as `*At` ISO strings); it falls back to status-only when a *At is null.

type Status =
  | 'PENDING' | 'CONFIRMED' | 'PREPARING' | 'READY'
  | 'IN_DELIVERY' | 'DELIVERED' | 'PICKED_UP'
  | 'REJECTED' | 'CANCELLED' | 'SCHEDULED' | string;

export interface OrderProgressProps {
  status: Status;
  /** 'pickup' renders the READY→PICKED_UP branch; anything else = delivery. */
  type?: 'delivery' | 'pickup' | string | null;
  // Per-transition timestamps (ISO strings), nullable until that step happens.
  confirmedAt?: string | null;
  preparingAt?: string | null;
  readyAt?: string | null;
  inDeliveryAt?: string | null;
  deliveredAt?: string | null;
  pickedUpAt?: string | null;
}

interface Step {
  key: string;
  label: string;
  at?: string | null;
}

export function OrderProgress(props: OrderProgressProps) {
  const { status, type } = props;
  const { t } = useI18n();

  const isPickup = type === 'pickup';
  const isRejected = status === 'REJECTED';
  const isCancelled = status === 'CANCELLED';
  const isTerminal = isRejected || isCancelled;

  // Shared head of both branches.
  const head: Step[] = [
    { key: 'PENDING', label: t('client.order_received', 'Received') },
    { key: 'CONFIRMED', label: t('order.confirmed', 'Confirmed'), at: props.confirmedAt },
    { key: 'PREPARING', label: t('client.order_preparing', 'Preparing'), at: props.preparingAt },
    { key: 'READY', label: t('client.order_ready', 'Ready'), at: props.readyAt },
  ];

  const deliveryTail: Step[] = [
    { key: 'IN_DELIVERY', label: t('client.order_on_the_way', 'On the way'), at: props.inDeliveryAt },
    { key: 'DELIVERED', label: t('client.order_delivered', 'Delivered'), at: props.deliveredAt },
  ];

  const pickupTail: Step[] = [
    { key: 'PICKED_UP', label: t('order.picked_up', 'Picked up'), at: props.pickedUpAt },
  ];

  // Honest path: pickup short flow vs delivery long flow. Terminal states
  // short-circuit — we show the path up to where it stopped, then a red
  // terminal node, rather than implying the order will continue.
  const happyPath: Step[] = isPickup ? [...head, ...pickupTail] : [...head, ...deliveryTail];

  const terminalStep: Step | null = isRejected
    ? { key: 'REJECTED', label: t('order.rejected', 'Rejected') }
    : isCancelled
      ? { key: 'CANCELLED', label: t('order.cancelled', 'Cancelled') }
      : null;

  const steps: Step[] = terminalStep ? [...happyPath, terminalStep] : happyPath;

  // currentIndex from status. A step is "filled" if it's at/before the current
  // status, OR if its timestamp is set (timestamps are the source of truth when
  // present — they can fill a step the live status hasn't caught up to).
  const statusIndex = steps.findIndex(s => s.key === status);
  const currentIndex = statusIndex >= 0 ? statusIndex : 0;

  const filledByTimestamp = (i: number) => {
    const at = steps[i]?.at;
    return typeof at === 'string' && at.length > 0;
  };

  // Progress bar width: furthest of (status index, last timestamped step).
  let lastFilled = currentIndex;
  for (let i = 0; i < steps.length; i++) {
    if (filledByTimestamp(i) && i > lastFilled) lastFilled = i;
  }
  if (isTerminal) lastFilled = steps.length - 1; // terminal node always reached

  const denom = steps.length - 1 || 1;

  const fmtTime = (iso?: string | null) => {
    if (!iso) return '';
    const d = new Date(iso);
    if (isNaN(d.getTime())) return '';
    try {
      return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } catch {
      return '';
    }
  };

  return (
    <div
      className="relative py-4"
      role="status"
      aria-live="polite"
      data-testid="order-progress"
      data-order-type={isPickup ? 'pickup' : 'delivery'}
    >
      <div className="absolute top-1/2 left-0 right-0 h-1 bg-[var(--brand-surface-raised)] -translate-y-1/2 z-0" />
      <div
        className="absolute top-1/2 left-0 h-1 -translate-y-1/2 z-0 transition-all duration-500"
        style={{
          width: `${(lastFilled / denom) * 100}%`,
          background: isTerminal ? 'var(--color-danger)' : 'var(--brand-primary)',
        }}
      />
      <div className="relative z-10 flex justify-between">
        {steps.map((step, i) => {
          const isTerminalStep = step.key === 'CANCELLED' || step.key === 'REJECTED';
          const isActive = isTerminalStep ? isTerminal : (i <= currentIndex || filledByTimestamp(i));
          const dotColor = isTerminalStep && isActive ? 'var(--color-danger)' : 'var(--brand-primary)';
          const time = fmtTime(step.at);
          return (
            <div
              key={step.key}
              className="flex flex-col items-center"
              data-testid={`order-step-${step.key.toLowerCase()}`}
              data-active={isActive ? 'true' : 'false'}
            >
              <div
                className="w-4 h-4 rounded-full border-2 transition-colors duration-500"
                style={{
                  background: isActive ? dotColor : 'var(--brand-surface)',
                  borderColor: isActive ? dotColor : 'var(--brand-border)',
                }}
              />
              <span className={`text-[10px] mt-1 ${isActive ? 'text-[var(--brand-text)] font-semibold' : 'text-[var(--brand-text-muted)]'}`}>
                {step.label}
              </span>
              {time && (
                <span
                  data-dynamic
                  className="text-[9px] text-[var(--brand-text-muted)] tabular-nums"
                  data-testid={`order-step-${step.key.toLowerCase()}-time`}
                >
                  {time}
                </span>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
