const STATUS_MAP: Record<string, { color: string; label: string }> = {
  PENDING: { color: 'bg-status-pending', label: 'Në pritje' },
  CONFIRMED: { color: 'bg-status-confirmed', label: 'Konfirmuar' },
  PREPARING: { color: 'bg-status-preparing', label: 'Duke u përgatitur' },
  READY: { color: 'bg-status-ready', label: 'Gati' },
  IN_DELIVERY: { color: 'bg-status-in-delivery', label: 'Në dorëzim' },
  DELIVERED: { color: 'bg-status-delivered', label: 'Dorëzuar' },
  REJECTED: { color: 'bg-status-rejected', label: 'Refuzuar' },
  CANCELLED: { color: 'bg-status-cancelled', label: 'Anuluar' },
  SCHEDULED: { color: 'bg-status-scheduled', label: 'Planifikuar' },
  PICKED_UP: { color: 'bg-status-picked-up', label: 'Marrë' },
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

STATUS_MAP['ASSIGNED'] = { color: 'bg-status-pending', label: 'Assigned' };
STATUS_MAP['ACCEPTED'] = { color: 'bg-status-confirmed', label: 'Accepted' };
