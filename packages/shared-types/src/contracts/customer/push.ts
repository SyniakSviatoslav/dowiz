import { z } from 'zod';

export const PushSubscriptionBody = z.object({
  subscription: z.object({
    endpoint: z.string().url(),
    keys: z.object({
      p256dh: z.string().min(1),
      auth: z.string().min(1),
    }),
  }),
  opted_in: z.boolean().default(true),
}).strict();
export type PushSubscriptionBody = z.infer<typeof PushSubscriptionBody>;

export const PushResponse = z.object({
  ok: z.literal(true),
}).strict();
export type PushResponse = z.infer<typeof PushResponse>;
