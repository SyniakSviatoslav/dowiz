import React from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { useI18n, ease, duration, PriceDisplay } from '@deliveryos/ui';
import type { DeliveryType } from './types.js';

// Mechanical extraction of the CheckoutPage order-summary card. All totals are
// computed in CheckoutPage (client MIRROR of the server math, ADR-0005) and passed
// down as-is — this component only renders them.
interface OrderSummarySectionProps {
  deliveryType: DeliveryType;
  subtotal: number;
  feeKnown: boolean;
  deliveryFee: number;
  taxTotal: number;
  tipAmount: number;
  total: number;
  hasNutrition: boolean;
  nutritionKcal: number;
}

export function OrderSummarySection({
  deliveryType, subtotal, feeKnown, deliveryFee, taxTotal, tipAmount, total,
  hasNutrition, nutritionKcal,
}: OrderSummarySectionProps) {
  const { t } = useI18n();
  const prefersReducedMotion = useReducedMotion();

  return (
    <motion.div
      initial={prefersReducedMotion ? false : { opacity: 0, y: 12 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-40px' }}
      transition={{ duration: 0.25, ease: ease.out, delay: 0.15 }}
      className="rounded-[var(--brand-radius)] p-4 border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', boxShadow: 'var(--elev-1)' }}>
      <h2 className="text-step-xl font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('order.title')}</h2>
      <div className="space-y-3 mb-4">
        <div className="flex justify-between items-baseline gap-3 text-step-sm">
          <span className="min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>{t('cart.subtotal')}</span>
          <span className="shrink-0 tabular-nums"><PriceDisplay amount={subtotal} /></span>
        </div>
        {deliveryType === 'delivery' && (
          <div className="flex justify-between items-baseline gap-3 text-step-sm">
            <span className="min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>{t('cart.delivery_fee')}</span>
            {feeKnown ? (
              <span className="shrink-0 tabular-nums"><PriceDisplay amount={deliveryFee} /></span>
            ) : (
              // Distance-tiered venue — the fee depends on the delivery address and is finalised by
              // the server. We never invent a number we can't collect at the door (ADR-0005).
              <span className="shrink-0 text-step-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.fee_at_checkout', 'Calculated at checkout')}</span>
            )}
          </div>
        )}
        {taxTotal > 0 && (
          <div className="flex justify-between items-baseline gap-3 text-step-sm">
            <span className="min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>{t('cart.tax', 'Tax')}</span>
            <span className="shrink-0 tabular-nums"><PriceDisplay amount={taxTotal} /></span>
          </div>
        )}
        {tipAmount > 0 && (
          <div className="flex justify-between items-baseline gap-3 text-step-sm" data-testid="checkout-tip-line">
            <span className="min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.tip_for_courier', 'Tip for courier (cash)')}</span>
            <span className="shrink-0 tabular-nums"><PriceDisplay amount={tipAmount} /></span>
          </div>
        )}
        {hasNutrition && (
          <div className="flex justify-between items-baseline gap-3 text-step-xs">
            <span className="min-w-0 truncate" style={{ color: 'var(--brand-text-muted)' }}>≈ {t('menu.nutrition')}</span>
            <span className="shrink-0 font-medium tabular-nums" style={{ color: 'var(--brand-text-muted)' }}>~{nutritionKcal} kcal</span>
          </div>
        )}
      </div>
      <div className="pt-4 border-t flex justify-between items-center gap-3" style={{ borderColor: 'var(--brand-border)' }}>
        <span className="text-step-base font-bold min-w-0 truncate" style={{ color: 'var(--brand-text)' }}>{t('cart.total')}</span>
        <motion.span
          key={total}
          data-testid="checkout-total"
          className="shrink-0 tabular-nums"
          initial={prefersReducedMotion ? false : { opacity: 0.4, y: -4 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: duration.base, ease: ease.out }}
        >
          <PriceDisplay amount={total} size="lg" />
        </motion.span>
      </div>
      {deliveryType === 'delivery' && !feeKnown && (
        <p className="text-step-xs mt-1 text-right" style={{ color: 'var(--brand-text-muted)' }}>
          {t('checkout.plus_delivery_fee', '+ delivery fee, calculated at checkout')}
        </p>
      )}
      {tipAmount > 0 && (
        <div className="flex justify-between items-center gap-3 text-step-sm mt-2" style={{ color: 'var(--brand-text-muted)' }} data-testid="checkout-cash-due">
          <span className="min-w-0 truncate">{t('checkout.cash_to_courier', 'Cash to courier (incl. tip)')}</span>
          <span className="shrink-0 tabular-nums"><PriceDisplay amount={total + tipAmount} /></span>
        </div>
      )}
      {/* Pre-order ETA — there is no order yet, so this is a deliberately WIDE
          approximate range that refines once the order is placed (the status page
          then shows the honest server range). Delivery only; pickup has no ETA. */}
      {deliveryType === 'delivery' && (
        <div className="mt-4 pt-4 border-t flex items-start gap-2.5" style={{ borderColor: 'var(--brand-border)' }} data-testid="checkout-eta-estimate">
          <i className="ti ti-clock text-lg shrink-0 mt-0.5" aria-hidden="true" style={{ color: 'var(--brand-text-muted)' }} />
          <div className="min-w-0">
            <div className="text-step-sm font-semibold tabular-nums" style={{ color: 'var(--brand-text)' }}>
              {t('order.eta_range', '{{low}}–{{high}} min', { low: 25, high: 45 })}
            </div>
            <p className="text-step-xs leading-snug" style={{ color: 'var(--brand-text-muted)' }}>
              {t('checkout.eta_estimate', 'Estimated time — refines after you place the order')}
            </p>
          </div>
        </div>
      )}
    </motion.div>
  );
}
