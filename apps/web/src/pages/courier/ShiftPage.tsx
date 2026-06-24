import React, { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Button, EmptyState, SkeletonBase, useI18n, PriceDisplay } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';
import { z } from 'zod';

const ShiftResponse = z.object({
  isActive: z.boolean().optional(),
  startedAt: z.string().nullable().optional(),
  elapsedSeconds: z.number().optional(),
  shiftId: z.string().nullable().optional(),
  status: z.string().nullable().optional(),
  stats: z.custom<ShiftStats>().optional(),
}).passthrough();

interface ShiftState {
  isActive: boolean;
  startedAt: string | null;
  elapsedSeconds: number;
  shiftId: string | null;
  status: string | null;
}

interface ShiftStats {
  deliveries: number;
  earnings: number;
  distance: number;
  onlineTime: string;
}

export function ShiftPage() {
  const [shift, setShift] = useState<ShiftState>({ isActive: false, startedAt: null, elapsedSeconds: 0, shiftId: null, status: null });
  const [stats, setStats] = useState<ShiftStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [actionLoading, setActionLoading] = useState(false);
  // UX-2: courier's messenger contact (the channel customers use during a delivery).
  const [msgKind, setMsgKind] = useState('');
  const [msgHandle, setMsgHandle] = useState('');
  const [msgSaving, setMsgSaving] = useState(false);
  const [msgSaved, setMsgSaved] = useState(false);
  const { t } = useI18n();

  const saveMessenger = async () => {
    setMsgSaving(true); setMsgSaved(false);
    try {
      await apiClient('/courier/me/messenger', {
        method: 'PATCH',
        body: { messenger_kind: msgKind || null, messenger_handle: msgHandle.trim() || null },
      });
      setMsgSaved(true);
    } catch { /* surfaced via the unchanged Save label */ } finally { setMsgSaving(false); }
  };

  const fetchShift = async () => {
    try {
      setLoading(true);
      const data = await apiClient<typeof ShiftResponse>('/courier/me/shift', { schema: ShiftResponse });
      setShift({
        isActive: data?.isActive ?? false,
        startedAt: data?.startedAt ?? null,
        elapsedSeconds: data?.elapsedSeconds ?? 0,
        shiftId: data?.shiftId ?? null,
        status: data?.status ?? null
      });
      setStats(data?.stats || null);
    } catch (err: any) {
      setError(t('courier.error_fetch_shift', 'Failed to fetch shift data'));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchShift();
    apiClient<any>('/courier/me').then((me: any) => {
      if (me?.messenger_kind) setMsgKind(me.messenger_kind);
      if (me?.messenger_handle) setMsgHandle(me.messenger_handle);
    }).catch(() => {});
  }, []);

  useEffect(() => {
    if (!shift.isActive || !shift.startedAt) return;

    const startTime = new Date(shift.startedAt).getTime();
    const updateElapsed = () => {
      const now = Date.now();
      setShift(prev => ({
        ...prev,
        elapsedSeconds: Math.floor((now - startTime) / 1000)
      }));
    };

    updateElapsed();
    const interval = setInterval(updateElapsed, 1000);
    return () => clearInterval(interval);
  }, [shift.isActive, shift.startedAt]);

  const formatElapsed = useCallback((seconds: number) => {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (h > 0) return `${h}h ${m}m ${s}s`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
  }, []);

  const handleStartShift = async () => {
    setActionLoading(true);
    try {
      const data = await apiClient<typeof ShiftResponse>('/courier/me/shift/start', { method: 'POST', schema: ShiftResponse });
      setShift({
        isActive: data?.isActive ?? true,
        startedAt: data?.startedAt ?? new Date().toISOString(),
        elapsedSeconds: data?.elapsedSeconds ?? 0,
        shiftId: data?.shiftId ?? null,
        status: data?.status ?? null
      });
    } catch (err: any) {
      setError(t('courier.error_start_shift', 'Failed to start shift'));
    } finally {
      setActionLoading(false);
    }
  };

  const handleEndShift = async () => {
    setActionLoading(true);
    try {
      await apiClient<typeof ShiftResponse>('/courier/me/shift/end', { method: 'POST', schema: ShiftResponse });
      setShift({ isActive: false, startedAt: null, elapsedSeconds: 0, shiftId: null, status: null });
      fetchShift();
    } catch (err: any) {
      setError(t('courier.error_end_shift', 'Failed to end shift'));
    } finally {
      setActionLoading(false);
    }
  };

  const statItems: { label: string; value: string | React.ReactNode; icon: React.ReactNode }[] = stats ? [
    { label: 'Deliveries', value: String(stats.deliveries), icon: <i className="ti ti-package" aria-hidden="true"></i> },
    { label: 'Earnings', value: <PriceDisplay amount={stats.earnings} />, icon: <i className="ti ti-moneybag" aria-hidden="true"></i> },
    { label: 'Distance', value: `${stats.distance} km`, icon: <i className="ti ti-motorbike" aria-hidden="true"></i> },
    { label: 'Online', value: stats.onlineTime, icon: <i className="ti ti-clock" aria-hidden="true"></i> },
  ] : [];

  return (
    <div className="p-4 space-y-6">
      <div className="flex justify-between items-center pb-4 border-b border-[var(--brand-border)]">
        <h1 className="text-2xl font-bold text-[var(--brand-text)]" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('courier.shift_title', 'Shift')}</h1>
        <div className="flex items-center gap-2 text-sm">
          <div className={`w-2 h-2 rounded-full ${shift.isActive ? 'bg-[var(--color-success)] animate-pulse' : 'bg-[var(--brand-text-muted)]'}`} />
          <span className={shift.isActive ? 'text-[var(--color-success)] font-medium' : 'text-[var(--brand-text-muted)]'}>
            {shift.isActive ? t('courier.on_shift', 'On Shift') : t('courier.offline', 'Offline')}
          </span>
        </div>
      </div>

      {loading ? (
        <div className="space-y-4">
          <SkeletonBase className="h-40 rounded-[var(--brand-radius)]" />
          <div className="grid grid-cols-2 gap-3">
            {[1, 2, 3, 4].map((i) => (
              <SkeletonBase key={i} className="h-24 rounded-[var(--brand-radius)]" />
            ))}
          </div>
        </div>
      ) : error ? (
        <EmptyState title={t('common.error', 'Error')} description={error} />
      ) : (
        <>
          <motion.div
            className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-6 text-center"
            initial={{ opacity: 0, y: 12, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            transition={{ type: 'spring', stiffness: 260, damping: 24 }}
          >
            {shift.isActive ? (
              <div className="space-y-4">
                <div className="text-sm font-semibold text-[var(--brand-text-muted)] uppercase tracking-wider">
                  {t('courier.current_shift', 'Current Shift')}
                </div>
                <div data-dynamic className="text-5xl font-black text-[var(--brand-primary)] tabular-nums" style={{ fontFamily: 'var(--font-mono, monospace)' }}>
                  {formatElapsed(shift.elapsedSeconds)}
                </div>
                <div data-dynamic className="text-xs text-[var(--brand-text-muted)]">
                  Started at {shift.startedAt ? new Date(shift.startedAt).toLocaleTimeString() : '--'}
                </div>
                <Button
                  variant="danger"
                  size="lg"
                  className="w-full mt-2"
                  onClick={handleEndShift}
                  isLoading={actionLoading}
                >
                  {t('courier.end_shift', 'End Shift')}
                </Button>
              </div>
            ) : (
              <div className="space-y-4">
                <div className="text-5xl mb-2"><i className="ti ti-clock" aria-hidden="true"></i></div>
                <div className="text-sm text-[var(--brand-text-muted)]">
                  {t('courier.offline_hint', 'You are currently offline. Start your shift to begin receiving delivery tasks.')}
                </div>
                <Button
                  variant="primary"
                  size="lg"
                  className="w-full"
                  onClick={handleStartShift}
                  isLoading={actionLoading}
                >
                  {t('courier.start_shift', 'Start Shift')}
                </Button>
              </div>
            )}
          </motion.div>

          {stats && (
            <div className="space-y-3">
              <h2 className="text-lg font-bold text-[var(--brand-text)]">{t('courier.today_stats', 'Today\'s Stats')}</h2>
              <motion.div
                className="grid grid-cols-2 gap-3"
                variants={{ hidden: {}, visible: { transition: { staggerChildren: 0.06, delayChildren: 0.1 } } }}
                initial="hidden"
                animate="visible"
              >
                {statItems.map((item) => (
                  <motion.div
                    key={item.label}
                    variants={{ hidden: { opacity: 0, y: 12, scale: 0.97 }, visible: { opacity: 1, y: 0, scale: 1, transition: { type: 'spring', stiffness: 260, damping: 24 } } }}
                    className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4"
                  >
                    <div className="text-2xl mb-1">{item.icon}</div>
                    <div className="text-xs text-[var(--brand-text-muted)] uppercase tracking-wider font-semibold mb-1">
                      {item.label}
                    </div>
                    <div className="text-lg font-bold text-[var(--brand-text)]">{item.value}</div>
                  </motion.div>
                ))}
              </motion.div>
            </div>
          )}
        </>
      )}

      {/* UX-2: how customers reach you during a delivery */}
      <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4 space-y-3">
        <div className="text-sm font-semibold text-[var(--brand-text)]">{t('courier.messenger_title', 'Messenger for customers')}</div>
        <div className="text-xs text-[var(--brand-text-muted)]">{t('courier.messenger_hint', 'Optional — lets customers text you during an active delivery instead of calling.')}</div>
        <div className="flex gap-2">
          <select value={msgKind} onChange={e => { setMsgKind(e.target.value); setMsgSaved(false); }} data-testid="courier-messenger-kind"
            className="h-[44px] px-2 border rounded-[8px] text-sm" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }}>
            <option value="">—</option>
            <option value="telegram">Telegram</option>
            <option value="whatsapp">WhatsApp</option>
            <option value="viber">Viber</option>
          </select>
          <input value={msgHandle} onChange={e => { setMsgHandle(e.target.value); setMsgSaved(false); }} disabled={!msgKind}
            placeholder={msgKind === 'telegram' ? '@username' : '+355 6X XXX XXXX'} data-testid="courier-messenger-handle"
            className="flex-1 h-[44px] px-3 border rounded-[8px] text-sm disabled:opacity-50" style={{ background: 'var(--brand-surface-raised)', borderColor: 'var(--brand-border)', color: 'var(--brand-text)' }} />
        </div>
        <Button onClick={saveMessenger} isLoading={msgSaving} variant="secondary" className="w-full">
          {msgSaved ? t('common.saved', 'Saved') : t('common.save', 'Save')}
        </Button>
      </div>
    </div>
  );
}
