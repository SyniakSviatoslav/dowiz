import { z } from 'zod';

export const SSRQueryParams = z.object({
  embed: z.string().optional(),
  widget: z.string().optional(),
  locale: z.string().optional(),
}).strict();

export type SSRQueryParams = z.infer<typeof SSRQueryParams>;
