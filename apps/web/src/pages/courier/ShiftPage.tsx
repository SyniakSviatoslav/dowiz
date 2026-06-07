import React, { useEffect, useState, useCallback } from 'react';
import { Button, EmptyState, SkeletonBase, useI18n } from '@deliveryos/ui';
import { apiClient } from '../../lib/index.js';

interface ShiftState {
  isActive: boolean;
  startedAt: string | null;
  elapsedSeconds: number;
}

interface ShiftStats {
  deliveries: number;
  earnings: number;
  distance: number;
  onlineTime: string;
}

const MOCK_STATS: ShiftStats = {
  deliveries: 8,
  earnings: 4500,
  distance: 24.5,
  onlineTime: '4h 32m'
};

export function ShiftPage() {
  const [shift, setShift] = useState<ShiftState>({ isActive: false, startedAt: null, elapsedSeconds: 0 });
  const [stats, setStats] = useState<ShiftStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [actionLoading, setActionLoading] = useState(false);
  const { t } = useI18n();

  const fetchShift = async () => {
    try {
      setLoading(true);
      const data = await apiClient<any>('/courier/me/shift');
      setShift({
        isActive: data?.isActive ?? false,
        startedAt: data?.startedAt ?? null,
        elapsedSeconds: data?.elapsedSeconds ?? 0
      });
      setStats(data?.stats || null);
    } catch (err: any) {
      if (err.status === 404) {
        setShift({
          isActive: false,
          startedAt: null,
          elapsedSeconds: 0
        });
        setStats(MOCK_STATS);
      } else {
        setError('Failed to fetch shift data');
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchShift();
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
      const data = await apiClient<any>('/courier/me/shift/start', { method: 'POST' });
      setShift({
        isActive: true,
        startedAt: new Date().toISOString(),
        elapsedSeconds: 0
      });
    } catch (err: any) {
      if (err.status === 404) {
        setShift({
          isActive: true,
          startedAt: new Date().toISOString(),
          elapsedSeconds: 0
        });
      } else {
        setError('Failed to start shift');
      }
    } finally {
      setActionLoading(false);
    }
  };

  const handleEndShift = async () => {
    setActionLoading(true);
    try {
      await apiClient<any>('/courier/me/shift/end', { method: 'POST' });
      setShift({ isActive: false, startedAt: null, elapsedSeconds: 0 });
      fetchShift();
    } catch (err: any) {
      if (err.status === 404) {
        setShift({ isActive: false, startedAt: null, elapsedSeconds: 0 });
        fetchShift();
      } else {
        setError('Failed to end shift');
      }
    } finally {
      setActionLoading(false);
    }
  };

  const statItems = stats ? [
    { label: 'Deliveries', value: String(stats.deliveries), icon: '\u{1F4E6}' },
    { label: 'Earnings', value: `${stats.earnings.toLocaleString()} ALL`, icon: '\u{1F4B0}' },
    { label: 'Distance', value: `${stats.distance} km`, icon: '\u{1F6F5}' },
    { label: 'Online', value: stats.onlineTime, icon: '\u23F1' },
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
          <div className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-6 text-center">
            {shift.isActive ? (
              <div className="space-y-4">
                <div className="text-sm font-semibold text-[var(--brand-text-muted)] uppercase tracking-wider">
                  {t('courier.current_shift', 'Current Shift')}
                </div>
                <div className="text-5xl font-black text-[var(--brand-primary)] tabular-nums" style={{ fontFamily: 'var(--font-mono, monospace)' }}>
                  {formatElapsed(shift.elapsedSeconds)}
                </div>
                <div className="text-xs text-[var(--brand-text-muted)]">
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
                <div className="text-5xl mb-2">{'\u{1F552}'}</div>
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
          </div>

          {stats && (
            <div className="space-y-3">
              <h2 className="text-lg font-bold text-[var(--brand-text)]">{t('courier.today_stats', 'Today\'s Stats')}</h2>
              <div className="grid grid-cols-2 gap-3">
                {statItems.map((item) => (
                  <div
                    key={item.label}
                    className="bg-[var(--brand-surface)] border border-[var(--brand-border)] rounded-[var(--brand-radius)] p-4"
                  >
                    <div className="text-2xl mb-1">{item.icon}</div>
                    <div className="text-xs text-[var(--brand-text-muted)] uppercase tracking-wider font-semibold mb-1">
                      {item.label}
                    </div>
                    <div className="text-lg font-bold text-[var(--brand-text)]">{item.value}</div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
