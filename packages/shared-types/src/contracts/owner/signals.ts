import { z } from 'zod';

export const SIGNAL_KINDS = ['no_show_recent','velocity_rapid','velocity_high_volume','ip_velocity_rapid','ip_velocity_high_volume','manual_flag'] as const;

export const SignalItem = z.object({
  id: z.string().uuid(),
  customerId: z.string().uuid(),
  kind: z.enum(SIGNAL_KINDS),
  severity: z.string(),
  evidence: z.unknown(),
  raisedAt: z.string(),
  acknowledgedAt: z.string().nullable(),
  dismissedAt: z.string().nullable(),
  customerNameMasked: z.string(),
  customerPhoneMasked: z.string(),
}).strict();
export type SignalItem = z.infer<typeof SignalItem>;

export const SignalListResponse = z.object({
  signals: z.array(SignalItem),
  nextCursor: z.string().nullable(),
}).strict();
export type SignalListResponse = z.infer<typeof SignalListResponse>;

export const ComputeSignalQuery = z.object({
  phone_hash: z.string().regex(/^[a-f0-9]{64}$/).optional(),
  ip_hash: z.string().regex(/^[a-f0-9]{64}$/).optional(),
  customer_id: z.string().uuid().optional(),
}).strict();
export type ComputeSignalQuery = z.infer<typeof ComputeSignalQuery>;

export const AcknowledgeSignalResponse = z.object({
  id: z.string().uuid(),
  acknowledgedAt: z.string(),
}).strict();
export type AcknowledgeSignalResponse = z.infer<typeof AcknowledgeSignalResponse>;

export const DismissSignalBody = z.object({
  reason: z.string().max(500).optional(),
}).strict();
export type DismissSignalBody = z.infer<typeof DismissSignalBody>;

export const DismissSignalResponse = z.object({
  id: z.string().uuid(),
  dismissedAt: z.string(),
}).strict();
export type DismissSignalResponse = z.infer<typeof DismissSignalResponse>;
