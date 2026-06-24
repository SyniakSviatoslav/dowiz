import { z } from 'zod';

export const CreateProductBody = z.object({
  category_id: z.string().uuid().nullable().optional(),
  name: z.string().min(1).max(300),
  description: z.string().nullable().optional(),
  price: z.number().int().min(0),
  prep_time_minutes: z.number().int().min(1).max(1440).optional(),
  available: z.boolean().default(true).optional(),
  image_key: z.string().nullable().optional(),
  taste: z.record(z.number()).nullable().optional(),
  recipeLines: z.array(z.object({
    supplyId: z.string(),
    supplyName: z.string(),
    qty: z.number(),
    unit: z.string(),
    kind: z.string(),
    kcal: z.number().nullable(),
    proteinG: z.number().nullable(),
    fatG: z.number().nullable(),
    carbsG: z.number().nullable(),
    allergens: z.array(z.string()),
  })).nullable().optional(),
  stockCount: z.number().int().nullable().optional(),
  attributes: z.record(z.unknown()).nullable().optional(),
  sort_order: z.number().int().default(0).optional(),
}).strict();
export type CreateProductBody = z.infer<typeof CreateProductBody>;

export const UpdateProductBody = z.object({
  category_id: z.string().uuid().nullable().optional(),
  name: z.string().min(1).max(300).optional(),
  description: z.string().nullable().optional(),
  price: z.number().int().min(0).optional(),
  prep_time_minutes: z.number().int().min(1).max(1440).optional(),
  available: z.boolean().optional(),
  image_key: z.string().nullable().optional(),
  taste: z.record(z.number()).nullable().optional(),
  recipeLines: z.array(z.object({
    supplyId: z.string(),
    supplyName: z.string(),
    qty: z.number(),
    unit: z.string(),
    kind: z.string(),
    kcal: z.number().nullable(),
    proteinG: z.number().nullable(),
    fatG: z.number().nullable(),
    carbsG: z.number().nullable(),
    allergens: z.array(z.string()),
  })).nullable().optional(),
  stockCount: z.number().int().nullable().optional(),
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
  prepTimeMinutes: z.number().int().nullable().optional(),
  available: z.boolean(),
  imageKey: z.string().nullable(),
  imageUrl: z.string().nullable().optional(),
  sortOrder: z.number().int(),
  taste: z.record(z.number()).nullable().optional(),
  recipeLines: z.array(z.object({
    supplyId: z.string(),
    supplyName: z.string(),
    qty: z.number(),
    unit: z.string(),
    kind: z.string(),
    kcal: z.number().nullable(),
    proteinG: z.number().nullable(),
    fatG: z.number().nullable(),
    carbsG: z.number().nullable(),
    allergens: z.array(z.string()),
  })).nullable().optional(),
  stockCount: z.number().int().nullable().optional(),
  attributes: z.record(z.unknown()).nullable().optional(),
  createdAt: z.string(),
});
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
