import { z } from 'zod';

export const CourierListItem = z.object({
  id: z.string().uuid(),
  name: z.string(),
  maskedPhone: z.string(),
  maskedEmail: z.string(),
  status: z.enum(['active','suspended','deactivated']),
  role: z.enum(['courier','dispatcher']),
  onlineStatus: z.enum(['online','busy','offline']).nullable(),
  ordersToday: z.number().int(),
  rating: z.number().nullable(),
  lastLoginAt: z.string().nullable(),
  createdAt: z.string(),
}).strict();
export type CourierListItem = z.infer<typeof CourierListItem>;

export const CourierListResponse = z.object({
  couriers: z.array(CourierListItem),
}).strict();
export type CourierListResponse = z.infer<typeof CourierListResponse>;

export const UpdateCourierBody = z.object({
  status: z.enum(['active','suspended','deactivated']).optional(),
  role: z.enum(['courier','dispatcher']).optional(),
}).strict();
export type UpdateCourierBody = z.infer<typeof UpdateCourierBody>;

export const LiveCourier = z.object({
  id: z.string().uuid(),
  name: z.string(),
  lat: z.number(),
  lng: z.number(),
  status: z.enum(['online','busy','offline']),
  orderId: z.string().uuid().nullable(),
  accuracy: z.number().optional(),
  heading: z.number().optional(),
}).strict();
export type LiveCourier = z.infer<typeof LiveCourier>;

export const LiveCourierResponse = z.object({
  success: z.literal(true),
  couriers: z.array(LiveCourier),
}).strict();
export type LiveCourierResponse = z.infer<typeof LiveCourierResponse>;
