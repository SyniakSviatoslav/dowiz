import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { TaskCard, EmptyState, useI18n } from '@deliveryos/ui';
import type { CourierTask } from '@deliveryos/ui';
import { apiClient, useWebSocket, useSound } from '../../lib/index.js';
import { AssignmentListResponse } from '@deliveryos/shared-types';

export function TasksPage() {
  const [tasks, setTasks] = useState<CourierTask[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const navigate = useNavigate();
  const { play: playPing } = useSound('/sounds/ping.mp3');
  const { t } = useI18n();

  // Extract courier ID from JWT stored in localStorage
  const getCourierId = (): string => {
    try {
      const token = localStorage.getItem('dos_access_token');
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
      const data = await apiClient<typeof AssignmentListResponse>('/courier/me/assignments', { schema: AssignmentListResponse });
      const raw = data?.success && Array.isArray(data.assignments) ? data.assignments : [];
      setTasks(raw.map((a: any) => ({ ...a, cashPayWith: a.cash_amount != null ? a.cash_amount : a.cashPayWith })));
    } catch (err: any) {
      setError('Failed to fetch tasks');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTasks();
  }, []);

  // WebSocket: Listen for new assignments
  useWebSocket({
    room: `courier:${courierId}`,
    onMessage: (msg) => {
      if (msg.type === 'task_assigned') {
        setTasks(prev => [msg.payload, ...prev]);
        playPing();
      }
    },
    onReconnect: fetchTasks
  });

  const handleAccept = async (id: string) => {
    try {
      await apiClient(`/courier/assignments/${id}/accept`, {
        method: 'POST'
      });
    } catch (err) {
      console.warn('[TasksPage] accept task failed:', err);
    }
    navigate(`/courier/delivery/${id}`);
  };

  const handleReject = (id: string) => {
    setTasks(prev => prev.filter(t => t.id !== id));
    // Usually would call API to release assignment
  };

  return (
    <div className="p-4 space-y-6">
      <div className="flex justify-between items-center pb-4 border-b border-[var(--brand-border)]">
        <h1 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('courier.tasks_title', 'Tasks')}</h1>
        <div className="flex items-center gap-2 text-sm text-[var(--brand-text-muted)]">
          <div className="w-2 h-2 rounded-full bg-[var(--color-success)]" /> {t('courier.online_status', 'Online')}
        </div>
      </div>

      {loading && tasks.length === 0 ? (
        <div className="animate-pulse space-y-4">
          <div className="h-48 bg-[var(--brand-surface)] rounded-[var(--brand-radius)]" />
        </div>
      ) : error ? (
        <EmptyState title={t('common.error', 'Error')} description={error} />
      ) : tasks.length === 0 ? (
        <EmptyState title={t('courier.no_tasks', 'No active tasks')} description={t('courier.no_tasks_desc', 'We\'ll notify you when a new delivery is ready.')} />
      ) : (
        <div className="space-y-4">
          {tasks.map(task => (
            <TaskCard 
              key={task.id} 
              task={task} 
              onAccept={handleAccept} 
              onReject={handleReject} 
            />
          ))}
        </div>
      )}
    </div>
  );
}
