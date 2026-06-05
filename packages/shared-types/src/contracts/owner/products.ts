import { z } from 'zod';

export const CreateProductBody = z.object({
  category_id: z.string().uuid().nullable().optional(),
  name: z.string().min(1).max(300),
  description: z.string().nullable().optional(),
  price: z.number().int().min(0),
  available: z.boolean().default(true).optional(),
  image_key: z.string().nullable().optional(),
  attributes: z.record(z.unknown()).nullable().optional(),
  sort_order: z.number().int().default(0).optional(),
}).strict();
export type CreateProductBody = z.infer<typeof CreateProductBody>;

export const UpdateProductBody = z.object({
  category_id: z.string().uuid().nullable().optional(),
  name: z.string().min(1).max(300).optional(),
  description: z.string().nullable().optional(),
  price: z.number().int().min(0).optional(),
  available: z.boolean().optional(),
  image_key: z.string().nullable().optional(),
  attributes: z.record(z.unknown()).nullable().optional(),
  sort_order: z.number().int().optional(),
}).strict();
export type UpdateProductBody = z.infer<typeof UpdateProductBody>;

export const ProductResponse = z.object({
  id: z.string().uuid(),
  categoryId: z.string().uuid().nullable(),
  name: z.string(),
  description: z.string().nullable(),
  price: z.number().int(),
  available: z.boolean(),
  imageKey: z.string().nullable(),
  sortOrder: z.number().int(),
  createdAt: z.string(),
}).strict();
export type ProductResponse = z.infer<typeof ProductResponse>;

export const ProductTranslationBody = z.object({
  name: z.string().min(1).max(300),
  description: z.string().nullable().optional(),
}).strict();
export type ProductTranslationBody = z.infer<typeof ProductTranslationBody>;

export const ProductTranslationResponse = z.object({
  id: z.string().uuid(),
  locale: z.string(),
  name: z.string(),
  description: z.string().nullable(),
}).strict();
export type ProductTranslationResponse = z.infer<typeof ProductTranslationResponse>;
