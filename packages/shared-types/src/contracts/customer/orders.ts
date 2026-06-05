import { z } from 'zod';

export const CancelOrderBody = z.object({
  reason: z.string().min(5).max(500),
}).strict();
export type CancelOrderBody = z.infer<typeof CancelOrderBody>;
