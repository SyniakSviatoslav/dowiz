import React, { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { EmptyState, SkeletonBase, StatusBadge, useI18n, PriceDisplay } from '@deliveryos/ui';
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

export function EarningsPage() {
  const [summary, setSummary] = useState<EarningSummary | null>(null);
  const [payouts, setPayouts] = useState<Payout[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const { t } = useI18n();

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
    { label: t('courier.today', 'Today'), amount: summary.today, tips: summary.today_tips, icon: <i className="ti ti-sun" aria-hidden="true"></i> },
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
        <EmptyState title={t('common.error', 'Error')} description={error} />
      ) : (
        <>
          <motion.div
            className="grid grid-cols-3 gap-3"
            variants={{ hidden: {}, visible: { transition: { staggerChildren: 0.06, delayChildren: 0.05 } } }}
            initial="hidden"
            animate="visible"
          >
            {summaryCards.map((card) => (
              <motion.div
                key={card.label}
                variants={{ hidden: { opacity: 0, y: 12, scale: 0.97 }, visible: { opacity: 1, y: 0, scale: 1, transition: { type: 'spring', stiffness: 260, damping: 24 } } }}
                className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4 text-center"
              >
                <div className="text-2xl mb-1">{card.icon}</div>
                <div className="text-xs text-[var(--brand-text-muted)] uppercase tracking-wider font-semibold mb-1">{card.label}</div>
                <div className="text-lg font-bold text-[var(--brand-text)]"><PriceDisplay amount={card.amount} /></div>
                {card.tips > 0 && (
                  <div className="text-[11px] mt-0.5" style={{ color: 'var(--brand-text-muted)' }} data-testid="earnings-tips">
                    {t('courier.incl_tips', 'incl. tips')} <PriceDisplay amount={card.tips} />
                  </div>
                )}
              </motion.div>
            ))}
          </motion.div>

          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] overflow-hidden">
            <div className="p-4 border-b border-[var(--brand-border)]">
              <h2 className="font-bold text-[var(--brand-text)]">{t('courier.payout_history', 'Payout History')}</h2>
            </div>

            {payouts.length === 0 ? (
              <div className="flex flex-col items-center justify-center p-8 text-center">
                <h3 className="mb-2 text-lg font-semibold text-[var(--brand-text)]">{t('courier.no_payouts', 'No payouts yet')}</h3>
                <p className="text-sm text-[var(--brand-text-muted)] max-w-sm">{t('courier.no_payouts_desc', 'Your payouts will appear here once processed.')}</p>
              </div>
            ) : (
              <motion.div
                className="divide-y divide-[var(--brand-border)]"
                variants={{ hidden: {}, visible: { transition: { staggerChildren: 0.03, delayChildren: 0.1 } } }}
                initial="hidden"
                animate="visible"
              >
                {payouts.map((payout) => (
                  <motion.div
                    key={payout.id}
                    variants={{ hidden: { opacity: 0, x: -8 }, visible: { opacity: 1, x: 0, transition: { type: 'spring', stiffness: 260, damping: 24 } } }}
                    className="p-4 flex items-center justify-between"
                  >
                    <div className="min-w-0 flex-1">
                      <div className="text-sm font-medium text-[var(--brand-text)]">{payout.reference}</div>
                      <div className="text-xs text-[var(--brand-text-muted)]">{payout.date}</div>
                    </div>
                    <div className="flex items-center gap-3">
                      <div className="text-sm font-bold text-[var(--brand-text)]"><PriceDisplay amount={payout.amount} /></div>
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
