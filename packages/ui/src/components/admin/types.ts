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
