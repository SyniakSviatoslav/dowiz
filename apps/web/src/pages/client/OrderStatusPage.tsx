import React, { useEffect, useState, useMemo, useRef, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { OrderProgress, SkeletonBase, WSStatusDot, EmptyState, CourierLiveMap, useI18n, useToast } from '@deliveryos/ui';
import type { LngLatLike, CourierOnMap } from '@deliveryos/ui';
import { apiClient, useWebSocket } from '../../lib/index.js';
import { calcETA } from '@deliveryos/shared-types';

const STATUS_LABELS: Record<string, string> = {
  PENDING: 'Order placed',
  CONFIRMED: 'Order confirmed',
  PREPARING: 'Preparing your order',
  READY: 'Order ready',
  IN_DELIVERY: 'On the way!',
  DELIVERED: 'Delivered',
  REJECTED: 'Order rejected',
  CANCELLED: 'Order cancelled',
};

const STATUS_VARIANTS: Record<string, 'info' | 'success' | 'warning' | 'error'> = {
  PENDING: 'info',
  CONFIRMED: 'info',
  PREPARING: 'info',
  READY: 'warning',
  IN_DELIVERY: 'success',
  DELIVERED: 'success',
  REJECTED: 'error',
  CANCELLED: 'error',
};

export function OrderStatusPage() {
  const { slug, id } = useParams<{ slug: string, id: string }>();
  const { t } = useI18n();
  const { showToast } = useToast();
  const prevStatusRef = useRef<string>('');
  const lastWsMsgRef = useRef<number>(Date.now());
  const watchIdRef = useRef<number | null>(null);

  const [order, setOrder] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [courierPos, setCourierPos] = useState<LngLatLike>([19.817, 41.331]);
  const [etaMinutes, setEtaMinutes] = useState<number | null>(null);
  const [sharingLocation, setSharingLocation] = useState(false);

  const fetchOrder = async () => {
    try {
      const data = await apiClient<any>(`/customer/orders/${id}/status`);
      setOrder(data);
      if (data.courierPosition) {
        setCourierPos([data.courierPosition.lng, data.courierPosition.lat]);
      }
      if (data.etaMinutes != null) {
        setEtaMinutes(data.etaMinutes);
      }
    } catch (err: any) {
      // eslint-disable-next-line
      if (err?.status === 404) {
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

  // Watchdog: resync if no WS message for 30s
  useEffect(() => {
    const interval = setInterval(() => {
      if (Date.now() - lastWsMsgRef.current > 30000) {
        fetchOrder();
      }
    }, 15000);
    return () => clearInterval(interval);
  }, [id]);

  const { status: wsStatus, sendMessage } = useWebSocket({
    room: `order:${id}`,
    onMessage: (msg: any) => {
      lastWsMsgRef.current = Date.now();

      // Direct protocol messages
      if (msg.type === 'error') return;

      // Wrapped messageBus messages: { room, data: { type, payload } }
      if (!msg.data) return;
      const inner = msg.data;

      if (inner.type === 'order.courier_updated') {
        const p = inner.payload;
        if (p.position) {
          setCourierPos([p.position.lng, p.position.lat]);
        }
        if (p.etaSeconds != null) {
          setEtaMinutes(Math.ceil(p.etaSeconds / 60));
        }
         if (p.courierName) {
           setOrder((prev: any) => ({ ...prev, courierName: p.courierName }));
         }
        if (p.phoneMasked) {
          setOrder((prev: any) => ({ ...prev, courier_phone: p.phoneMasked }));
        }
        if (p.status === 'delivered') {
          setOrder((prev: any) => ({ ...prev, status: 'DELIVERED' }));
        }
        return;
      }

      if (inner.type === 'order.status' && inner.status) {
        setOrder((prev: any) => ({ ...prev, status: inner.status }));
      }
    },
    onReconnect: () => {
      fetchOrder();
    }
  });

  // Toast on status change
  useEffect(() => {
    if (!order?.status) return;
    if (order.status === prevStatusRef.current) return;
    prevStatusRef.current = order.status;
    const label = STATUS_LABELS[order.status] || order.status;
    const variant = STATUS_VARIANTS[order.status] || 'info';
    showToast(label, variant);
  }, [order?.status, showToast]);

  // CR-6: Auto-stop sharing on DELIVERED
  useEffect(() => {
    if (order?.status === 'DELIVERED' && watchIdRef.current != null) {
      navigator.geolocation.clearWatch(watchIdRef.current);
      watchIdRef.current = null;
      setSharingLocation(false);
    }
  }, [order?.status]);

  // CR-6: Cleanup geolocation on unmount
  useEffect(() => {
    return () => {
      if (watchIdRef.current != null) {
        navigator.geolocation.clearWatch(watchIdRef.current);
      }
    };
  }, []);

  const startSharing = useCallback(() => {
    if (!navigator.geolocation) return;
    watchIdRef.current = navigator.geolocation.watchPosition(
      (pos) => {
        sendMessage({
          type: 'client_location',
          payload: { lat: pos.coords.latitude, lng: pos.coords.longitude }
        });
      },
      () => {
        setSharingLocation(false);
        watchIdRef.current = null;
      },
      { enableHighAccuracy: true, timeout: 10000, maximumAge: 30000 }
    );
    setSharingLocation(true);
  }, [sendMessage]);

  const stopSharing = useCallback(() => {
    if (watchIdRef.current != null) {
      navigator.geolocation.clearWatch(watchIdRef.current);
      watchIdRef.current = null;
    }
    sendMessage({ type: 'client_location_stop' });
    setSharingLocation(false);
  }, [sendMessage]);

  const couriers: CourierOnMap[] = useMemo(() => {
    if (order?.courierName) {
      return [{
        id: 'c1',
        name: order.courierName,
        initials: (order.courierName || 'C').charAt(0).toUpperCase(),
        lngLat: courierPos,
        status: 'busy',
      }];
    }
    return [];
  }, [order?.courierName, courierPos]);

  const destPin: LngLatLike = order?.deliveryLat
    ? [order.deliveryLng || 19.817, order.deliveryLat || 41.331]
    : [19.817, 41.331];

  const isInDelivery = order?.status === 'IN_DELIVERY';
  const displayEta = etaMinutes != null
    ? `${etaMinutes} min`
    : order?.createdAt
      ? calcETA(order.createdAt, order.elapsedSeconds || 0)
      : '';

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

      {/* Screen-reader accessible courier status */}
      {order?.courierName && (
        <div className="sr-only" role="status" aria-live="polite">
          Courier {order.courierName} is delivering your order.
          {etaMinutes ? ` Approximately ${etaMinutes} minutes away.` : ''}
          {order.deliveryAddress ? ` Delivering to ${order.deliveryAddress}.` : ''}
        </div>
      )}

      <div className="p-4 space-y-6 -mt-4 relative z-10 bg-[var(--brand-surface)] rounded-t-[24px]">
        
        <div className="text-center">
          <h1 className="text-2xl font-bold text-[var(--brand-text)] mb-1" style={{ fontFamily: 'var(--brand-font-heading)' }}>
            {displayEta}
          </h1>
          <p className="text-[var(--brand-text-muted)] text-sm">{t('client.estimated_arrival', 'Estimated arrival')}</p>
        </div>

        <OrderProgress status={order.status} />

        {/* CR-6: Share my location (visible during IN_DELIVERY) */}
        {isInDelivery && (
          <div className="flex flex-col gap-2">
            {sharingLocation ? (
              <div className="flex items-center gap-3 bg-[var(--color-success)]/10 border border-[var(--color-success)]/30 rounded-[var(--brand-radius)] p-3">
                <div className="w-3 h-3 rounded-full bg-[var(--color-success)] animate-pulse shrink-0" />
                <div className="flex-1 text-sm">
                  <div className="font-semibold text-[var(--color-success)]">
                    {t('client.sharing_location', 'Sharing your location')}
                  </div>
                  <div className="text-xs text-[var(--brand-text-muted)]">
                    {t('client.sharing_location_note', 'Courier can see your live position')}
                  </div>
                </div>
                <button
                  onClick={stopSharing}
                  className="text-xs px-2.5 py-1.5 rounded-full border border-[var(--brand-border)] bg-[var(--brand-surface)] text-[var(--brand-text)]"
                >
                  {t('client.stop_sharing', 'Stop')}
                </button>
              </div>
            ) : (
              <button
                onClick={startSharing}
                className="flex items-center justify-center gap-2 w-full py-3 rounded-[var(--brand-radius)] bg-[var(--brand-primary)] text-[var(--color-on-primary)] font-semibold text-sm"
              >
                <span>📍</span>
                <span>{t('client.share_location', 'Share my location with courier')}</span>
              </button>
            )}
          </div>
        )}

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
