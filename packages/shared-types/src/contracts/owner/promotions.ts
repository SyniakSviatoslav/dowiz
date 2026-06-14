import { z } from 'zod';

export const PromotionType = z.enum(['percentage', 'fixed', 'free_delivery']);

export const PromotionSchema = z.object({
  id: z.string().uuid(),
  location_id: z.string().uuid(),
  code: z.string().min(1).max(50),
  type: PromotionType,
  discount_value: z.number().int().positive(),
  min_order_amount: z.number().int().min(0).default(0),
  max_uses: z.number().int().positive().nullable().optional(),
  current_uses: z.number().int().min(0).default(0),
  max_uses_per_customer: z.number().int().positive().default(1),
  valid_from: z.string().datetime(),
  valid_until: z.string().datetime().nullable().optional(),
  is_active: z.boolean().default(true),
  applicable_product_ids: z.array(z.string().uuid()).default([]),
  description: z.string().max(500).nullable().optional(),
  created_at: z.string().datetime(),
  updated_at: z.string().datetime(),
});

export const CreatePromotionSchema = PromotionSchema.omit({
  id: true, location_id: true, current_uses: true, created_at: true, updated_at: true
});

export const UpdatePromotionSchema = CreatePromotionSchema.partial();

export const PromotionListResponse = z.object({
  promotions: z.array(PromotionSchema),
  total: z.number().int(),
});

export const PromotionValidateSchema = z.object({
  code: z.string().min(1).max(50),
  order_subtotal: z.number().int().min(0),
  product_ids: z.array(z.string().uuid()).optional(),
});

export const PromotionValidateResponse = z.object({
  valid: z.boolean(),
  promotion: PromotionSchema.nullable().optional(),
  discount_amount: z.number().int().optional(),
  error: z.string().optional(),
});
