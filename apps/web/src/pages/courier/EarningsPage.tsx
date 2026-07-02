import React, { useEffect, useState } from 'react';
import { motion, useReducedMotion } from 'framer-motion';
import { EmptyState, SkeletonBase, StatusBadge, useI18n, PriceDisplay, ease, duration } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';

const CourierEarningsResponse = z.object({
  summary: z.object({
    today: z.number(),
    week: z.number(),
    month: z.number(),
    today_tips: z.number().optional(),
    week_tips: z.number().optional(),
    month_tips: z.number().optional(),
    currency: z.string().optional(),
  }).optional(),
  payouts: z.array(z.object({
    id: z.string(),
    date: z.string(),
    amount: z.number(),
    status: z.string(),
    reference: z.string(),
  })).optional(),
}).passthrough();

interface EarningSummary {
  today: number;
  week: number;
  month: number;
  today_tips: number;
  week_tips: number;
  month_tips: number;
  currency: string;
}

interface Payout {
  id: string;
  date: string;
  amount: number;
  status: string;
  reference: string;
}

// Count-up wrapper: animates the numeric value on mount, then renders the real
// PriceDisplay (formatMoney remains the single money-display authority — display only,
// no math). Collapses to the final value instantly under reduced-motion.
function CountUpPrice({ amount, reduce }: { amount: number; reduce: boolean }) {
  const [shown, setShown] = useState(reduce ? amount : 0);
  useEffect(() => {
    if (reduce) { setShown(amount); return; }
    const from = 0;
    const start = Date.now();
    const duration = 400;
    let raf = 0;
    const tick = () => {
      const p = Math.min((Date.now() - start) / duration, 1);
      const eased = 1 - Math.pow(1 - p, 3);
      setShown(Math.round(from + (amount - from) * eased));
      if (p < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [amount, reduce]);
  return <PriceDisplay amount={shown} />;
}

export function EarningsPage() {
  const [summary, setSummary] = useState<EarningSummary | null>(null);
  const [payouts, setPayouts] = useState<Payout[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const { t } = useI18n();
  const reduce = useReducedMotion();

  // Motion: ease-out reveal + stagger; collapses to instant under reduced-motion.
  const listStagger = {
    hidden: {},
    visible: { transition: reduce ? {} : { staggerChildren: 0.06, delayChildren: 0.05 } },
  };
  const cardItem = {
    hidden: reduce ? { opacity: 0 } : { opacity: 0, y: 12, scale: 0.97 },
    visible: { opacity: 1, y: 0, scale: 1, transition: { duration: duration.base, ease: ease.out } },
  };
  const rowStagger = {
    hidden: {},
    visible: { transition: reduce ? {} : { staggerChildren: 0.03, delayChildren: 0.1 } },
  };
  const rowItem = {
    hidden: reduce ? { opacity: 0 } : { opacity: 0, x: -8 },
    visible: { opacity: 1, x: 0, transition: { duration: duration.base, ease: ease.out } },
  };

  const fetchEarnings = async () => {
    try {
      setLoading(true);
      const data = await apiClient<typeof CourierEarningsResponse>('/courier/me/earnings', { schema: CourierEarningsResponse });
      if (data?.summary) {
        setSummary({
          today: data.summary.today,
          week: data.summary.week,
          month: data.summary.month,
          today_tips: data.summary.today_tips ?? 0,
          week_tips: data.summary.week_tips ?? 0,
          month_tips: data.summary.month_tips ?? 0,
          currency: data.summary.currency || 'ALL',
        });
        setPayouts(Array.isArray(data?.payouts) ? data.payouts : []);
      } else {
        setError(t('courier.error_unexpected_response', 'Unexpected response format'));
      }
    } catch (err: any) {
      setError(t('courier.error_fetch_earnings', 'Failed to fetch earnings data'));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchEarnings();
  }, []);

  const summaryCards = summary ? [
    { label: t('courier.today', 'Today'), amount: summary.today, tips: summary.today_tips, icon: <i className="ti ti-clock-hour-4" aria-hidden="true"></i> },
    { label: t('courier.this_week', 'This Week'), amount: summary.week, tips: summary.week_tips, icon: <i className="ti ti-calendar" aria-hidden="true"></i> },
    { label: t('courier.this_month', 'This Month'), amount: summary.month, tips: summary.month_tips, icon: <i className="ti ti-moneybag" aria-hidden="true"></i> },
  ] : [];

  return (
    <div className="p-4 space-y-6">
      <div className="flex justify-between items-center pb-4 border-b border-[var(--brand-border)]">
        <h1 className="text-2xl font-bold text-[var(--brand-text)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('courier.earnings_title', 'Earnings')}</h1>
      </div>

      {loading ? (
        <div className="space-y-4">
          <div className="grid grid-cols-3 gap-3">
            {[1, 2, 3].map((i) => (
              <SkeletonBase key={i} className="h-24 rounded-[var(--brand-radius)]" />
            ))}
          </div>
          <SkeletonBase className="h-64 rounded-[var(--brand-radius)]" />
        </div>
      ) : error ? (
        <EmptyState
          title={t('courier.earnings_error_title', 'Could not load earnings')}
          description={error}
          icon={<i className="ti ti-alert-triangle" aria-hidden="true" />}
          action={
            <button
              type="button"
              onClick={fetchEarnings}
              className="inline-flex items-center gap-2 min-h-tap px-5 rounded-[var(--brand-radius-btn)] bg-[var(--brand-primary)] text-[var(--brand-on-primary)] font-semibold transition-[transform,box-shadow,opacity] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 hover:hover:shadow-[var(--elev-2)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]"
            >
              <i className="ti ti-refresh" aria-hidden="true" />
              {t('common.retry', 'Retry')}
            </button>
          }
        />
      ) : (
        <>
          <motion.div
            className="grid grid-cols-3 gap-3"
            variants={listStagger}
            initial="hidden"
            animate="visible"
          >
            {summaryCards.map((card) => (
              <motion.div
                key={card.label}
                variants={cardItem}
                className="min-w-0 flex flex-col items-center text-center bg-[var(--brand-surface)] rounded-[var(--brand-radius)] p-4 shadow-[var(--elev-1)] transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 hover:hover:shadow-[var(--elev-2)]"
              >
                <div className="text-2xl mb-1 text-[var(--brand-primary)]">{card.icon}</div>
                <div className="text-step-2xs text-[var(--brand-text-muted)] uppercase tracking-wider font-semibold mb-1 truncate max-w-full">{card.label}</div>
                <div className="text-lg font-bold text-[var(--brand-text)] tabular-nums min-w-0 truncate max-w-full">
                  <CountUpPrice amount={card.amount} reduce={!!reduce} />
                </div>
                {card.tips > 0 && (
                  <div className="text-step-2xs mt-0.5 text-[var(--brand-text-muted)] tabular-nums truncate max-w-full" data-testid="earnings-tips">
                    {t('courier.incl_tips', 'incl. tips')} <PriceDisplay amount={card.tips} />
                  </div>
                )}
              </motion.div>
            ))}
          </motion.div>

          <div className="bg-[var(--brand-surface)] rounded-[var(--brand-radius)] shadow-[var(--elev-1)] overflow-hidden">
            <div className="p-4 border-b border-[var(--brand-border)]">
              <h2 className="font-bold text-[var(--brand-text)]">{t('courier.payout_history', 'Payout History')}</h2>
            </div>

            {payouts.length === 0 ? (
              <div className="p-4">
                <EmptyState
                  icon={<i className="ti ti-cash-banknote" aria-hidden="true" />}
                  title={t('courier.no_payouts', 'No payouts yet')}
                  description={t('courier.no_payouts_desc', 'Your payouts will appear here once processed.')}
                />
              </div>
            ) : (
              <motion.div
                className="divide-y divide-[var(--brand-border)]"
                variants={rowStagger}
                initial="hidden"
                animate="visible"
              >
                {payouts.map((payout) => (
                  <motion.div
                    key={payout.id}
                    variants={rowItem}
                    className="p-4 flex items-center justify-between gap-3 transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:bg-[var(--brand-surface-raised)]"
                  >
                    <div className="min-w-0 flex-1">
                      <div className="text-sm font-medium text-[var(--brand-text)] truncate">{payout.reference}</div>
                      <div className="text-xs text-[var(--brand-text-muted)] tabular-nums truncate">{payout.date}</div>
                    </div>
                    <div className="flex items-center gap-3 shrink-0">
                      <div className="text-sm font-bold text-[var(--brand-text)] tabular-nums whitespace-nowrap"><PriceDisplay amount={payout.amount} /></div>
                      <StatusBadge status={payout.status} />
                    </div>
                  </motion.div>
                ))}
              </motion.div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
