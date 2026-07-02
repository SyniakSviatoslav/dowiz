import React, { useState } from 'react';
import { useI18n, PriceDisplay } from '@deliveryos/ui';
import type { CartItem } from '@deliveryos/ui';
import { DishStats } from '../../../components/client/DishStats.js';
import type { OrderMenuEntry } from './useOrderMenuMap.js';

interface OrderSummaryAccordionProps {
  items: CartItem[];
  total: number;
  orderMenuMap: Record<string, OrderMenuEntry>;
  hasNutrition: boolean;
  nutritionTotal: { kcal: number; protein: number; fat: number; carbs: number };
}

// Order summary — subtle collapsed accordion at the very top: items (photos), combined price,
// combined nutrition. Revealed on tap; quiet by default so it doesn't dominate the form.
export function OrderSummaryAccordion({ items, total, orderMenuMap, hasNutrition, nutritionTotal }: OrderSummaryAccordionProps) {
  const { t } = useI18n();
  // Order-summary accordion (subtle, collapsed by default) — items + photos + combined price + nutrition.
  const [summaryOpen, setSummaryOpen] = useState(false);

  return (
    <div className="rounded-[var(--brand-radius)] border overflow-hidden" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)' }} data-testid="order-summary">
      <button
        type="button"
        onClick={() => setSummaryOpen(o => !o)}
        aria-expanded={summaryOpen}
        className="w-full flex items-center justify-between gap-3 px-4 py-3 text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-inset"
      >
        <span className="inline-flex items-center gap-2 min-w-0">
          <i className="ti ti-receipt text-lg shrink-0" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
          <span className="text-step-sm font-semibold truncate" style={{ color: 'var(--brand-text)' }}>
            {t('checkout.your_order', 'Your order')} · {t('checkout.item_count', '{{n}} items', { n: items.reduce((s, i) => s + i.quantity, 0) })}
          </span>
        </span>
        <span className="inline-flex items-center gap-2 shrink-0">
          <span className="text-step-sm font-bold" style={{ color: 'var(--brand-primary-readable, var(--brand-text))' }}><PriceDisplay amount={total} /></span>
          <i className={`ti ti-chevron-${summaryOpen ? 'up' : 'down'}`} aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
        </span>
      </button>
      {summaryOpen && (
        <div className="px-4 pb-4 pt-1 border-t flex flex-col gap-3" style={{ borderColor: 'var(--brand-border)' }}>
          {/* Item rows */}
          <ul className="flex flex-col gap-2 mt-2">
            {items.map(item => {
              const img = orderMenuMap[item.productId]?.image;
              return (
                <li key={item.id} className="flex items-center gap-3">
                  {img ? (
                    <img src={img} alt="" aria-hidden="true" className="w-11 h-11 rounded-lg object-cover shrink-0" style={{ background: 'var(--brand-surface-raised)' }} />
                  ) : (
                    <span className="w-11 h-11 rounded-lg shrink-0 flex items-center justify-center" style={{ background: 'var(--brand-surface-raised)' }}><i className="ti ti-tools-kitchen-2" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} /></span>
                  )}
                  <span className="flex-1 min-w-0">
                    <span className="block text-step-sm font-medium truncate" style={{ color: 'var(--brand-text)' }}>{item.name}</span>
                    <span className="block text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>× {item.quantity}</span>
                  </span>
                  <span className="text-step-sm font-semibold shrink-0 tabular-nums" style={{ color: 'var(--brand-text)' }}><PriceDisplay amount={item.price * item.quantity} size="sm" /></span>
                </li>
              );
            })}
          </ul>
          {/* Combined nutrition — reuse the DishStats viz over the whole-order totals (no ingredients). */}
          {hasNutrition && (
            <DishStats variant="compact" macros={nutritionTotal} className="pt-1" />
          )}
        </div>
      )}
    </div>
  );
}
