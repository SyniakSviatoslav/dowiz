import { z } from 'zod';

export const SettlementItem = z.object({
  id: z.string().uuid(),
  courierName: z.string(),
  amount: z.number().int(),
  status: z.enum(['pending','approved','paid','disputed']),
  periodStart: z.string(),
  periodEnd: z.string(),
  createdAt: z.string(),
}).strict();
export type SettlementItem = z.infer<typeof SettlementItem>;

export const SettlementListResponse = z.object({
  settlements: z.array(SettlementItem),
  nextCursor: z.string().nullable(),
}).strict();
export type SettlementListResponse = z.infer<typeof SettlementListResponse>;

export const PaySettlementBody = z.object({
  payment_reference: z.string().optional(),
  payment_method: z.enum(['cash','bank_transfer','other']).optional(),
}).strict();
export type PaySettlementBody = z.infer<typeof PaySettlementBody>;

export const DisputeSettlementBody = z.object({
  reason: z.string().min(1).max(500),
  items: z.array(z.string().uuid()).optional(),
}).strict();
export type DisputeSettlementBody = z.infer<typeof DisputeSettlementBody>;

export const ReopenSettlementBody = z.object({
  reason: z.string().min(1).max(500),
}).strict();
export type ReopenSettlementBody = z.infer<typeof ReopenSettlementBody>;

export const RegenerateSettlementsBody = z.object({
  referenceDate: z.string(),
}).strict();
export type RegenerateSettlementsBody = z.infer<typeof RegenerateSettlementsBody>;
