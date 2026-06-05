import { z } from 'zod';

export const OrderStatusTransition = z.object({
  status: z.enum(['PENDING','CONFIRMED','PREPARING','READY','IN_DELIVERY','DELIVERED','REJECTED','CANCELLED','SCHEDULED','PICKED_UP']),
}).strict();
export type OrderStatusTransition = z.infer<typeof OrderStatusTransition>;

export const ConfirmOrderResponse = z.object({
  id: z.string().uuid(),
  status: z.enum(['CONFIRMED','REJECTED']),
  transitionedAt: z.string(),
}).strict();
export type ConfirmOrderResponse = z.infer<typeof ConfirmOrderResponse>;

export const RejectOrderBody = z.object({
  reason: z.string().max(500).optional(),
}).strict();
export type RejectOrderBody = z.infer<typeof RejectOrderBody>;

export const AssignCourierBody = z.object({
  courierId: z.string().uuid(),
}).strict();
export type AssignCourierBody = z.infer<typeof AssignCourierBody>;

export const AssignCourierResponse = z.object({
  id: z.string().uuid(),
  orderId: z.string().uuid(),
  courierId: z.string().uuid(),
  status: z.string(),
}).strict();
export type AssignCourierResponse = z.infer<typeof AssignCourierResponse>;

export const UpdateMetadataBody = z.object({
  test_order: z.boolean().optional(),
}).strict();
export type UpdateMetadataBody = z.infer<typeof UpdateMetadataBody>;

export const RevealContactResponse = z.object({
  orderId: z.string().uuid(),
  customerId: z.string().uuid(),
  name: z.string(),
  phone: z.string(),
}).strict();
export type RevealContactResponse = z.infer<typeof RevealContactResponse>;

export const MarkNoShowResponse = z.object({
  success: z.literal(true),
  customerId: z.string().uuid(),
}).strict();
export type MarkNoShowResponse = z.infer<typeof MarkNoShowResponse>;
