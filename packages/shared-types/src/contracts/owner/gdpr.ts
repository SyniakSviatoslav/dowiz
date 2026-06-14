import { z } from 'zod';

export const CreateGDPRRequest = z.object({
  customerId: z.string().uuid().optional(),
  phone: z.string().optional(),
  reason: z.string().max(500).optional(),
}).strict().refine(data => data.customerId || data.phone, {
  message: 'Either customerId or phone is required',
});
export type CreateGDPRRequest = z.infer<typeof CreateGDPRRequest>;

export const GDPRRequestItem = z.object({
  id: z.string().uuid(),
  status: z.enum(['pending','in_progress','completed','failed']),
  createdAt: z.string(),
  completedAt: z.string().nullable(),
  reason: z.string().nullable(),
}).strict();
export type GDPRRequestItem = z.infer<typeof GDPRRequestItem>;

export const GDPRRequestListResponse = z.object({
  requests: z.array(GDPRRequestItem),
  nextCursor: z.string().nullable(),
}).strict();
export type GDPRRequestListResponse = z.infer<typeof GDPRRequestListResponse>;

export const CreateGDPRResponse = z.object({
  requestId: z.string().uuid(),
  status: z.literal('pending'),
}).strict();
export type CreateGDPRResponse = z.infer<typeof CreateGDPRResponse>;
