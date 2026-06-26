import React, { useEffect, useState, useMemo, useCallback, useRef } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useReducedMotion } from 'framer-motion';
import { SwipeToComplete, EmptyState, WSStatusDot, SkeletonBase, CourierLiveMap, MessageThread, useI18n, useGeolocation, AnimatedCheck, LiveDot, PriceDisplay, Button, ease } from '@deliveryos/ui';
import type { CourierTask, CourierOnMap, LngLatLike } from '@deliveryos/ui';
import { apiClient, useWebSocket } from '../../lib/index.js';
import { messengerLink } from '../../lib/messenger.js';
import { z } from 'zod';

// P0-1: courier GPS heartbeat interval. Mirrors the server ping rate-limit (1/10s);
// 12s leaves headroom. Time-based so a stationary courier keeps re-posting.
const COURIER_GPS_POST_INTERVAL_MS = 12_000;

const MessagesResponse = z.object({
  messages: z.array(z.any()),
}).passthrough();

const MessageSendResponse = z.object({
  message: z.any(),
}).passthrough();

const CourierTaskDetail = z.custom<CourierTask>();

const TIRANA_CENTER: LngLatLike = [19.817, 41.331];
const MOCK_RESTAURANT: LngLatLike = [19.812, 41.328];
const MOCK_CUSTOMER: LngLatLike = [19.825, 41.337];

export function DeliveryPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [task, setTask] = useState<CourierTask | null>(null);
  const [photoOpen, setPhotoOpen] = useState(false); // UX-3 entry-photo fullscreen
  const [loading, setLoading] = useState(true);
  const [courierPos, setCourierPos] = useState<LngLatLike>(TIRANA_CENTER);
  const [showCelebration, setShowCelebration] = useState(false);
  const [deliverError, setDeliverError] = useState<string | null>(null);
  const [orderClosed, setOrderClosed] = useState<string | null>(null); // CANCELLED/REJECTED while en route
  const [clientLocation, setClientLocation] = useState<LngLatLike | null>(null);
  const { t } = useI18n();
  const reduceMotion = useReducedMotion();
  const [messages, setMessages] = useState<any[]>([]);
  const [pickedUp, setPickedUp] = useState(false);
  const [pickupLoading, setPickupLoading] = useState(false);
  const [cashCollected, setCashCollected] = useState<number | null>(null);

  const fetchMessages = useCallback(async () => {
    try {
      const data = await apiClient<typeof MessagesResponse>(`/orders/${id}/messages`, { schema: MessagesResponse });
      if (data?.messages) setMessages(data.messages);
    } catch (err) {
      console.debug('[DeliveryPage] fetch messages failed:', err);
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
      console.warn('[DeliveryPage] send message failed:', err);
    }
  }, [id]);

  const handleMarkRead = useCallback(async () => {
    try {
      await apiClient(`/orders/${id}/messages/read`, { method: 'POST' });
    } catch (err) {
      console.debug('[DeliveryPage] mark read failed:', err);
    }
  }, [id]);

  const { position, error: geoError } = useGeolocation({
    enableHighAccuracy: true,
    timeout: 5000,
    maximumAge: 0
  });

  useEffect(() => {
    if (position) {
      setCourierPos([position.lng, position.lat]);
    }
  }, [position]);

  const fetchTask = async () => {
    try {
      const data = await apiClient<typeof CourierTaskDetail>(`/courier/assignments/${id}`, { schema: CourierTaskDetail });
      setTask(data);
    } catch (err: any) {
      // DEV-ONLY mock so the courier UI can be previewed without a live assignment. In prod a
      // 404 (expired/reassigned) must NOT fabricate a fake drop-off the courier could act on —
      // it falls through to the real "task not found" soft state below.
      if (err.status === 404 && import.meta.env.DEV) {
        setTask({
          id: id!,
          status: 'IN_DELIVERY',
          restaurant: { name: 'Burger King', address: 'Blloku, Tirana', lat: 41.328, lng: 19.812 },
          customer: { address: 'Rruga e Elbasanit 12', phone: '+355 69 123 4567', instructions: 'Call when near', lat: 41.337, lng: 19.825 },
          total: 120000,
          cashPayWith: 150000,
          eta: '10 min'
        });
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTask();
    fetchMessages();
  }, [id, fetchMessages]);

  const { status: wsStatus } = useWebSocket({
    room: `order:${id}`,
    onMessage: (msg: any) => {
      if (msg.type === 'client_location' && msg.payload) {
        const { lat, lng } = msg.payload;
        if (typeof lat === 'number' && typeof lng === 'number') {
          setClientLocation([lng, lat]);
        }
      }
      if (msg.type === 'client_location_stop') {
        setClientLocation(null);
      }
      if (msg.data?.type === 'order.message' && msg.data?.data) {
        setMessages(prev => {
          const exists = prev.some(m => m.id === msg.data.data.id);
          return exists ? prev : [...prev, msg.data.data];
        });
      }
      // S2/S6 seam: the courier must learn if the restaurant cancels/rejects mid-delivery,
      // otherwise they drive to a dead order. A soft banner (not a wall — the human is never
      // blocked), reconciled from the same WS the client uses.
      if (msg.data?.type === 'order.status' && msg.data?.status) {
        const s = String(msg.data.status).toUpperCase();
        if (s === 'CANCELLED' || s === 'REJECTED') setOrderClosed(s);
      }
    }
  });

  // Push the courier's real GPS to the server via REST /shifts/ping. This drives the
  // live map and persists the delivery breadcrumb. P0-1: a TIME-BASED heartbeat (every
  // COURIER_GPS_POST_INTERVAL_MS), NOT position-driven — a stationary courier (e.g.
  // waiting at pickup, exactly when the assignment is still pre-accept) must keep
  // re-posting so a server withhold/403 is retried and tracking resumes within one
  // interval of the courier accepting. The SERVER is the hard gate: it stores a row
  // only on an active delivery and returns { gps_stored:false } otherwise. This page is
  // only mounted during a delivery, so the timer is naturally delivery-scoped.
  const positionRef = useRef(position);
  positionRef.current = position;
  useEffect(() => {
    const post = () => {
      const p = positionRef.current;
      if (!p) return;
      apiClient('/courier/shifts/ping', {
        method: 'POST',
        body: {
          lat: p.lat,
          lng: p.lng,
          ...(Number.isFinite(p.accuracy) ? { accuracy_meters: Math.round(p.accuracy) } : {}),
        },
      }).catch((err: any) => {
        console.debug('[DeliveryPage] shift ping skipped:', err?.status || err?.message);
      });
    };
    post();
    const timer = setInterval(post, COURIER_GPS_POST_INTERVAL_MS);
    return () => clearInterval(timer);
  }, []);

  const handlePickup = async () => {
    setPickupLoading(true);
    try {
      await apiClient(`/courier/assignments/${id}/picked-up`, { method: 'POST' });
      setPickedUp(true);
    } catch (err) {
      console.warn('[DeliveryPage] pickup failed:', err);
    } finally {
      setPickupLoading(false);
    }
  };

  const handleComplete = async () => {
    // INVARIANT: never fake success. The celebration + navigate happen ONLY after the server
    // confirms the delivery (optimistic UI must always reconcile to server truth). The human is
    // never blocked: a transient failure surfaces a retry-able error and resets the swipe; a
    // terminal order (already cancelled/delivered) gets a soft "closed" message, not a fake win.
    setDeliverError(null);
    const isCash = Boolean(task?.cashPayWith);
    const body: Record<string, unknown> = {
      cash_collected: isCash,
      ...(isCash && task?.total != null ? { cash_amount: task.total } : {}),
    };
    try {
      await apiClient(`/courier/assignments/${id}/delivered`, { method: 'POST', body });
    } catch (e: any) {
      const status = e?.status;
      if (status === 409 || status === 422) {
        // Reconcile to server: the order is no longer deliverable (cancelled / already closed).
        setOrderClosed((prev) => prev ?? 'CLOSED');
        setDeliverError(t('courier.delivery_already_closed', 'This order was already closed. Returning to your tasks.'));
        setTimeout(() => navigate('/courier'), 1800);
        return;
      }
      setDeliverError(t('courier.delivery_failed_retry', 'Could not confirm delivery — check your connection and slide again.'));
      throw e; // re-throw so SwipeToComplete resets and the courier can retry (not blocked, not faked)
    }
    setShowCelebration(true);
    setTimeout(() => navigate('/courier'), 1500);
  };

  const couriers: CourierOnMap[] = useMemo(() => [{
    id: 'me',
    name: 'You',
    initials: 'ME',
    lngLat: courierPos,
    status: 'busy',
  }], [courierPos]);

  const destPin: LngLatLike = task
    ? [task.customer.lng || MOCK_CUSTOMER[0], task.customer.lat || MOCK_CUSTOMER[1]]
    : MOCK_CUSTOMER;

  const routeLine: LngLatLike[] = [
    courierPos,
    [task?.restaurant?.lng || MOCK_RESTAURANT[0], task?.restaurant?.lat || MOCK_RESTAURANT[1]],
    destPin,
  ];

  if (loading) return (
    <div className="flex flex-col h-screen bg-[var(--brand-surface)]">
      <SkeletonBase className="flex-1 w-full rounded-none" />
      <div className="bg-[var(--brand-surface)] rounded-t-[var(--brand-radius)] -mt-6 relative z-10 p-6 flex flex-col gap-4" style={{ boxShadow: 'var(--elevation-3)' }}>
        <div className="w-12 h-1.5 bg-[var(--brand-border)] rounded-full mx-auto -mt-2" />
        <div className="flex justify-between items-start gap-4">
          <div className="flex flex-col gap-2 min-w-0 flex-1">
            <SkeletonBase className="h-6 w-28" />
            <SkeletonBase className="h-4 w-40" />
          </div>
          <SkeletonBase className="h-8 w-16" />
        </div>
        <div className="flex gap-4">
          <SkeletonBase className="h-12 flex-1 rounded-full" />
          <SkeletonBase className="h-12 flex-1 rounded-full" />
        </div>
        <SkeletonBase className="h-14 w-full rounded-full" />
      </div>
    </div>
  );
  // The delivery layout hides the bottom tab bar, so a bare EmptyState strands the
  // courier with no way back. Give them an explicit route home.
  // The delivery layout renders full-screen with no header/centered shell. When there is no
  // task we fall back to the standard centered courier shell (header + max-w-md + safe-area top)
  // so the not-found state reads like every other courier page instead of a stranded full-bleed card.
  if (!task) return (
    <div className="min-h-screen bg-[var(--brand-bg)] text-[var(--brand-text)]">
      <div className="flex items-center px-4 h-14 bg-[var(--brand-surface)]/95 backdrop-blur-sm border-b border-[var(--brand-border)]" style={{ paddingTop: 'env(safe-area-inset-top)' }}>
        <div className="w-full max-w-md mx-auto">
          <span className="text-sm font-semibold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('courier.dropoff', 'Drop-off')}</span>
        </div>
      </div>
      <div className="w-full max-w-md mx-auto p-5">
        <EmptyState
          fullPage
          icon={<i className="ti ti-map-off" aria-hidden="true" />}
          title={t('courier.not_found', 'Not found')}
          description={t('courier.task_not_found', 'Delivery task not found.')}
          action={
            <Button variant="primary" onClick={() => navigate('/courier')}>
              {t('courier.back_to_tasks', 'Back to tasks')}
            </Button>
          }
        />
      </div>
    </div>
  );

  return (
    <div className="flex flex-col h-screen bg-[var(--brand-surface)] text-[var(--brand-text)] relative">
      <AnimatePresence>
        {showCelebration && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-[999] flex items-center justify-center pointer-events-none"
            style={{ background: 'color-mix(in srgb, var(--brand-bg) 60%, transparent)' }}
          >
            <motion.div
              initial={reduceMotion ? { opacity: 0 } : { scale: 0.5, opacity: 0 }}
              animate={reduceMotion ? { opacity: 1 } : { scale: 1, opacity: 1 }}
              exit={reduceMotion ? { opacity: 0 } : { scale: 0.8, opacity: 0 }}
              transition={reduceMotion ? { duration: 0.2, ease: ease.out } : { type: 'spring', stiffness: 260, damping: 24 }}
              className="flex flex-col items-center gap-3"
            >
              <AnimatedCheck size={80} strokeWidth={4} />
              <span className="text-2xl font-bold text-white">{t('courier.delivered', 'Delivered!')}</span>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
      
      <div className="flex-1 relative">
        <CourierLiveMap
          className="h-full w-full"
          couriers={couriers}
          destinationPin={destPin}
          clientLocation={clientLocation || undefined}
          routeLine={routeLine}
          center={courierPos}
          zoom={14}
        />

        {geoError && (
          <div role="status" aria-live="polite" className="absolute left-1/2 -translate-x-1/2 bg-[var(--color-danger-strong)] text-[var(--color-on-danger)] px-4 py-2 rounded-[var(--brand-radius-sm)] text-sm text-center max-w-[18rem] z-10" style={{ top: 'max(1rem, env(safe-area-inset-top))', boxShadow: 'var(--elev-2)' }}>
            {t('courier.gps_unavailable', 'Location unavailable — turn on GPS to track the route.')}
          </div>
        )}

        <motion.button
          onClick={() => navigate('/courier')}
          whileTap={reduceMotion ? undefined : { scale: 0.97 }}
          aria-label={t('common.close', 'Close')}
          style={{ top: 'max(1rem, env(safe-area-inset-top))', boxShadow: 'var(--elev-2)' }}
          className="absolute left-4 w-11 h-11 bg-[var(--brand-surface)] text-[var(--brand-text)] rounded-full flex items-center justify-center text-xl font-bold z-10 transition-[transform,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
        >
          &times;
        </motion.button>

        <div className="absolute right-4 bg-[var(--brand-surface)]/95 backdrop-blur-sm p-1.5 rounded-full flex gap-2 items-center px-3 z-10" style={{ top: 'max(1rem, env(safe-area-inset-top))', boxShadow: 'var(--elev-1)' }}>
          <div className="flex items-center gap-1.5">
            <LiveDot size={6} pulse={wsStatus !== 'disabled' && wsStatus !== 'disconnected'} color="var(--color-success)" />
            <WSStatusDot status={wsStatus === 'disabled' ? 'disconnected' : wsStatus} />
          </div>
          {position && (
            <div className="flex items-center gap-1">
              <LiveDot size={6} pulse={true} color="var(--color-info)" />
              <span className="text-step-2xs font-medium text-[var(--brand-text-muted)]">GPS</span>
            </div>
          )}
        </div>
      </div>

      <div
        className="bg-[var(--brand-surface)] rounded-t-[var(--brand-radius)] -mt-6 relative z-10 p-6 flex flex-col gap-6 overflow-y-auto"
        style={{ boxShadow: 'var(--elevation-3)', paddingBottom: 'max(1.5rem, env(safe-area-inset-bottom))' }}
      >

        <div className="w-12 h-1.5 bg-[var(--brand-border)] rounded-full mx-auto -mt-2" />

        <div className="flex justify-between items-start gap-4">
          <div className="min-w-0">
            <h2 className="text-xl font-bold text-[var(--brand-text)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('courier.dropoff', 'Drop-off')}</h2>
            <div className="text-[var(--brand-text)] break-words">{task.customer.address}</div>
          </div>
          <div className="text-right shrink-0">
            <div data-dynamic className="text-2xl font-black text-[var(--brand-primary)] tabular-nums">{task.eta}</div>
            <div className="text-sm text-[var(--brand-text-muted)]">{t('courier.to_destination', 'to destination')}</div>
          </div>
        </div>

        {task.customer.instructions && (
          <div className="bg-[var(--status-pending-light)] border border-[var(--status-pending-border)] text-[var(--status-pending)] p-3 rounded-[var(--brand-radius-sm)] text-sm font-medium break-words">
            <span className="font-bold">{t('courier.note_label', 'Note')}:</span> {task.customer.instructions}
          </div>
        )}

        {/* UX-3: entry-anchor photo — show the entrance, tap to enlarge */}
        {(task.customer as any).entryPhotoUrl && (
          <button type="button" onClick={() => setPhotoOpen(true)} data-testid="entry-photo-thumb"
            className="block w-full text-left rounded-[var(--brand-radius-sm)] overflow-hidden border" style={{ borderColor: 'var(--brand-border)' }}>
            <img src={(task.customer as any).entryPhotoUrl} alt={t('courier.entry_photo', 'Entrance')} className="w-full h-28 object-cover" />
            <div className="px-3 py-1.5 text-xs font-medium" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-text-muted)' }}>
              <i className="ti ti-photo" aria-hidden="true" /> {t('courier.entry_photo_hint', 'Entrance — tap to enlarge')}
            </div>
          </button>
        )}
        {photoOpen && (task.customer as any).entryPhotoUrl && (
          <button type="button" aria-label={t('common.close', 'Close')} onClick={() => setPhotoOpen(false)} data-testid="entry-photo-modal"
            className="fixed inset-0 z-50 bg-black/90 flex items-center justify-center p-4">
            <img src={(task.customer as any).entryPhotoUrl} alt={t('courier.entry_photo', 'Entrance')} className="max-h-full max-w-full object-contain rounded" />
          </button>
        )}

        {/* UX-4: tip — informative, the courier collects it in cash */}
        {(task as any).tipAmount > 0 && (
          <div data-testid="task-tip" className="bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] rounded-[var(--brand-radius-sm)] p-3 flex justify-between items-center text-sm">
            <span className="text-[var(--brand-text-muted)]">{t('courier.tip', 'Tip (collect in cash)')}</span>
            <span className="font-bold text-[var(--brand-primary)]"><PriceDisplay amount={(task as any).tipAmount} /></span>
          </div>
        )}

        {task.cashPayWith && (
          <div data-testid="task-cash-amount" className="bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] rounded-[var(--brand-radius-sm)] p-4 space-y-2">
            <div className="text-xs text-[var(--brand-text-muted)] uppercase font-bold tracking-wider">
              {t('checkout.payment_method', 'Payment')} — Cash
            </div>
            <div className="flex justify-between text-sm">
              <span>{t('checkout.total', 'Total')}</span>
              <span className="font-semibold"><PriceDisplay amount={task.total} /></span>
            </div>
            <div className="flex justify-between text-sm">
              <span>{t('checkout.cash_amount', 'Customer pays')}</span>
              <span className="font-semibold"><PriceDisplay amount={task.cashPayWith} /></span>
            </div>
            <div className="flex justify-between text-sm border-t border-[var(--brand-border)] pt-2">
              <span className="text-[var(--color-success)] font-bold">{t('checkout.change', 'Change')}</span>
              <span className="text-[var(--color-success)] font-bold">
                <PriceDisplay amount={task.cashPayWith - task.total} />
              </span>
            </div>
          </div>
        )}

        <div className="flex gap-4">
          <a href={`tel:${task.customer.phone}`}
            className="flex-1 min-h-[44px] bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] py-3 rounded-full flex items-center justify-center font-bold gap-2 text-[var(--brand-text)] transition-[transform,box-shadow,background-color] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 hover:hover:shadow-[var(--elev-2)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]">
            <i className="ti ti-phone" aria-hidden="true" /> {t('courier.call_button', 'Call')}
          </a>
          {/* UX-2: message the customer in their app, if they shared a messenger */}
          {messengerLink((task.customer as any).messengerKind, (task.customer as any).messengerHandle) && (
            <a href={messengerLink((task.customer as any).messengerKind, (task.customer as any).messengerHandle)!}
              target="_blank" rel="noopener noreferrer" data-testid="message-customer-btn"
              className="flex-1 min-h-[44px] bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] py-3 rounded-full flex items-center justify-center font-bold gap-2 text-[var(--brand-text)] transition-[transform,box-shadow,background-color] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 hover:hover:shadow-[var(--elev-2)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]">
              <i className="ti ti-message-circle" aria-hidden="true" /> {t('courier.message_button', 'Message')}
            </a>
          )}
        </div>

        {/* eslint-disable jsx-a11y/aria-role */}
        <MessageThread
          orderId={id!}
          role="courier"
          currentStatus={task.status}
          messages={messages}
          onSend={handleSendMessage}
          onMarkRead={handleMarkRead}
        />
        {/* eslint-enable jsx-a11y/aria-role */}

        {/* Cancellation notice shows REGARDLESS of pickup state — a cancel BEFORE pickup is the
            case worth surfacing (saves a wasted trip). Soft, not a wall; the human is never blocked. */}
        {orderClosed && (
          <div role="status" aria-live="polite" data-testid="courier-order-closed" className="rounded-[var(--brand-radius)] px-3 py-2 text-sm text-center font-medium" style={{ background: 'var(--status-cancelled-light)', border: '1px solid var(--status-cancelled-border)', color: 'var(--brand-text)' }}>
            {t('courier.order_closed_banner', 'The restaurant closed this order. You can stop — no delivery needed.')}
          </div>
        )}
        {!pickedUp ? (
          <motion.button
            onClick={handlePickup}
            disabled={pickupLoading}
            whileTap={reduceMotion ? undefined : { scale: 0.97 }}
            className="w-full h-14 bg-[var(--brand-primary)] text-[var(--brand-on-primary)] font-bold text-base rounded-full shadow-[var(--elev-3)] transition-[transform,box-shadow,opacity] duration-[var(--motion-fast)] ease-[var(--ease-soft)] hover:hover:-translate-y-0.5 hover:hover:shadow-[var(--elev-4)] active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]"
          >
            {pickupLoading ? (
              <span className="inline-flex items-center gap-2">
                <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24" fill="none">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                </svg>
                {t('common.loading', 'Loading...')}
              </span>
            ) : t('courier.mark_picked_up', 'Mark as Picked Up')}
          </motion.button>
        ) : (
          <>
            {task?.cashPayWith && (
              <div className="space-y-2">
                <label className="text-sm font-bold block" style={{ color: 'var(--brand-text)' }}>
                  {t('checkout.cash_amount', 'How much did the customer pay?')}
                </label>
                <input
                  type="number"
                  inputMode="decimal"
                  value={cashCollected ?? task.cashPayWith}
                  onChange={e => setCashCollected(e.target.value ? parseFloat(e.target.value) : null)}
                  min={0}
                  className="w-full h-12 px-4 outline-none text-base font-bold border rounded-[var(--brand-radius)] transition-[border-color,box-shadow] duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]"
                  style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
                />
              </div>
            )}
            {deliverError && (
              <div role="alert" aria-live="assertive" data-testid="courier-deliver-error" className="rounded-[var(--brand-radius)] px-3 py-2 text-sm text-center font-medium" style={{ background: 'var(--status-cancelled-light)', border: '1px solid var(--status-cancelled-border)', color: 'var(--color-danger)' }}>
                {deliverError}
              </div>
            )}
            <div data-testid="courier-advance-action">
              <SwipeToComplete onComplete={handleComplete} label={t('courier.slide_to_deliver', 'Slide to Deliver')} />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
