import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { TaskCard, EmptyState, useI18n } from '@deliveryos/ui';
import type { CourierTask } from '@deliveryos/ui';
import { apiClient, useWebSocket, useSound } from '../../lib/index.js';
import { AssignmentListResponse } from '@deliveryos/shared-types';

const containerVariants = { hidden: {}, visible: { transition: { staggerChildren: 0.04, delayChildren: 0.05 } } };
const itemVariants = { hidden: { opacity: 0, y: 10, scale: 0.98 }, visible: { opacity: 1, y: 0, scale: 1 } };

export function TasksPage() {
  const [tasks, setTasks] = useState<CourierTask[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [acceptingId, setAcceptingId] = useState<string | null>(null);
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
    setAcceptingId(id);
    try {
      await apiClient(`/courier/assignments/${id}/accept`, {
        method: 'POST'
      });
      navigate(`/courier/delivery/${id}`);
    } catch (err) {
      console.warn('[TasksPage] accept task failed:', err);
      setAcceptingId(null);
    }
  };

  const handleReject = (id: string) => {
    setTasks(prev => prev.filter(t => t.id !== id));
    // Usually would call API to release assignment
  };

  return (
    <div className="p-5 space-y-5">
      <div className="flex justify-between items-center pb-4 border-b" style={{ borderColor: 'var(--brand-border)' }}>
        <h1 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>{t('courier.tasks_title', 'Tasks')}</h1>
        <div className="flex items-center gap-2 text-sm" style={{ color: 'var(--brand-text-muted)' }}>
          <span className="relative flex h-2.5 w-2.5">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--color-success)] opacity-75"></span>
            <span className="relative inline-flex rounded-full h-2.5 w-2.5 bg-[var(--color-success)]"></span>
          </span>
          <span className="text-xs">{t('courier.online_status', 'Online')}</span>
        </div>
      </div>

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
        <EmptyState title={t('common.error', 'Error')} description={error} />
      ) : tasks.length === 0 ? (
        <EmptyState title={t('courier.no_tasks', 'No active tasks')} description={t('courier.no_tasks_desc', 'We\'ll notify you when a new delivery is ready.')} />
      ) : (
        <motion.div className="space-y-4" variants={containerVariants} initial="hidden" animate="visible">
          <AnimatePresence mode="popLayout">
            {tasks.map(task => (
              <motion.div key={task.id} variants={itemVariants} exit={{ opacity: 0, y: -8, scale: 0.97, transition: { duration: 0.2 } }} layout>
                <TaskCard task={task} onAccept={handleAccept} onReject={handleReject} isLoading={acceptingId === task.id} />
              </motion.div>
            ))}
          </AnimatePresence>
        </motion.div>
      )}
    </div>
  );
}
