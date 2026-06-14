import { z } from 'zod';

export const GeoPoint = z.object({
  lat: z.number().min(-90).max(90),
  lng: z.number().min(-180).max(180),
}).strict();

export type GeoPoint = z.infer<typeof GeoPoint>;

export const DeliveryPolygon = z.array(GeoPoint).min(3);

export type DeliveryPolygon = z.infer<typeof DeliveryPolygon>;

export const BusinessHours = z.object({
  day_of_week: z.number().int().min(0).max(6),
  open: z.string().regex(/^\d{2}:\d{2}$/),
  close: z.string().regex(/^\d{2}:\d{2}$/),
}).strict();

export type BusinessHours = z.infer<typeof BusinessHours>;

export const CursorPagination = z.object({
  cursor: z.string().optional(),
  limit: z.coerce.number().int().min(1).max(100).default(50),
}).strict();

export type CursorPagination = z.infer<typeof CursorPagination>;

export const FallbackConfig = z.object({
  phone: z.string().nullable(),
  showPhoneOnError: z.boolean(),
  showPhoneOnOffline: z.boolean(),
}).strict();

export type FallbackConfig = z.infer<typeof FallbackConfig>;

export const DegradationStatus = z.object({
  phone: z.string().nullable(),
  showPhoneOnError: z.boolean(),
  showPhoneOnOffline: z.boolean(),
  deadChannels: z.array(z.string()),
}).strict();

export type DegradationStatus = z.infer<typeof DegradationStatus>;
