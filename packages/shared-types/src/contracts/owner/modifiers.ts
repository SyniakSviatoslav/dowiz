import { z } from 'zod';

export const CreateModifierGroupBody = z.object({
  name: z.string().min(1).max(200),
  min_select: z.number().int().min(0).default(0).optional(),
  max_select: z.number().int().min(0).default(1).optional(),
  required: z.boolean().default(false).optional(),
}).strict();
export type CreateModifierGroupBody = z.infer<typeof CreateModifierGroupBody>;

export const UpdateModifierGroupBody = z.object({
  name: z.string().min(1).max(200).optional(),
  min_select: z.number().int().min(0).optional(),
  max_select: z.number().int().min(0).optional(),
  required: z.boolean().optional(),
}).strict();
export type UpdateModifierGroupBody = z.infer<typeof UpdateModifierGroupBody>;

export const ModifierGroupResponse = z.object({
  id: z.string().uuid(),
  name: z.string(),
  minSelect: z.number().int(),
  maxSelect: z.number().int(),
  required: z.boolean(),
  modifierCount: z.number().int(),
}).strict();
export type ModifierGroupResponse = z.infer<typeof ModifierGroupResponse>;

export const CreateModifierBody = z.object({
  name: z.string().min(1).max(200),
  price_delta: z.number().int().default(0).optional(),
  available: z.boolean().default(true).optional(),
  sort_order: z.number().int().default(0).optional(),
}).strict();
export type CreateModifierBody = z.infer<typeof CreateModifierBody>;

export const UpdateModifierBody = z.object({
  name: z.string().min(1).max(200).optional(),
  price_delta: z.number().int().optional(),
  available: z.boolean().optional(),
  sort_order: z.number().int().optional(),
}).strict();
export type UpdateModifierBody = z.infer<typeof UpdateModifierBody>;

export const ModifierResponse = z.object({
  id: z.string().uuid(),
  groupId: z.string().uuid(),
  name: z.string(),
  priceDelta: z.number().int(),
  available: z.boolean(),
  sortOrder: z.number().int(),
}).strict();
export type ModifierResponse = z.infer<typeof ModifierResponse>;
