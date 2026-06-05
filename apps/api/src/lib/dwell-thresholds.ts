// @ts-nocheck
import { z } from 'zod';

export const dwellThresholdsSchema = z.object({
  v: z.literal(1),
  pending_s: z.number().int().min(10).max(3600),
  confirmed_s: z.number().int().min(10).max(3600),
  preparing_s: z.number().int().min(10).max(7200),
  en_route_s: z.number().int().min(10).max(7200),
}).strict();

export type DwellThresholds = z.infer<typeof dwellThresholdsSchema>;

export const DEFAULT_DWELL_THRESHOLDS: DwellThresholds = {
  v: 1,
  pending_s: 60,
  confirmed_s: 300,
  preparing_s: 600,
  en_route_s: 900,
};

export const STATUS_KIND_MAP: Record<string, string> = {
  PENDING: 'dwell_pending',
  CONFIRMED: 'dwell_confirmed',
  PREPARING: 'dwell_preparing',
  IN_DELIVERY: 'dwell_en_route',
  EN_ROUTE: 'dwell_en_route',
};

export const KIND_TO_THRESHOLD_KEY: Record<string, string> = {
  dwell_pending: 'pending_s',
  dwell_confirmed: 'confirmed_s',
  dwell_preparing: 'preparing_s',
  dwell_en_route: 'en_route_s',
};
