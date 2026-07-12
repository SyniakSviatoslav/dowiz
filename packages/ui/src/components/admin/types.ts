export interface AdminOrder {
  id: string;
  status: 'PENDING' | 'CONFIRMED' | 'PREPARING' | 'READY' | 'IN_DELIVERY' | 'DELIVERED' | 'CANCELLED';
  createdAt: string;
  confirmedAt?: string;
  readyAt?: string;
  deliveredAt?: string;
  items: { name: string; quantity: number }[];
  total: number;
  customerName?: string;
  customerPhone?: string;
  shortId?: string;
  itemCount?: number;
  itemsSummary?: string;
  etaMinutes?: number | null;
  elapsedSeconds?: number;
  courierName?: string | null;
  deliveryAddress?: string;
  signals?: {
    reputationScore: number;
    otpVerified: boolean;
  };
}

/**
 * F7: a fresh order arrives over the WS carrying only non-PII fields (status, total,
 * itemCount) — name/phone/items backfill from the authed endpoint moments later. While
 * the count says items exist but the items haven't loaded, the card should show a
 * placeholder rather than a hollow nameless / "0 items" card (flash of wrong state).
 */
// eslint-disable-next-line local/no-hardcoded-string -- Pick<> type-key literals, not UI copy
export function isOrderDetailsPending(order: Pick<AdminOrder, 'itemCount' | 'items'>): boolean {
  return (order.itemCount ?? 0) > 0 && (!order.items || order.items.length === 0);
}
