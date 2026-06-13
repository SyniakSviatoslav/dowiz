import React, { useEffect, useState } from 'react';
import { EmptyState, SkeletonBase, StatusBadge, useI18n, PriceDisplay } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

interface DeliveryHistory {
  id: string;
  orderId: string;
  date: string;
  restaurant: string;
  customerAddress: string;
  amount: number;
  status: string;
  rating?: number;
  feedback?: string;
}

export function HistoryPage() {
  const [deliveries, setDeliveries] = useState<DeliveryHistory[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const { t } = useI18n();

  const fetchHistory = async () => {
    try {
      setLoading(true);
      const data = await apiClient<any>('/courier/me/history');
      setDeliveries(Array.isArray(data) ? data : []);
    } catch (err: any) {
      setError('Failed to fetch delivery history');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchHistory();
  }, []);

  const formatDate = (iso: string) => {
    const d = new Date(iso);
    return d.toLocaleDateString('en-GB', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
  };

  const renderStars = (rating?: number) => {
    if (!rating) return <span className="text-xs text-[var(--brand-text-muted)]">{t('courier.no_rating', 'No rating')}</span>;
    return (
      <div className="flex items-center gap-0.5">
        {[1, 2, 3, 4, 5].map((star) => (
          <span key={star} className={`text-xs ${star <= rating ? 'text-[var(--color-warning)]' : 'text-[var(--brand-border)]'}`}>
            &#9733;
          </span>
        ))}
      </div>
    );
  };

  return (
    <div className="p-4 space-y-6">
      <div className="flex justify-between items-center pb-4 border-b border-[var(--brand-border)]">
        <h1 className="text-2xl font-bold text-[var(--brand-text)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('courier.history_title', 'History')}</h1>
        <div className="text-sm text-[var(--brand-text-muted)]">
          {deliveries.length} {t('courier.deliveries_count', 'deliveries')}
        </div>
      </div>

      {loading ? (
        <div className="space-y-3">
          {[1, 2, 3, 4, 5].map((i) => (
            <SkeletonBase key={i} className="h-20 rounded-[var(--brand-radius)]" />
          ))}
        </div>
      ) : error ? (
        <EmptyState title={t('common.error', 'Error')} description={error} />
      ) : deliveries.length === 0 ? (
        <EmptyState title={t('courier.no_deliveries', 'No deliveries yet')} description={t('courier.no_deliveries_desc', 'Your completed deliveries will appear here.')} />
      ) : (
        <div className="space-y-3">
          {deliveries.map((delivery) => (
            <div
              key={delivery.id}
              className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4"
            >
              <div className="flex items-start justify-between mb-2">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="text-xs font-mono text-[var(--brand-text-muted)]">{delivery.orderId}</span>
                    <StatusBadge status={delivery.status} />
                  </div>
                  <div className="text-sm font-medium text-[var(--brand-text)]">{delivery.restaurant}</div>
                  <div className="text-xs text-[var(--brand-text-muted)]">{delivery.customerAddress}</div>
                </div>
                <div className="text-right ml-3">
                  <div className="text-sm font-bold text-[var(--brand-text)]"><PriceDisplay amount={delivery.amount} /></div>
                  <div className="text-xs text-[var(--brand-text-muted)]">{formatDate(delivery.date)}</div>
                </div>
              </div>

              <div className="flex items-center justify-between border-t border-[var(--brand-border)] pt-2">
                {renderStars(delivery.rating)}
                {delivery.feedback && (
                  <div className="text-xs text-[var(--brand-text-muted)] italic truncate max-w-[180px]">
                    &ldquo;{delivery.feedback}&rdquo;
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
