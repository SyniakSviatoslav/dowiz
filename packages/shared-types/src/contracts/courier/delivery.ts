import { z } from 'zod';

export const CourierLocationBody = z.object({
  lat: z.number().min(-90).max(90),
  lng: z.number().min(-180).max(180),
  accuracy: z.number().min(0).optional(),
  heading: z.number().min(0).max(360).optional(),
  speed: z.number().min(0).optional(),
}).strict();
export type CourierLocationBody = z.infer<typeof CourierLocationBody>;

export const CourierStatusBody = z.object({
  status: z.enum(['online','offline']),
}).strict();
export type CourierStatusBody = z.infer<typeof CourierStatusBody>;

export const PickupDeliverResponse = z.object({
  success: z.literal(true),
}).strict();
export type PickupDeliverResponse = z.infer<typeof PickupDeliverResponse>;

export const CourierEarningsResponse = z.object({
  today: z.number().int(),
  week: z.number().int(),
  month: z.number().int(),
  payouts: z.array(z.object({
    id: z.string().uuid(),
    amount: z.number().int(),
    status: z.string(),
    createdAt: z.string(),
  })),
}).strict();
export type CourierEarningsResponse = z.infer<typeof CourierEarningsResponse>;
