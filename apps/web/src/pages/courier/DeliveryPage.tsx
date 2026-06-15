import React, { useEffect, useState, useMemo, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { SwipeToComplete, EmptyState, WSStatusDot, SkeletonBase, CourierLiveMap, MessageThread, useI18n, useGeolocation, AnimatedCheck, LiveDot, PriceDisplay } from '@deliveryos/ui';
import type { CourierTask, CourierOnMap, LngLatLike } from '@deliveryos/ui';
import { apiClient, useWebSocket } from '../../lib/index.js';
import { z } from 'zod';

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
  const [loading, setLoading] = useState(true);
  const [courierPos, setCourierPos] = useState<LngLatLike>(TIRANA_CENTER);
  const [showCelebration, setShowCelebration] = useState(false);
  const [clientLocation, setClientLocation] = useState<LngLatLike | null>(null);
  const { t } = useI18n();
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
      const data = await apiClient<typeof CourierTaskDetail>(`/courier/orders/${id}`, { schema: CourierTaskDetail });
      setTask(data);
    } catch (err: any) {
      if (err.status === 404) {
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

  const { status: wsStatus, sendMessage } = useWebSocket({
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
    }
  });

  useEffect(() => {
    if (position && wsStatus === 'connected') {
      sendMessage({
        type: 'location_update',
        payload: {
          lat: position.lat,
          lng: position.lng,
          heading: position.heading,
          speed: position.speed,
          timestamp: Date.now()
        }
      });
    }
  }, [position, wsStatus, sendMessage]);

  const handlePickup = async () => {
    setPickupLoading(true);
    try {
      await apiClient(`/orders/${id}/pickup`, { method: 'PATCH' });
      setPickedUp(true);
    } catch (err) {
      console.warn('[DeliveryPage] pickup failed:', err);
    } finally {
      setPickupLoading(false);
    }
  };

  const handleComplete = async () => {
    setShowCelebration(true);
    try {
      const body: Record<string, unknown> = {};
      if (task?.cashPayWith && cashCollected != null) {
        body.cash_collected = cashCollected;
      }
      await apiClient(`/courier/assignments/${id}/delivered`, {
        method: 'POST',
        body,
      });
    } catch (e) {
      console.debug('[DeliveryPage] delivery status update failed', e);
    }
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

  if (loading) return <div className="p-4"><SkeletonBase className="h-64 w-full" /></div>;
  if (!task) return <EmptyState title={t('courier.not_found', 'Not found')} description={t('courier.task_not_found', 'Delivery task not found.')} />;

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
              initial={{ scale: 0.5, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.8, opacity: 0 }}
              transition={{ type: 'spring', stiffness: 260, damping: 24 }}
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
          <div className="absolute top-4 left-1/2 -translate-x-1/2 bg-[var(--color-danger)] text-[var(--color-on-danger)] px-4 py-2 rounded-lg text-sm text-center max-w-xs shadow-lg z-10">
            {geoError.message}
          </div>
        )}

        <motion.button
          onClick={() => navigate('/courier')}
          whileTap={{ scale: 0.97 }}
          className="absolute top-4 left-4 w-10 h-10 bg-[var(--brand-surface)] text-[var(--brand-text)] rounded-full shadow-lg flex items-center justify-center text-xl font-bold z-10"
        >
          &times;
        </motion.button>

        <div className="absolute top-4 right-4 bg-white/90 p-1.5 rounded-full shadow-md flex gap-2 items-center px-3 z-10">
          <div className="flex items-center gap-1.5">
            <LiveDot size={6} pulse={wsStatus !== 'disabled' && wsStatus !== 'disconnected'} color="var(--color-success)" />
            <WSStatusDot status={wsStatus === 'disabled' ? 'disconnected' : wsStatus} />
          </div>
          {position && (
            <div className="flex items-center gap-1">
              <LiveDot size={6} pulse={true} color="var(--color-info)" />
              <span className="text-[10px] font-medium text-[var(--brand-text-muted)]">GPS</span>
            </div>
          )}
        </div>
      </div>

      <div className="bg-[var(--brand-surface)] rounded-t-3xl shadow-elevation-3 -mt-6 relative z-10 p-6 flex flex-col gap-6">
        
        <div className="w-12 h-1.5 bg-[var(--brand-border)] rounded-full mx-auto -mt-2" />

        <div className="flex justify-between items-center">
          <div>
            <h2 className="text-xl font-bold text-[var(--brand-text)]">{t('courier.dropoff', 'Drop-off')}</h2>
            <div className="text-[var(--brand-text-muted)]">{task.customer.address}</div>
          </div>
          <div className="text-right">
            <div className="text-2xl font-black text-[var(--brand-primary)]">{task.eta}</div>
            <div className="text-sm text-[var(--brand-text-muted)]">to destination</div>
          </div>
        </div>

        {task.customer.instructions && (
          <div className="bg-[var(--status-pending-light)] border border-[var(--status-pending-border)] text-[var(--status-pending)] p-3 rounded-[var(--brand-radius-sm)] text-sm font-medium">
            Note: {task.customer.instructions}
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
          <a href={`tel:${task.customer.phone}`} className="flex-1 bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] py-3 rounded-full flex items-center justify-center font-bold gap-2">
            {t('courier.call_button', 'Call')}
          </a>
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

        {!pickedUp ? (
          <motion.button
            onClick={handlePickup}
            disabled={pickupLoading}
            whileTap={{ scale: 0.97 }}
            className="w-full h-14 bg-[var(--brand-primary)] text-white font-bold text-base rounded-full shadow-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
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
                  className="w-full h-12 px-4 outline-none text-base font-bold border rounded-xl transition-colors"
                  style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}
                />
              </div>
            )}
            <SwipeToComplete onComplete={handleComplete} label={t('courier.slide_to_deliver', 'Slide to Deliver')} />
          </>
        )}
      </div>
    </div>
  );
}
