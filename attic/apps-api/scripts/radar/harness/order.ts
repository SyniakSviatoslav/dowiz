import { apiPost, apiGet, apiPatch, authHeaders } from './auth.js';
import { BASE_URL } from '../config.js';

export interface OrderResult {
  id: string;
  status: string;
  subtotal: number;
  total: number;
  createdAt: string;
  preflight?: { outcome: string; reasons: string[]; confirmedReasons: string[] };
}

export function uuid(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = Math.random() * 16 | 0;
    return (c === 'x' ? r : (r & 0x3 | 0x8)).toString(16);
  });
}

export async function getMenu(locationId: string): Promise<any> {
  const res = await fetch(`${BASE_URL}/public/locations/${locationId}/menu`);
  if (!res.ok) throw new Error(`Menu fetch failed (${res.status})`);
  return res.json();
}

export async function getLocationInfo(slug: string): Promise<any> {
  const res = await fetch(`${BASE_URL}/public/locations/${slug}/info`);
  if (!res.ok) throw new Error(`Info fetch failed (${res.status})`);
  return res.json();
}

export async function placeOrder(
  locationId: string,
  productId: string,
  overrides?: Partial<{
    phone: string;
    name: string;
    quantity: number;
    lat: number;
    lng: number;
    addressText: string;
    idempotencyKey: string;
  }>,
): Promise<OrderResult> {
  const idKey = overrides?.idempotencyKey || uuid();
  const { status, body } = await apiPost('/api/orders', {
    locationId,
    type: 'delivery',
    items: [{ product_id: productId, quantity: overrides?.quantity || 1, modifier_ids: [] }],
    customer: {
      phone: overrides?.phone || '+355600000000',
      name: overrides?.name || 'Radar Test',
    },
    delivery: {
      pin: { lat: overrides?.lat || 41.331, lng: overrides?.lng || 19.817 },
      address_text: overrides?.addressText || 'Rruga e Barrikadave, Tirana',
    },
    payment: { method: 'cash' },
    idempotency_key: idKey,
    acknowledged_codes: [],
  });

  if (status !== 201) {
    throw new Error(`Place order failed (${status}): ${JSON.stringify(body)}`);
  }
  return body as OrderResult;
}

export async function confirmOrder(orderId: string, locationId: string): Promise<number> {
  const { status } = await apiPost(`/api/owner/locations/${locationId}/orders/${orderId}/confirm`);
  return status;
}

export async function rejectOrder(orderId: string, locationId: string, reason?: string): Promise<number> {
  const { status } = await apiPost(`/api/owner/locations/${locationId}/orders/${orderId}/reject`, reason ? { reason } : {});
  return status;
}

export async function advanceStatus(orderId: string, newStatus: string): Promise<number> {
  const { status } = await apiPatch(`/api/orders/${orderId}/status`, { status: newStatus });
  return status;
}

export async function getOrderStatus(orderId: string): Promise<{ status: number; body: any }> {
  // Cannot use apiGet here because customer orders endpoint has different auth
  const res = await fetch(`${BASE_URL}/api/orders/${orderId}`, {
    headers: await authHeaders(),
  });
  return { status: res.status, body: res.status === 200 ? await res.json() : null };
}

export async function findFirstProduct(menu: any): Promise<{ id: string; name: string; price: number } | null> {
  for (const cat of menu.categories || []) {
    for (const p of cat.products || []) {
      if (p.available !== false) {
        return { id: p.id, name: p.name, price: p.price };
      }
    }
  }
  return null;
}
