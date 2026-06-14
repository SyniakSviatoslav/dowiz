export interface CourierTask {
  id: string;
  order_id?: string;
  status: string;
  restaurant: { name: string; address: string; lat?: number; lng?: number; };
  customer: { address: string; phone?: string; instructions?: string; lat?: number; lng?: number; };
  total: number;
  eta: string;
  cashPayWith?: number | null;
}
