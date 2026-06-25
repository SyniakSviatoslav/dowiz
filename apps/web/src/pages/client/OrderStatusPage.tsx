import { safeStorage } from '../../lib/safeStorage.js';
import React, { useEffect, useState, useMemo, useRef, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { motion, useReducedMotion } from 'framer-motion';
import { OrderProgress, SkeletonBase, WSStatusDot, EmptyState, CourierLiveMap, MessageThread, useI18n, useToast, PriceDisplay, ease, duration } from '@deliveryos/ui';
import type { LngLatLike, CourierOnMap } from '@deliveryos/ui';
import { apiClient, useWebSocket } from '../../lib/index.js';
import { messengerLink } from '../../lib/messenger.js';
import { z } from 'zod';

const MessagesResponse = z.object({
  messages: z.array(z.any()),
}).passthrough();

const MessageSendResponse = z.object({
  message: z.any(),
}).passthrough();

const STATUS_LABELS_KEYS: Record<string, string> = {
  PENDING: 'order.placed',
  CONFIRMED: 'order.confirmed',
  PREPARING: 'order.preparing',
  READY: 'order.ready',
  IN_DELIVERY: 'order.in_delivery',
  DELIVERED: 'order.delivered',
  REJECTED: 'order.rejected',
  CANCELLED: 'order.cancelled',
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

// Per-status accent (drives the hero glow + reassuring subline color). Uses --status-* tokens.
const STATUS_ACCENT: Record<string, string> = {
  PENDING: 'var(--status-pending)',
  CONFIRMED: 'var(--status-confirmed)',
  PREPARING: 'var(--status-preparing)',
  READY: 'var(--status-ready)',
  IN_DELIVERY: 'var(--status-in-delivery)',
  DELIVERED: 'var(--status-delivered)',
  REJECTED: 'var(--status-rejected)',
  CANCELLED: 'var(--status-cancelled)',
};

// Reassuring, status-aware subline. Each lifecycle state reads clearly and never as a dead-end.
const STATUS_MESSAGE_KEYS: Record<string, { key: string; fallback: string }> = {
  PENDING: { key: 'order.msg_pending', fallback: 'Sending your order to the restaurant…' },
  CONFIRMED: { key: 'order.msg_confirmed', fallback: 'The restaurant has your order.' },
  PREPARING: { key: 'order.msg_preparing', fallback: 'Your food is being prepared.' },
  READY: { key: 'order.msg_ready', fallback: 'Your order is ready.' },
  IN_DELIVERY: { key: 'order.msg_in_delivery', fallback: 'Your courier is on the way.' },
  DELIVERED: { key: 'order.msg_delivered', fallback: 'Delivered — enjoy your meal!' },
  REJECTED: { key: 'order.msg_rejected', fallback: "The restaurant couldn't take this order." },
  CANCELLED: { key: 'order.msg_cancelled', fallback: 'This order was cancelled.' },
};

export function OrderStatusPage() {
  const { slug, id } = useParams<{ slug: string, id: string }>();
  const { t } = useI18n();
  const { showToast } = useToast();
  const prefersReducedMotion = useReducedMotion();
  const prevStatusRef = useRef<string>('');
  const lastWsMsgRef = useRef<number>(Date.now());
  const watchIdRef = useRef<number | null>(null);

  const [order, setOrder] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [courierPos, setCourierPos] = useState<LngLatLike>([19.817, 41.331]);
  const [hasCourierFix, setHasCourierFix] = useState(false);
  // HONEST ETA range from the server (Prep-Time + Client ETA v1). Null for terminal
  // orders or when the server can't estimate. Drives the headline range text below.
  const [etaRange, setEtaRange] = useState<{ lowMin: number; highMin: number; phase: 'pre_assign' | 'assigned'; overdue: boolean } | null>(null);
  // Real road route (G1/G2): polyline drawn on the map; duration paces local ETA.
  const [routePolyline, setRoutePolyline] = useState<{ lat: number; lng: number }[] | null>(null);
  const [sharingLocation, setSharingLocation] = useState(false);
  const [messages, setMessages] = useState<any[]>([]);
  const [ratingComment, setRatingComment] = useState('');
  const [ratingBusy, setRatingBusy] = useState(false);
  // UX-1: Google review invite. Place ID drives the writereview deep link; shown to
  // ALL delivered customers (no review-gating), no incentive, no pre-filled rating.
  const [googlePlaceId, setGooglePlaceId] = useState<string | null>(null);
  // Offline fallback: restaurant phone to call when live updates drop. Owner-gated
  // by show_phone_on_offline; we only surface it when enabled and a number exists.
  const [fallbackPhone, setFallbackPhone] = useState<string | null>(null);
  useEffect(() => {
    if (!slug) return;
    fetch(`/public/locations/${slug}/info`)
      .then(r => r.ok ? r.json() : null)
      .then((d: any) => { if (d?.googlePlaceId) setGooglePlaceId(d.googlePlaceId); })
      .catch(() => {});
    fetch(`/api/public/locations/${slug}/fallback-config`)
      .then(r => r.ok ? r.json() : null)
      .then((d: any) => { if (d?.phone && d.showPhoneOnOffline !== false) setFallbackPhone(d.phone); })
      .catch(() => {});
  }, [slug]);

  const submitRating = useCallback(async (stars: number) => {
    if (ratingBusy) return;
    setRatingBusy(true);
    try {
      await apiClient(`/orders/${id}/rating`, { method: 'POST', body: { rating: stars, feedback: ratingComment || undefined } });
      setOrder((o: any) => ({ ...o, rating: stars, feedback: ratingComment || null, canRate: false }));
      showToast(t('client.rating_thanks', 'Thanks for your feedback!'), 'success');
    } catch {
      showToast(t('client.rating_failed', 'Could not submit rating. Please try again.'), 'error');
    } finally {
      setRatingBusy(false);
    }
  }, [id, ratingComment, ratingBusy, showToast, t]);

  const fetchMessages = useCallback(async () => {
    try {
      const data = await apiClient<typeof MessagesResponse>(`/orders/${id}/messages`, { schema: MessagesResponse });
      if (data?.messages) setMessages(data.messages);
    } catch (err) {
      console.debug('[OrderStatusPage] fetch messages failed:', err);
    }
  }, [id]);

  const handleSendMessage = useCallback(async (presetKey: string, params?: Record<string, unknown>) => {
    try {
      const data = await apiClient<typeof MessageSendResponse>(`/orders/${id}/messages`, {
        method: 'POST',
        body: { preset_key: presetKey, params: params || {} },
        schema: MessageSendResponse,
      });
      if (data?.message) {
        setMessages(prev => [...prev, data.message]);
      }
    } catch (err) {
      console.warn('[OrderStatusPage] send message failed:', err);
    }
  }, [id]);

  const handleMarkRead = useCallback(async () => {
    try {
      await apiClient(`/orders/${id}/messages/read`, { method: 'POST' });
    } catch (err) {
      console.debug('[OrderStatusPage] mark read failed:', err);
    }
  }, [id]);

  const fetchOrder = async (allowExchange = true) => {
    try {
      const data = await apiClient<any>(`/customer/orders/${id}/status`);
      setOrder(data);
      if (data.courierPosition) {
        setCourierPos([data.courierPosition.lng, data.courierPosition.lat]);
        setHasCourierFix(true);
      }
      // HONEST range (range or nothing — never a single number). Null clears it.
      setEtaRange(data.etaRange ?? null);
      // Stored road route (served for reconnecting clients).
      if (Array.isArray(data.route?.polyline) && data.route.polyline.length >= 2) {
        setRoutePolyline(data.route.polyline);
      }
    } catch (err: any) {
      // 401 (no/expired token) or 403 (wrong role) → no valid customer session.
      if (err?.status === 401 || err?.status === 403) {
        // Tracking-link handoff: if the order URL carries a ?t= grant code, trade
        // it for a real customer JWT once, then retry. This is how a visitor who
        // opened the link on a fresh device (no stored token) gets a session.
        const code = typeof window !== 'undefined'
          ? new URLSearchParams(window.location.search).get('t')
          : null;
        if (allowExchange && code) {
          try {
            const res = await apiClient<any>('/customer/track/exchange', {
              method: 'POST',
              body: { code },
            });
            if (res?.token) {
              safeStorage.set('dos_access_token', res.token);
              // Strip ?t= so the secret never persists in history or leaks via Referer.
              window.history.replaceState({}, '', window.location.pathname + window.location.hash);
              return await fetchOrder(false); // retry once with the fresh token
            }
          } catch {
            // Exchange failed (expired/invalid grant) → fall through to the message.
          }
        }
        // No valid session and no usable grant. Show a clear "reload the menu"
        // message instead of bouncing to the owner login (the global apiClient
        // redirect is scoped to /admin) or fabricating a fake order.
        setError(t('order.auth_expired', 'Session expired. Please reload the menu and try again.'));
        return;
      }
      if (err?.status === 404) {
        setOrder({
          id,
          status: 'PENDING',
          createdAt: new Date().toISOString(),
          items: [],
          total: 0,
        });
      } else {
        setError('Failed to fetch order status');
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (id) { fetchOrder(); fetchMessages(); }
  }, [id, fetchMessages]);

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

      // Real road route pushed once at picked_up (G1/G2). Draw it + pace ETA.
      if (inner.type === 'order.route') {
        const p = inner.payload;
        if (Array.isArray(p?.polyline) && p.polyline.length >= 2) {
          setRoutePolyline(p.polyline);
        }
        return;
      }

      if (inner.type === 'order.courier_updated') {
        const p = inner.payload;
        if (p.position) {
          setCourierPos([p.position.lng, p.position.lat]);
          setHasCourierFix(true);
        }
        // NOTE: etaSeconds was removed server-side — the honest ETA is the range from
        // /status (refetched on status transitions below). The map keeps p.position.
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
        // ORDER-TRACKING: additively merge the just-stamped *_at so the stepper
        // lights up the new step live (statusAtField names the camelCase key).
        setOrder((prev: any) => {
          // Terminal lock: WS frames can arrive reordered across pub/sub instances. Once the
          // order is terminal (DELIVERED/REJECTED/CANCELLED), ignore a late non-terminal frame
          // that would visibly revert "Delivered!" back to "in delivery" (money-adjacent truth).
          const TERMINAL = new Set(['DELIVERED', 'REJECTED', 'CANCELLED']);
          if (prev?.status && TERMINAL.has(prev.status) && !TERMINAL.has(inner.status)) {
            return prev;
          }
          const next = { ...prev, status: inner.status };
          if (inner.statusAtField && inner.statusAt) {
            next[inner.statusAtField] = inner.statusAt;
          }
          return next;
        });
        // Refetch so the ETA range re-derives and visibly narrows as the order
        // advances (e.g. pre_assign → assigned on IN_DELIVERY). The 30s watchdog
        // is a backstop; this makes the narrowing immediate on a status change.
        fetchOrder();
      }

      if (inner.type === 'order.message' && inner.data) {
        setMessages(prev => {
          const exists = prev.some(m => m.id === inner.data.id);
          return exists ? prev : [...prev, inner.data];
        });
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
    // Cold open: adopt the current status silently — don't announce a "change" the user
    // didn't witness (phantom toast). Only real transitions after the page is open toast.
    const isFirstObservation = prevStatusRef.current === '';
    prevStatusRef.current = order.status;
    if (isFirstObservation) return;
    const label = t(STATUS_LABELS_KEYS[order.status] || '', order.status.replace(/_/g, ' '));
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

  // ── SEAM (COURIER agent owns the emit) ─────────────────────────────────────
  // The live courier pin below is fed by `courierPos`/`hasCourierFix`, which are
  // set from the existing `order.courier_updated` WS event (payload.position =
  // {lat,lng}) handled in the onMessage block above, and seeded by the REST
  // `courierPosition` field. This ORDER-TRACKING change does NOT emit courier
  // location — that is the courier-events worker's job
  // (apps/api/src/workers/courier-events.ts → 'order.courier_updated').
  // TODO(courier-agent): if the pin needs richer data (heading, accuracy, eta),
  // extend the courier-events emit + this handler — reuse this event, do not
  // invent a parallel courier-location channel.
  const couriers: CourierOnMap[] = useMemo(() => {
    // Don't render a courier marker until we have a REAL fix — otherwise it sits at the
    // hardcoded default (FOWS: a pin at the wrong city before the first ping lands).
    if (order?.courierName && hasCourierFix) {
      return [{
        id: 'c1',
        name: order.courierName,
        initials: (order.courierName || 'C').charAt(0).toUpperCase(),
        lngLat: courierPos,
        status: 'busy',
      }];
    }
    return [];
  }, [order?.courierName, courierPos, hasCourierFix]);

  // Live courier position as {lat,lng} for the tweened marker + local ETA.
  const courierLatLng = useMemo(
    () => (hasCourierFix ? { lat: courierPos[1], lng: courierPos[0] } : null),
    [hasCourierFix, courierPos],
  );
  const routeLine: LngLatLike[] | undefined = useMemo(
    () => routePolyline?.map((p) => [p.lng, p.lat] as LngLatLike),
    [routePolyline],
  );
  // NOTE: the headline ETA is now the HONEST server range (etaRange), never a single
  // local number — so the useDeliveryEta single-number hook is no longer used for the
  // headline. The map is fed by routeLine + courierLatLng (the live polyline + pin).

  const destPin: LngLatLike = order?.deliveryLat
    ? [order.deliveryLng || 19.817, order.deliveryLat || 41.331]
    : [19.817, 41.331];

  const isInDelivery = order?.status === 'IN_DELIVERY';
  const isPickup = order?.type === 'pickup';
  // HONEST headline ETA — ALWAYS a range, NEVER a single number, NEVER "0".
  // overdue → reassuring line instead of implying on-time. Null/terminal → no ETA.
  const isTerminalStatus = order?.status === 'DELIVERED' || order?.status === 'REJECTED' || order?.status === 'CANCELLED';
  const showEtaRange = !!etaRange && !isTerminalStatus;
  const displayEta = etaRange?.overdue
    ? t('order.eta_overdue', 'A little longer than expected — almost there')
    : etaRange
      ? t('order.eta_range', '{{low}}–{{high}} min', { low: etaRange.lowMin, high: etaRange.highMin })
      : '';

  if (loading) {
    // Skeleton matches the real layout: map → hero ETA → timeline → summary card.
    return (
      <div className="max-w-md mx-auto min-h-screen bg-[var(--brand-surface)] pb-10" aria-busy="true" aria-label={t('order.loading', 'Loading your order')}>
        <SkeletonBase className="h-64 w-full rounded-none" />
        <div className="p-4 space-y-6 -mt-4 relative z-10 bg-[var(--brand-surface)] rounded-t-[var(--brand-radius)]">
          <div className="flex flex-col items-center gap-2 pt-2">
            <SkeletonBase className="h-7 w-32" />
            <SkeletonBase className="h-4 w-40" />
          </div>
          <div className="flex items-center justify-between gap-2 px-2">
            {[0, 1, 2, 3, 4].map((i) => (
              <SkeletonBase key={i} className="h-9 w-9 rounded-full" />
            ))}
          </div>
          <div className="rounded-[var(--brand-radius)] p-4 space-y-3" style={{ boxShadow: 'var(--elev-1)', background: 'var(--brand-surface-raised)' }}>
            <SkeletonBase className="h-5 w-28" />
            <SkeletonBase className="h-4 w-full" />
            <SkeletonBase className="h-4 w-3/4" />
            <SkeletonBase className="h-4 w-1/2" />
          </div>
        </div>
      </div>
    );
  }

  if (error || !order) {
    // Never a dead-end: a session-expired / not-found tracking page always offers a way back to
    // the menu and a way to reach the restaurant (the fallback phone, un-gated from the WS banner).
    const backToMenu = slug ? `/s/${slug}` : '/';
    return (
      <div className="max-w-md mx-auto p-6">
        <EmptyState
          title={error ? t('order.unavailable_title', 'This link is no longer active') : t('order.not_found_title', 'Order not found')}
          description={error || t('order.not_found_desc', 'Order not found')}
          action={
            <div className="flex flex-col gap-3 w-full max-w-xs">
              <a href={backToMenu} data-testid="order-back-to-menu" className="w-full min-h-11 inline-flex items-center justify-center rounded-[var(--brand-radius-btn)] font-semibold transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2" style={{ background: 'var(--brand-primary)', color: 'var(--color-on-primary, var(--brand-bg))', boxShadow: 'var(--elev-1)' }}>
                {t('order.back_to_menu', 'Back to the menu')}
              </a>
              {fallbackPhone && (
                <a href={`tel:${fallbackPhone}`} data-testid="order-call-restaurant" className="w-full min-h-11 inline-flex items-center justify-center gap-2 rounded-[var(--brand-radius-btn)] border font-medium transition-[transform,background-color] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2" style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
                  <i className="ti ti-phone" aria-hidden="true" />{t('order.call_restaurant', 'Call the restaurant')}
                </a>
              )}
            </div>
          }
        />
      </div>
    );
  }

  const isDisconnected = wsStatus === 'disconnected' || wsStatus === 'reconnecting' || wsStatus === 'error';

  const statusAccent = STATUS_ACCENT[order.status] || 'var(--brand-primary)';
  const statusMsg = STATUS_MESSAGE_KEYS[order.status];
  const isTerminal = order.status === 'REJECTED' || order.status === 'CANCELLED' || order.status === 'DELIVERED';
  const isLive = order.status === 'IN_DELIVERY';
  // Shared soft-UI card surface (elev-1, no ghost-card: shadow xor heavy border).
  const cardStyle: React.CSSProperties = { boxShadow: 'var(--elev-1)', background: 'var(--brand-surface-raised)' };
  // Entrance: gentle staggered fade/rise; instant under reduced-motion.
  const enter = prefersReducedMotion
    ? { initial: { opacity: 1 }, animate: { opacity: 1 } }
    : {
        initial: { opacity: 0, y: 8 },
        animate: { opacity: 1, y: 0 },
        transition: { duration: duration.base, ease: ease.out },
      };

  return (
    <div className="max-w-md mx-auto min-h-screen bg-[var(--brand-surface)] pb-10" role="region" aria-live="polite" aria-label={t('order.status_updates', 'Order status updates')}>
      {/* WS Disconnect Banner — own the failure, give a human a way to reach the restaurant */}
      {isDisconnected && (
        <div className="sticky top-0 z-50 px-4 py-2 text-xs font-semibold flex flex-wrap items-center justify-center gap-x-3 gap-y-1.5" style={{ background: 'var(--color-warning)', color: '#fff' }} data-testid="offline-banner">
          <span className="inline-flex items-center gap-2">
            <i className="ti ti-wifi-off" aria-hidden="true" />
            {t('order.live_paused', 'Live updates paused. Refreshing automatically.')}
          </span>
          {fallbackPhone && (
            <a
              href={`tel:${fallbackPhone}`}
              data-testid="offline-call-restaurant"
              title={t('order.call_restaurant', 'Call the restaurant')}
              className="inline-flex items-center gap-1.5 rounded-full bg-white/20 hover:bg-white/30 active:bg-white/40 px-3 py-1.5 transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-offset-1"
              style={{ color: '#fff' }}
            >
              <i className="ti ti-phone" aria-hidden="true" />
              <span>{t('order.call_restaurant', 'Call the restaurant')}</span>
            </a>
          )}
        </div>
      )}

      {/* Live Courier Map — delivery only (pickup has no courier) */}
      {!isPickup && (
        <div className="h-64 relative w-full" title={t('tooltip.courier_location', 'Courier current location')}>
          <CourierLiveMap
            className="h-full w-full"
            couriers={courierLatLng ? [] : couriers}
            liveCourier={courierLatLng}
            routeLine={routeLine}
            destinationPin={destPin}
            center={hasCourierFix ? courierPos : destPin}
            zoom={14}
          />
          <div className="absolute top-4 left-4 bg-white/90 p-1.5 rounded-full shadow-md z-10" title={t('tooltip.ws_status', 'Connection status')}>
            <WSStatusDot status={wsStatus === 'disabled' ? 'disconnected' : wsStatus} />
          </div>
        </div>
      )}

      {/* Screen-reader status announcer — speaks every status transition with explicit
          localized text. The visual toast isn't reliably announced and the stepper is
          graphical; this dedicated role="status" region guarantees the change is spoken. */}
      <div className="sr-only" role="status" aria-live="polite" aria-atomic="true" data-testid="sr-status-announcer">
        {order?.status
          ? `${t('order.status_now', 'Order status:')} ${t(STATUS_LABELS_KEYS[order.status] || '', order.status.replace(/_/g, ' '))}`
          : ''}
      </div>

      {/* Screen-reader accessible courier status */}
      {order?.courierName && (
        <div data-dynamic className="sr-only" role="status" aria-live="polite">
          {t('order.sr_courier_delivering', '{{name}} is delivering your order.', { name: order.courierName })}
          {etaRange && !isTerminalStatus ? ` ${t('order.sr_courier_eta_range', 'Estimated {{low}} to {{high}} minutes away.', { low: etaRange.lowMin, high: etaRange.highMin })}` : ''}
          {order.deliveryAddress ? ` ${t('order.sr_courier_dest', 'Delivering to {{addr}}.', { addr: order.deliveryAddress })}` : ''}
        </div>
      )}

      <div className="p-4 space-y-6 -mt-4 relative z-10 bg-[var(--brand-surface)] rounded-t-[var(--brand-radius)]">

        <motion.div className="text-center" {...enter}>
          <h1 data-dynamic data-testid="order-eta-headline" className="text-2xl font-bold text-[var(--brand-text)] mb-1 break-words" style={{ fontFamily: 'var(--brand-font-heading)' }}>
            {isPickup
              ? (order.status === 'READY' ? t('order.ready_for_pickup', 'Ready for pickup')
                 : order.status === 'PICKED_UP' ? t('order.picked_up', 'Picked up')
                 : t('order.preparing', 'Preparing your order'))
              : showEtaRange
                ? displayEta
                : (statusMsg
                    ? t(statusMsg.key, statusMsg.fallback)
                    : t(STATUS_LABELS_KEYS[order.status] || '', order.status.replace(/_/g, ' ')))}
          </h1>
          {/* Subline: only claim "estimated arrival" when we actually show a range.
              pre_assign → caption that the estimate refines after restaurant confirms. */}
          {isPickup ? (
            <p className="text-sm text-[var(--brand-text-muted)]">{t('order.pickup_at_restaurant', 'Collect at the restaurant')}</p>
          ) : showEtaRange && !etaRange?.overdue ? (
            <p className="text-sm text-[var(--brand-text-muted)]">
              {etaRange?.phase === 'pre_assign'
                ? t('order.eta_refines', 'Refines once the restaurant confirms')
                : t('client.estimated_arrival', 'Estimated arrival')}
            </p>
          ) : null}
          {/* Status-aware reassuring line with the lifecycle accent + a gentle live pulse while in delivery.
              Only when the h1 ISN'T already the same statusMsg (delivery-no-ETA falls back to it) — else
              the identical line renders twice ("Your food is being prepared." stacked). */}
          {statusMsg && (showEtaRange || isPickup) && (
            <p
              data-testid="order-status-message"
              data-dynamic
              className="mt-2 inline-flex items-center gap-2 text-sm font-medium break-words"
              style={{ color: statusAccent }}
            >
              {isLive && (
                <span
                  aria-hidden="true"
                  className={`inline-block w-2 h-2 rounded-full shrink-0${prefersReducedMotion ? '' : ' animate-pulse'}`}
                  style={{ background: statusAccent }}
                />
              )}
              {t(statusMsg.key, statusMsg.fallback)}
            </p>
          )}
        </motion.div>

        <div data-testid="order-status-badge" data-status={order?.status} aria-live="polite" aria-atomic="true">
          {/* ORDER-TRACKING: real-machine stepper. type drives pickup vs delivery
              branch; the *At timestamps light up filled steps (status-only fallback). */}
          <OrderProgress
            status={order.status}
            type={order.type}
            confirmedAt={order.confirmedAt}
            preparingAt={order.preparingAt}
            readyAt={order.readyAt}
            inDeliveryAt={order.inDeliveryAt}
            deliveredAt={order.deliveredAt}
            pickedUpAt={order.pickedUpAt}
          />
        </div>

        {/* S2/S6/S7 seam — every terminal state gets an exit (never a dead-end) and a humane,
            non-accusing explanation (the customer is never blamed; they were never charged). */}
        {isTerminal && (
          <motion.div data-testid="order-terminal-exit" className="flex flex-col gap-3 px-1" {...enter}>
            {(order.status === 'REJECTED' || order.status === 'CANCELLED') && (
              <p className="text-sm text-center break-words" style={{ color: 'var(--brand-text)' }}>
                {order.status === 'REJECTED'
                  ? t('order.rejected_help', "The restaurant couldn't take this order — you haven't been charged. Try again or give them a call.")
                  : t('order.cancelled_help', "This order was cancelled — you haven't been charged. You can order again or call the restaurant.")}
              </p>
            )}
            <a href={slug ? `/s/${slug}` : '/'} data-testid="order-again"
              className="w-full min-h-11 inline-flex items-center justify-center rounded-[var(--brand-radius-btn)] font-semibold transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
              style={{ background: 'var(--brand-primary)', color: 'var(--color-on-primary, var(--brand-bg))', boxShadow: 'var(--elev-1)' }}>
              {t('order.order_again', 'Order again')}
            </a>
            {fallbackPhone && order.status !== 'DELIVERED' && (
              <a href={`tel:${fallbackPhone}`} data-testid="order-call-restaurant-terminal"
                className="w-full min-h-11 inline-flex items-center justify-center gap-2 rounded-[var(--brand-radius-btn)] border font-medium transition-[transform,background-color] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
                style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
                <i className="ti ti-phone" aria-hidden="true" />{t('order.call_restaurant', 'Call the restaurant')}
              </a>
            )}
          </motion.div>
        )}

        {/* CR-6: Share my location (visible during IN_DELIVERY) */}
        {isInDelivery && (
          <motion.div className="flex flex-col gap-3" {...enter}>
            {sharingLocation ? (
              <div className="flex items-center gap-3 rounded-[var(--brand-radius)] p-3" style={{ background: 'var(--status-delivered-light)', boxShadow: 'var(--elev-1)' }}>
                <span className={`w-2.5 h-2.5 rounded-full shrink-0${prefersReducedMotion ? '' : ' animate-pulse'}`} style={{ background: 'var(--color-success)' }} aria-hidden="true" />
                <div className="flex-1 min-w-0 text-sm">
                  <div className="font-semibold break-words" style={{ color: 'var(--color-success)' }}>
                    {t('client.sharing_location', 'Sharing your location')}
                  </div>
                  <div className="text-xs break-words text-[var(--brand-text-muted)]">
                    {t('client.sharing_location_note', 'Courier can see your live position')}
                  </div>
                </div>
                <button
                  onClick={stopSharing}
                  className="shrink-0 text-xs px-3 py-1.5 rounded-full border border-[var(--brand-border)] bg-[var(--brand-surface)] text-[var(--brand-text)] transition-[transform,background-color] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
                >
                  {t('client.stop_sharing', 'Stop')}
                </button>
              </div>
            ) : (
              <button
                onClick={startSharing}
                className="flex items-center justify-center gap-2 w-full min-h-11 py-3 rounded-[var(--brand-radius)] bg-[var(--brand-primary-light)] text-[var(--brand-text)] font-semibold text-sm border border-[var(--brand-primary)] transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
              >
                <i className="ti ti-map-pin" aria-hidden="true" />
                <span className="min-w-0 break-words">{t('client.share_location', 'Share my location with courier')}</span>
              </button>
            )}
            {/* CR-8: CourierContactBtn — call courier */}
            {order?.courier_phone && (
              <a
                href={`tel:${order.courier_phone}`}
                title={t('tooltip.call_courier', 'Call your courier')}
                className="flex items-center justify-center gap-2 w-full min-h-11 py-3 rounded-[var(--brand-radius)] bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] text-[var(--brand-text)] font-semibold text-sm transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
                style={{ boxShadow: 'var(--elev-1)' }}
              >
                <i className="ti ti-phone" aria-hidden="true" />
                <span className="min-w-0 break-words">{t('client.call_courier', 'Call courier')}</span>
              </a>
            )}
            {/* UX-2: message courier in their app (only within the active order) */}
            {order?.courierMessenger && messengerLink(order.courierMessenger.kind, order.courierMessenger.handle) && (
              <a
                href={messengerLink(order.courierMessenger.kind, order.courierMessenger.handle)!}
                target="_blank" rel="noopener noreferrer"
                title={t('client.message_courier', 'Message your courier')}
                data-testid="message-courier-btn"
                className="flex items-center justify-center gap-2 w-full min-h-11 py-3 rounded-[var(--brand-radius)] bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] text-[var(--brand-text)] font-semibold text-sm transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
                style={{ boxShadow: 'var(--elev-1)' }}
              >
                <i className="ti ti-message-circle" aria-hidden="true" />
                <span className="min-w-0 break-words">{t('client.message_courier', 'Message courier')}</span>
              </a>
            )}
          </motion.div>
        )}

        <motion.div className="rounded-[var(--brand-radius)] p-4" style={cardStyle} {...enter}>
          <h2 className="font-semibold mb-3 text-[var(--brand-text)]">{t('client.order_details', 'Order Details #{{id}}', { id: order.id.substring(0, 4) })}</h2>
          <div className="space-y-2 text-sm text-[var(--brand-text)]">
            {order.items?.map((item: any, i: number) => (
              <div key={i} className="flex justify-between gap-3">
                <span className="min-w-0 break-words">{item.quantity ?? 1}× {item.nameSnapshot ?? item.name}</span>
                {/* Show the LINE total (unit × qty), not the unit price — otherwise a 2× line
                    reads as "800 ALL" yet the order Total counts 1600, looking like broken math. */}
                <span className="shrink-0 tabular-nums"><PriceDisplay amount={(item.priceSnapshot ?? item.price ?? 0) * (item.quantity ?? 1)} /></span>
              </div>
            ))}
          </div>
          <div className="border-t border-[var(--brand-border)] mt-4 pt-4 flex justify-between gap-3 font-bold text-[var(--brand-text)]">
            <span className="min-w-0">{t('client.total', 'Total')}</span>
            <span className="shrink-0 tabular-nums"><PriceDisplay amount={order.total} /></span>
          </div>
          {order.tipAmount > 0 && (
            <div className="flex justify-between gap-3 text-sm mt-2" style={{ color: 'var(--brand-text-muted)' }} data-testid="order-tip">
              <span className="min-w-0 break-words">{t('client.tip_for_courier', 'Tip for courier (cash)')}</span>
              <span className="shrink-0 tabular-nums"><PriceDisplay amount={order.tipAmount} /></span>
            </div>
          )}
        </motion.div>

        {/* Rate your order — shown once DELIVERED; tap a star to submit */}
        {order.status === 'DELIVERED' && (
          <motion.div className="rounded-[var(--brand-radius)] p-4" style={cardStyle} data-testid="rating-block" {...enter}>
            <h2 className="font-semibold mb-3 text-[var(--brand-text)]">{t('client.rate_order', 'Rate your order')}</h2>
            {order.rating ? (
              <div>
                <div className="flex gap-1 text-2xl" aria-label={`${order.rating}/5`} data-testid="rating-submitted">
                  {[1, 2, 3, 4, 5].map(s => (
                    <i key={s} className={`ti ti-star-filled ${s <= order.rating ? '' : 'opacity-25'}`} style={{ color: 'var(--brand-primary)' }} aria-hidden="true" />
                  ))}
                </div>
                {order.feedback && <p className="text-sm mt-2" style={{ color: 'var(--brand-text-muted)' }}>{order.feedback}</p>}
                <p className="text-xs mt-2" style={{ color: 'var(--brand-text-muted)' }}>{t('client.rating_saved', 'Thanks — your feedback was saved.')}</p>
              </div>
            ) : (
              <div>
                <div className="flex gap-2 text-3xl mb-3">
                  {[1, 2, 3, 4, 5].map(s => (
                    <button key={s} type="button" disabled={ratingBusy} aria-label={`${s} ${t('client.stars', 'stars')}`}
                      data-testid={`rate-star-${s}`} onClick={() => submitRating(s)}
                      className="rounded-[var(--brand-radius-sm)] transition-transform duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:scale-110 active:scale-90 disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2" style={{ color: 'var(--brand-primary)' }}>
                      <i className="ti ti-star" aria-hidden="true" />
                    </button>
                  ))}
                </div>
                <textarea value={ratingComment} onChange={e => setRatingComment(e.target.value)} maxLength={1000} rows={2}
                  placeholder={t('client.feedback_placeholder', 'Add a comment (optional)')}
                  className="w-full text-sm rounded-[var(--brand-radius)] p-2 bg-[var(--brand-surface)] border border-[var(--brand-border)] text-[var(--brand-text)] transition-shadow duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-1" />
              </div>
            )}
            {/* Google review invite — shown to everyone (anti-gating), no incentive, no pre-fill. */}
            {googlePlaceId && (
              <div className="mt-4 pt-3 border-t" style={{ borderColor: 'var(--brand-border)' }}>
                <a href={`https://search.google.com/local/writereview?placeid=${encodeURIComponent(googlePlaceId)}`}
                  target="_blank" rel="noopener noreferrer" data-testid="google-review-link"
                  className="inline-flex items-center gap-2 min-h-11 text-sm font-medium rounded-[var(--brand-radius-sm)] transition-opacity duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:opacity-80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2" style={{ color: 'var(--brand-primary)' }}>
                  <i className="ti ti-brand-google" aria-hidden="true" /> {t('client.leave_google_review', 'Leave a review on Google')}
                </a>
              </div>
            )}
          </motion.div>
        )}

        {/* Nutrition snapshot */}
        {order.kcal_total != null && (
          <motion.div className="rounded-[var(--brand-radius)] p-4" style={cardStyle} {...enter}>
            <div className="flex items-center gap-2 mb-3">
              <span className="text-sm font-semibold text-[var(--brand-text)]">≈ {t('client.nutrition', 'Nutrition')}</span>
              <span className="text-step-2xs px-1.5 py-0.5 rounded-[var(--brand-radius-sm)]" style={{ background: 'var(--brand-surface)', color: 'var(--brand-text-muted)' }}>{t('client.nutrition_estimate', 'estimate only')}</span>
            </div>
            <div className="grid grid-cols-4 gap-2 text-center">
              {[
                { label: t('client.nutrition_calories', 'Calories'), value: order.kcal_total, unit: 'kcal' },
                { label: t('client.nutrition_protein', 'Protein'), value: order.protein_mg_total, unit: 'g' },
                { label: t('client.nutrition_fat', 'Fat'), value: order.fat_mg_total, unit: 'g' },
                { label: t('client.nutrition_carbs', 'Carbs'), value: order.carb_mg_total, unit: 'g' },
              ].map(n => (
                <div key={n.label} className="p-2 rounded-[var(--brand-radius-sm)] min-w-0" style={{ background: 'var(--brand-surface)' }}>
                  <div className="text-lg font-bold tabular-nums" style={{ color: 'var(--brand-primary)' }}>{n.value || '—'}</div>
                  <div className="text-step-2xs truncate" style={{ color: 'var(--brand-text-muted)' }}>{n.label}</div>
                  <div className="text-step-2xs" style={{ color: 'var(--brand-text-muted)' }}>{n.unit}</div>
                </div>
              ))}
            </div>
          </motion.div>
        )}

        {/* CR-8: Message Thread */}
        {/* eslint-disable jsx-a11y/aria-role */}
        <MessageThread
          orderId={id!}
          role="customer"
          currentStatus={order.status}
          messages={messages}
          onSend={handleSendMessage}
          onMarkRead={handleMarkRead}
        />
        {/* eslint-enable jsx-a11y/aria-role */}

      </div>
    </div>
  );
}
 