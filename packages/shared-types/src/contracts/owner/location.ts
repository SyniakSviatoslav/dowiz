import { z } from 'zod';

export const UpdateLocationBody = z.object({
  default_locale: z.string().min(2).optional(),
  supported_locales: z.array(z.string().min(2)).optional(),
  name: z.string().min(1).max(200).optional(),
  phone: z.string().min(3).max(30).optional(),
  currency_code: z.string().length(3).optional(),
  delivery_fee_flat: z.number().int().min(0).optional(),
  min_order_value: z.number().int().min(0).nullish(),
  free_delivery_threshold: z.number().int().min(0).nullish(),
  delivery_radius_km: z.number().min(0).nullish(),
  tax_rate: z.number().min(0).max(100).optional(),
  lat: z.number().min(-90).max(90).nullish(),
  lng: z.number().min(-180).max(180).nullish(),
  delivery_address: z.string().max(500).nullish(),
}).strict();
export type UpdateLocationBody = z.infer<typeof UpdateLocationBody>;

export const LocationResponse = z.object({
  id: z.string().uuid(),
  name: z.string(),
  slug: z.string(),
  phone: z.string(),
  address: z.string().nullable(),
  lat: z.number().nullable(),
  lng: z.number().nullable(),
  status: z.string(),
  currency_code: z.string(),
  delivery_fee_flat: z.number().int(),
  min_order_value: z.number().int(),
  free_delivery_threshold: z.number().int(),
  delivery_radius_km: z.number(),
  tax_rate: z.number(),
  default_locale: z.string(),
  supported_locales: z.array(z.string()),
  created_at: z.string(),
}).strict();
export type LocationResponse = z.infer<typeof LocationResponse>;
