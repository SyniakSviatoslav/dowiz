import { z } from 'zod';

export const SendOTPBody = z.object({
  phone: z.string().regex(/^\+[1-9]\d{6,14}$/),
  order_intent: z.object({
    items: z.array(z.object({
      product_id: z.string().uuid(),
      quantity: z.number().int().positive(),
    })).min(1),
    total: z.number().positive(),
    currency: z.string().length(3),
  }),
}).strict();
export type SendOTPBody = z.infer<typeof SendOTPBody>;

export const SendOTPResponse = z.object({
  otp_token: z.string(),
  expires_in_ms: z.number().int(),
}).strict();
export type SendOTPResponse = z.infer<typeof SendOTPResponse>;

export const VerifyOTPBody = z.object({
  phone: z.string().regex(/^\+[1-9]\d{6,14}$/),
  code: z.string().length(6).regex(/^\d{6}$/),
  otp_token: z.string().min(1),
  order_intent_hash: z.string().min(1),
}).strict();
export type VerifyOTPBody = z.infer<typeof VerifyOTPBody>;

export const VerifyOTPResponse = z.object({
  verified_token: z.string(),
  expires_in_ms: z.number().int(),
}).strict();
export type VerifyOTPResponse = z.infer<typeof VerifyOTPResponse>;
