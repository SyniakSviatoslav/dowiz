import React from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { useI18n, ease, PriceDisplay } from '@deliveryos/ui';

// Mechanical extraction of the CheckoutPage payment card (cash + tip). All money
// math stays in CheckoutPage (client MIRROR, ADR-0005); this renders the exact
// same JSX against the state/handlers it always consumed.
interface PaymentSectionProps {
  currencySymbol: string;
  total: number;
  cashAmount: number;
  setCashAmount: React.Dispatch<React.SetStateAction<number>>;
  tipAmount: number;
  setTipAmount: React.Dispatch<React.SetStateAction<number>>;
}

export function PaymentSection({
  currencySymbol, total,
  cashAmount, setCashAmount,
  tipAmount, setTipAmount,
}: PaymentSectionProps) {
  const { t } = useI18n();
  const prefersReducedMotion = useReducedMotion();

  return (
    <motion.div
      initial={prefersReducedMotion ? false : { opacity: 0, y: 12 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: '-40px' }}
      transition={{ duration: 0.25, ease: ease.out, delay: 0.1 }}
      className="rounded-[var(--brand-radius)] p-4 border" style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', boxShadow: 'var(--elev-1)' }}>
      <h2 className="text-step-xl font-semibold mb-4" style={{ color: 'var(--brand-text)', fontFamily: 'var(--brand-font-heading)' }}>{t('checkout.payment_method')}</h2>
      <div className="border rounded-[var(--brand-radius-sm)] p-3 mb-3" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-primary)' }}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <i className="ti ti-cash text-xl" aria-hidden="true" style={{ color: 'var(--brand-primary)' }} />
            <div>
              <div className="text-step-sm font-bold" style={{ color: 'var(--brand-text)' }}>{t('checkout.cash')}</div>
              <div className="text-step-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.place_order')}</div>
            </div>
          </div>
          <i className="ti ti-check" aria-hidden="true" style={{ color: 'var(--brand-primary)' }} />
        </div>
        <div className="mt-3 pt-3 border-t" style={{ borderColor: 'var(--brand-border)' }}>
          <label htmlFor="cash-amount" className="text-step-xs font-semibold mb-1.5 block" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.cash_amount', 'Cash amount')}</label>
          <div className="flex gap-2">
            <div className="relative flex-1">
              <span className="absolute left-3 top-1/2 -translate-y-1/2 text-step-sm font-bold" style={{ color: 'var(--brand-text-muted)' }}>{currencySymbol}</span>
              <input
                id="cash-amount"
                type="number"
                inputMode="decimal"
                min={total}
                value={cashAmount || ''}
                onChange={e => setCashAmount(parseInt(e.target.value) || 0)}
                className="w-full h-[48px] pl-11 pr-3 outline-none text-step-sm font-bold border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]"
                style={{ background: 'var(--brand-surface)', borderColor: cashAmount > 0 && cashAmount < total ? 'var(--color-danger)' : 'var(--brand-border)', color: 'var(--brand-text)' }}
                placeholder={String(total)}
              />
            </div>
          </div>
          {/* UX-4: optional courier tip (single amount, replaces %-badges) */}
          <div className="mt-3 pt-3 border-t" style={{ borderColor: 'var(--brand-border)' }}>
            <label htmlFor="tip-amount" className="text-step-xs font-semibold mb-1.5 block" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.tip_amount', 'Tip for courier (optional)')}</label>
            <div className="relative">
              <span className="absolute left-3 top-1/2 -translate-y-1/2 text-step-sm font-bold" style={{ color: 'var(--brand-text-muted)' }}>{currencySymbol}</span>
              <input
                id="tip-amount"
                type="number"
                inputMode="decimal"
                min={0}
                max={1000000}
                value={tipAmount || ''}
                data-testid="checkout-tip"
                onChange={e => setTipAmount(Math.min(1000000, Math.max(0, parseInt(e.target.value) || 0)))}
                className="w-full h-[48px] pl-11 pr-3 outline-none text-step-sm font-bold border rounded-[var(--brand-radius-sm)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1 focus-visible:border-[var(--brand-primary)]"
                style={{ background: 'var(--brand-surface)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
                placeholder="0"
              />
            </div>
            <p className="text-step-xs mt-1" style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.tip_hint', 'Goes entirely to your courier, in cash on delivery.')}</p>
          </div>
          {cashAmount > 0 && (
            <div className="flex justify-between text-step-sm mt-2 px-1">
              {cashAmount >= total ? (
                <>
                  <span style={{ color: 'var(--brand-text-muted)' }}>{t('checkout.change', 'Change')}</span>
                  <span className="font-bold" style={{ color: 'var(--brand-primary)' }}><PriceDisplay amount={cashAmount - total} /></span>
                </>
              ) : (
                <span style={{ color: 'var(--color-danger)' }}>{t('checkout.cash_amount_too_low', 'Amount must be at least')} <PriceDisplay amount={total} /></span>
              )}
            </div>
          )}
        </div>
      </div>
    </motion.div>
  );
}
