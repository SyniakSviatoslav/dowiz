import type { AdminOrder } from '@deliveryos/ui';

export function orderDeltaChanged(a: AdminOrder, b: AdminOrder): boolean {
  return a.status !== b.status
    || a.total !== b.total
    || a.courierName !== b.courierName
    || a.itemCount !== b.itemCount
    || a.itemsSummary !== b.itemsSummary
    || a.shortId !== b.shortId
    || a.customerName !== b.customerName
    || a.customerPhone !== b.customerPhone;
}

export function mergeDelta(prev: AdminOrder[], payload: any, isNew: boolean): AdminOrder[] {
  const orderId = payload.orderId;
  const i = prev.findIndex(o => o.id === orderId);

  if (isNew) {
    if (i !== -1) return prev;
    const newOrder: AdminOrder = {
      id: orderId,
      status: payload.status,
      total: payload.total ?? 0,
      createdAt: payload.createdAt || new Date().toISOString(),
      items: payload.items || [],
      customerName: payload.customerNameMasked || undefined,
      customerPhone: payload.customerPhoneMasked || undefined,
      shortId: payload.shortId || '#' + orderId.substring(0, 4).toUpperCase(),
      itemCount: payload.itemCount ?? 0,
      itemsSummary: payload.itemsSummary || '',
      courierName: payload.courierName || null,
    };
    return [newOrder, ...prev];
  }

  if (i === -1) return prev;

  const existing = prev[i]!;
  const merged: AdminOrder = {
    ...existing,
    status: payload.status,
    ...(payload.total != null && { total: payload.total }),
    ...(payload.courierName !== undefined && { courierName: payload.courierName }),
    ...(payload.itemsSummary != null && { itemsSummary: payload.itemsSummary }),
    ...(payload.itemCount != null && { itemCount: payload.itemCount }),
    ...(payload.shortId != null && { shortId: payload.shortId }),
    ...(payload.customerNameMasked != null && { customerName: payload.customerNameMasked }),
    ...(payload.customerPhoneMasked != null && { customerPhone: payload.customerPhoneMasked }),
  };

  if (!orderDeltaChanged(existing, merged)) return prev;

  const next = prev.slice();
  next[i] = merged;
  return next;
}
