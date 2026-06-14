import { z } from 'zod';

export const CourierProfile = z.object({
  id: z.string().uuid(),
  full_name: z.string(),
  masked_email: z.string(),
  masked_phone: z.string().nullable(),
  last_login_at: z.string().nullable(),
  active_location: z.object({
    id: z.string().uuid(),
    role: z.string(),
  }),
}).strict();
export type CourierProfile = z.infer<typeof CourierProfile>;

export const CourierAssignment = z.object({
  id: z.string().uuid(),
  orderId: z.string().uuid(),
  status: z.enum(['assigned','accepted','picked_up','delivered','cancelled']),
  assignedAt: z.string().nullable(),
  acceptedAt: z.string().nullable(),
  pickedUpAt: z.string().nullable(),
  deliveredAt: z.string().nullable(),
  cashCollected: z.boolean(),
  cashAmount: z.number().int().nullable(),
}).strict();
export type CourierAssignment = z.infer<typeof CourierAssignment>;

export const AssignmentListResponse = z.object({
  success: z.literal(true),
  assignments: z.array(CourierAssignment),
}).strict();
export type AssignmentListResponse = z.infer<typeof AssignmentListResponse>;

export const AcceptRejectResponse = z.object({
  success: z.literal(true),
}).strict();
export type AcceptRejectResponse = z.infer<typeof AcceptRejectResponse>;

export const CourierShift = z.object({
  id: z.string().uuid(),
  status: z.string(),
  startedAt: z.string(),
  endedAt: z.string().nullable(),
  deliveriesCount: z.number().int(),
}).strict();
export type CourierShift = z.infer<typeof CourierShift>;

export const CourierShiftResponse = z.object({
  shifts: z.array(CourierShift),
}).strict();
export type CourierShiftResponse = z.infer<typeof CourierShiftResponse>;

export const CourierAuditLogResponse = z.object({
  logs: z.array(z.object({
    action: z.string(),
    actorKind: z.string(),
    createdAt: z.string(),
  })),
}).strict();
export type CourierAuditLogResponse = z.infer<typeof CourierAuditLogResponse>;

export const ChangePasswordBody = z.object({
  current_password: z.string().min(1),
  new_password: z.string().min(12),
}).strict();
export type ChangePasswordBody = z.infer<typeof ChangePasswordBody>;
