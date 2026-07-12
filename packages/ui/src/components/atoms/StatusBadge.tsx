import { type ReactNode } from 'react';
import { t } from '../../lib/i18n.js';

const STATUS_MAP: Record<string, { color: string; label: string; tooltip: string }> = {
  PENDING: { color: 'bg-status-pending', label: t('order.pending', 'Pending'), tooltip: t('tooltip.status_pending', 'Order received, awaiting confirmation') },
  CONFIRMED: { color: 'bg-status-confirmed', label: t('order.confirmed', 'Confirmed'), tooltip: t('tooltip.status_confirmed', 'Order confirmed by restaurant') },
  PREPARING: { color: 'bg-status-preparing', label: t('order.preparing', 'Preparing'), tooltip: t('tooltip.status_preparing', 'Restaurant is preparing your order') },
  READY: { color: 'bg-status-ready', label: t('order.ready', 'Ready'), tooltip: t('tooltip.status_ready', 'Order is ready') },
  IN_DELIVERY: { color: 'bg-status-in-delivery', label: t('order.in_delivery', 'In delivery'), tooltip: t('tooltip.status_in_delivery', 'Courier is on the way') },
  DELIVERED: { color: 'bg-status-delivered', label: t('order.delivered', 'Delivered'), tooltip: t('tooltip.status_delivered', 'Order delivered successfully') },
  REJECTED: { color: 'bg-status-rejected', label: t('order.rejected', 'Rejected'), tooltip: t('tooltip.status_rejected', 'Order was rejected') },
  CANCELLED: { color: 'bg-status-cancelled', label: t('order.cancelled', 'Cancelled'), tooltip: t('tooltip.status_cancelled', 'Order was cancelled') },
  SCHEDULED: { color: 'bg-status-scheduled', label: t('order.scheduled', 'Scheduled'), tooltip: '' },
  PICKED_UP: { color: 'bg-status-picked-up', label: t('order.picked_up', 'Picked up'), tooltip: '' },
  ASSIGNED: { color: 'bg-status-pending', label: t('order.pending', 'Pending'), tooltip: '' },
  ACCEPTED: { color: 'bg-status-confirmed', label: t('order.confirmed', 'Confirmed'), tooltip: '' },
};

interface StatusBadgeProps {
  status: string;
  pulse?: boolean;
  icon?: ReactNode;
}

export function StatusBadge({ status, pulse, icon }: StatusBadgeProps) {
  const key = status.toUpperCase().replace(/-/g, '_');
  const config = STATUS_MAP[key] || STATUS_MAP[status] || { color: 'bg-brand-text-muted', label: status, tooltip: '' };
  return (
    <span
      className={`inline-flex items-center gap-1.5 px-3 py-1 text-xs font-semibold text-white ${config.color} ${pulse ? 'animate-pulse' : ''}`}
      style={{ borderRadius: `borderRadius`, transition: `transition` }}
      title={config.tooltip || undefined}
    >
      {pulse && <span className="w-1.5 h-1.5 rounded-full bg-white" aria-hidden="true" />}
      {icon && <span className="inline-flex shrink-0 items-center" aria-hidden="true">{icon}</span>}
      {config.label}
    </span>
  );
}

