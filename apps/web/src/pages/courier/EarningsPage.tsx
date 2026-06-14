import React, { useEffect, useState } from 'react';
import { EmptyState, SkeletonBase, StatusBadge, useI18n, PriceDisplay } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';

const CourierEarningsResponse = z.object({
  summary: z.object({
    today: z.number(),
    week: z.number(),
    month: z.number(),
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
          currency: data.summary.currency || 'ALL',
        });
        setPayouts(Array.isArray(data?.payouts) ? data.payouts : []);
      } else {
        setError('Unexpected response format');
      }
    } catch (err: any) {
      setError('Failed to fetch earnings data');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchEarnings();
  }, []);

  const summaryCards = summary ? [
    { label: t('courier.today', 'Today'), amount: summary.today, icon: '\u2600' },
    { label: t('courier.this_week', 'This Week'), amount: summary.week, icon: '\u{1F4C5}' },
    { label: t('courier.this_month', 'This Month'), amount: summary.month, icon: '\u{1F4B0}' },
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
          <div className="grid grid-cols-3 gap-3">
            {summaryCards.map((card) => (
              <div
                key={card.label}
                className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4 text-center"
              >
                <div className="text-2xl mb-1">{card.icon}</div>
                <div className="text-xs text-[var(--brand-text-muted)] uppercase tracking-wider font-semibold mb-1">{card.label}</div>
                <div className="text-lg font-bold text-[var(--brand-text)]"><PriceDisplay amount={card.amount} /></div>
              </div>
            ))}
          </div>

          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] overflow-hidden">
            <div className="p-4 border-b border-[var(--brand-border)]">
              <h2 className="font-bold text-[var(--brand-text)]">{t('courier.payout_history', 'Payout History')}</h2>
            </div>

            {payouts.length === 0 ? (
              <div className="p-8">
                <EmptyState title={t('courier.no_payouts', 'No payouts yet')} description={t('courier.no_payouts_desc', 'Your payouts will appear here once processed.')} />
              </div>
            ) : (
              <div className="divide-y divide-[var(--brand-border)]">
                {payouts.map((payout) => (
                  <div key={payout.id} className="p-4 flex items-center justify-between">
                    <div className="min-w-0 flex-1">
                      <div className="text-sm font-medium text-[var(--brand-text)]">{payout.reference}</div>
                      <div className="text-xs text-[var(--brand-text-muted)]">{payout.date}</div>
                    </div>
                    <div className="flex items-center gap-3">
                      <div className="text-sm font-bold text-[var(--brand-text)]"><PriceDisplay amount={payout.amount} /></div>
                      <StatusBadge status={payout.status} />
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
