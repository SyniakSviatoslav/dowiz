import { safeStorage } from '../../lib/safeStorage.js';
import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { TaskCard, EmptyState, Button, useI18n, PaperIllustration, isPaperSkinEnabled, ease } from '@deliveryos/ui';
import type { CourierTask } from '@deliveryos/ui';
import { apiClient, useWebSocket, useSound } from '../../lib/index.js';

const containerVariants = { hidden: {}, visible: { transition: { staggerChildren: 0.04, delayChildren: 0.05 } } };
const itemVariants = { hidden: { opacity: 0, y: 10, scale: 0.98 }, visible: { opacity: 1, y: 0, scale: 1 } };

export function TasksPage() {
  const [tasks, setTasks] = useState<CourierTask[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [acceptingId, setAcceptingId] = useState<string | null>(null);
  // Real shift state — the "Online" indicator must reflect this, not be a static lie.
  // Mirrors ShiftPage's source of truth (GET /courier/me/shift → isActive).
  const [onShift, setOnShift] = useState(false);
  const navigate = useNavigate();
  const [actionError, setActionError] = useState<string | null>(null);
  const { play: playPing } = useSound('/sounds/ping.wav');
  const { t } = useI18n();

  // Extract courier ID from JWT stored in localStorage
  const getCourierId = (): string => {
    try {
      const token = safeStorage.get('dos_access_token');
      if (!token) return 'c1';
      const payloadBase64 = token.split('.')[1] || '';
      if (!payloadBase64) return 'c1';
      const payload = JSON.parse(atob(payloadBase64)) as Record<string, unknown>;
      return String(payload.sub || payload.userId || 'c1');
    } catch (err) { console.warn('[TasksPage] failed to parse JWT:', err); return 'c1'; }
  };
  const courierId = getCourierId();

  const fetchTasks = async () => {
    try {
      setLoading(true);
      const data = await apiClient<any>('/courier/me/assignments');
      const raw = data?.success && Array.isArray(data.assignments) ? data.assignments : [];
      setTasks(raw);
    } catch (err: any) {
      setError(t('courier.error_fetch_tasks', 'Failed to fetch tasks'));
    } finally {
      setLoading(false);
    }
  };

  // Derive the real online status from shift state so the badge can't contradict
  // the Shift tab. Best-effort: a failed read leaves the courier shown as offline.
  const fetchShiftStatus = async () => {
    try {
      const data = await apiClient<any>('/courier/me/shift');
      setOnShift(Boolean(data?.isActive));
    } catch (err) {
      setOnShift(false);
    }
  };

  useEffect(() => {
    fetchTasks();
    fetchShiftStatus();
  }, []);

  // WebSocket: Listen for new assignments
  useWebSocket({
    room: `courier:${courierId}`,
    onMessage: (msg) => {
      const envelope = msg?.data || msg;
      if (envelope.type === 'task_assigned') {
        const incoming = envelope.payload;
        setTasks(prev => {
          // Guard against duplicate task_assigned events (reconnect / re-delivery)
          // creating a second card for the same assignment.
          if (prev.some(t => t.id === incoming?.id)) return prev;
          return [incoming, ...prev];
        });
        playPing();
      }
    },
    onReconnect: fetchTasks
  });

  const handleAccept = async (id: string) => {
    setActionError(null);
    const task = tasks.find(t => t.id === id);
    // BUGFIX: an owner-direct-assigned task is already 'accepted'/'picked_up' (no offer handshake on
    // staging). It does NOT need a /accept call — the server only accepts 'offered'/'assigned' and 400s
    // an already-accepted one, which left the courier STUCK on Tasks (the error was swallowed). Go
    // straight to the delivery screen; only call /accept for a genuine offer.
    if (task && (task.status === 'accepted' || task.status === 'picked_up')) {
      navigate(`/courier/delivery/${id}`);
      return;
    }
    setAcceptingId(id);
    try {
      await apiClient(`/courier/assignments/${id}/accept`, { method: 'POST' });
      navigate(`/courier/delivery/${id}`);
    } catch (err) {
      console.warn('[TasksPage] accept task failed:', err);
      setActionError(t('courier.accept_failed', 'Could not accept the task — please try again.')); // surface, don't swallow
      setAcceptingId(null);
    }
  };

  const handleReject = async (id: string) => {
    // Optimistically drop the card, then actually release the assignment on the
    // server. The old code never called the API, so the assignment stayed
    // 'assigned' — the courier was hidden the card but remained blocked from new
    // dispatch (the dispatcher excludes couriers with an active assignment).
    setTasks(prev => prev.filter(t => t.id !== id));
    try {
      await apiClient(`/courier/assignments/${id}/reject`, { method: 'POST' });
    } catch (err) {
      console.warn('[TasksPage] reject task failed:', err);
      fetchTasks(); // restore the true server state if the release failed
    }
  };

  return (
    <div className="p-5 space-y-5">
      <div className="flex justify-between items-center gap-3 pb-4 border-b" style={{ borderColor: 'var(--brand-border)' }}>
        <h1 className="text-2xl font-bold min-w-0 truncate" style={{ fontFamily: 'var(--brand-font-heading)', color: 'var(--brand-text)' }}>{t('courier.tasks_title', 'Tasks')}</h1>
        <div
          className="flex items-center gap-2 shrink-0 rounded-[var(--brand-radius-btn)] px-2.5 py-1 text-sm"
          style={{ color: onShift ? 'var(--color-success)' : 'var(--brand-text-muted)', backgroundColor: 'var(--brand-surface-raised)' }}
          role="status"
          aria-label={onShift ? t('courier.online_status', 'Online') : t('courier.offline', 'Offline')}
        >
          <span className="relative flex h-2.5 w-2.5">
            {onShift && <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--color-success)] opacity-75"></span>}
            <span className={`relative inline-flex rounded-full h-2.5 w-2.5 ${onShift ? 'bg-[var(--color-success)]' : 'bg-[var(--brand-text-muted)]'}`}></span>
          </span>
          <span className="text-xs font-medium">{onShift ? t('courier.online_status', 'Online') : t('courier.offline', 'Offline')}</span>
        </div>
      </div>

      {/* Shown ABOVE the list/empty branches so an accept failure (e.g. a 410 expired-offer) stays
          visible even when the failed task vanishes and the list empties (was lost inside the list). */}
      {actionError && (
        <div role="alert" aria-live="assertive" data-testid="courier-task-error" className="rounded-[var(--brand-radius)] px-3 py-2 text-sm text-center font-medium" style={{ background: 'var(--status-cancelled-light)', border: '1px solid var(--status-cancelled-border)', color: 'var(--color-danger)' }}>
          {actionError}
        </div>
      )}

      {loading && tasks.length === 0 ? (
        <div className="space-y-4">
          {[1, 2, 3].map(i => (
            <div key={i} className="card-base p-5 space-y-3">
              <div className="skeleton-block h-5 w-3/4" />
              <div className="skeleton-block h-3 w-1/2" />
              <div className="skeleton-block h-3 w-2/3" />
            </div>
          ))}
        </div>
      ) : error ? (
        <EmptyState
          fullPage
          icon={<i className="ti ti-alert-triangle" aria-hidden="true" />}
          title={t('common.error', 'Error')}
          description={error}
          action={
            <Button type="button" variant="primary" className="min-h-tap" onClick={() => { setError(''); fetchTasks(); }}>
              {t('common.retry', 'Try again')}
            </Button>
          }
        />
      ) : tasks.length === 0 ? (
        <EmptyState
          fullPage
          title={onShift ? t('courier.no_tasks', 'No active tasks') : t('courier.offline_title', 'You\'re offline')}
          description={onShift
            ? t('courier.no_tasks_desc', 'We\'ll notify you when a new delivery is ready.')
            : t('courier.offline_desc', 'Go online from the Shift tab to start receiving deliveries.')}
          icon={isPaperSkinEnabled() ? <PaperIllustration name="island" animated className="mx-auto max-w-[200px]" /> : <i className={onShift ? 'ti ti-checkup-list' : 'ti ti-zzz'} aria-hidden="true" />}
          action={!onShift ? (
            <Button type="button" variant="primary" className="min-h-tap" onClick={() => navigate('/courier/shift')}>
              {t('courier.go_online', 'Go to Shift')}
            </Button>
          ) : undefined}
        />
      ) : (
        <motion.div className="space-y-4" variants={containerVariants} initial="hidden" animate="visible">
          <AnimatePresence mode="popLayout">
            {tasks.map(task => {
              // An owner-direct-assigned / already-accepted task has NO offer window — passing onReject
              // undefined stops TaskCard's countdown + auto-reject (it keys `timed` on !!onReject),
              // which was wrongly auto-releasing owner-assigned tasks. An 'offered'/'assigned' offer keeps
              // the accept/decline window, sized to the server's 30s accept window (was a desynced 60s).
              const isOffer = task.status === 'offered' || task.status === 'assigned';
              return (
                <motion.div key={task.id} variants={itemVariants} exit={{ opacity: 0, y: -8, scale: 0.97, transition: { duration: 0.15, ease: ease.out } }} layout>
                  <TaskCard task={task} onAccept={handleAccept} onReject={isOffer ? handleReject : undefined} offerSeconds={30} isLoading={acceptingId === task.id} />
                </motion.div>
              );
            })}
          </AnimatePresence>
        </motion.div>
      )}
    </div>
  );
}
