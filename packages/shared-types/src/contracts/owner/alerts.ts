import { z } from 'zod';

export const AlertItem = z.object({
  id: z.string().uuid(),
  kind: z.string(),
  severity: z.enum(['warning','info','danger']),
  message: z.string(),
  createdAt: z.string(),
  dwellSeconds: z.number().int().nullable(),
  acknowledgedAt: z.string().nullable(),
}).strict();
export type AlertItem = z.infer<typeof AlertItem>;

export const AlertListResponse = z.object({
  alerts: z.array(AlertItem),
  nextCursor: z.string().nullable(),
}).strict();
export type AlertListResponse = z.infer<typeof AlertListResponse>;

export const AcknowledgeAlertResponse = z.object({
  success: z.literal(true),
}).strict();
export type AcknowledgeAlertResponse = z.infer<typeof AcknowledgeAlertResponse>;

export const AcknowledgeAllAlertsBody = z.object({
  kind: z.string().optional(),
}).strict();
export type AcknowledgeAllAlertsBody = z.infer<typeof AcknowledgeAllAlertsBody>;
