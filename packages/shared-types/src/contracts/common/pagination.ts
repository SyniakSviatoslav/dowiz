import { z } from 'zod';

export const CursorResponse = z.object({
  nextCursor: z.string().nullable(),
}).strict();

export type CursorResponse = z.infer<typeof CursorResponse>;

export const ErrorResponse = z.object({
  error: z.string(),
  message: z.string().optional(),
  code: z.string().optional(),
}).passthrough();

export type ErrorResponse = z.infer<typeof ErrorResponse>;

export const PaginationParams = z.object({
  cursor: z.string().optional(),
  limit: z.coerce.number().int().min(1).max(100).default(50),
}).strict();

export type PaginationParams = z.infer<typeof PaginationParams>;
