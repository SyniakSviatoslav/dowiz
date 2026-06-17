import React, { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { EmptyState, PriceDisplay, useI18n } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';

const POLL_INTERVAL_MS = 30_000;

const SnapshotOrderSchema = z.object({
  orderId: z.string(),
  status: z.string(),
  total: z.number().optional().nullable(),
  currency: z.string().optional(),
  createdAt: z.string(),
  statusUpdatedAt: z.string().optional().nullable(),
  customerNameMasked: z.string().optional().nullable(),
  customerPhoneMasked: z.string().optional().nullable(),
  itemCount: z.union([z.number(), z.string()]).optional().nullable(),
  paymentMethod: z.string().optional().nullable(),
  dwellSeconds: z.number().optional().nullable(),
  metadata: z.any().optional().nullable(),
});

const SnapshotResponseSchema = z.object({
  orders: z.array(SnapshotOrderSchema),
  counts: z.record(z.string(), z.number()).optional(),
  nextCursor: z.string().optional().nullable(),
  serverTime: z.string().optional(),
  activeAlertCount: z.number().optional(),
  activeSignalCount: z.number().optional(),
});

type SnapshotOrder = z.infer<typeof SnapshotOrderSchema>;

interface DispatchViewProps {
  locationId: string;
}

function formatElapsed(seconds: number | null | undefined): string {
  if (seconds == null) return '—';
  const m = Math.floor(seconds / 60);
  const h = Math.floor(m / 60);
  if (h > 0) return `${h}h ${m % 60}m`;
  return `${m}m`;
}

function formatTime(iso: string): string {
  try {
    return new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(new Date(iso));
  } catch {
    return iso;
  }
}

export function DispatchView({ locationId }: DispatchViewProps) {
  const { t } = useI18n();
  const [orders, setOrders] = useState<SnapshotOrder[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  const fetchDelivered = useCallback(async () => {
    if (!locationId) return;
    try {
      const data = await apiClient<typeof SnapshotResponseSchema>(
        `/owner/locations/${locationId}/dashboard/snapshot?status=DELIVERED&limit=50`,
        { schema: SnapshotResponseSchema },
      );
      setOrders(data.orders ?? []);
      setError('');
    } catch (err: any) {
      setError(err?.message || 'Failed to load deliveries');
    } finally {
      setLoading(false);
    }
  }, [locationId]);

  useEffect(() => {
    fetchDelivered();
    const timer = setInterval(fetchDelivered, POLL_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [fetchDelivered]);

  if (loading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {[1, 2, 3, 4].map(i => (
          <div key={i} className="h-24 rounded-xl shimmer" />
        ))}
      </div>
    );
  }

  if (error) {
    return <EmptyState title={t('common.error', 'Error')} description={error} />;
  }

  if (orders.length === 0) {
    return (
      <EmptyState
        title={t('admin.dispatch_empty_title', 'No deliveries yet today')}
        description={t('admin.dispatch_empty_desc', 'Delivered orders will appear here as they complete.')}
        icon={<i className="ti ti-truck-delivery text-4xl" style={{ color: 'var(--brand-text-muted)', opacity: 0.4 }} />}
      />
    );
  }

  return (
    <motion.div
      className="grid grid-cols-1 md:grid-cols-2 gap-4"
      variants={{ visible: { transition: { staggerChildren: 0.025 } } }}
      initial="hidden"
      animate="visible"
      aria-live="polite"
      aria-label={t('admin.dispatch_view_label', 'Delivered orders')}
    >
      {orders.map(order => (
        <motion.div
          key={order.orderId}
          variants={{ hidden: { opacity: 0, y: 10 }, visible: { opacity: 1, y: 0, transition: { duration: 0.2, ease: [0.16, 1, 0.3, 1] } } }}
          data-testid={`dispatch-card-${order.orderId}`}
          data-status={order.status}
        >
          <div className="card-base p-4 flex flex-col gap-2">
            {/* Header row: short ID + time */}
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <i className="ti ti-circle-check-filled text-sm" style={{ color: 'var(--color-success)' }} />
                <span className="text-xs font-mono font-semibold" style={{ color: 'var(--brand-text)' }}>
                  {order.orderId.slice(0, 8).toUpperCase()}
                </span>
              </div>
              <span className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>
                {formatTime(order.createdAt)}
              </span>
            </div>

            {/* Customer + items */}
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                {order.customerNameMasked && (
                  <div className="text-sm font-medium truncate" style={{ color: 'var(--brand-text)' }}>
                    {order.customerNameMasked}
                  </div>
                )}
                {order.itemCount != null && (
                  <div className="text-xs mt-0.5" style={{ color: 'var(--brand-text-muted)' }}>
                    {t('admin.item_count', '{{count}} items').replace('{{count}}', String(order.itemCount))}
                  </div>
                )}
              </div>

              {/* Total */}
              {order.total != null && (
                <div className="shrink-0">
                  <PriceDisplay amount={order.total} size="lg" />
                </div>
              )}
            </div>

            {/* Footer row: payment method + dwell time */}
            <div className="flex items-center justify-between pt-1 border-t" style={{ borderColor: 'var(--brand-border)' }}>
              <div className="flex items-center gap-1.5">
                {order.paymentMethod && (
                  <span className="inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
                    <i className={`ti ${order.paymentMethod === 'CASH' ? 'ti-cash' : 'ti-credit-card'} text-[10px]`} />
                    {order.paymentMethod}
                  </span>
                )}
              </div>
              {order.dwellSeconds != null && (
                <span className="text-xs" style={{ color: 'var(--brand-text-muted)' }}>
                  <i className="ti ti-clock text-[10px] mr-1" />
                  {formatElapsed(order.dwellSeconds)}
                </span>
              )}
            </div>
          </div>
        </motion.div>
      ))}
    </motion.div>
  );
}
