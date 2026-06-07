import React, { useEffect, useState, useMemo } from 'react';
import { useParams } from 'react-router-dom';
import { OrderProgress, SkeletonBase, WSStatusDot, EmptyState, CourierLiveMap, useI18n } from '@deliveryos/ui';
import type { LngLatLike, CourierOnMap } from '@deliveryos/ui';
import { apiClient, useWebSocket } from '../../lib/index.js';
import { calcETA } from '@deliveryos/shared-types';

export function OrderStatusPage() {
  const { slug, id } = useParams<{ slug: string, id: string }>();
  const { t } = useI18n();
  
  const [order, setOrder] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [courierPos, setCourierPos] = useState<LngLatLike>([19.817, 41.331]);

  // 1. Initial snapshot fetch
  const fetchOrder = async () => {
    try {
      const data = await apiClient<any>(`/customer/orders/${id}/status`);
      setOrder(data);
    } catch (err: any) {
      if (err.status === 404) {
        // Mock fallback for Stage 2
        setOrder({
          id,
          status: 'PREPARING',
          createdAt: new Date().toISOString(),
          items: [{ name: 'Classic Burger', quantity: 1, price: 650 }],
          total: 650,
          elapsedSeconds: 120,
          kcal_total: 550,
          protein_mg_total: 32,
          fat_mg_total: 24,
          carb_mg_total: 48,
        });
      } else {
        setError('Failed to fetch order status');
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (id) fetchOrder();
  }, [id]);

  // 2. WebSocket Subscription
  const { status: wsStatus } = useWebSocket({
    room: `order:${id}`,
    onMessage: (msg) => {
      if (msg.type === 'order_updated') {
        setOrder((prev: any) => ({ ...prev, ...msg.payload }));
      }
      if (msg.type === 'courier_position') {
        const { lng, lat } = msg.payload;
        setCourierPos([lng, lat]);
      }
    },
    onReconnect: () => {
      fetchOrder();
    }
  });

  const couriers: CourierOnMap[] = useMemo(() => {
    if (order?.courier_name) {
      return [{
        id: 'c1',
        name: order.courier_name,
        initials: (order.courier_name || 'C').charAt(0).toUpperCase(),
        lngLat: courierPos,
        status: 'busy',
      }];
    }
    return [];
  }, [order?.courier_name, courierPos]);

  const destPin: LngLatLike = order?.delivery_lat
    ? [order.delivery_lng || 19.817, order.delivery_lat || 41.331]
    : [19.817, 41.331];

  if (loading) {
    return <div className="p-6 space-y-4"><SkeletonBase className="h-40 w-full" /></div>;
  }

  if (error || !order) {
    return <EmptyState title="Not Found" description={error || 'Order not found'} />;
  }

  return (
    <div className="max-w-md mx-auto min-h-screen bg-[var(--brand-surface)] pb-10">
      {/* Live Courier Map */}
      <div className="h-64 relative w-full">
        <CourierLiveMap
          className="h-full w-full"
          couriers={couriers}
          destinationPin={destPin}
          center={courierPos}
          zoom={14}
        />
        <div className="absolute top-4 left-4 bg-white/90 p-1.5 rounded-full shadow-md z-10">
          <WSStatusDot status={wsStatus === 'disabled' ? 'disconnected' : wsStatus} />
        </div>
      </div>

      {/* Screen-reader accessible courier status (non-visual map alternative) */}
      {order?.courier_name && (
        <div className="sr-only" role="status" aria-live="polite">
          Courier {order.courier_name} is delivering your order.
          {order.eta_minutes ? ` Approximately ${order.eta_minutes} minutes away.` : ''}
          {order.delivery_address ? ` Delivering to ${order.delivery_address}.` : ''}
        </div>
      )}

      <div className="p-4 space-y-6 -mt-4 relative z-10 bg-[var(--brand-surface)] rounded-t-[24px]">
        
        <div className="text-center">
          <h1 className="text-2xl font-bold text-[var(--brand-text)] mb-1" style={{ fontFamily: 'var(--brand-font-heading)' }}>
            {calcETA(order.createdAt, order.elapsedSeconds || 0)}
          </h1>
          <p className="text-[var(--brand-text-muted)] text-sm">{t('client.estimated_arrival', 'Estimated arrival')}</p>
        </div>

        <OrderProgress status={order.status} />

        <div className="bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4">
          <h2 className="font-semibold mb-3">{t('client.order_details', 'Order Details #{{id}}', { id: order.id.substring(0, 4) })}</h2>
          <div className="space-y-2 text-sm">
            {order.items?.map((item: any, i: number) => (
              <div key={i} className="flex justify-between">
                <span>{item.quantity}x {item.name}</span>
                <span>{item.price} ALL</span>
              </div>
            ))}
          </div>
          <div className="border-t border-[var(--brand-border)] mt-4 pt-4 flex justify-between font-bold">
            <span>{t('client.total', 'Total')}</span>
            <span>{order.total} ALL</span>
          </div>
        </div>

        {/* Nutrition snapshot */}
        {order.kcal_total != null && (
          <div className="bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4">
            <div className="flex items-center gap-2 mb-3">
              <span className="text-sm font-semibold">≈ Nutrition</span>
              <span className="text-[10px] px-1.5 py-0.5 rounded" style={{ background: 'var(--brand-surface)', color: 'var(--brand-text-muted)' }}>estimate only</span>
            </div>
            <div className="grid grid-cols-4 gap-2 text-center">
              {[
                { label: 'Calories', value: order.kcal_total, unit: 'kcal' },
                { label: 'Protein', value: order.protein_mg_total, unit: 'g' },
                { label: 'Fat', value: order.fat_mg_total, unit: 'g' },
                { label: 'Carbs', value: order.carb_mg_total, unit: 'g' },
              ].map(n => (
                <div key={n.label} className="p-2 rounded-lg" style={{ background: 'var(--brand-surface)' }}>
                  <div className="text-lg font-bold" style={{ color: 'var(--brand-primary)' }}>{n.value || '—'}</div>
                  <div className="text-[10px]" style={{ color: 'var(--brand-text-muted)' }}>{n.label}</div>
                  <div className="text-[9px] opacity-50">{n.unit}</div>
                </div>
              ))}
            </div>
          </div>
        )}

      </div>
    </div>
  );
}
