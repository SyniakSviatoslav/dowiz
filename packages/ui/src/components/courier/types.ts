export interface CourierTask {
  id: string;
  /** Order id — the API returns it as `orderId` (camelCase); `order_id` kept for back-compat. */
  orderId?: string;
  order_id?: string;
  status: string;
  restaurant: { name: string; address: string; lat?: number; lng?: number; };
  customer: { address: string; phone?: string; instructions?: string; lat?: number; lng?: number; };
  total: number;
  eta: string;
  cashPayWith?: number | null;
}
