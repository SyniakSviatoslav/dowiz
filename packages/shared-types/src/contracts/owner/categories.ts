import { z } from 'zod';

export const CreateCategoryBody = z.object({
  name: z.string().min(1).max(200),
  sort_order: z.number().int().optional(),
  image_key: z.string().nullable().optional(),
}).strict();
export type CreateCategoryBody = z.infer<typeof CreateCategoryBody>;

export const UpdateCategoryBody = z.object({
  name: z.string().min(1).max(200).optional(),
  sort_order: z.number().int().optional(),
}).strict();
export type UpdateCategoryBody = z.infer<typeof UpdateCategoryBody>;

export const CategoryResponse = z.object({
  id: z.string(),
  name: z.string(),
  sort_order: z.number().int(),
  product_count: z.number().int(),
}).passthrough();
export type CategoryResponse = z.infer<typeof CategoryResponse>;
