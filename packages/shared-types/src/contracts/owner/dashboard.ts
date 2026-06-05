import { z } from 'zod';

export const DashboardCounts = z.object({
  pending: z.number().int(),
  confirmed: z.number().int(),
  preparing: z.number().int(),
  ready: z.number().int(),
  inDelivery: z.number().int(),
  deliveredToday: z.number().int(),
  revenueToday: z.number().int(),
  revenueTrend: z.string(),
  ordersToday: z.number().int(),
  ordersTrend: z.string(),
  activeDeliveries: z.number().int(),
  couriersOnline: z.number().int(),
  avgDeliveryMin: z.number(),
}).strict();
export type DashboardCounts = z.infer<typeof DashboardCounts>;

export const ActiveOrderSummary = z.object({
  id: z.string().uuid(),
  shortId: z.string(),
  status: z.string(),
  customerName: z.string(),
  customerPhone: z.string(),
  itemsSummary: z.string(),
  itemCount: z.number().int(),
  total: z.number().int(),
  createdAt: z.string(),
  elapsedSeconds: z.number().int(),
  courierName: z.string().nullable(),
  etaMinutes: z.number().nullable(),
}).strict();
export type ActiveOrderSummary = z.infer<typeof ActiveOrderSummary>;

export const DashboardSnapshotResponse = z.object({
  serverTime: z.string(),
  counts: DashboardCounts,
  orders: z.array(ActiveOrderSummary),
  activeDeliveries: z.number().int(),
  nextCursor: z.string().nullable(),
  activeAlertCount: z.number().int(),
  activeSignalCount: z.number().int(),
}).strict();
export type DashboardSnapshotResponse = z.infer<typeof DashboardSnapshotResponse>;
