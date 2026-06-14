import { t } from '../../lib/i18n.js';

const STATUS_MAP: Record<string, { color: string; label: string }> = {
  PENDING: { color: 'bg-status-pending', label: t('order.pending', 'Pending') },
  CONFIRMED: { color: 'bg-status-confirmed', label: t('order.confirmed', 'Confirmed') },
  PREPARING: { color: 'bg-status-preparing', label: t('order.preparing', 'Preparing') },
  READY: { color: 'bg-status-ready', label: t('order.ready', 'Ready') },
  IN_DELIVERY: { color: 'bg-status-in-delivery', label: t('order.in_delivery', 'In delivery') },
  DELIVERED: { color: 'bg-status-delivered', label: t('order.delivered', 'Delivered') },
  REJECTED: { color: 'bg-status-rejected', label: t('order.rejected', 'Rejected') },
  CANCELLED: { color: 'bg-status-cancelled', label: t('order.cancelled', 'Cancelled') },
  SCHEDULED: { color: 'bg-status-scheduled', label: t('order.scheduled', 'Scheduled') },
  PICKED_UP: { color: 'bg-status-picked-up', label: t('order.picked_up', 'Picked up') },
};

interface StatusBadgeProps {
  status: string;
  pulse?: boolean;
}

export function StatusBadge({ status, pulse }: StatusBadgeProps) {
  const key = status.toUpperCase().replace(/-/g, '_');
  const config = STATUS_MAP[key] || STATUS_MAP[status] || { color: 'bg-brand-text-muted', label: status };
  return (
    <span className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold text-white ${config.color} ${pulse ? 'animate-pulse' : ''}`}>
      {pulse && <span className="w-1.5 h-1.5 rounded-full bg-white" />}
      {config.label}
    </span>
  );
}

STATUS_MAP['ASSIGNED'] = { color: 'bg-status-pending', label: t('order.pending', 'Pending') };
STATUS_MAP['ACCEPTED'] = { color: 'bg-status-confirmed', label: t('order.confirmed', 'Confirmed') };
