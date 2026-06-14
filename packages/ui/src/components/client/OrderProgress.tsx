import React from 'react';
import { useI18n } from '../../lib/I18nProvider.js';

export function OrderProgress({ status }: { status: string }) {
  const { t } = useI18n();

  const happyPath = [
    { key: 'PENDING', label: t('client.order_received', 'Received') },
    { key: 'PREPARING', label: t('client.order_preparing', 'Preparing') },
    { key: 'READY', label: t('client.order_ready', 'Ready') },
    { key: 'IN_DELIVERY', label: t('client.order_on_the_way', 'On the way') },
    { key: 'DELIVERED', label: t('client.order_delivered', 'Delivered') },
  ];

  const terminalStates = [
    { key: 'CANCELLED', label: t('client.order_cancelled', 'Cancelled') },
    { key: 'REJECTED', label: t('client.order_rejected', 'Rejected') },
  ];

  const isTerminal = status === 'CANCELLED' || status === 'REJECTED';

  const steps = isTerminal
    ? [...happyPath, ...terminalStates]
    : happyPath;

  const currentIndex = steps.findIndex(s => s.key === status) >= 0 ? steps.findIndex(s => s.key === status) : 0;

  return (
    <div className="relative py-4" role="status" aria-live="polite">
      <div className="absolute top-1/2 left-0 right-0 h-1 bg-[var(--brand-surface-raised)] -translate-y-1/2 z-0" />
      <div 
        className="absolute top-1/2 left-0 h-1 -translate-y-1/2 z-0 transition-all duration-500" 
        style={{
          width: `${(currentIndex / (steps.length - 1)) * 100}%`,
          background: isTerminal ? 'var(--color-danger)' : 'var(--brand-primary)',
        }}
      />
      <div className="relative z-10 flex justify-between">
        {steps.map((step, i) => {
          const isActive = i <= currentIndex;
          const isTerminalStep = step.key === 'CANCELLED' || step.key === 'REJECTED';
          const dotColor = isTerminalStep && isActive ? 'var(--color-danger)' : 'var(--brand-primary)';
          return (
            <div key={step.key} className="flex flex-col items-center">
              <div className={`w-4 h-4 rounded-full border-2 transition-colors duration-500`}
                style={{
                  background: isActive ? dotColor : 'var(--brand-surface)',
                  borderColor: isActive ? dotColor : 'var(--brand-border)',
                }}
              />
              <span className={`text-[10px] mt-1 ${isActive ? 'text-[var(--brand-text)] font-semibold' : 'text-[var(--brand-text-muted)]'}`}>{step.label}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
