// @ts-nocheck
import { getCart } from '../cart/store.js';
import { showFallbackBanner } from '../shared/fallback-phone.js';

export interface PlaceOrderPayload {
  locationId: string;
  idempotency_key: string;
  customer: {
    phone: string;
    name?: string;
  };
  delivery: {
    pin: { lat: number; lng: number } | null;
    address_text?: string;
  };
  cash_pay_with: number | null;
  items: Array<{
    product_id: string;
    quantity: number;
    modifier_ids: string[];
  }>;
}

export async function placeOrder(payload: PlaceOrderPayload): Promise<{
  success: boolean;
  data?: any;
  code?: string;
  message?: string;
}> {
  let res: Response;
  try {
    res = await fetch('/api/orders', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    });
  } catch (err) {
    window.dispatchEvent(new CustomEvent('fallback:needed', { detail: { reason: 'post_failed' } }));
    return { success: false, code: 'NETWORK_ERROR', message: 'Could not reach the server.' };
  }

  const data = await res.json().catch(() => null);

  if (res.status === 201 || res.status === 200) {
    if (data.jwt) {
      localStorage.setItem(`dowiz:session:${data.orderId}`, data.jwt);
    }
    return { success: true, data };
  }

  if (res.status === 422) {
    return { success: false, code: data.code || 'UNKNOWN_VALIDATION_ERROR', message: data.message };
  }
  
  if (res.status === 429) {
    return { success: false, code: 'RATE_LIMIT', message: 'Забагато спроб. Спробуйте за хвилину.' };
  }

  if (res.status >= 500) {
    window.dispatchEvent(new CustomEvent('fallback:needed', { detail: { reason: 'post_failed' } }));
  }

  return { success: false, code: 'SERVER_ERROR', message: 'Сервер тимчасово недоступний.' };
}
